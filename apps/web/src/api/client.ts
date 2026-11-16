export interface ApiUser {
  id: number
  name: string
  is_guest: boolean
}

export interface AuthResponse {
  token: string
  user: ApiUser
}

export interface MeResponse {
  user: ApiUser
  ratings: Record<string, number>
}

export interface ServerInfoResponse {
  version: string
  lan_urls: string[]
  port: number
}

export interface ModeStats {
  games: number
  wins: number
  elo: number
}

export interface EloHistoryEntry {
  t: string
  mode_id: string
  elo: number
}

export interface UserStatsResponse {
  games: number
  wins: number
  by_mode: Record<string, ModeStats>
  elo_history: EloHistoryEntry[]
}

export interface UserMatchSummary {
  match_id: number
  mode_id: string
  created_at: string
  rank: number
  num_players: number
  score: number
}

export interface LeaderboardEntry {
  user: ApiUser
  elo: number
  games: number
  wins: number
}

export class ApiError extends Error {
  readonly status: number

  constructor(status: number, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

export const AUTH_TOKEN_KEY = 'uno-auth-token'

export function getStoredToken(): string | null {
  try {
    return localStorage.getItem(AUTH_TOKEN_KEY)
  } catch {
    return null
  }
}

export function setStoredToken(token: string | null): void {
  try {
    if (token) {
      localStorage.setItem(AUTH_TOKEN_KEY, token)
    } else {
      localStorage.removeItem(AUTH_TOKEN_KEY)
    }
  } catch {
    // ignore storage errors in restricted contexts
  }
}

async function parseErrorMessage(resp: Response): Promise<string> {
  try {
    const data = (await resp.json()) as { error?: unknown }
    if (typeof data.error === 'string' && data.error.length > 0) {
      return data.error
    }
  } catch {
    // non-JSON response
  }
  return `HTTP ${resp.status}`
}

export async function apiRequest<T>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers)
  const token = getStoredToken()
  if (token && !headers.has('Authorization')) {
    headers.set('Authorization', `Bearer ${token}`)
  }
  if (init.body !== undefined && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json')
  }

  const resp = await fetch(path, { ...init, headers })
  if (!resp.ok) {
    const msg = await parseErrorMessage(resp)
    throw new ApiError(resp.status, msg)
  }

  if (resp.status === 204) {
    return undefined as T
  }

  const contentType = resp.headers.get('content-type') ?? ''
  if (!contentType.includes('application/json')) {
    throw new ApiError(502, 'Unexpected non-JSON response')
  }

  return (await resp.json()) as T
}

export const apiClient = {
  serverInfo(): Promise<ServerInfoResponse> {
    return apiRequest<ServerInfoResponse>('/api/server-info')
  },
  me(): Promise<MeResponse> {
    return apiRequest<MeResponse>('/api/me')
  },
  guest(nickname: string): Promise<AuthResponse> {
    return apiRequest<AuthResponse>('/api/guest', {
      method: 'POST',
      body: JSON.stringify({ nickname }),
    })
  },
  register(username: string, password: string): Promise<AuthResponse> {
    return apiRequest<AuthResponse>('/api/register', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    })
  },
  login(username: string, password: string): Promise<AuthResponse> {
    return apiRequest<AuthResponse>('/api/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    })
  },
  logout(): Promise<void> {
    return apiRequest<void>('/api/logout', { method: 'POST' })
  },
  userStats(userId: number): Promise<UserStatsResponse> {
    return apiRequest<UserStatsResponse>(`/api/users/${userId}/stats`)
  },
  userMatches(userId: number, limit = 50, offset = 0): Promise<UserMatchSummary[]> {
    return apiRequest<UserMatchSummary[]>(
      `/api/users/${userId}/matches?limit=${limit}&offset=${offset}`,
    )
  },
  matchReplay(matchId: number): Promise<unknown> {
    return apiRequest<unknown>(`/api/matches/${matchId}/replay`)
  },
  leaderboard(mode = 'classic', limit = 50): Promise<LeaderboardEntry[]> {
    return apiRequest<LeaderboardEntry[]>(
      `/api/leaderboard?mode=${encodeURIComponent(mode)}&limit=${limit}`,
    )
  },
  daily(): Promise<DailyInfo> {
    return apiRequest<DailyInfo>('/api/daily')
  },
  submitDaily(replay: unknown): Promise<DailySubmitResult> {
    return apiRequest<DailySubmitResult>('/api/daily/submit', {
      method: 'POST',
      body: JSON.stringify(replay),
    })
  },
  submitSoloMatch(replay: unknown): Promise<{ match_id: number }> {
    return apiRequest<{ match_id: number }>('/api/matches/solo', {
      method: 'POST',
      body: JSON.stringify(replay),
    })
  },
  models(): Promise<ModelInfo[]> {
    return apiRequest<ModelInfo[]>('/api/models')
  },
}

export interface DailyInfo {
  date: string
  seed: number
  config: import('@/generated/RuleConfig').RuleConfig
  my_best: number | null
}

export interface DailySubmitResult {
  score: number
  counted: boolean
  rank: number
}

export interface ModelInfo {
  name: string
  mode_id?: string
  num_players?: number
  [key: string]: unknown
}
