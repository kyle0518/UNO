import type { Action } from '@/generated/Action'
import type { Event } from '@/generated/Event'
import type { PlayerView } from '@/generated/PlayerView'
import type { RuleConfig } from '@/generated/RuleConfig'
import type { AnimSpeed } from '@/settings/useSettings'
import init, {
  WasmBot,
  WasmGame,
  filter_events,
} from '@/wasm/pkg/uno_wasm'
import type { BotKind, GameUpdate, SeatKind } from './GameClient'
import { LocalGameHost, type WasmFactory } from './LocalGameHost'

export type WorkerRequest =
  | {
      type: 'init'
      config: RuleConfig
      seed: number
      seats: SeatKind[]
      playerNames?: string[]
      animSpeed?: AnimSpeed
    }
  | { type: 'action'; action: Action }
  | { type: 'next_round' }
  | { type: 'set_speed'; animSpeed: AnimSpeed }
  | { type: 'dispose' }

export type WorkerResponse =
  | GameUpdate
  | { kind: 'replay_ready'; replayJson: string }

const wasmFactory: WasmFactory = {
  createGame: (config, seed) => new WasmGame(config, BigInt(seed)),
  createBot: (kind: BotKind, seed: number) => new WasmBot(kind, BigInt(seed)),
  filterEvents: (events: Event[], seat: number | undefined) =>
    filter_events(events, seat) as Event[],
}

let wasmReady: Promise<void> | null = null
function ensureWorkerWasm(): Promise<void> {
  if (!wasmReady) {
    wasmReady = init().then(() => undefined)
  }
  return wasmReady
}

const host = new LocalGameHost({
  factory: wasmFactory,
  emit: (update) => {
    self.postMessage(update satisfies WorkerResponse)
  },
  onGameOver: (replayJson) => {
    self.postMessage({ kind: 'replay_ready', replayJson } satisfies WorkerResponse)
  },
})

self.addEventListener('message', (ev: MessageEvent<WorkerRequest>) => {
  const msg = ev.data
  void (async () => {
    try {
      await ensureWorkerWasm()
      switch (msg.type) {
        case 'init':
          host.init({
            config: msg.config,
            seed: msg.seed,
            seats: msg.seats,
            playerNames: msg.playerNames,
            animSpeed: msg.animSpeed,
          })
          break
        case 'action':
          host.handleAction(msg.action)
          break
        case 'next_round':
          host.handleNextRound()
          break
        case 'set_speed':
          host.setAnimSpeed(msg.animSpeed)
          break
        case 'dispose':
          host.dispose()
          break
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      self.postMessage({ kind: 'error', message } satisfies WorkerResponse)
    }
  })()
})

export type { PlayerView }
