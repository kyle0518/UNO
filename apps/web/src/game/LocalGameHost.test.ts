/// <reference types="node" />
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import type { Event } from '@/generated/Event'
import type { PlayerView } from '@/generated/PlayerView'
import type { RuleConfig } from '@/generated/RuleConfig'
import {
  WasmBot,
  WasmGame,
  filter_events,
  initSync,
  preset,
  replay_validate,
} from '@/wasm/pkg/uno_wasm'
import type { BotKind, GameUpdate } from './GameClient'
import { LocalGameHost, type WasmFactory } from './LocalGameHost'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const wasmBytes = readFileSync(
  path.join(__dirname, '../wasm/pkg/uno_wasm_bg.wasm'),
)
initSync({ module: wasmBytes })

const testFactory: WasmFactory = {
  createGame: (config, seed) => new WasmGame(config, BigInt(seed)),
  createBot: (kind: BotKind, seed: number) => new WasmBot(kind, BigInt(seed)),
  filterEvents: (events: Event[], seat: number | undefined) =>
    filter_events(events, seat) as Event[],
}

describe('LocalGameHost', () => {
  it('completes a full 4P game when human always picks first legal action', async () => {
    const cfg = preset('classic', 4) as RuleConfig

    const result = await new Promise<{ finalView: PlayerView; replayJson: string }>(
      (resolve, reject) => {
        let finalReplay = ''
        let lastView: PlayerView | null = null
        let actionsTaken = 0

        const host = new LocalGameHost({
          factory: testFactory,
          onGameOver: (replayJson) => {
            finalReplay = replayJson
            if (lastView) {
              host.dispose()
              resolve({ finalView: lastView, replayJson: finalReplay })
            }
          },
          emit: (u: GameUpdate) => {
            if (u.kind === 'error') {
              host.dispose()
              reject(new Error(u.message))
              return
            }
            if (u.kind === 'snapshot' || u.kind === 'events') {
              lastView = u.view
              if (u.view.phase === 'game_over') {
                if (finalReplay) {
                  host.dispose()
                  resolve({ finalView: u.view, replayJson: finalReplay })
                }
                return
              }
              if (u.view.phase === 'round_over') {
                queueMicrotask(() => host.handleNextRound())
                return
              }
              if (u.view.decision_player === 0 && u.view.legal[0]) {
                actionsTaken += 1
                if (actionsTaken > 3000) {
                  host.dispose()
                  reject(new Error('Match exceeded 3000 human actions'))
                  return
                }
                const firstLegal = u.view.legal[0]
                queueMicrotask(() => host.handleAction(firstLegal))
              }
            }
          },
        })

        host.init({
          config: cfg,
          seed: 2026,
          seats: ['human', 'random', 'heuristic', 'random'],
          animSpeed: 'off',
        })
      },
    )

    expect(result.finalView.phase).toBe('game_over')
    expect(result.finalView.rankings).toHaveLength(4)
    const validated = replay_validate(result.replayJson) as number[]
    expect(validated).toEqual(result.finalView.rankings)
  })
})
