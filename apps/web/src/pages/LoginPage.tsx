import { useState, type FormEvent } from 'react'
import { Link, useNavigate } from 'react-router'
import { useAuth } from '@/auth/useAuth'
import { AppShell } from '@/components/layout/AppShell'
import { useTranslation } from '@/i18n'

export function LoginPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { serverStatus, loading, error, login, clearError } = useAuth()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault()
    if (!username.trim() || !password) return
    try {
      await login(username.trim(), password)
      void navigate('/')
    } catch {
      // error is captured in store
    }
  }

  return (
    <AppShell title={t('route.login')}>
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
            <label htmlFor="login-username" className="text-sm font-semibold">
              {t('auth.username')}
            </label>
            <input
              id="login-username"
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
          </div>

          <div className="flex flex-col gap-1.5">
            <label htmlFor="login-password" className="text-sm font-semibold">
              {t('auth.password')}
            </label>
            <input
              id="login-password"
              type="password"
              autoComplete="current-password"
              required
              minLength={6}
              value={password}
              onChange={(e) => {
                clearError()
                setPassword(e.target.value)
              }}
              className="min-h-11 rounded-xl border border-surface-border bg-black/20 px-3 text-base outline-none focus:border-accent"
            />
          </div>

          {error && (
            <p role="alert" className="text-sm font-medium text-text-error">
              {error}
            </p>
          )}

          <button
            type="submit"
            disabled={loading}
            className="min-h-11 rounded-xl bg-btn-blue px-4 text-sm font-semibold text-white transition hover:brightness-110 disabled:opacity-60"
          >
            {loading ? t('common.loading') : t('auth.loginSubmit')}
          </button>

          <Link
            to="/register"
            className="text-center text-sm text-text-muted underline-offset-4 hover:underline"
          >
            {t('auth.switchToRegister')}
          </Link>
        </form>
      </section>
    </AppShell>
  )
}
