import { useEffect, useRef, useState } from 'react'
import { useNavigate, useParams } from 'react-router'
import { useAuth } from '@/auth/useAuth'
import { AppShell } from '@/components/layout/AppShell'
import { QrCanvas } from '@/components/layout/QrCanvas'
import type { RoomInfo } from '@/generated/RoomInfo'
import { savePendingRemoteRoom } from '@/game/remoteSession'
import { useTranslation, type TranslationKey } from '@/i18n'
import { gameSocket, type SocketStatus } from '@/net/socket'

const BOT_KINDS = ['random', 'heuristic', 'hard', 'hell'] as const
const EMOTES = ['thumbs_up', 'laugh', 'angry', 'cry', 'fire', 'uno'] as const

interface ChatLine {
  from: string
  text: string
  seq: number
}

export function RoomPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const params = useParams()
  const code = (params.code ?? '').toUpperCase()
  const user = useAuth((s) => s.user)
  const token = useAuth((s) => s.token)
  const guestLogin = useAuth((s) => s.guestLogin)

  const [room, setRoom] = useState<RoomInfo | null>(() => gameSocket.room)
  const [status, setStatus] = useState<SocketStatus>(gameSocket.status)
  const [nickname, setNickname] = useState('')
  const [botKind, setBotKind] = useState<string>('heuristic')
  const [chat, setChat] = useState<ChatLine[]>([])
  const [chatText, setChatText] = useState('')
  const [copied, setCopied] = useState(false)
  const [joinError, setJoinError] = useState<string | null>(null)
  const joinedKeyRef = useRef<string | null>(null)
  const chatSeqRef = useRef(0)

  useEffect(() => {
    if (token) gameSocket.connect(token)
  }, [token])

  useEffect(() => gameSocket.subscribeStatus(setStatus), [])

  useEffect(
    () =>
      gameSocket.subscribe((msg) => {
        if (msg.type === 'room_state') {
          if (msg.room.code === code) setRoom(msg.room)
        } else if (msg.type === 'chat_msg') {
          chatSeqRef.current += 1
          const seq = chatSeqRef.current
          setChat((prev) => [...prev.slice(-49), { from: msg.from, text: msg.text, seq }])
        } else if (msg.type === 'error' && msg.code === 'room_not_found') {
          setJoinError(t('room.notFound'))
        }
      }),
    [code, t],
  )

  // 連線就緒後自動加入（/r/:code 直接開啟的關鍵流程）。
  useEffect(() => {
    if (!user || status !== 'connected') return
    const key = `${code}:${user.id}`
    if (room?.code === code) {
      joinedKeyRef.current = key
      return
    }
    if (joinedKeyRef.current === key) return
    joinedKeyRef.current = key
    gameSocket.send({ type: 'join_room', code, as_spectator: false })
  }, [user, status, code, room])

  const mySeat = room?.seats.find(
    (s) => s.occupant.type === 'human' && s.occupant.user.id === user?.id,
  )
  const isHost = user !== null && room?.host_user_id === user.id
  const isSeated = mySeat !== undefined
  const inGame =
    room?.status === 'playing' || room?.status === 'round_over' || room?.status === 'game_over'

  const handleGuest = async () => {
    if (!nickname.trim()) return
    try {
      await guestLogin(nickname.trim())
    } catch {
      // useAuth 已記錄 error
    }
  }

  const handleShare = async () => {
    if (!room) return
    const url = room.join_url
    if (typeof navigator !== 'undefined' && 'share' in navigator) {
      try {
        await navigator.share({ url })
        return
      } catch {
        // 使用者取消或不支援時改走複製
      }
    }
    try {
      await navigator.clipboard.writeText(url)
      setCopied(true)
      window.setTimeout(() => setCopied(false), 2000)
    } catch {
      // clipboard 不可用時忽略
    }
  }

  const statusKey = (s: RoomInfo['status']): TranslationKey =>
    (`room.status${s.charAt(0).toUpperCase()}${s.slice(1).replace(/_(\w)/g, (_, c: string) =>
      c.toUpperCase(),
    )}`) as TranslationKey

  return (
    <AppShell title={t('route.room', { code })}>
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-5">
        {status === 'reconnecting' && (
          <p className="rounded-xl bg-amber-500/20 px-4 py-2 text-sm font-bold text-amber-200">
            {t('room.reconnecting')}
          </p>
        )}

        {!user && (
          <section className="uno-page-panel flex flex-col gap-3 p-5">
            <p className="text-sm font-semibold">{t('room.needLogin')}</p>
            <div className="flex gap-2">
              <input
                value={nickname}
                onChange={(e) => setNickname(e.target.value)}
                placeholder={t('room.nickname')}
                className="min-h-11 flex-1 rounded-xl border border-white/15 bg-black/40 px-3 text-sm"
              />
              <button
                type="button"
                onClick={() => void handleGuest()}
                className="min-h-11 rounded-xl bg-btn-green px-4 text-sm font-bold text-white"
              >
                {t('room.enter')}
              </button>
            </div>
          </section>
        )}

        {joinError && (
          <p role="alert" className="text-sm font-semibold text-red-300">
            {joinError}
          </p>
        )}

        {room && (
          <>
            <section className="uno-page-panel flex flex-col gap-3 p-5">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className="font-display text-2xl font-bold tracking-[0.2em]">
                  {room.code}
                </span>
                <span className="rounded-full bg-white/10 px-3 py-1 text-xs font-bold">
                  {t(statusKey(room.status))}
                </span>
                <span className="text-sm text-text-muted">
                  {t(`mode.${room.config.mode_id}` as TranslationKey)} · {room.config.num_players}
                  {t('room.players')}
                </span>
              </div>
              <div className="flex flex-wrap items-center gap-4">
                <QrCanvas value={room.join_url} size={132} />
                <div className="flex min-w-40 flex-1 flex-col gap-2">
                  <span className="break-all text-xs text-text-muted">{room.join_url}</span>
                  <div className="flex gap-2">
                    <button
                      type="button"
                      onClick={() => void handleShare()}
                      className="min-h-11 rounded-xl bg-btn-blue px-4 text-sm font-bold text-white"
                    >
                      {t('room.share')}
                    </button>
                    <button
                      type="button"
                      onClick={() => void handleShare()}
                      className="min-h-11 rounded-xl bg-white/10 px-4 text-sm font-semibold"
                    >
                      {copied ? t('room.copied') : t('room.copyLink')}
                    </button>
                  </div>
                  {!isSeated && <span className="text-xs text-accent">{t('room.spectating')}</span>}
                </div>
              </div>
            </section>

            <section className="flex flex-col gap-2">
              {room.seats.map((seatInfo) => {
                const occ = seatInfo.occupant
                const isMe = occ.type === 'human' && occ.user.id === user?.id
                return (
                  <div
                    key={seatInfo.seat}
                    className="flex min-h-14 flex-wrap items-center justify-between gap-2 rounded-2xl bg-black/25 px-4 py-2"
                  >
                    <span className="text-sm font-bold">
                      {t('room.seat', { seat: seatInfo.seat + 1 })} ·{' '}
                      {occ.type === 'empty' &&
                        (room.status === 'waiting' ? t('room.emptySeat') : '—')}
                      {occ.type === 'bot' && t('room.botSeat', { kind: occ.kind })}
                      {occ.type === 'human' && (
                        <>
                          {occ.user.name}
                          {isMe && ` (${t('room.you')})`}
                          {occ.user.id === room.host_user_id && ` · ${t('room.host')}`}
                        </>
                      )}
                    </span>
                    <span className="flex items-center gap-2 text-xs">
                      {occ.type !== 'empty' && (
                        <span
                          className={
                            seatInfo.connected ? 'text-emerald-300' : 'text-red-300'
                          }
                        >
                          {seatInfo.connected ? t('room.connected') : t('room.disconnected')}
                        </span>
                      )}
                      {occ.type === 'human' && (
                        <span>{seatInfo.ready ? t('room.ready') : t('room.notReady')}</span>
                      )}
                      {room.status === 'waiting' && occ.type === 'empty' && !isSeated && user && (
                        <button
                          type="button"
                          onClick={() =>
                            gameSocket.send({ type: 'take_seat', seat: seatInfo.seat })
                          }
                          className="min-h-11 rounded-xl bg-btn-green px-3 font-bold text-white"
                        >
                          {t('room.takeSeat')}
                        </button>
                      )}
                      {room.status === 'waiting' && occ.type === 'empty' && isHost && (
                        <span className="flex items-center gap-1">
                          <select
                            aria-label={t('room.botKind')}
                            value={botKind}
                            onChange={(e) => setBotKind(e.target.value)}
                            className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-2 text-xs"
                          >
                            {BOT_KINDS.map((k) => (
                              <option key={k} value={k}>
                                {t(`solo.bot${k.charAt(0).toUpperCase()}${k.slice(1)}` as TranslationKey)}
                              </option>
                            ))}
                          </select>
                          <button
                            type="button"
                            onClick={() =>
                              gameSocket.send({
                                type: 'add_bot',
                                seat: seatInfo.seat,
                                bot: botKind,
                              })
                            }
                            className="min-h-11 rounded-xl bg-btn-blue px-3 font-bold text-white"
                          >
                            {t('room.addBot')}
                          </button>
                        </span>
                      )}
                      {room.status === 'waiting' &&
                        isHost &&
                        occ.type !== 'empty' &&
                        !isMe && (
                          <button
                            type="button"
                            onClick={() =>
                              gameSocket.send({ type: 'remove_seat', seat: seatInfo.seat })
                            }
                            className="min-h-11 rounded-xl bg-white/10 px-3 font-semibold"
                          >
                            {t('room.removeSeat')}
                          </button>
                        )}
                    </span>
                  </div>
                )
              })}
              {room.spectators.length > 0 && (
                <p className="text-xs text-text-muted">
                  {t('room.spectators')}: {room.spectators.map((s) => s.name).join(', ')}
                </p>
              )}
            </section>

            <section className="flex flex-wrap gap-2">
              {isSeated && room.status === 'waiting' && (
                <button
                  type="button"
                  onClick={() =>
                    gameSocket.send({ type: 'set_ready', ready: !(mySeat?.ready ?? false) })
                  }
                  className="min-h-12 flex-1 rounded-2xl bg-btn-blue px-4 font-bold text-white"
                >
                  {t('room.toggleReady')} ({mySeat?.ready ? t('room.ready') : t('room.notReady')})
                </button>
              )}
              {isHost && room.status === 'waiting' && (
                <button
                  type="button"
                  onClick={() => gameSocket.send({ type: 'start_game' })}
                  className="min-h-12 flex-1 rounded-2xl bg-btn-green px-4 font-bold text-white"
                >
                  {t('room.start')}
                </button>
              )}
              {isHost && room.status === 'game_over' && (
                <button
                  type="button"
                  onClick={() => gameSocket.send({ type: 'rematch' })}
                  className="min-h-12 flex-1 rounded-2xl bg-btn-green px-4 font-bold text-white"
                >
                  {t('room.rematch')}
                </button>
              )}
              {inGame && (
                <button
                  type="button"
                  onClick={() => {
                    savePendingRemoteRoom(code)
                    void navigate('/game')
                  }}
                  className="min-h-12 flex-1 rounded-2xl bg-btn-green px-4 font-bold text-white"
                >
                  {t('room.enterGame')}
                </button>
              )}
              <button
                type="button"
                onClick={() => {
                  gameSocket.send({ type: 'leave_room' })
                  void navigate('/')
                }}
                className="min-h-12 rounded-2xl bg-white/10 px-4 font-semibold"
              >
                {t('room.leave')}
              </button>
            </section>

            <section className="uno-page-panel flex flex-col gap-3 p-5">
              <h2 className="text-sm font-bold">{t('room.chatTitle')}</h2>
              <div className="flex gap-1">
                {EMOTES.map((e) => (
                  <button
                    key={e}
                    type="button"
                    disabled={!isSeated}
                    onClick={() => gameSocket.send({ type: 'emote', emote: e })}
                    className="min-h-11 rounded-xl bg-white/10 px-3 text-xs font-semibold disabled:opacity-40"
                  >
                    {e}
                  </button>
                ))}
              </div>
              <ul className="flex max-h-44 flex-col gap-1 overflow-y-auto text-sm">
                {chat.map((line) => (
                  <li key={line.seq}>
                    <span className="font-bold">{line.from}</span>: {line.text}
                  </li>
                ))}
              </ul>
              <form
                className="flex gap-2"
                onSubmit={(ev) => {
                  ev.preventDefault()
                  const text = chatText.trim()
                  if (!text) return
                  gameSocket.send({ type: 'chat', text })
                  setChatText('')
                }}
              >
                <input
                  value={chatText}
                  onChange={(e) => setChatText(e.target.value)}
                  placeholder={t('room.chatPlaceholder')}
                  maxLength={200}
                  className="min-h-11 flex-1 rounded-xl border border-white/15 bg-black/40 px-3 text-sm"
                />
                <button
                  type="submit"
                  className="min-h-11 rounded-xl bg-btn-blue px-4 text-sm font-bold text-white"
                >
                  {t('room.chatSend')}
                </button>
              </form>
            </section>
          </>
        )}
      </div>
    </AppShell>
  )
}
