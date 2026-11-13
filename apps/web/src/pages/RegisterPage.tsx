import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'
import { useAuth } from '@/auth/useAuth'
import { AppShell } from '@/components/layout/AppShell'
import { useTranslation } from '@/i18n'

export function RegisterPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { user, serverStatus, loading, error, register, clearError } = useAuth()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault()
    if (!username.trim() || password.length < 6) return
    try {
      await register(username.trim(), password)
      void navigate('/')
    } catch {
      // error is captured in store
    }
  }

  return (
    <AppShell title={t('route.register')}>
      <section className="mx-auto flex w-full max-w-md flex-1 flex-col justify-center gap-4">
        {serverStatus === 'offline' && (
          <div className="rounded-2xl uno-surface p-4 text-sm text-text-muted">
            <p>{t('auth.offlineBanner')}</p>
            <Link
              to="/solo"
              className="mt-3 inline-flex min-h-11 items-center justify-center rounded-xl bg-btn-red px-4 text-sm font-semibold text-white"
            >
              {t('auth.playSolo')}
            </Link>
          </div>
        )}

        <form
          onSubmit={(e) => void handleSubmit(e)}
          className="flex flex-col gap-4 rounded-3xl uno-surface p-6"
        >
          <div className="flex flex-col gap-1.5">
            <label htmlFor="reg-username" className="text-sm font-semibold">
              {t('auth.username')}
            </label>
            <input
              id="reg-username"
              type="text"
              autoComplete="username"
              required
              minLength={3}
              maxLength={20}
              value={username}
              onChange={(e) => {
                clearError()
                setUsername(e.target.value)
              }}
              className="min-h-11 rounded-xl border border-surface-border bg-black/20 px-3 text-base outline-none focus:border-accent"
            />
            <span className="text-xs text-text-muted">
              {t('auth.usernameHint')}
            </span>
          </div>

          <div className="flex flex-col gap-1.5">
            <label htmlFor="reg-password" className="text-sm font-semibold">
              {t('auth.password')}
            </label>
            <input
              id="reg-password"
              type="password"
              autoComplete="new-password"
              required
              minLength={6}
              value={password}
              onChange={(e) => {
                clearError()
                setPassword(e.target.value)
              }}
              className="min-h-11 rounded-xl border border-surface-border bg-black/20 px-3 text-base outline-none focus:border-accent"
            />
            <span className="text-xs text-text-muted">
              {t('auth.passwordHint')}
            </span>
          </div>

          {error && (
            <p role="alert" className="text-sm font-medium text-text-error">
              {error}
            </p>
          )}

          <button
            type="submit"
            disabled={loading}
            className="min-h-11 rounded-xl bg-btn-green px-4 text-sm font-semibold text-white transition hover:brightness-110 disabled:opacity-60"
          >
            {loading
              ? t('common.loading')
              : user?.is_guest
                ? t('auth.upgradeSubmit')
                : t('auth.registerSubmit')}
          </button>

          <Link
            to="/login"
            className="text-center text-sm text-text-muted underline-offset-4 hover:underline"
          >
            {t('auth.switchToLogin')}
          </Link>
        </form>
      </section>
    </AppShell>
  )
}
