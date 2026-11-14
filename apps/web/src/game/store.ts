import { create } from 'zustand'
import type { Action } from '@/generated/Action'
import type { Color } from '@/generated/Color'
import type { Event } from '@/generated/Event'
import type { PlayerView } from '@/generated/PlayerView'
import { useSettings } from '@/settings/useSettings'
import {
  AnimationQueue,
  type AnimationQueueCallbacks,
} from './AnimationQueue'
import type { GameClient, GameUpdate, LocalTurnTimer } from './GameClient'
import { vibratePenalty, vibrateTurn } from './haptics'
import { playSound } from './sound'

export interface ActiveFx {
  event: Event
  durationMs: number
  seq: number
}

export interface ChatEntry {
  from: string
  text: string
  at: number
}

export interface GameStoreState {
  client: GameClient | null
  view: PlayerView | null
  displayView: PlayerView | null
  animating: boolean
  activeFx: ActiveFx | null
  unoPopup: { seat: number; caught: boolean; penalty?: number; seq: number } | null
  skippedSeat: number | null
  colorRipple: { color: Color; seq: number } | null
  flipping: boolean
  confettiSeq: number
  selectedCardId: number | null
  nextRoundPending: boolean
  timer: LocalTurnTimer | null
  lastError: string | null
  chat: ChatEntry[]
  emotes: { seat: number; emote: string; seq: number }[]
  connect: (client: GameClient) => () => void
  disconnect: () => void
  applyUpdate: (update: GameUpdate) => void
  setSelectedCardId: (id: number | null) => void
  clearError: () => void
  sendAction: (action: Action) => void
  nextRound: () => void
}

let fxSeq = 0
let unsubscribeClient: (() => void) | null = null
let activeQueue: AnimationQueue | null = null
let fxClearTimer: ReturnType<typeof setTimeout> | null = null

function isActionInLegal(legal: readonly Action[], action: Action): boolean {
  return legal.some((candidate) => {
    if (candidate.type !== action.type) return false
    switch (candidate.type) {
      case 'play':
      case 'jump_in':
        return candidate.card === (action as { card: number }).card
      case 'choose_color':
        return candidate.color === (action as { color: Color }).color
      case 'choose_target':
      case 'catch_uno':
        return candidate.target === (action as { target: number }).target
      default:
        return true
    }
  })
}

function scheduleFxClear(ms: number, clearFn: () => void): void {
  if (fxClearTimer !== null) {
    clearTimeout(fxClearTimer)
  }
  fxClearTimer = setTimeout(() => {
    fxClearTimer = null
    clearFn()
  }, Math.max(ms, 200))
}

function handleEventFeedback(event: Event, view: PlayerView): void {
  switch (event.type) {
    case 'card_played':
      playSound('play')
      break
    case 'cards_drawn':
      playSound('draw')
      break
    case 'uno_called':
      playSound('uno')
      break
    case 'uno_caught':
      playSound('error')
      if (event.target === view.me) {
        vibratePenalty()
      }
      break
    case 'round_ended':
      if (event.winner !== null && event.winner === view.me) {
        playSound('win')
      }
      break
    case 'game_ended':
      if (event.rankings[0] === view.me) {
        playSound('win')
      }
      break
    default:
      break
  }
}

function getOrCreateQueue(): AnimationQueue {
  if (activeQueue) return activeQueue
  const callbacks: AnimationQueueCallbacks = {
    getSpeed: () => useSettings.getState().animSpeed,
    getDisplayView: () => useGameStore.getState().displayView,
    onStep: (event, displayView, durationMs) => {
      handleEventFeedback(event, displayView)
      fxSeq += 1
      const seq = fxSeq

      let unoPopup = useGameStore.getState().unoPopup
      let skippedSeat = useGameStore.getState().skippedSeat
      let colorRipple = useGameStore.getState().colorRipple
      let flipping = useGameStore.getState().flipping
      let confettiSeq = useGameStore.getState().confettiSeq

      if (event.type === 'uno_called') {
        unoPopup = { seat: event.seat, caught: false, seq }
      } else if (event.type === 'uno_caught') {
        unoPopup = { seat: event.target, caught: true, penalty: event.penalty, seq }
      } else if (event.type === 'player_skipped') {
        skippedSeat = event.seat
      } else if (event.type === 'color_chosen') {
        colorRipple = { color: event.color, seq }
      } else if (event.type === 'side_flipped') {
        flipping = true
      } else if (event.type === 'round_ended') {
        confettiSeq = seq
      }

      useGameStore.setState({
        displayView,
        animating: durationMs > 0,
        activeFx: { event, durationMs, seq },
        unoPopup,
        skippedSeat,
        colorRipple,
        flipping,
        confettiSeq,
      })

      if (durationMs > 0) {
        scheduleFxClear(durationMs, () => {
          useGameStore.setState({
            unoPopup: null,
            skippedSeat: null,
            colorRipple: null,
            flipping: false,
          })
        })
      } else {
        useGameStore.setState({
          unoPopup: null,
          skippedSeat: null,
          colorRipple: null,
          flipping: false,
        })
      }
    },
    onComplete: (finalView) => {
      const prevDecision = useGameStore.getState().displayView?.decision_player
      useGameStore.setState({
        displayView: finalView,
        activeFx: null,
      })
      if (
        finalView.me !== null &&
        finalView.decision_player === finalView.me &&
        prevDecision !== finalView.me
      ) {
        playSound('turn')
        vibrateTurn()
      }
    },
    onIdle: () => {
      useGameStore.setState({ animating: false, activeFx: null })
    },
  }
  activeQueue = new AnimationQueue(callbacks)
  return activeQueue
}

export const useGameStore = create<GameStoreState>()((set, get) => ({
  client: null,
  view: null,
  displayView: null,
  animating: false,
  activeFx: null,
  unoPopup: null,
  skippedSeat: null,
  colorRipple: null,
  flipping: false,
  confettiSeq: 0,
  selectedCardId: null,
  nextRoundPending: false,
  timer: null,
  lastError: null,
  chat: [],
  emotes: [],

  connect: (client) => {
    get().disconnect()
    const queue = getOrCreateQueue()
    queue.clear()
    set({
      client,
      view: null,
      displayView: null,
      animating: false,
      activeFx: null,
      unoPopup: null,
      skippedSeat: null,
      colorRipple: null,
      flipping: false,
      confettiSeq: 0,
      selectedCardId: null,
      nextRoundPending: false,
      timer: null,
      lastError: null,
      chat: [],
      emotes: [],
    })
    unsubscribeClient = client.subscribe((update) => {
      get().applyUpdate(update)
    })
    return () => {
      get().disconnect()
    }
  },

  disconnect: () => {
    if (unsubscribeClient) {
      unsubscribeClient()
      unsubscribeClient = null
    }
    if (activeQueue) {
      activeQueue.clear()
    }
    if (fxClearTimer !== null) {
      clearTimeout(fxClearTimer)
      fxClearTimer = null
    }
    const existing = get().client
    if (existing) {
      existing.leave()
    }
    set({
      client: null,
      view: null,
      displayView: null,
      animating: false,
      activeFx: null,
      selectedCardId: null,
      nextRoundPending: false,
      timer: null,
    })
  },

  applyUpdate: (update) => {
    const queue = getOrCreateQueue()
    switch (update.kind) {
      case 'snapshot': {
        const prevDecision = get().view?.decision_player
        queue.clear()
        set({
          view: update.view,
          displayView: update.view,
          timer: update.timer ?? null,
          animating: false,
          activeFx: null,
          selectedCardId: null,
          nextRoundPending: false,
          lastError: null,
        })
        if (
          update.view.me !== null &&
          update.view.decision_player === update.view.me &&
          prevDecision !== update.view.me
        ) {
          playSound('turn')
          vibrateTurn()
        }
        break
      }
      case 'events': {
        set({
          view: update.view,
          timer: update.timer ?? null,
          selectedCardId: null,
          nextRoundPending: update.view.phase === 'round_over' ? get().nextRoundPending : false,
          lastError: null,
        })
        if (!get().displayView) {
          set({ displayView: update.view })
        }
        queue.enqueue(update.events, update.view)
        break
      }
      case 'error': {
        playSound('error')
        set({ lastError: update.message, nextRoundPending: false })
        break
      }
      case 'chat': {
        set((s) => ({
          chat: [...s.chat.slice(-49), { from: update.from, text: update.text, at: Date.now() }],
        }))
        break
      }
      case 'emote': {
        fxSeq += 1
        const seq = fxSeq
        set((s) => ({
          emotes: [...s.emotes.slice(-7), { seat: update.seat, emote: update.emote, seq }],
        }))
        break
      }
    }
  },

  setSelectedCardId: (selectedCardId) => set({ selectedCardId }),
  clearError: () => set({ lastError: null }),

  sendAction: (action) => {
    const { client, view } = get()
    if (!client) return
    if (
      view &&
      action.type !== 'timeout' &&
      action.type !== 'end_by_time' &&
      !isActionInLegal(view.legal, action)
    ) {
      set({ selectedCardId: null })
      return
    }
    set({ selectedCardId: null, lastError: null })
    client.send(action)
  },

  nextRound: () => {
    const { client, view, nextRoundPending } = get()
    if (!client || nextRoundPending || view?.phase !== 'round_over') return
    set({ nextRoundPending: true, lastError: null })
    client.nextRound()
  },
}))
