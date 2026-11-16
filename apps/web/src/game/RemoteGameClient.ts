import type { Action } from '@/generated/Action'
import type { ServerMsg } from '@/generated/ServerMsg'
import { gameSocket, type GameSocket } from '@/net/socket'
import type { GameClient, GameUpdate, LocalTurnTimer } from './GameClient'

function toLocalTimer(
  timer: { seat: number; deadline_ms: number; total_ms: number } | null,
): LocalTurnTimer | null {
  if (!timer) return null
  return { seat: timer.seat, deadline_ms: timer.deadline_ms, total_ms: timer.total_ms }
}

/**
 * 連線版 GameClient：包裝 WebSocket（gameSocket），
 * 將 ServerMsg 轉成與 LocalGameClient 相同的 GameUpdate 流。
 */
export class RemoteGameClient implements GameClient {
  private readonly socket: Pick<GameSocket, 'subscribe' | 'send'>
  private seq = 0
  private seat: number | null = null

  constructor(socket: Pick<GameSocket, 'subscribe' | 'send'> = gameSocket) {
    this.socket = socket
  }

  get mySeat(): number | null {
    return this.seat
  }

  subscribe(listener: (u: GameUpdate) => void): () => void {
    return this.socket.subscribe((msg: ServerMsg) => {
      switch (msg.type) {
        case 'game_snapshot':
          this.seat = msg.view.me
          listener({
            kind: 'snapshot',
            view: msg.view,
            timer: toLocalTimer(msg.timer),
            gameTimer: null,
          })
          break
        case 'game_events':
          this.seat = msg.view.me
          listener({
            kind: 'events',
            events: msg.events,
            view: msg.view,
            timer: toLocalTimer(msg.timer),
            gameTimer: null,
          })
          break
        case 'error':
          listener({ kind: 'error', message: msg.message })
          break
        case 'chat_msg':
          listener({ kind: 'chat', from: msg.from, text: msg.text })
          break
        case 'emote_msg':
          listener({ kind: 'emote', seat: msg.seat, emote: msg.emote })
          break
        default:
          break
      }
    })
  }

  send(action: Action): void {
    this.seq += 1
    this.socket.send({ type: 'action', seq: this.seq, action })
  }

  nextRound(): void {
    this.socket.send({ type: 'next_round' })
  }

  sendChat(text: string): void {
    this.socket.send({ type: 'chat', text })
  }

  sendEmote(emote: string): void {
    this.socket.send({ type: 'emote', emote })
  }

  leave(): void {
    // 連線版離開遊戲桌不釋放座位（保留重連回座）；真正離房由大廳頁送 leave_room。
  }
}
