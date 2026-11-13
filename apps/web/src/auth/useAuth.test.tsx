import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { MemoryRouter } from 'react-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { AUTH_TOKEN_KEY } from '@/api/client'
import { AppRoutes } from '@/routes'
import { useSettings } from '@/settings/useSettings'
import { useAuth } from './useAuth'

function jsonResp(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

describe('useAuth and account pages', () => {
  const originalFetch = globalThis.fetch

  beforeEach(() => {
    localStorage.clear()
    useSettings.getState().resetSettings()
    useAuth.getState().resetAuthForTest()
  })

  afterEach(() => {
    globalThis.fetch = originalFetch
    vi.restoreAllMocks()
  })

  it('falls back to offline mode when server-info fails', async () => {
    globalThis.fetch = vi.fn(() => Promise.reject(new Error('offline')))
    await useAuth.getState().bootstrap()
    expect(useAuth.getState().serverStatus).toBe('offline')
  })

  it('supports guest login and token persistence when server is online', async () => {
    const fetchMock = vi.fn((input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString()
      if (url === '/api/server-info') {
        return Promise.resolve(
          jsonResp({
            version: '0.1.0',
            lan_urls: ['http://192.168.1.23:8787'],
            port: 8787,
          }),
        )
      }
      if (url === '/api/guest') {
        return Promise.resolve(
          jsonResp({
            token: 'tok-123',
            user: { id: 1, name: 'Alice', is_guest: true },
          }),
        )
      }
      if (url === '/api/me') {
        return Promise.resolve(
          jsonResp({
            user: { id: 1, name: 'Alice', is_guest: true },
            ratings: {},
          }),
        )
      }
      if (url === '/api/logout') {
        return Promise.resolve(new Response(null, { status: 204 }))
      }
      return Promise.resolve(jsonResp({ error: 'not found' }, 404))
    })
    globalThis.fetch = fetchMock

    await useAuth.getState().guestLogin('Alice')
    expect(useAuth.getState().user?.name).toBe('Alice')
    expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBe('tok-123')

    // Simulate reload with stored token
    useAuth.setState({ user: null, serverStatus: 'unknown' })
    await useAuth.getState().bootstrap()
    expect(useAuth.getState().user?.name).toBe('Alice')

    await useAuth.getState().logout()
    expect(useAuth.getState().user).toBeNull()
    expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBeNull()
  })

  it('clears expired token on 401 from /api/me', async () => {
    localStorage.setItem(AUTH_TOKEN_KEY, 'expired-tok')
    globalThis.fetch = vi.fn((input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString()
      if (url === '/api/server-info') {
        return Promise.resolve(
          jsonResp({ version: '0.1.0', lan_urls: [], port: 8787 }),
        )
      }
      if (url === '/api/me') {
        return Promise.resolve(jsonResp({ error: 'unauthorized' }, 401))
      }
      return Promise.resolve(jsonResp({ error: 'not found' }, 404))
    })

    await useAuth.getState().bootstrap()
    expect(useAuth.getState().serverStatus).toBe('online')
    expect(useAuth.getState().token).toBeNull()
    expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBeNull()
  })

  it('handles /login submit and displays error alert on failure', async () => {
    globalThis.fetch = vi.fn((input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString()
      if (url === '/api/server-info') {
        return Promise.resolve(
          jsonResp({ version: '0.1.0', lan_urls: [], port: 8787 }),
        )
      }
      if (url === '/api/login') {
        return Promise.resolve(jsonResp({ error: '帳號或密碼錯誤' }, 401))
      }
      return Promise.resolve(jsonResp({ error: 'not found' }, 404))
    })

    render(
      <MemoryRouter initialEntries={['/login']}>
        <AppRoutes />
      </MemoryRouter>,
    )

    fireEvent.change(screen.getByLabelText('帳號名稱'), {
      target: { value: 'bob' },
    })
    fireEvent.change(screen.getByLabelText('密碼'), {
      target: { value: 'secret123' },
    })
    fireEvent.click(screen.getByRole('button', { name: '登入' }))

    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('帳號或密碼錯誤')
  })

  it('renders offline banner and upgrades guest account on /register', async () => {
    globalThis.fetch = vi.fn((input: RequestInfo | URL) => {
      const url = typeof input === 'string' ? input : input.toString()
      if (url === '/api/server-info') {
        return Promise.reject(new Error('offline'))
      }
      if (url === '/api/register') {
        return Promise.resolve(
          jsonResp({
            token: 'upgraded-tok',
            user: { id: 1, name: 'BobUser', is_guest: false },
          }),
        )
      }
      return Promise.resolve(jsonResp({ error: 'not found' }, 404))
    })

    useAuth.setState({
      user: { id: 1, name: 'Guest_1', is_guest: true },
      token: 'guest-tok',
      serverStatus: 'offline',
    })

    render(
      <MemoryRouter initialEntries={['/register']}>
        <AppRoutes />
      </MemoryRouter>,
    )

    expect(
      await screen.findByText('找不到伺服器連線，目前僅可使用單機 vs AI 模式。'),
    ).toBeInTheDocument()

    const submitBtn = screen.getByRole('button', { name: '升級目前訪客帳號' })
    fireEvent.change(screen.getByLabelText('帳號名稱'), {
      target: { value: 'BobUser' },
    })
    fireEvent.change(screen.getByLabelText('密碼'), {
      target: { value: 'secret123' },
    })
    fireEvent.click(submitBtn)

    await waitFor(() => {
      expect(useAuth.getState().user?.is_guest).toBe(false)
      expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBe('upgraded-tok')
    })
  })
})
