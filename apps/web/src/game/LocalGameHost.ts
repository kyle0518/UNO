import type { Action } from '@/generated/Action'
import type { Event } from '@/generated/Event'
import type { PlayerView } from '@/generated/PlayerView'
import type { ReplayPlayer } from '@/generated/ReplayPlayer'
import type { RuleConfig } from '@/generated/RuleConfig'
import { animDurationMultiplier, type AnimSpeed } from '@/settings/useSettings'
import { getEventDurationMs } from './AnimationQueue'
import type {
  BotKind,
  GameUpdate,
  LocalGameClock,
  LocalTurnTimer,
  SeatKind,
} from './GameClient'

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

const START_FX_TYPES = new Set<Event['type']>([
  'player_skipped',
  'direction_changed',
  'color_chosen',
  'side_flipped',
])

function computePrngSeed(seed: number): number {
  return Math.imul((seed ^ 0x9e37_79b9) >>> 0, 0x85eb_ca6b) >>> 0 || 1
}

function buildReplayPlayers(
  seats: SeatKind[],
  playerNames?: string[],
): ReplayPlayer[] {
  return seats.map((kind, seat) => ({
    seat,
    name: playerNames?.[seat] ?? (kind === 'human' ? 'You' : `AI ${seat}`),
    user_id: null,
    kind: kind === 'human' ? 'human' : 'bot',
    ...(kind !== 'human' ? { bot: kind } : {}),
  }))
}

function extractStartEffectEvents(events: readonly Event[]): Event[] {
  return events.filter((ev) => START_FX_TYPES.has(ev.type))
}

function computeRoundStartDelayMs(
  events: readonly Event[],
  animSpeed: AnimSpeed,
): number {
  let total = 0
  for (const ev of events) {
    if (ev.type === 'round_ended') continue
    total += getEventDurationMs(ev, animSpeed)
  }
  return total
}

function buildTurnTimer(
  view: PlayerView,
  config: RuleConfig | null,
  nowMs: number,
  extraDelayMs: number,
  paused: boolean,
): LocalTurnTimer | null {
  if (
    !config ||
    config.turn_seconds <= 0 ||
    view.phase === 'round_over' ||
    view.phase === 'game_over' ||
    view.decision_player === null
  ) {
    return null
  }
  const totalMs = config.turn_seconds * 1000
  const remaining = totalMs + extraDelayMs
  return {
    seat: view.decision_player,
    deadline_ms: nowMs + remaining,
    total_ms: totalMs,
    ...(paused ? { paused_remaining_ms: remaining } : {}),
  }
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
  private gameClockDeadlineMs: number | null = null
  private gameClockRemainingMs: number | null = null
  private paused = false
  private pausedAt: number | null = null
  private pendingStartFxEvents: Event[] = []
  private pendingStartExtraDelayMs = 0
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
    paused?: boolean
  }): void {
    this.clearAllTimers()
    this.bots.clear()
    this.gameOverNotified = false
    this.catchCheckedKey = null
    this.config = params.config
    this.animSpeed = params.animSpeed ?? '1x'
    this.startedAt = this.now()
    this.paused = Boolean(params.paused)
    this.pausedAt = this.paused ? this.startedAt : null
    this.prngState = computePrngSeed(params.seed)

    const firstHuman = params.seats.indexOf('human')
    this.humanSeat = firstHuman >= 0 ? firstHuman : null

    const game = this.factory.createGame(params.config, params.seed)
    const replayPlayers = buildReplayPlayers(params.seats, params.playerNames)
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
    this.initGameClock(params.config.game_seconds)

    const startFx = this.factory.filterEvents(
      extractStartEffectEvents(game.initial_events()),
      this.humanSeat ?? undefined,
    )
    const extraDelayMs = computeRoundStartDelayMs(startFx, this.animSpeed)

    const view = this.currentView()
    this.refreshTurnDeadline(view, extraDelayMs)
    this.emit({
      kind: 'snapshot',
      view,
      timer: this.activeTimer,
      gameTimer: this.currentGameTimer(),
    })

    if (this.paused) {
      this.pendingStartFxEvents = startFx
      this.pendingStartExtraDelayMs = extraDelayMs
      return
    }

    this.pendingStartFxEvents = []
    this.pendingStartExtraDelayMs = 0
    if (startFx.length > 0) {
      this.emit({
        kind: 'events',
        events: startFx,
        view,
        timer: this.activeTimer,
        gameTimer: this.currentGameTimer(),
      })
    }
    this.scheduleNext(extraDelayMs)
  }

  setAnimSpeed(speed: AnimSpeed): void {
    this.animSpeed = speed
  }

  pause(): void {
    if (this.disposed || !this.game || this.paused || this.game.is_over()) return
    const nowMs = this.now()
    this.paused = true
    this.pausedAt = nowMs

    if (this.activeTimer) {
      const remaining = Math.max(0, this.activeTimer.deadline_ms - nowMs)
      this.activeTimer = {
        ...this.activeTimer,
        deadline_ms: nowMs + remaining,
        paused_remaining_ms: remaining,
      }
    }
    if (this.gameClockDeadlineMs !== null) {
      this.gameClockRemainingMs = Math.max(0, this.gameClockDeadlineMs - nowMs)
      this.gameClockDeadlineMs = null
    }
    if (this.catchTimer !== null) {
      this.catchCheckedKey = null
    }

    this.clearAllTimers()
    this.emit({
      kind: 'timers',
      timer: this.activeTimer,
      gameTimer: this.currentGameTimer(),
    })
  }

  resume(): void {
    if (this.disposed || !this.game || !this.paused || this.game.is_over()) return
    const nowMs = this.now()
    this.startedAt += Math.max(0, nowMs - (this.pausedAt ?? nowMs))
    this.paused = false
    this.pausedAt = null

    if (this.activeTimer) {
      const remaining =
        this.activeTimer.paused_remaining_ms ??
        Math.max(0, this.activeTimer.deadline_ms - nowMs)
      this.activeTimer = {
        seat: this.activeTimer.seat,
        deadline_ms: nowMs + remaining,
        total_ms: this.activeTimer.total_ms,
      }
    }

    const startFx = this.pendingStartFxEvents
    const extraDelay = this.pendingStartExtraDelayMs
    this.pendingStartFxEvents = []
    this.pendingStartExtraDelayMs = 0

    if (this.config && this.config.game_seconds > 0) {
      const remainingGameMs = this.gameClockRemainingMs ?? 0
      if (remainingGameMs <= 0) {
        this.applyInternal(this.humanSeat ?? 0, { type: 'end_by_time' })
        return
      }
      this.armGameClockTimer(remainingGameMs)
    }

    this.emit({
      kind: 'timers',
      timer: this.activeTimer,
      gameTimer: this.currentGameTimer(),
    })

    if (startFx.length > 0) {
      this.emit({
        kind: 'events',
        events: startFx,
        view: this.currentView(),
        timer: this.activeTimer,
        gameTimer: this.currentGameTimer(),
      })
    }

    this.scheduleNext(extraDelay)
  }

  handleAction(action: Action): void {
    if (this.disposed || !this.game || this.paused || this.humanSeat === null) return
    this.applyInternal(this.humanSeat, action)
  }

  handleNextRound(): void {
    if (this.disposed || !this.game) return
    this.clearStepTimers()
    try {
      const events = this.game.next_round()
      const delayMs = computeRoundStartDelayMs(events, this.animSpeed)
      this.afterGameStep(events, delayMs)
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

  private initGameClock(gameSeconds: number): void {
    this.gameClockDeadlineMs = null
    this.gameClockRemainingMs = null
    if (gameSeconds <= 0) return

    const totalMs = gameSeconds * 1000
    if (this.paused) {
      this.gameClockRemainingMs = totalMs
      return
    }
    this.armGameClockTimer(totalMs)
  }

  private armGameClockTimer(remainingMs: number): void {
    if (this.gameClockTimer !== null) {
      clearTimeout(this.gameClockTimer)
    }
    this.gameClockRemainingMs = remainingMs
    this.gameClockDeadlineMs = this.now() + remainingMs
    this.gameClockTimer = setTimeout(() => {
      this.gameClockTimer = null
      this.gameClockDeadlineMs = null
      this.gameClockRemainingMs = 0
      if (!this.disposed && this.game && !this.game.is_over()) {
        this.applyInternal(this.humanSeat ?? 0, { type: 'end_by_time' })
      }
    }, remainingMs)
  }

  private currentGameTimer(): LocalGameClock | null {
    if (!this.config || this.config.game_seconds <= 0) return null
    const totalMs = this.config.game_seconds * 1000
    if (this.paused) {
      const remaining = this.gameClockRemainingMs ?? totalMs
      return {
        deadline_ms: this.now() + remaining,
        total_ms: totalMs,
        paused_remaining_ms: remaining,
      }
    }
    if (this.gameClockDeadlineMs === null) return null
    return {
      deadline_ms: this.gameClockDeadlineMs,
      total_ms: totalMs,
    }
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
      gameTimer: this.currentGameTimer(),
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

  private currentView(): PlayerView {
    if (!this.game) {
      throw new Error('LocalGameHost not initialized')
    }
    return this.humanSeat !== null
      ? this.game.view(this.humanSeat)
      : this.game.view(0)
  }

  private refreshTurnDeadline(view: PlayerView, extraDelayMs = 0): void {
    this.activeTimer = buildTurnTimer(
      view,
      this.config,
      this.now(),
      extraDelayMs,
      this.paused,
    )
  }

  private scheduleNext(extraDelayMs = 0): void {
    if (this.disposed || !this.game || this.paused || this.game.is_over()) return
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
        if (!this.game || this.disposed || this.paused) return
        const cur = this.currentView()
        if (cur.turn === expectedTurn && this.game.decision_player() === dp) {
          this.applyInternal(dp, { type: 'timeout' })
        }
      }, remaining)
    }
  }

  private runBotStep(seat: number): void {
    if (this.disposed || !this.game || this.paused || this.game.is_over()) return
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
        entry.kind === 'hell' ? 0.9 : entry.kind === 'hard' ? 0.7 : entry.kind === 'heuristic' ? 0.3 : 0
      if (chance <= 0 || this.nextRand() >= chance) continue

      const delay =
        this.animSpeed === 'off'
          ? 0
          : Math.round((550 + this.nextRand() * 450) * animDurationMultiplier(this.animSpeed))
      this.catchTimer = setTimeout(() => {
        this.catchTimer = null
        if (this.disposed || !this.game || this.paused || this.game.is_over()) return
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
