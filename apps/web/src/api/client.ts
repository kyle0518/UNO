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
}
