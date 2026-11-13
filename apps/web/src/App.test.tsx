import { fireEvent, render, screen, within } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useAuth } from './auth/useAuth'
import { AppRoutes } from './routes'
import { useSettings } from './settings/useSettings'

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <AppRoutes />
    </MemoryRouter>,
  )
}

describe('AppRoutes', () => {
  const originalFetch = globalThis.fetch

  beforeEach(() => {
    localStorage.clear()
    useSettings.getState().resetSettings()
    useAuth.getState().resetAuthForTest()
    globalThis.fetch = vi.fn(() => Promise.reject(new Error('offline')))
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
  })

  it('renders home page and SPEC routes', () => {
    const { unmount } = renderAt('/')
    expect(screen.getByText('快速 vs AI')).toBeInTheDocument()
    unmount()

    const routes: [string, string][] = [
      ['/login', '登入'],
      ['/register', '註冊帳號'],
      ['/solo', '單機對戰設定'],
      ['/r/XYZ789', '房間大廳 #XYZ789'],
      ['/game', '遊戲桌'],
      ['/results', '對局結算'],
      ['/replay/local', '重播檢視器 #local'],
      ['/stats', '個人戰績'],
      ['/stats/42', '玩家戰績 #42'],
      ['/leaderboard', '排行榜'],
      ['/lab', 'AI 實驗室'],
      ['/rules', '規則說明'],
    ]

    for (const [path, heading] of routes) {
      const view = renderAt(path)
      expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(
        heading,
      )
      view.unmount()
    }
  })

  it('switches theme and language interactively on settings page', () => {
    renderAt('/settings')

    const themeRegion = screen.getByRole('region', { name: '主題外觀' })
    fireEvent.click(within(themeRegion).getByRole('button', { name: '霓虹' }))
    expect(document.documentElement.dataset.theme).toBe('neon')

    fireEvent.click(screen.getByRole('button', { name: 'English' }))
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(
      'Settings',
    )
    expect(document.documentElement.lang).toBe('en')
  })
})
