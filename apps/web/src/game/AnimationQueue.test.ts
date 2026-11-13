import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Event } from '@/generated/Event'
import type { PlayerView } from '@/generated/PlayerView'
import type { AnimSpeed } from '@/settings/useSettings'
import { AnimationQueue, getEventDurationMs } from './AnimationQueue'

function makeView(overrides?: Partial<PlayerView>): PlayerView {
  return {
    me: 0,
    config: {
      mode_id: 'classic',
      mode: 'classic',
      num_players: 4,
      hand_size: 7,
      deck_count: 1,
      stack_draw: 'none',
      seven_zero: false,
      jump_in: false,
      draw_to_match: false,
      force_play: false,
      wild4_rule: 'challenge',
      uno_rule: 'manual',
      uno_penalty: 2,
      scoring: { type: 'points', target: 500 },
      teams: null,
      elimination: false,
      turn_seconds: 30,
      game_seconds: 0,
    },
    phase: 'play',
    decision_player: 0,
    current: 0,
    direction: 1,
    side: 'light',
    top_card: { id: 1, light: { color: 'red', face: { k: 'number', v: 5 } }, dark: null },
    active_color: 'red',
    pending_draw: 0,
    draw_pile_size: 70,
    draw_pile_top_back: null,
    discard: [{ id: 1, light: { color: 'red', face: { k: 'number', v: 5 } }, dark: null }],
    my_hand: [
      { id: 10, light: { color: 'red', face: { k: 'reverse' } }, dark: null },
      { id: 11, light: { color: null, face: { k: 'wild' } }, dark: null },
    ],
    drawn_card: null,
    players: [
      { seat: 0, hand_size: 2, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
      { seat: 1, hand_size: 7, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
      { seat: 2, hand_size: 7, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
      { seat: 3, hand_size: 7, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
    ],
    uno_window: null,
    round: 1,
    turn: 1,
    dealer: 0,
    legal: [],
    rankings: null,
    ...overrides,
  }
}

describe('AnimationQueue', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('plays events in sequence with fake timers and finishes with finalView', () => {
    let speed: AnimSpeed = '1x'
    let displayView: PlayerView = makeView()
    const stepOrder: string[] = []
    let completed = false

    const playedCard = displayView.my_hand[0]!
    const wildCard = displayView.my_hand[1]!

    const events: Event[] = [
      { type: 'card_played', seat: 0, card: playedCard, jump_in: false },
      { type: 'direction_changed', direction: -1 },
      { type: 'color_chosen', seat: 1, color: 'blue' },
      { type: 'cards_drawn', seat: 2, count: 2, cards: null, reason: 'penalty' },
    ]

    const finalView = makeView({
      top_card: wildCard,
      direction: -1,
      active_color: 'blue',
      current: 2,
      decision_player: 2,
      turn: 4,
      my_hand: [],
      players: [
        { seat: 0, hand_size: 0, hand_backs: null, uno_called: true, eliminated: false, team: null, score: 15 },
        { seat: 1, hand_size: 7, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
        { seat: 2, hand_size: 9, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
        { seat: 3, hand_size: 7, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
      ],
    })

    const queue = new AnimationQueue({
      getSpeed: () => speed,
      getDisplayView: () => displayView,
      onStep: (ev, nextView) => {
        stepOrder.push(ev.type)
        displayView = nextView
      },
      onComplete: (fv) => {
        displayView = fv
        completed = true
      },
    })

    queue.enqueue(events, finalView)

    // Step 1 started synchronously: card_played (350ms)
    expect(stepOrder).toEqual(['card_played'])
    expect(displayView.top_card.id).toBe(playedCard.id)
    expect(displayView.players[0]?.hand_size).toBe(1)
    expect(completed).toBe(false)

    // Advance 350ms -> direction_changed (500ms)
    vi.advanceTimersByTime(350)
    expect(stepOrder).toEqual(['card_played', 'direction_changed'])
    expect(displayView.direction).toBe(-1)
    expect(completed).toBe(false)

    // Advance 500ms -> color_chosen (400ms)
    vi.advanceTimersByTime(500)
    expect(stepOrder).toEqual(['card_played', 'direction_changed', 'color_chosen'])
    expect(displayView.active_color).toBe('blue')

    // Advance 400ms -> cards_drawn (2 * 120 = 240ms)
    vi.advanceTimersByTime(400)
    expect(stepOrder).toEqual([
      'card_played',
      'direction_changed',
      'color_chosen',
      'cards_drawn',
    ])
    expect(displayView.players[2]?.hand_size).toBe(9)
    expect(completed).toBe(false)

    // Advance final 240ms -> completes and guarantees displayView === finalView
    vi.advanceTimersByTime(240)
    expect(completed).toBe(true)
    expect(displayView).toBe(finalView)
    expect(queue.isPlaying).toBe(false)

    speed = 'off'
    const final2 = makeView({ round: 2, active_color: 'green' })
    queue.enqueue([{ type: 'color_chosen', seat: 0, color: 'green' }], final2)
    expect(displayView).toBe(final2)
  })

  it('scales durations by animSpeed', () => {
    expect(getEventDurationMs({ type: 'card_played', seat: 0, card: makeView().top_card, jump_in: false }, '0.5x')).toBe(700)
    expect(getEventDurationMs({ type: 'card_played', seat: 0, card: makeView().top_card, jump_in: false }, '2x')).toBe(175)
    expect(getEventDurationMs({ type: 'card_played', seat: 0, card: makeView().top_card, jump_in: false }, 'off')).toBe(0)
  })
})
