import type { Action } from '@/generated/Action'
import type { Event } from '@/generated/Event'
import type { PlayerView } from '@/generated/PlayerView'

export type BotKind = 'random' | 'heuristic' | 'hard'
export type SeatKind = 'human' | BotKind

export interface LocalTurnTimer {
  seat: number
  deadline_ms: number
  total_ms: number
}

export type GameUpdate =
  | { kind: 'snapshot'; view: PlayerView; timer?: LocalTurnTimer | null }
  | {
      kind: 'events'
      events: Event[]
      view: PlayerView
      timer?: LocalTurnTimer | null
    }
  | { kind: 'error'; message: string }
  | { kind: 'chat'; from: string; text: string }
  | { kind: 'emote'; seat: number; emote: string }

export interface GameClient {
  readonly mySeat: number | null
  subscribe(listener: (u: GameUpdate) => void): () => void
  send(action: Action): void
  nextRound(): void
  leave(): void
}
