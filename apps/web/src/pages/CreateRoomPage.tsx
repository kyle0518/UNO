import { useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router'
import { useAuth } from '@/auth/useAuth'
import { AppShell } from '@/components/layout/AppShell'
import type { RuleConfig } from '@/generated/RuleConfig'
import { useTranslation, type TranslationKey } from '@/i18n'
import { gameSocket } from '@/net/socket'
import { ensureWasmInit } from '@/wasm/init'
import { preset } from '@/wasm/pkg/uno_wasm'

interface ModeMeta {
  id: string
  icon: string
  minPlayers: number
  maxPlayers: number
  defaultPlayers: number
}

const MODES: ModeMeta[] = [
  { id: 'classic', icon: '🎴', minPlayers: 2, maxPlayers: 10, defaultPlayers: 4 },
  { id: 'party', icon: '🎉', minPlayers: 2, maxPlayers: 10, defaultPlayers: 4 },
  { id: 'flip', icon: '🔄', minPlayers: 2, maxPlayers: 10, defaultPlayers: 4 },
  { id: 'no_mercy', icon: '🔥', minPlayers: 2, maxPlayers: 6, defaultPlayers: 4 },
  { id: 'teams', icon: '🤝', minPlayers: 4, maxPlayers: 6, defaultPlayers: 4 },
  { id: 'elimination', icon: '🏆', minPlayers: 3, maxPlayers: 10, defaultPlayers: 4 },
  { id: 'blitz', icon: '⚡', minPlayers: 2, maxPlayers: 10, defaultPlayers: 4 },
]

export function CreateRoomPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const user = useAuth((s) => s.user)
  const token = useAuth((s) => s.token)
  const guestLogin = useAuth((s) => s.guestLogin)
  const [modeId, setModeId] = useState('classic')
  const [numPlayers, setNumPlayers] = useState(4)
  const [isPublic, setIsPublic] = useState(true)
  const [nickname, setNickname] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const pendingRef = useRef(false)

  const mode = MODES.find((m) => m.id === modeId) ?? MODES[0]!

  useEffect(() => {
    if (token) gameSocket.connect(token)
  }, [token])

  useEffect(
    () =>
      gameSocket.subscribe((msg) => {
        if (!pendingRef.current) return
        if (msg.type === 'room_state') {
          pendingRef.current = false
          void navigate(`/r/${msg.room.code}`)
        } else if (msg.type === 'error') {
          pendingRef.current = false
          setBusy(false)
          setError(msg.message)
        }
      }),
    [navigate],
  )

  const handleGuest = async () => {
    if (!nickname.trim()) return
    setError(null)
    try {
      await guestLogin(nickname.trim())
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  const handleCreate = async () => {
    setBusy(true)
    setError(null)
    try {
      await ensureWasmInit()
      const config = preset(modeId, numPlayers) as RuleConfig
      pendingRef.current = true
      const sent = gameSocket.send({
        type: 'create_room',
        mode_id: modeId,
        config,
        public: isPublic,
      })
      if (!sent) {
        pendingRef.current = false
        setBusy(false)
        setError(t('room.serverOffline'))
      }
    } catch (e) {
      setBusy(false)
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  return (
    <AppShell title={t('route.createRoom')}>
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-5">
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

        <section>
          <h2 className="mb-2.5 text-sm font-bold text-text-muted">{t('room.mode')}</h2>
          <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
            {MODES.map((m) => (
              <button
                key={m.id}
                type="button"
                onClick={() => {
                  setModeId(m.id)
                  setNumPlayers(m.defaultPlayers)
                }}
                className={[
                  'flex min-h-14 items-center gap-3 rounded-2xl bg-black/25 p-4 text-left transition',
                  modeId === m.id ? 'ring-1 ring-accent/60' : 'hover:bg-surface-strong',
                ].join(' ')}
              >
                <span className="text-2xl" aria-hidden="true">
                  {m.icon}
                </span>
                <span className="font-bold">{t(`mode.${m.id}` as TranslationKey)}</span>
              </button>
            ))}
          </div>
        </section>

        <section className="uno-page-panel flex flex-col gap-4 p-5">
          <div className="flex items-center justify-between gap-3">
            <label htmlFor="room-players" className="text-sm font-bold">
              {t('room.players')}: {numPlayers}
            </label>
            <input
              id="room-players"
              type="range"
              min={mode.minPlayers}
              max={mode.maxPlayers}
              step={mode.id === 'teams' ? 2 : 1}
              value={numPlayers}
              onChange={(e) => setNumPlayers(Number(e.target.value))}
              className="w-44 accent-accent"
            />
          </div>
          <button
            type="button"
            role="switch"
            aria-checked={isPublic}
            onClick={() => setIsPublic((v) => !v)}
            className="flex min-h-11 items-center justify-between rounded-xl bg-black/25 px-4 text-sm font-semibold"
          >
            <span>{isPublic ? t('room.public') : t('room.private')}</span>
            <span>{isPublic ? t('common.on') : t('common.off')}</span>
          </button>
        </section>

        {error && (
          <p role="alert" className="text-sm font-semibold text-red-300">
            {error}
          </p>
        )}
        <button
          type="button"
          disabled={!user || busy}
          onClick={() => void handleCreate()}
          className="min-h-12 rounded-2xl bg-btn-green px-5 text-base font-bold text-white disabled:opacity-40"
        >
          {busy ? t('room.creating') : t('room.create')}
        </button>
      </div>
    </AppShell>
  )
}
