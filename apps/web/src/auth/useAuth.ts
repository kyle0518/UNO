import { create } from 'zustand'
import {
  ApiError,
  apiClient,
  getStoredToken,
  setStoredToken,
  type ApiUser,
  type ServerInfoResponse,
} from '@/api/client'

export type ServerStatus = 'unknown' | 'online' | 'offline'

export interface AuthState {
  user: ApiUser | null
  token: string | null
  ratings: Record<string, number>
  serverStatus: ServerStatus
  serverInfo: ServerInfoResponse | null
  loading: boolean
  error: string | null
  bootstrap: () => Promise<void>
  guestLogin: (nickname: string) => Promise<void>
  login: (username: string, password: string) => Promise<void>
  register: (username: string, password: string) => Promise<void>
  logout: () => Promise<void>
  clearError: () => void
  resetAuthForTest: () => void
}

export const useAuth = create<AuthState>((set) => ({
  user: null,
  token: getStoredToken(),
  ratings: {},
  serverStatus: 'unknown',
  serverInfo: null,
  loading: false,
  error: null,

  bootstrap: async () => {
    set({ loading: true, error: null })
    try {
      const info = await apiClient.serverInfo()
      const storedToken = getStoredToken()
      if (!storedToken) {
        set({
          serverStatus: 'online',
          serverInfo: info,
          token: null,
          user: null,
          ratings: {},
          loading: false,
        })
        return
      }

      try {
        const me = await apiClient.me()
        set({
          serverStatus: 'online',
          serverInfo: info,
          token: storedToken,
          user: me.user,
          ratings: me.ratings,
          loading: false,
        })
      } catch (err) {
        if (err instanceof ApiError && err.status === 401) {
          setStoredToken(null)
          set({
            serverStatus: 'online',
            serverInfo: info,
            token: null,
            user: null,
            ratings: {},
            loading: false,
          })
          return
        }
        throw err
      }
    } catch {
      set({
        serverStatus: 'offline',
        serverInfo: null,
        loading: false,
      })
    }
  },

  guestLogin: async (nickname: string) => {
    set({ loading: true, error: null })
    try {
      const res = await apiClient.guest(nickname)
      setStoredToken(res.token)
      set({
        token: res.token,
        user: res.user,
        serverStatus: 'online',
        loading: false,
      })
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Request failed'
      set({ loading: false, error: message })
      throw err
    }
  },

  login: async (username: string, password: string) => {
    set({ loading: true, error: null })
    try {
      const res = await apiClient.login(username, password)
      setStoredToken(res.token)
      set({
        token: res.token,
        user: res.user,
        serverStatus: 'online',
        loading: false,
      })
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Request failed'
      set({ loading: false, error: message })
      throw err
    }
  },

  register: async (username: string, password: string) => {
    set({ loading: true, error: null })
    try {
      const res = await apiClient.register(username, password)
      setStoredToken(res.token)
      set({
        token: res.token,
        user: res.user,
        serverStatus: 'online',
        loading: false,
      })
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Request failed'
      set({ loading: false, error: message })
      throw err
    }
  },

  logout: async () => {
    set({ loading: true, error: null })
    try {
      await apiClient.logout()
    } catch {
      // clear local session even if server is unavailable
    } finally {
      setStoredToken(null)
      set({
        token: null,
        user: null,
        ratings: {},
        loading: false,
      })
    }
  },

  clearError: () => set({ error: null }),

  resetAuthForTest: () => {
    setStoredToken(null)
    set({
      user: null,
      token: null,
      ratings: {},
      serverStatus: 'unknown',
      serverInfo: null,
      loading: false,
      error: null,
    })
  },
}))
