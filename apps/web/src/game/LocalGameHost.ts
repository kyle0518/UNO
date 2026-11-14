import type { Action } from '@/generated/Action'
import type { Event } from '@/generated/Event'
import type { PlayerView } from '@/generated/PlayerView'
import type { ReplayPlayer } from '@/generated/ReplayPlayer'
import type { RuleConfig } from '@/generated/RuleConfig'
import { animDurationMultiplier, type AnimSpeed } from '@/settings/useSettings'
import type { BotKind, GameUpdate, LocalTurnTimer, SeatKind } from './GameClient'

export interface WasmGameLike {
  set_replay_meta(players: ReplayPlayer[], created_at: string): void
  initial_events(): Event[]
  view(seat: number): PlayerView
  decision_player(): number | undefined
  legal(seat: number): Action[]
  apply_with_time(seat: number, action: Action, t_ms: bigint): Event[]
  next_round(): Event[]
  is_over(): boolean
  replay_json(): string
}

export interface WasmBotLike {
  act(view: PlayerView): Action
}

export interface WasmFactory {
  createGame(config: RuleConfig, seed: number): WasmGameLike
  createBot(kind: BotKind, seed: number): WasmBotLike
  filterEvents(events: Event[], seat: number | undefined): Event[]
}

export interface LocalGameHostOptions {
  factory: WasmFactory
  emit: (update: GameUpdate) => void
  onGameOver?: (replayJson: string) => void
  now?: () => number
}

export class LocalGameHost {
  private readonly factory: WasmFactory
  private readonly emit: (update: GameUpdate) => void
  private readonly onGameOver?: (replayJson: string) => void
  private readonly now: () => number

  private game: WasmGameLike | null = null
  private config: RuleConfig | null = null
  private bots = new Map<number, { kind: BotKind; instance: WasmBotLike }>()
  private humanSeat: number | null = 0
  private animSpeed: AnimSpeed = '1x'
  private startedAt = 0
  private botTimer: ReturnType<typeof setTimeout> | null = null
  private turnTimer: ReturnType<typeof setTimeout> | null = null
  private catchTimer: ReturnType<typeof setTimeout> | null = null
  private gameClockTimer: ReturnType<typeof setTimeout> | null = null
  private activeTimer: LocalTurnTimer | null = null
  private catchCheckedKey: string | null = null
  private prngState = 1
  private gameOverNotified = false
  private disposed = false

  constructor(options: LocalGameHostOptions) {
    this.factory = options.factory
    this.emit = options.emit
    this.onGameOver = options.onGameOver
    this.now = options.now ?? (() => Date.now())
  }

  init(params: {
    config: RuleConfig
    seed: number
    seats: SeatKind[]
    playerNames?: string[]
    animSpeed?: AnimSpeed
  }): void {
    this.clearAllTimers()
    this.bots.clear()
    this.gameOverNotified = false
    this.catchCheckedKey = null
    this.config = params.config
    this.animSpeed = params.animSpeed ?? '1x'
    this.startedAt = this.now()
    this.prngState = Math.imul((params.seed ^ 0x9e37_79b9) >>> 0, 0x85eb_ca6b) >>> 0 || 1

    const firstHuman = params.seats.indexOf('human')
    this.humanSeat = firstHuman >= 0 ? firstHuman : null

    const game = this.factory.createGame(params.config, params.seed)
    const replayPlayers: ReplayPlayer[] = params.seats.map((kind, seat) => ({
      seat,
      name:
        params.playerNames?.[seat] ??
        (kind === 'human' ? 'You' : `AI ${seat}`),
      user_id: null,
      kind: kind === 'human' ? 'human' : 'bot',
      ...(kind !== 'human' ? { bot: kind } : {}),
    }))
    game.set_replay_meta(replayPlayers, new Date(this.startedAt).toISOString())

    params.seats.forEach((kind, seat) => {
      if (kind !== 'human') {
        const botSeed = (params.seed + seat * 1009 + 17) % 9_007_199_254_740_991
        this.bots.set(seat, {
          kind,
          instance: this.factory.createBot(kind, botSeed),
        })
      }
    })

    this.game = game

    if (params.config.game_seconds > 0) {
      this.gameClockTimer = setTimeout(() => {
        this.gameClockTimer = null
        if (!this.disposed && this.game && !this.game.is_over()) {
          this.applyInternal(this.humanSeat ?? 0, { type: 'end_by_time' })
        }
      }, params.config.game_seconds * 1000)
    }

    const view = this.currentView()
    this.refreshTurnDeadline(view)
    this.emit({
      kind: 'snapshot',
      view,
      timer: this.activeTimer,
    })

    this.scheduleNext()
  }

  setAnimSpeed(speed: AnimSpeed): void {
    this.animSpeed = speed
  }

  handleAction(action: Action): void {
    if (this.disposed || !this.game || this.humanSeat === null) return
    this.applyInternal(this.humanSeat, action)
  }

  handleNextRound(): void {
    if (this.disposed || !this.game) return
    this.clearStepTimers()
    try {
      const events = this.game.next_round()
      this.afterGameStep(events, this.getRoundStartDelayMs(events))
    } catch (err) {
      this.emit({
        kind: 'error',
        message: err instanceof Error ? err.message : String(err),
      })
    }
  }

  getReplayJson(): string {
    return this.game ? this.game.replay_json() : ''
  }

  dispose(): void {
    this.disposed = true
    this.clearAllTimers()
    this.game = null
    this.bots.clear()
  }

  private applyInternal(seat: number, action: Action): void {
    if (!this.game || this.disposed) return
    this.clearStepTimers()
    const elapsedMs = Math.max(0, Math.round(this.now() - this.startedAt))
    try {
      const events = this.game.apply_with_time(seat, action, BigInt(elapsedMs))
      this.afterGameStep(events)
    } catch (err) {
      this.emit({
        kind: 'error',
        message: err instanceof Error ? err.message : String(err),
      })
      this.scheduleNext()
    }
  }

  private afterGameStep(events: Event[], extraDelayMs = 0): void {
    if (!this.game || this.disposed) return
    const filtered = this.factory.filterEvents(
      events,
      this.humanSeat ?? undefined,
    )
    const view = this.currentView()
    this.refreshTurnDeadline(view, extraDelayMs)
    this.emit({
      kind: 'events',
      events: filtered,
      view,
      timer: this.activeTimer,
    })

    if (this.game.is_over()) {
      this.clearAllTimers()
      if (!this.gameOverNotified) {
        this.gameOverNotified = true
        this.onGameOver?.(this.game.replay_json())
      }
      return
    }

    this.scheduleNext(extraDelayMs)
  }

  private getRoundStartDelayMs(events: Event[]): number {
    const mult = animDurationMultiplier(this.animSpeed)
    if (mult === 0) return 0
    let base = 0
    for (const ev of events) {
      if (ev.type === 'round_started') {
        base += ev.hand_sizes.reduce((sum, n) => sum + n, 0) * 60
      } else if (ev.type === 'start_card_flipped') {
        base += 200
      }
    }
    return Math.round(base * mult)
  }

  private currentView(): PlayerView {
    if (!this.game) {
      throw new Error('LocalGameHost not initialized')
    }
    return this.humanSeat !== null
      ? this.game.view(this.humanSeat)
      : this.game.view(0)
  }

  private refreshTurnDeadline(view: PlayerView, extraDelayMs = 0): void {
    if (
      !this.config ||
      this.config.turn_seconds <= 0 ||
      view.phase === 'round_over' ||
      view.phase === 'game_over' ||
      view.decision_player === null
    ) {
      this.activeTimer = null
      return
    }
    const totalMs = this.config.turn_seconds * 1000
    this.activeTimer = {
      seat: view.decision_player,
      deadline_ms: this.now() + totalMs + extraDelayMs,
      total_ms: totalMs,
    }
  }

  private scheduleNext(extraDelayMs = 0): void {
    if (this.disposed || !this.game || this.game.is_over()) return
    const dp = this.game.decision_player()
    if (dp === undefined || dp === null) return

    this.maybeScheduleBotCatchUno()

    if (this.bots.has(dp)) {
      if (this.catchTimer !== null) {
        return
      }
      const budgetMs =
        extraDelayMs + Math.round(700 * animDurationMultiplier(this.animSpeed))
      this.botTimer = setTimeout(() => {
        this.botTimer = null
        this.runBotStep(dp)
      }, budgetMs)
      return
    }

    if (
      this.config &&
      this.config.turn_seconds > 0 &&
      this.activeTimer &&
      dp === this.humanSeat
    ) {
      const remaining = Math.max(0, this.activeTimer.deadline_ms - this.now())
      const expectedTurn = this.currentView().turn
      this.turnTimer = setTimeout(() => {
        this.turnTimer = null
        if (!this.game || this.disposed) return
        const cur = this.currentView()
        if (cur.turn === expectedTurn && this.game.decision_player() === dp) {
          this.applyInternal(dp, { type: 'timeout' })
        }
      }, remaining)
    }
  }

  private runBotStep(seat: number): void {
    if (this.disposed || !this.game || this.game.is_over()) return
    if (this.game.decision_player() !== seat) {
      this.scheduleNext()
      return
    }
    const entry = this.bots.get(seat)
    if (!entry) return
    const botView = this.game.view(seat)
    const effectiveView =
      entry.kind === 'random'
        ? botView
        : {
            ...botView,
            legal: botView.legal.filter((a) => a.type !== 'catch_uno'),
          }
    const action = entry.instance.act(effectiveView)
    this.applyInternal(seat, action)
  }

  private maybeScheduleBotCatchUno(): void {
    if (!this.game || this.humanSeat === null) return
    const view = this.currentView()
    if (view.uno_window === null) {
      this.catchCheckedKey = null
      return
    }
    const target = view.uno_window
    const windowKey = `${view.round}:${view.turn}:${target}`
    if (this.catchCheckedKey === windowKey) return
    this.catchCheckedKey = windowKey

    for (const [botSeat, entry] of this.bots.entries()) {
      if (botSeat === target) continue
      const legal = this.game.legal(botSeat)
      const canCatch = legal.some(
        (a) => a.type === 'catch_uno' && a.target === target,
      )
      if (!canCatch) continue

      const chance =
        entry.kind === 'hard' ? 0.7 : entry.kind === 'heuristic' ? 0.3 : 0
      if (chance <= 0 || this.nextRand() >= chance) continue

      const delay =
        this.animSpeed === 'off'
          ? 0
          : Math.round((350 + this.nextRand() * 450) * animDurationMultiplier(this.animSpeed))
      this.catchTimer = setTimeout(() => {
        this.catchTimer = null
        if (this.disposed || !this.game || this.game.is_over()) return
        const stillCan = this.game
          .legal(botSeat)
          .some((a) => a.type === 'catch_uno' && a.target === target)
        if (stillCan) {
          this.applyInternal(botSeat, { type: 'catch_uno', target })
        } else {
          this.scheduleNext()
        }
      }, delay)
      break
    }
  }

  private nextRand(): number {
    this.prngState = (Math.imul(1664525, this.prngState) + 1013904223) >>> 0
    return this.prngState / 0x1_0000_0000
  }

  private clearStepTimers(): void {
    if (this.botTimer !== null) {
      clearTimeout(this.botTimer)
      this.botTimer = null
    }
    if (this.turnTimer !== null) {
      clearTimeout(this.turnTimer)
      this.turnTimer = null
    }
    if (this.catchTimer !== null) {
      clearTimeout(this.catchTimer)
      this.catchTimer = null
    }
  }

  private clearAllTimers(): void {
    this.clearStepTimers()
    if (this.gameClockTimer !== null) {
      clearTimeout(this.gameClockTimer)
      this.gameClockTimer = null
    }
  }
}
