import type { ClientMsg } from '@/generated/ClientMsg'
import type { RoomInfo } from '@/generated/RoomInfo'
import type { ServerMsg } from '@/generated/ServerMsg'
import type { UserPublic } from '@/generated/UserPublic'
import { getStoredToken } from '@/api/client'

export type SocketStatus = 'disconnected' | 'connecting' | 'connected' | 'reconnecting'

type ServerListener = (msg: ServerMsg) => void
type StatusListener = (status: SocketStatus) => void

function wsUrl(): string {
  const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${proto}//${window.location.host}/ws`
}

/**
 * WebSocket 單例：負責 Hello 驗證、訊息分派與自動重連
 * （間隔 1s、2s、4s，最多 10s；重連後自動重新加入原房間）。
 */
export class GameSocket {
  private ws: WebSocket | null = null
  private readonly listeners = new Set<ServerListener>()
  private readonly statusListeners = new Set<StatusListener>()
  private token: string | null = null
  private shouldStayConnected = false
  private reconnectDelayMs = 1000
  private reconnectTimer: number | null = null

  status: SocketStatus = 'disconnected'
  user: UserPublic | null = null
  room: RoomInfo | null = null
  private lastJoin: { code: string; asSpectator: boolean } | null = null

  connect(token?: string): void {
    const t = token ?? getStoredToken()
    if (!t) return
    this.token = t
    this.shouldStayConnected = true
    if (
      this.ws &&
      (this.ws.readyState === WebSocket.OPEN || this.ws.readyState === WebSocket.CONNECTING)
    ) {
      return
    }
    this.openSocket()
  }

  private openSocket(): void {
    if (!this.token) return
    this.setStatus(this.room ? 'reconnecting' : 'connecting')
    const ws = new WebSocket(wsUrl())
    this.ws = ws
    ws.onopen = () => {
      const hello: ClientMsg = { type: 'hello', token: this.token ?? '', protocol: 1 }
      ws.send(JSON.stringify(hello))
    }
    ws.onmessage = (ev: MessageEvent) => {
      let msg: ServerMsg
      try {
        msg = JSON.parse(String(ev.data)) as ServerMsg
      } catch {
        return
      }
      if (msg.type === 'welcome') {
        this.user = msg.user
        this.reconnectDelayMs = 1000
        this.setStatus('connected')
        // 重連後自動回原房間（若有）
        if (this.lastJoin) {
          this.send({
            type: 'join_room',
            code: this.lastJoin.code,
            as_spectator: this.lastJoin.asSpectator,
          })
        }
      } else if (msg.type === 'room_state') {
        this.room = msg.room
        if (this.user) {
          const seated = msg.room.seats.some(
            (s) => s.occupant.type === 'human' && s.occupant.user.id === this.user?.id,
          )
          this.lastJoin = {
            code: msg.room.code,
            asSpectator: !seated,
          }
        }
      }
      for (const fn of [...this.listeners]) fn(msg)
    }
    ws.onclose = () => {
      if (this.ws === ws) this.ws = null
      if (!this.shouldStayConnected) {
        this.setStatus('disconnected')
        return
      }
      this.setStatus('reconnecting')
      const delay = this.reconnectDelayMs
      this.reconnectDelayMs = Math.min(10_000, this.reconnectDelayMs * 2)
      this.reconnectTimer = window.setTimeout(() => this.openSocket(), delay)
    }
    ws.onerror = () => {
      try {
        ws.close()
      } catch {
        // ignore
      }
    }
  }

  private setStatus(status: SocketStatus): void {
    if (this.status === status) return
    this.status = status
    for (const fn of [...this.statusListeners]) fn(status)
  }

  send(msg: ClientMsg): boolean {
    if (msg.type === 'join_room') {
      this.lastJoin = { code: msg.code, asSpectator: msg.as_spectator }
    }
    if (msg.type === 'leave_room') {
      this.lastJoin = null
      this.room = null
    }
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return false
    this.ws.send(JSON.stringify(msg))
    return true
  }

  subscribe(fn: ServerListener): () => void {
    this.listeners.add(fn)
    return () => {
      this.listeners.delete(fn)
    }
  }

  subscribeStatus(fn: StatusListener): () => void {
    this.statusListeners.add(fn)
    fn(this.status)
    return () => {
      this.statusListeners.delete(fn)
    }
  }

  disconnect(): void {
    this.shouldStayConnected = false
    if (this.reconnectTimer !== null) {
      window.clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
    if (this.ws) {
      try {
        this.ws.close()
      } catch {
        // ignore
      }
      this.ws = null
    }
    this.room = null
    this.lastJoin = null
    this.setStatus('disconnected')
  }
}

export const gameSocket = new GameSocket()
