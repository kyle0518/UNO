import { fireEvent, render, screen } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { beforeEach, describe, expect, it } from 'vitest'
import { useSettings } from '@/settings/useSettings'
import { RulesPage } from './RulesPage'

describe('RulesPage', () => {
  beforeEach(() => {
    localStorage.clear()
    useSettings.getState().resetSettings()
  })

  it('renders mode sections, card illustrations, and opens tutorial overlay', () => {
    render(
      <MemoryRouter>
        <RulesPage />
      </MemoryRouter>,
    )

    expect(screen.getByText('基本玩法')).toBeInTheDocument()
    expect(screen.getByText(/經典 Classic/)).toBeInTheDocument()
    expect(screen.getByText(/翻面 UNO Flip/)).toBeInTheDocument()
    expect(screen.getByText(/無情 No Mercy/)).toBeInTheDocument()
    expect(screen.getAllByRole('img').length).toBeGreaterThanOrEqual(10)

    fireEvent.click(screen.getByRole('button', { name: '查看 4 步新手教學' }))
    expect(screen.getByRole('dialog')).toBeInTheDocument()
    expect(screen.getByText('配對顏色或點數')).toBeInTheDocument()
  })
})
