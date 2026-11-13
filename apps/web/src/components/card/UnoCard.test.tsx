import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { CardFace } from '@/generated/CardFace'
import { useSettings } from '@/settings/useSettings'
import { UnoCard } from './UnoCard'

const ALL_FACES: CardFace[] = [
  { color: 'red', face: { k: 'number', v: 7 } },
  { color: 'yellow', face: { k: 'number', v: 6 } },
  { color: 'green', face: { k: 'number', v: 9 } },
  { color: 'blue', face: { k: 'skip' } },
  { color: 'red', face: { k: 'reverse' } },
  { color: 'green', face: { k: 'draw_two' } },
  { color: null, face: { k: 'wild' } },
  { color: null, face: { k: 'wild_draw_four' } },
  { color: 'yellow', face: { k: 'draw_one' } },
  { color: 'blue', face: { k: 'flip' } },
  { color: null, face: { k: 'wild_draw_two' } },
  { color: 'teal', face: { k: 'draw_five' } },
  { color: 'pink', face: { k: 'skip_everyone' } },
  { color: null, face: { k: 'wild_draw_color' } },
  { color: 'orange', face: { k: 'draw_four' } },
  { color: 'purple', face: { k: 'discard_all' } },
  { color: null, face: { k: 'wild_reverse_draw_four' } },
  { color: null, face: { k: 'wild_draw_six' } },
  { color: null, face: { k: 'wild_draw_ten' } },
  { color: null, face: { k: 'wild_color_roulette' } },
]

describe('UnoCard', () => {
  beforeEach(() => {
    localStorage.clear()
    useSettings.getState().resetSettings()
  })

  it('renders every Face variant without throwing', () => {
    for (const cardFace of ALL_FACES) {
      const { unmount } = render(<UnoCard face={cardFace} size="md" />)
      expect(screen.getByRole('img')).toBeInTheDocument()
      unmount()
    }
  })

  it('produces expected aria-labels including classic examples', () => {
    const { rerender } = render(
      <UnoCard face={{ color: 'red', face: { k: 'number', v: 7 } }} size="md" />,
    )
    expect(screen.getByRole('img')).toHaveAttribute('aria-label', '紅色 7')

    rerender(
      <UnoCard face={{ color: null, face: { k: 'wild_draw_four' } }} size="md" />,
    )
    expect(screen.getByRole('img')).toHaveAttribute('aria-label', '萬用 +4')

    rerender(<UnoCard face={null} size="sm" />)
    expect(screen.getByRole('img')).toHaveAttribute('aria-label', 'UNO 牌背')
  })

  it('renders color-blind symbols and handles click states', () => {
    const onClick = vi.fn()
    const { container, rerender } = render(
      <UnoCard
        face={{ color: 'red', face: { k: 'skip' } }}
        size="lg"
        state="playable"
        colorBlind
        onClick={onClick}
      />,
    )
    expect(container.textContent).toContain('◆')
    fireEvent.click(screen.getByRole('button', { name: '紅色 跳過' }))
    expect(onClick).toHaveBeenCalledTimes(1)

    onClick.mockClear()
    rerender(
      <UnoCard
        face={{ color: 'red', face: { k: 'skip' } }}
        size="lg"
        state="disabled"
        onClick={onClick}
      />,
    )
    const btn = screen.getByRole('button', { name: '紅色 跳過' })
    expect(btn).toBeDisabled()
    fireEvent.click(btn)
    expect(onClick).not.toHaveBeenCalled()
  })

  it('renders distinct card back themes from settings', () => {
    useSettings.getState().setCardBack('neon')
    const { container, rerender } = render(<UnoCard face={null} size="md" />)
    expect(container.querySelector('[data-card-back="neon"]')).toBeInTheDocument()

    useSettings.getState().setCardBack('minimal')
    rerender(<UnoCard face={null} size="md" />)
    expect(
      container.querySelector('[data-card-back="minimal"]'),
    ).toBeInTheDocument()
  })
})
