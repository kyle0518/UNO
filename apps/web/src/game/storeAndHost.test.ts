import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Action } from '@/generated/Action'
import type { Event } from '@/generated/Event'
import type { PlayerView } from '@/generated/PlayerView'
import type { RuleConfig } from '@/generated/RuleConfig'
import type { GameClient, GameUpdate } from './GameClient'
import { LocalGameHost, type WasmFactory, type WasmGameLike } from './LocalGameHost'
import { useGameStore } from './store'

function baseConfig(): RuleConfig {
  return {
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
  }
}

function makeView(overrides?: Partial<PlayerView>): PlayerView {
  return {
    me: 0,
    config: baseConfig(),
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
    my_hand: [{ id: 10, light: { color: 'red', face: { k: 'number', v: 8 } }, dark: null }],
    drawn_card: null,
    players: [
      { seat: 0, hand_size: 1, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
      { seat: 1, hand_size: 7, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
      { seat: 2, hand_size: 7, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
      { seat: 3, hand_size: 7, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
    ],
    uno_window: null,
    round: 1,
    turn: 1,
    dealer: 0,
    legal: [{ type: 'play', card: 10 }],
    rankings: null,
    ...overrides,
  }
}

describe('useGameStore guards', () => {
  afterEach(() => {
    useGameStore.getState().disconnect()
  })

  it('guards nextRound against duplicate clicks and drops stale sendAction', () => {
    const subscribers = new Set<(u: GameUpdate) => void>()
    const emit = (u: GameUpdate) => {
      for (const fn of subscribers) fn(u)
    }
    const send = vi.fn()
    const nextRound = vi.fn()
    const client: GameClient = {
      mySeat: 0,
      subscribe: (fn) => {
        subscribers.add(fn)
        return () => {
          subscribers.delete(fn)
        }
      },
      send,
      nextRound,
      leave: vi.fn(),
    }

    useGameStore.getState().connect(client)

    emit({
      kind: 'snapshot',
      view: makeView({ phase: 'round_over', decision_player: null, legal: [] }),
    })

    useGameStore.getState().nextRound()
    useGameStore.getState().nextRound()
    expect(nextRound).toHaveBeenCalledTimes(1)
    expect(useGameStore.getState().nextRoundPending).toBe(true)

    useGameStore.getState().sendAction({ type: 'play', card: 10 })
    expect(send).not.toHaveBeenCalled()

    emit({
      kind: 'snapshot',
      view: makeView({ phase: 'play', round: 2, legal: [{ type: 'play', card: 10 }] }),
    })
    expect(useGameStore.getState().nextRoundPending).toBe(false)
    useGameStore.getState().nextRound()
    expect(nextRound).toHaveBeenCalledTimes(1)

    useGameStore.getState().sendAction({ type: 'play', card: 10 })
    expect(send).toHaveBeenCalledTimes(1)
  })
})

describe('LocalGameHost round transition & UNO catch behavior', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('delays round 2 bot move until round_started deal animation finishes', () => {
    const appliedActions: { seat: number; action: Action }[] = []
    let currentView = makeView({
      phase: 'round_over',
      decision_player: null,
      uno_window: null,
      legal: [],
    })

    const fakeGame: WasmGameLike = {
      set_replay_meta: vi.fn(),
      initial_events: () => [],
      view: (seat) => ({ ...currentView, me: seat }),
      decision_player: () => currentView.decision_player ?? undefined,
      legal: () => currentView.legal,
      apply_with_time: (seat, action) => {
        appliedActions.push({ seat, action })
        currentView = makeView({
          phase: 'play',
          round: 2,
          decision_player: 0,
          current: 0,
          legal: [{ type: 'draw' }],
        })
        return []
      },
      next_round: () => {
        currentView = makeView({
          phase: 'play',
          round: 2,
          decision_player: 1,
          current: 1,
          uno_window: null,
          legal: [{ type: 'draw' }],
        })
        const events: Event[] = [
          { type: 'round_started', round: 2, dealer: 1, hand_sizes: [7, 7, 7, 7] },
          { type: 'start_card_flipped', card: currentView.top_card },
          { type: 'turn_started', seat: 1 },
        ]
        return events
      },
      is_over: () => false,
      replay_json: () => '{}',
    }

    const factory: WasmFactory = {
      createGame: () => fakeGame,
      createBot: () => ({ act: () => ({ type: 'draw' }) }),
      filterEvents: (events) => events,
    }

    const host = new LocalGameHost({ factory, emit: vi.fn() })
    host.init({
      config: baseConfig(),
      seed: 1,
      seats: ['human', 'heuristic', 'random', 'random'],
      animSpeed: '1x',
    })

    host.handleNextRound()

    // Deal animation (28 * 60 + 200 = 1880ms) + 700ms bot think = 2580ms
    vi.advanceTimersByTime(1500)
    expect(appliedActions).toHaveLength(0)

    vi.advanceTimersByTime(1200)
    expect(appliedActions).toEqual([{ seat: 1, action: { type: 'draw' } }])
    host.dispose()
  })

  it('catches uncalled UNO probabilistically instead of 100% on heuristic bot turns', () => {
    let catches = 0
    let misses = 0

    for (let seed = 1; seed <= 20; seed += 1) {
      const botSeenViews: PlayerView[] = []
      const appliedActions: { seat: number; action: Action }[] = []
      let currentView = makeView({
        phase: 'play',
        decision_player: 1,
        current: 1,
        uno_window: 0,
        legal: [{ type: 'draw' }, { type: 'catch_uno', target: 0 }],
      })

      const fakeGame: WasmGameLike = {
        set_replay_meta: vi.fn(),
        initial_events: () => [],
        view: (seat) => ({ ...currentView, me: seat }),
        decision_player: () => currentView.decision_player ?? undefined,
        legal: () => currentView.legal,
        apply_with_time: (seat, action) => {
          appliedActions.push({ seat, action })
          if (action.type === 'catch_uno') {
            currentView = {
              ...currentView,
              uno_window: null,
              legal: [{ type: 'draw' }],
            }
          } else {
            currentView = {
              ...currentView,
              decision_player: 0,
              current: 0,
              uno_window: null,
              legal: [{ type: 'draw' }],
            }
          }
          return []
        },
        next_round: () => [],
        is_over: () => false,
        replay_json: () => '{}',
      }

      const factory: WasmFactory = {
        createGame: () => fakeGame,
        createBot: () => ({
          act: (view) => {
            botSeenViews.push(view)
            return view.legal[0] ?? { type: 'draw' }
          },
        }),
        filterEvents: (events) => events,
      }

      const host = new LocalGameHost({ factory, emit: vi.fn() })
      host.init({
        config: baseConfig(),
        seed,
        seats: ['human', 'heuristic', 'random', 'random'],
        animSpeed: '1x',
      })

      vi.advanceTimersByTime(2000)
      host.dispose()

      for (const v of botSeenViews) {
        expect(v.legal.some((a) => a.type === 'catch_uno')).toBe(false)
      }
      if (appliedActions.some((a) => a.action.type === 'catch_uno')) {
        catches += 1
      } else {
        misses += 1
      }
    }

    expect(catches).toBeGreaterThan(0)
    expect(misses).toBeGreaterThan(0)
  })
})
