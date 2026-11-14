import { fireEvent, render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { describe, expect, it, vi } from 'vitest'
import type { PlayerView } from '@/generated/PlayerView'
import { GameTable } from './GameTable'

function sampleView(): PlayerView {
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
    pending_draw: 2,
    draw_pile_size: 62,
    draw_pile_top_back: null,
    discard: [{ id: 1, light: { color: 'red', face: { k: 'number', v: 5 } }, dark: null }],
    my_hand: [
      { id: 10, light: { color: 'red', face: { k: 'number', v: 8 } }, dark: null },
      { id: 11, light: { color: 'blue', face: { k: 'number', v: 2 } }, dark: null },
    ],
    drawn_card: null,
    players: [
      { seat: 0, hand_size: 2, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
      { seat: 1, hand_size: 5, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
      { seat: 2, hand_size: 1, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 0 },
      { seat: 3, hand_size: 6, hand_backs: null, uno_called: true, eliminated: false, team: null, score: 0 },
    ],
    uno_window: 2,
    round: 1,
    turn: 5,
    dealer: 0,
    legal: [
      { type: 'play', card: 10 },
      { type: 'draw' },
      { type: 'call_uno' },
      { type: 'catch_uno', target: 2 },
    ],
    rankings: null,
  }
}

describe('GameTable', () => {
  it('renders opponents, draw pile, pending draw, UNO & Catch buttons, and emits actions', () => {
    const onSendAction = vi.fn()
    render(
      <MemoryRouter>
        <GameTable
          view={sampleView()}
          playerNames={['You', 'AI 1', 'AI 2', 'AI 3']}
          selectedCardId={null}
          timer={null}
          lastError={null}
          activeFx={null}
          unoPopup={null}
          skippedSeat={null}
          colorRipple={null}
          flipping={false}
          confettiSeq={0}
          onSelectCard={vi.fn()}
          onSendAction={onSendAction}
          onNextRound={vi.fn()}
          onLeaveRequest={vi.fn()}
        />
      </MemoryRouter>,
    )

    expect(screen.getByTestId('pending-draw-badge')).toHaveTextContent('+2')
    expect(screen.getByText('AI 1')).toBeInTheDocument()
    expect(screen.getByText('AI 2')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: /紅色 8|Red 8/i }))
    expect(onSendAction).toHaveBeenCalledWith({ type: 'play', card: 10 })

    fireEvent.click(screen.getByTestId('draw-pile'))
    expect(onSendAction).toHaveBeenCalledWith({ type: 'draw' })

    fireEvent.click(screen.getByTestId('uno-button'))
    expect(onSendAction).toHaveBeenCalledWith({ type: 'call_uno' })

    fireEvent.click(screen.getByRole('button', { name: /抓!|Catch!/i }))
    expect(onSendAction).toHaveBeenCalledWith({ type: 'catch_uno', target: 2 })
  })

  it('deselects card when clicking empty table background and shows team / eliminated notices', () => {
    const view = sampleView()
    view.decision_player = 1
    view.players[0] = { ...view.players[0]!, team: 0, eliminated: true }
    view.players[1] = { ...view.players[1]!, team: 1 }
    const onSelectCard = vi.fn()

    const { container } = render(
      <MemoryRouter>
        <GameTable
          view={view}
          playerNames={['You', 'AI 1', 'AI 2', 'AI 3']}
          selectedCardId={10}
          timer={null}
          lastError={null}
          activeFx={null}
          unoPopup={null}
          skippedSeat={null}
          colorRipple={null}
          flipping={false}
          confettiSeq={0}
          onSelectCard={onSelectCard}
          onSendAction={vi.fn()}
          onNextRound={vi.fn()}
          onLeaveRequest={vi.fn()}
        />
      </MemoryRouter>,
    )

    expect(screen.getByText(/第 1 隊|Team 1/i)).toBeInTheDocument()
    expect(screen.getByRole('status')).toHaveTextContent(/淘汰|eliminated/i)
    const main = container.querySelector('main')
    expect(main).not.toBeNull()
    fireEvent.click(main!)
    expect(onSelectCard).toHaveBeenCalledWith(null)
  })
  it('supports touch two-tap select-then-play and single-gesture swipe-up play', () => {
    const onSelectCard = vi.fn()
    const onSendAction = vi.fn()
    const view = sampleView()

    const { rerender } = render(
      <MemoryRouter>
        <GameTable
          view={view}
          playerNames={['You', 'AI 1', 'AI 2', 'AI 3']}
          selectedCardId={null}
          timer={null}
          lastError={null}
          activeFx={null}
          unoPopup={null}
          skippedSeat={null}
          colorRipple={null}
          flipping={false}
          confettiSeq={0}
          onSelectCard={onSelectCard}
          onSendAction={onSendAction}
          onNextRound={vi.fn()}
          onLeaveRequest={vi.fn()}
        />
      </MemoryRouter>,
    )

    const cardBtn = screen.getByRole('button', { name: /紅色 8|Red 8/i })
    const cardWrap = cardBtn.closest('[data-card-id="10"]')!

    // Touch tap 1: selects card
    fireEvent.pointerDown(cardWrap, { pointerId: 1, pointerType: 'touch', clientX: 100, clientY: 200 })
    fireEvent.pointerUp(cardWrap, { pointerId: 1, pointerType: 'touch', clientX: 100, clientY: 200 })
    fireEvent.click(cardBtn)
    expect(onSelectCard).toHaveBeenCalledWith(10)
    expect(onSendAction).not.toHaveBeenCalled()

    // Touch tap 2 (when selected): plays card
    rerender(
      <MemoryRouter>
        <GameTable
          view={view}
          playerNames={['You', 'AI 1', 'AI 2', 'AI 3']}
          selectedCardId={10}
          timer={null}
          lastError={null}
          activeFx={null}
          unoPopup={null}
          skippedSeat={null}
          colorRipple={null}
          flipping={false}
          confettiSeq={0}
          onSelectCard={onSelectCard}
          onSendAction={onSendAction}
          onNextRound={vi.fn()}
          onLeaveRequest={vi.fn()}
        />
      </MemoryRouter>,
    )
    fireEvent.pointerDown(cardWrap, { pointerId: 2, pointerType: 'touch', clientX: 100, clientY: 200 })
    fireEvent.pointerUp(cardWrap, { pointerId: 2, pointerType: 'touch', clientX: 100, clientY: 200 })
    fireEvent.click(cardBtn)
    expect(onSendAction).toHaveBeenCalledTimes(1)
    expect(onSendAction).toHaveBeenCalledWith({ type: 'play', card: 10 })

    // Touch swipe-up without prior selection: plays once and ignores trailing click
    onSendAction.mockClear()
    fireEvent.pointerDown(cardWrap, { pointerId: 3, pointerType: 'touch', clientX: 100, clientY: 200 })
    fireEvent.pointerMove(cardWrap, { pointerId: 3, pointerType: 'touch', clientX: 102, clientY: 185 })
    fireEvent.pointerMove(cardWrap, { pointerId: 3, pointerType: 'touch', clientX: 102, clientY: 150 })
    fireEvent.pointerUp(cardWrap, { pointerId: 3, pointerType: 'touch', clientX: 102, clientY: 150 })
    fireEvent.click(cardBtn)
    expect(onSendAction).toHaveBeenCalledTimes(1)
    expect(onSendAction).toHaveBeenCalledWith({ type: 'play', card: 10 })
  })

  it('disables Next Round button while nextRoundPending is true', () => {
    const view = sampleView()
    view.phase = 'round_over'
    view.decision_player = null
    view.legal = []
    const onNextRound = vi.fn()

    render(
      <MemoryRouter>
        <GameTable
          view={view}
          playerNames={['You', 'AI 1', 'AI 2', 'AI 3']}
          selectedCardId={null}
          timer={null}
          lastError={null}
          activeFx={null}
          unoPopup={null}
          skippedSeat={null}
          colorRipple={null}
          flipping={false}
          confettiSeq={0}
          nextRoundPending={true}
          onSelectCard={vi.fn()}
          onSendAction={vi.fn()}
          onNextRound={onNextRound}
          onLeaveRequest={vi.fn()}
        />
      </MemoryRouter>,
    )

    const nextBtn = screen.getByRole('button', { name: /進入下一輪|Next Round/i })
    expect(nextBtn).toBeDisabled()
    fireEvent.click(nextBtn)
    expect(onNextRound).not.toHaveBeenCalled()
  })
})
