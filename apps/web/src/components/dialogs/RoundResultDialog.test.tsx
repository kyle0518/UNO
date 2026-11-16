import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { PublicPlayer } from '@/generated/PublicPlayer'
import { RoundResultDialog } from './RoundResultDialog'

function players(scores: number[]): PublicPlayer[] {
  return scores.map((score, seat) => ({
    seat,
    hand_size: seat === 0 ? 0 : 4,
    hand_backs: null,
    uno_called: false,
    eliminated: false,
    team: null,
    score,
  }))
}

describe('RoundResultDialog scoring clarity', () => {
  it('separates this-round points from cumulative totals in consecutive rounds', () => {
    const onNextRound = vi.fn()
    const { rerender } = render(
      <RoundResultDialog
        open
        round={1}
        players={players([125, 0, 0, 0])}
        playerNames={['You', 'AI 1', 'AI 2', 'AI 3']}
        winnerSeat={0}
        roundDeltas={[125, 0, 0, 0]}
        targetScore={500}
        onNextRound={onNextRound}
      />,
    )

    expect(screen.getByText(/本局得分|This round/)).toBeInTheDocument()
    expect(screen.getByText(/累積總分|Total score/)).toBeInTheDocument()
    expect(screen.getByTestId('round-earned-0')).toHaveTextContent('+125')
    expect(screen.getByTestId('round-total-0')).toHaveTextContent('125')
    expect(screen.getByText(/500/)).toBeInTheDocument()

    rerender(
      <RoundResultDialog
        open
        round={2}
        players={players([125, 80, 0, 0])}
        playerNames={['You', 'AI 1', 'AI 2', 'AI 3']}
        winnerSeat={1}
        roundDeltas={[0, 80, 0, 0]}
        targetScore={500}
        onNextRound={onNextRound}
      />,
    )

    expect(screen.getByTestId('round-earned-0')).toHaveTextContent('0')
    expect(screen.getByTestId('round-total-0')).toHaveTextContent('125')
    expect(screen.getByTestId('round-earned-1')).toHaveTextContent('+80')
    expect(screen.getByTestId('round-total-1')).toHaveTextContent('80')
    fireEvent.click(screen.getByRole('button', { name: /進入下一輪|Next Round/ }))
    expect(onNextRound).toHaveBeenCalledTimes(1)
  })

  it('does not show a target score note in single-round mode', () => {
    render(
      <RoundResultDialog
        open
        round={1}
        players={players([75, 0])}
        playerNames={['You', 'AI 1']}
        winnerSeat={0}
        roundDeltas={[75, 0]}
        onNextRound={vi.fn()}
      />,
    )

    const rows = screen.getAllByRole('listitem')
    const winnerRow = rows.find((row) => within(row).queryByText('You'))
    expect(winnerRow).toBeDefined()
    expect(screen.queryByText(/先達 500 分|First to 500/)).not.toBeInTheDocument()
  })
})
