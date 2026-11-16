import { describe, expect, it, vi } from 'vitest'
import type { ClientMsg } from '@/generated/ClientMsg'
import type { PlayerView } from '@/generated/PlayerView'
import type { ServerMsg } from '@/generated/ServerMsg'
import type { GameUpdate } from './GameClient'
import { RemoteGameClient } from './RemoteGameClient'

function fakeView(me: number | null): PlayerView {
  return { me } as unknown as PlayerView
}

function makeFakeSocket() {
  let listener: ((m: ServerMsg) => void) | null = null
  const sent: ClientMsg[] = []
  return {
    sent,
    emit(msg: ServerMsg) {
      listener?.(msg)
    },
    socket: {
      subscribe: (fn: (m: ServerMsg) => void) => {
        listener = fn
        return () => {
          listener = null
        }
      },
      send: (msg: ClientMsg) => {
        sent.push(msg)
        return true
      },
    },
  }
}

describe('RemoteGameClient', () => {
  it('maps snapshot/events/error/chat/emote to GameUpdate', () => {
    const fake = makeFakeSocket()
    const client = new RemoteGameClient(fake.socket)
    const updates: GameUpdate[] = []
    client.subscribe((u) => updates.push(u))

    fake.emit({
      type: 'game_snapshot',
      view: fakeView(2),
      timer: { seat: 2, deadline_ms: 5000, total_ms: 15000 },
    })
    fake.emit({ type: 'game_events', events: [], view: fakeView(2), timer: null })
    fake.emit({ type: 'error', code: 'illegal_action', message: 'nope', seq: 3 })
    fake.emit({ type: 'chat_msg', from: 'Bob', seat: 1, text: 'hi', t: 42 })
    fake.emit({ type: 'emote_msg', seat: 1, emote: 'laugh' })

    expect(client.mySeat).toBe(2)
    expect(updates.map((u) => u.kind)).toEqual([
      'snapshot',
      'events',
      'error',
      'chat',
      'emote',
    ])
    const snap = updates[0]
    if (snap?.kind === 'snapshot') {
      expect(snap.timer).toEqual({ seat: 2, deadline_ms: 5000, total_ms: 15000 })
    }
  })

  it('sends actions with increasing seq and next_round', () => {
    const fake = makeFakeSocket()
    const client = new RemoteGameClient(fake.socket)
    client.send({ type: 'draw' })
    client.send({ type: 'call_uno' })
    client.nextRound()
    expect(fake.sent).toEqual([
      { type: 'action', seq: 1, action: { type: 'draw' } },
      { type: 'action', seq: 2, action: { type: 'call_uno' } },
      { type: 'next_round' },
    ])
    // leave() 不應送 leave_room（保留座位重連）
    client.leave()
    expect(fake.sent).toHaveLength(3)
    expect(vi.fn()).toBeDefined()
  })
})
