import { fireEvent, render, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useSettings } from '@/settings/useSettings'
import { TutorialOverlay } from './TutorialOverlay'
import { TUTORIAL_SEEN_KEY } from './tutorialStorage'

describe('TutorialOverlay', () => {
  beforeEach(() => {
    localStorage.clear()
    useSettings.getState().resetSettings()
  })

  it('opens on first visit, navigates through 4 steps, and persists completion', () => {
    const onClose = vi.fn()
    render(<TutorialOverlay onClose={onClose} />)

    expect(screen.getByRole('dialog')).toBeInTheDocument()
    expect(screen.getByText('教學 1 / 4')).toBeInTheDocument()
    expect(screen.getByText('配對顏色或點數')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: '下一步' }))
    expect(screen.getByText('教學 2 / 4')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: '下一步' }))
    expect(screen.getByText('教學 3 / 4')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: '上一步' }))
    expect(screen.getByText('教學 2 / 4')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: '下一步' }))
    fireEvent.click(screen.getByRole('button', { name: '下一步' }))
    expect(screen.getByText('教學 4 / 4')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: '開始遊戲' }))
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(localStorage.getItem(TUTORIAL_SEEN_KEY)).toBe('1')
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('stays hidden after seen unless forceOpen is true, and supports skip + Esc', () => {
    localStorage.setItem(TUTORIAL_SEEN_KEY, '1')
    const { unmount } = render(<TutorialOverlay />)
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    unmount()

    render(<TutorialOverlay forceOpen />)
    expect(screen.getByRole('dialog')).toBeInTheDocument()

    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })
})
