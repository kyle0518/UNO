import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useAuth } from '@/auth/useAuth'
import { AppShell } from '@/components/layout/AppShell'
import { useTranslation } from '@/i18n'

export function JoinRoomPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const user = useAuth((s) => s.user)
  const guestLogin = useAuth((s) => s.guestLogin)
  const [code, setCode] = useState('')
  const [nickname, setNickname] = useState('')
  const [error, setError] = useState<string | null>(null)

  const normalized = code.trim().toUpperCase()
  const valid = /^[A-Z0-9]{6}$/.test(normalized)

  const handleGuest = async () => {
    if (!nickname.trim()) return
    setError(null)
    try {
      await guestLogin(nickname.trim())
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  return (
    <AppShell title={t('route.joinRoom')}>
      <div className="mx-auto flex w-full max-w-md flex-col gap-5">
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

        <section className="uno-page-panel flex flex-col gap-3 p-5">
          <label htmlFor="room-code" className="text-sm font-bold">
            {t('room.joinCode')}
          </label>
          <input
            id="room-code"
            value={code}
            onChange={(e) => setCode(e.target.value.toUpperCase().slice(0, 6))}
            placeholder={t('room.codePlaceholder')}
            inputMode="text"
            autoCapitalize="characters"
            autoComplete="off"
            className="min-h-14 rounded-xl border border-white/15 bg-black/40 px-4 text-center font-display text-2xl font-bold tracking-[0.3em]"
          />
        </section>

        {error && (
          <p role="alert" className="text-sm font-semibold text-red-300">
            {error}
          </p>
        )}
        <button
          type="button"
          disabled={!user || !valid}
          onClick={() => void navigate(`/r/${normalized}`)}
          className="min-h-12 rounded-2xl bg-btn-green px-5 text-base font-bold text-white disabled:opacity-40"
        >
          {t('room.join')}
        </button>
      </div>
    </AppShell>
  )
}
