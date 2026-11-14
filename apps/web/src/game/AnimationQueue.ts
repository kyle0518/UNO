import type { Card } from '@/generated/Card'
import type { Event } from '@/generated/Event'
import type { PlayerView } from '@/generated/PlayerView'
import { animDurationMultiplier, type AnimSpeed } from '@/settings/useSettings'
import { activeCardFace } from './handSort'

export function getEventDurationMs(event: Event, speed: AnimSpeed): number {
  const mult = animDurationMultiplier(speed)
  if (mult === 0) return 0

  let base = 0
  switch (event.type) {
    case 'round_started':
      base = event.hand_sizes.reduce((sum, n) => sum + n, 0) * 60
      break
    case 'card_played':
      base = 350
      break
    case 'cards_drawn':
      base = Math.max(120, event.count * 120)
      break
    case 'direction_changed':
      base = 500
      break
    case 'color_chosen':
      base = 400
      break
    case 'player_skipped':
      base = 400
      break
    case 'uno_called':
      base = 700
      break
    case 'uno_caught':
      base = 600
      break
    case 'hands_swapped':
    case 'hands_rotated':
      base = 600
      break
    case 'side_flipped':
      base = 700
      break
    case 'round_ended':
      base = 1500
      break
    case 'challenge_resolved':
      base = 400
      break
    case 'player_eliminated':
      base = 500
      break
    case 'cards_discarded':
      base = 300
      break
    case 'deck_reshuffled':
    case 'start_card_flipped':
      base = 200
      break
    case 'turn_started':
    case 'pending_draw_changed':
    case 'game_ended':
      base = 0
      break
  }

  return Math.round(base * mult)
}

function updatePlayerAt(
  players: PlayerView['players'],
  seat: number,
  patch: Partial<PlayerView['players'][number]>,
): PlayerView['players'] {
  return players.map((p) => (p.seat === seat ? { ...p, ...patch } : p))
}

function removeCardsFromHand(hand: readonly Card[], ids: Set<number>): Card[] {
  return hand.filter((c) => !ids.has(c.id))
}

export function applyEventToDisplayView(view: PlayerView, event: Event): PlayerView {
  switch (event.type) {
    case 'round_started': {
      const players = view.players.map((p, idx) => ({
        ...p,
        hand_size: event.hand_sizes[idx] ?? p.hand_size,
        uno_called: false,
      }))
      return {
        ...view,
        phase: 'play',
        decision_player: null,
        uno_window: null,
        drawn_card: null,
        discard: [],
        my_hand: [],
        legal: [],
        round: event.round,
        dealer: event.dealer,
        side: 'light',
        direction: 1,
        pending_draw: 0,
        players,
      }
    }
    case 'start_card_flipped': {
      const face = activeCardFace(event.card, view.side)
      return {
        ...view,
        top_card: event.card,
        discard: [...view.discard, event.card],
        active_color: face.color ?? view.active_color,
        legal: [],
      }
    }
    case 'turn_started':
      return { ...view, current: event.seat, legal: [] }
    case 'card_played': {
      const face = activeCardFace(event.card, view.side)
      const targetPlayer = view.players.find((p) => p.seat === event.seat)
      const nextSize = Math.max(0, (targetPlayer?.hand_size ?? 1) - 1)
      const my_hand =
        view.me === event.seat
          ? removeCardsFromHand(view.my_hand, new Set([event.card.id]))
          : view.my_hand
      return {
        ...view,
        top_card: event.card,
        discard: [...view.discard, event.card],
        active_color: face.color ?? view.active_color,
        drawn_card: null,
        legal: [],
        players: updatePlayerAt(view.players, event.seat, { hand_size: nextSize }),
        my_hand,
      }
    }
    case 'cards_discarded': {
      if (event.cards.length === 0) return view
      const lastCard = event.cards[event.cards.length - 1]
      const targetPlayer = view.players.find((p) => p.seat === event.seat)
      const nextSize = Math.max(0, (targetPlayer?.hand_size ?? 0) - event.cards.length)
      const removeIds = new Set(event.cards.map((c) => c.id))
      const my_hand =
        view.me === event.seat
          ? removeCardsFromHand(view.my_hand, removeIds)
          : view.my_hand
      const lastFace = lastCard ? activeCardFace(lastCard, view.side) : null
      return {
        ...view,
        top_card: lastCard ?? view.top_card,
        discard: [...view.discard, ...event.cards],
        active_color: lastFace?.color ?? view.active_color,
        legal: [],
        players: updatePlayerAt(view.players, event.seat, { hand_size: nextSize }),
        my_hand,
      }
    }
    case 'cards_drawn': {
      const targetPlayer = view.players.find((p) => p.seat === event.seat)
      const nextSize = (targetPlayer?.hand_size ?? 0) + event.count
      const my_hand =
        view.me === event.seat && event.cards
          ? [...view.my_hand, ...event.cards]
          : view.my_hand
      return {
        ...view,
        draw_pile_size: Math.max(0, view.draw_pile_size - event.count),
        legal: [],
        players: updatePlayerAt(view.players, event.seat, {
          hand_size: nextSize,
          uno_called: nextSize >= 2 ? false : (targetPlayer?.uno_called ?? false),
        }),
        my_hand,
      }
    }
    case 'deck_reshuffled':
      return {
        ...view,
        draw_pile_size: event.size,
      }
    case 'color_chosen':
      return {
        ...view,
        active_color: event.color,
        legal: [],
      }
    case 'direction_changed':
      return {
        ...view,
        direction: event.direction,
      }
    case 'pending_draw_changed':
      return {
        ...view,
        pending_draw: event.amount,
      }
    case 'uno_called':
      return {
        ...view,
        uno_window: view.uno_window === event.seat ? null : view.uno_window,
        legal: view.legal.filter(
          (a) =>
            !(a.type === 'call_uno' && view.me === event.seat) &&
            !(a.type === 'catch_uno' && a.target === event.seat),
        ),
        players: updatePlayerAt(view.players, event.seat, { uno_called: true }),
      }
    case 'uno_caught':
      return {
        ...view,
        uno_window: view.uno_window === event.target ? null : view.uno_window,
        legal: view.legal.filter(
          (a) =>
            !(a.type === 'call_uno' && view.me === event.target) &&
            !(a.type === 'catch_uno' && a.target === event.target),
        ),
        players: updatePlayerAt(view.players, event.target, { uno_called: false }),
      }
    case 'side_flipped':
      return {
        ...view,
        side: event.side,
        active_color: activeCardFace(view.top_card, event.side).color ?? null,
        legal: [],
      }
    case 'player_eliminated':
      return {
        ...view,
        players: updatePlayerAt(view.players, event.seat, { eliminated: true }),
      }
    case 'round_ended':
      return {
        ...view,
        phase: 'round_over',
        decision_player: null,
        uno_window: null,
        drawn_card: null,
        legal: [],
        players: view.players.map((p, idx) => ({
          ...p,
          score: event.totals[idx] ?? p.score,
        })),
      }
    case 'game_ended':
      return {
        ...view,
        phase: 'game_over',
        decision_player: null,
        uno_window: null,
        drawn_card: null,
        legal: [],
        rankings: event.rankings,
      }
    case 'player_skipped':
    case 'challenge_resolved':
    case 'hands_swapped':
    case 'hands_rotated':
      return { ...view, legal: [] }
  }
}

export interface AnimationBatch {
  events: Event[]
  finalView: PlayerView
}

export interface AnimationQueueCallbacks {
  getSpeed: () => AnimSpeed
  getDisplayView: () => PlayerView | null
  onStep: (event: Event, displayView: PlayerView, durationMs: number) => void
  onComplete: (finalView: PlayerView) => void
  onIdle?: () => void
}

export class AnimationQueue {
  private readonly queue: AnimationBatch[] = []
  private activeBatch: AnimationBatch | null = null
  private stepIndex = 0
  private timer: ReturnType<typeof setTimeout> | null = null
  private readonly callbacks: AnimationQueueCallbacks

  constructor(callbacks: AnimationQueueCallbacks) {
    this.callbacks = callbacks
  }

  get isPlaying(): boolean {
    return this.activeBatch !== null || this.queue.length > 0
  }

  enqueue(events: Event[], finalView: PlayerView): void {
    const speed = this.callbacks.getSpeed()
    if (speed === 'off' || events.length === 0) {
      if (this.activeBatch === null && this.queue.length === 0) {
        for (const ev of events) {
          this.callbacks.onStep(ev, finalView, 0)
        }
        this.callbacks.onComplete(finalView)
        this.callbacks.onIdle?.()
        return
      }
    }

    this.queue.push({ events, finalView })
    if (!this.activeBatch) {
      this.startNextBatch()
    }
  }

  flush(): void {
    if (this.timer !== null) {
      clearTimeout(this.timer)
      this.timer = null
    }
    const all = [
      ...(this.activeBatch ? [this.activeBatch] : []),
      ...this.queue.splice(0),
    ]
    this.activeBatch = null
    this.stepIndex = 0
    const last = all[all.length - 1]
    if (last) {
      this.callbacks.onComplete(last.finalView)
    }
    this.callbacks.onIdle?.()
  }

  clear(): void {
    if (this.timer !== null) {
      clearTimeout(this.timer)
      this.timer = null
    }
    this.queue.length = 0
    this.activeBatch = null
    this.stepIndex = 0
  }

  private startNextBatch(): void {
    const next = this.queue.shift()
    if (!next) {
      this.activeBatch = null
      this.stepIndex = 0
      this.callbacks.onIdle?.()
      return
    }
    this.activeBatch = next
    this.stepIndex = 0
    this.pump()
  }

  private pump(): void {
    while (this.activeBatch) {
      const batch = this.activeBatch
      if (this.stepIndex >= batch.events.length) {
        this.callbacks.onComplete(batch.finalView)
        this.activeBatch = null
        this.stepIndex = 0
        if (this.queue.length > 0) {
          this.startNextBatch()
          return
        }
        this.callbacks.onIdle?.()
        return
      }

      const ev = batch.events[this.stepIndex]
      this.stepIndex += 1
      if (!ev) continue

      const currentView = this.callbacks.getDisplayView() ?? batch.finalView
      const nextDisplayView = applyEventToDisplayView(currentView, ev)
      const durationMs = getEventDurationMs(ev, this.callbacks.getSpeed())
      this.callbacks.onStep(ev, nextDisplayView, durationMs)

      if (durationMs > 0) {
        this.timer = setTimeout(() => {
          this.timer = null
          this.pump()
        }, durationMs)
        return
      }
    }
  }
}
