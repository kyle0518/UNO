import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { ChallengeDialog } from './ChallengeDialog'
import { ColorPicker } from './ColorPicker'
import { DrawnCardDialog } from './DrawnCardDialog'
import { RoundResultDialog } from './RoundResultDialog'
import { TargetPicker } from './TargetPicker'
import { TurnTimer } from './TurnTimer'

describe('Game dialogs', () => {
  it('ColorPicker renders light/dark colors with symbol option and responds to click & keys', () => {
    const onSelect = vi.fn()
    const { rerender } = render(
      <ColorPicker open={true} side="light" colorBlind={true} onSelect={onSelect} />,
    )
    expect(screen.getByText('◆')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: /紅色|Red/i }))
    expect(onSelect).toHaveBeenCalledWith('red')

    fireEvent.keyDown(window, { key: '4' })
    expect(onSelect).toHaveBeenCalledWith('blue')

    rerender(
      <ColorPicker open={true} side="dark" colorBlind={true} onSelect={onSelect} />,
    )
    expect(screen.getByText('★')).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: /粉色|Pink/i }))
    expect(onSelect).toHaveBeenCalledWith('pink')
  })

  it('DrawnCardDialog allows playing or keeping the drawn card', () => {
    const onPlay = vi.fn()
    const onKeep = vi.fn()
    render(
      <DrawnCardDialog
        open={true}
        card={{ id: 12, light: { color: 'green', face: { k: 'number', v: 7 } }, dark: null }}
        side="light"
        canPlay={true}
        canKeep={true}
        onPlay={onPlay}
        onKeep={onKeep}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: /立即打出|Play Card/i }))
    expect(onPlay).toHaveBeenCalledWith(12)
    fireEvent.click(screen.getByRole('button', { name: /保留手牌|Keep Card/i }))
    expect(onKeep).toHaveBeenCalledTimes(1)
  })

  it('ChallengeDialog triggers challenge or accept', () => {
    const onChallenge = vi.fn()
    const onAccept = vi.fn()
    render(
      <ChallengeDialog
        open={true}
        timer={{ seat: 0, deadline_ms: Date.now() + 12000, total_ms: 30000 }}
        onChallenge={onChallenge}
        onAccept={onAccept}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: /提出質疑|Challenge/i }))
    expect(onChallenge).toHaveBeenCalledTimes(1)
    fireEvent.click(screen.getByRole('button', { name: /接受|Accept/i }))
    expect(onAccept).toHaveBeenCalledTimes(1)
  })

  it('ChallengeDialog adapts text for Flip wild_draw_two and wild_draw_color', () => {
    const { rerender } = render(
      <ChallengeDialog
        open={true}
        timer={null}
        side="light"
        topCard={{
          id: 99,
          light: { color: null, face: { k: 'wild_draw_two' } },
          dark: { color: null, face: { k: 'wild_draw_color' } },
        }}
        onChallenge={vi.fn()}
        onAccept={vi.fn()}
      />,
    )
    expect(screen.getByRole('button', { name: /\+2/ })).toBeInTheDocument()

    rerender(
      <ChallengeDialog
        open={true}
        timer={null}
        side="dark"
        topCard={{
          id: 99,
          light: { color: null, face: { k: 'wild_draw_two' } },
          dark: { color: null, face: { k: 'wild_draw_color' } },
        }}
        onChallenge={vi.fn()}
        onAccept={vi.fn()}
      />,
    )
    expect(screen.getByText(/指定色|Draw Color/i)).toBeInTheDocument()
  })

  it('TargetPicker selects target seat on click', () => {
    const onSelect = vi.fn()
    render(
      <TargetPicker
        open={true}
        candidates={[
          {
            seat: 2,
            name: 'AI 2',
            player: {
              seat: 2,
              hand_size: 1,
              hand_backs: null,
              uno_called: true,
              eliminated: false,
              team: null,
              score: 0,
            },
          },
        ]}
        onSelect={onSelect}
      />,
    )
    fireEvent.click(screen.getByRole('button', { name: /AI 2/ }))
    expect(onSelect).toHaveBeenCalledWith(2)
  })

  it('RoundResultDialog renders scores and triggers Next Round', () => {
    const onNextRound = vi.fn()
    render(
      <RoundResultDialog
        open={true}
        round={2}
        winnerSeat={0}
        playerNames={['You', 'AI 1']}
        players={[
          { seat: 0, hand_size: 0, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 120 },
          { seat: 1, hand_size: 4, hand_backs: null, uno_called: false, eliminated: false, team: null, score: 30 },
        ]}
        roundDeltas={[90, 0]}
        onNextRound={onNextRound}
      />,
    )
    expect(screen.getByText(/120/)).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: /進入下一輪|Next Round/i }))
    expect(onNextRound).toHaveBeenCalledTimes(1)
  })

  it('TurnTimer renders remaining seconds when active and hides when null', () => {
    const { container, rerender } = render(<TurnTimer timer={null} />)
    expect(container.firstChild).toBeNull()

    rerender(
      <TurnTimer timer={{ seat: 0, deadline_ms: Date.now() + 9000, total_ms: 15000 }} />,
    )
    expect(screen.getByText(/\ds/)).toBeInTheDocument()
  })
})
