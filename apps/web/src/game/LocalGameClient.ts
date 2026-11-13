import type { Action } from '@/generated/Action'
import type { RuleConfig } from '@/generated/RuleConfig'
import { useSettings } from '@/settings/useSettings'
import type { GameClient, GameUpdate, SeatKind } from './GameClient'
import type { WorkerRequest, WorkerResponse } from './game.worker'

export interface LocalGameClientParams {
  config: RuleConfig
  seed: number
  seats: SeatKind[]
  playerNames?: string[]
  onReplayReady?: (replayJson: string) => void
}

export class LocalGameClient implements GameClient {
  readonly mySeat: number | null
  private readonly worker: Worker
  private readonly listeners = new Set<(u: GameUpdate) => void>()
  private readonly unsubSettings: () => void
  private readonly onReplayReady?: (replayJson: string) => void
  private replayJson: string | null = null
  private disposed = false

  constructor(params: LocalGameClientParams) {
    const firstHuman = params.seats.indexOf('human')
    this.mySeat = firstHuman >= 0 ? firstHuman : null
    this.onReplayReady = params.onReplayReady

    this.worker = new Worker(new URL('./game.worker.ts', import.meta.url), {
      type: 'module',
    })

    this.worker.addEventListener('message', (ev: MessageEvent<WorkerResponse>) => {
      const msg = ev.data
      if (msg.kind === 'replay_ready') {
        this.replayJson = msg.replayJson
        this.onReplayReady?.(msg.replayJson)
        return
      }
      for (const listener of this.listeners) {
        listener(msg)
      }
    })

    this.worker.addEventListener('error', (err) => {
      for (const listener of this.listeners) {
        listener({ kind: 'error', message: err.message || 'Worker error' })
      }
    })

    let lastSpeed = useSettings.getState().animSpeed
    this.unsubSettings = useSettings.subscribe((state) => {
      if (state.animSpeed !== lastSpeed) {
        lastSpeed = state.animSpeed
        this.post({ type: 'set_speed', animSpeed: lastSpeed })
      }
    })

    this.post({
      type: 'init',
      config: params.config,
      seed: params.seed,
      seats: params.seats,
      playerNames: params.playerNames,
      animSpeed: lastSpeed,
    })
  }

  getReplayJson(): string | null {
    return this.replayJson
  }

  subscribe(listener: (u: GameUpdate) => void): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  send(action: Action): void {
    this.post({ type: 'action', action })
  }

  nextRound(): void {
    this.post({ type: 'next_round' })
  }

  leave(): void {
    if (this.disposed) return
    this.disposed = true
    this.unsubSettings()
    this.listeners.clear()
    this.post({ type: 'dispose' })
    this.worker.terminate()
  }

  private post(req: WorkerRequest): void {
    if (this.disposed) return
    this.worker.postMessage(req)
  }
}
