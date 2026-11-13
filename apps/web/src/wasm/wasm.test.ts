/// <reference types="node" />
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import type { Action } from '@/generated/Action'
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
} from './pkg/uno_wasm'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const wasmBytes = readFileSync(path.join(__dirname, 'pkg/uno_wasm_bg.wasm'))
initSync({ module: wasmBytes })

describe('uno-wasm integration', () => {
  it('runs a full 4P classic game with WasmBot variants and validates replay', () => {
    const cfg = preset('classic', 4) as RuleConfig
    const game = new WasmGame(cfg, 42n)
    const bots = [
      new WasmBot('random', 101n),
      new WasmBot('heuristic', 102n),
      new WasmBot('hard', 103n),
      new WasmBot('heuristic', 104n),
    ]

    const obs = game.observe(0)
    expect(obs.length).toBe(295)
    const mask = game.action_mask(0)
    expect(mask.length).toBe(81)

    const initial = game.initial_events() as Event[]
    const filteredInitial = filter_events(initial, 0) as Event[]
    expect(filteredInitial.length).toBe(initial.length)

    let steps = 0
    while (!game.is_over() && steps < 5000) {
      const dp = game.decision_player()
      if (dp !== undefined) {
        const view = game.view(dp) as PlayerView
        const action = bots[dp]!.act(view) as Action
        game.apply(dp, action)
      } else {
        game.next_round()
      }
      steps += 1
    }

    expect(game.is_over()).toBe(true)
    const finalView = game.view(0) as PlayerView
    expect(finalView.rankings).toHaveLength(4)

    const replayJson = game.replay_json()
    const rankings = replay_validate(replayJson) as number[] | null
    expect(rankings).toEqual(finalView.rankings)
  })
})
