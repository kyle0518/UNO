import type { ReactNode } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import { useTranslation } from '@/i18n'

export interface AppShellProps {
  title: string
  children: ReactNode
  showBack?: boolean
  rightSlot?: ReactNode
}

export function AppShell({
  title,
  children,
  showBack,
  rightSlot,
}: AppShellProps) {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const location = useLocation()
  const shouldShowBack = showBack ?? location.pathname !== '/'

  const handleBack = () => {
    if (window.history.length > 1) {
      void navigate(-1)
    } else {
      void navigate('/')
    }
  }

  return (
    <div className="flex min-h-dvh flex-col uno-table-bg text-text-primary">
      <header
        className="sticky top-0 z-20 flex min-h-12 items-center justify-between gap-2 uno-surface px-3"
        style={{
          paddingTop: 'env(safe-area-inset-top)',
          paddingLeft: 'max(0.75rem, env(safe-area-inset-left))',
          paddingRight: 'max(0.75rem, env(safe-area-inset-right))',
        }}
      >
        <div className="flex min-w-11 items-center">
          {shouldShowBack ? (
            <button
              type="button"
              onClick={handleBack}
              aria-label={t('common.back')}
              className="inline-flex min-h-11 min-w-11 items-center justify-center rounded-xl px-2 text-sm font-medium transition hover:bg-surface-strong active:scale-95"
            >
              <span aria-hidden="true" className="text-lg">
                ←
              </span>
            </button>
          ) : (
            <Link
              to="/"
              aria-label={`UNO - ${t('common.home')}`}
              className="inline-flex min-h-11 min-w-11 items-center justify-center rounded-xl font-display text-lg font-bold tracking-wider text-accent"
            >
              UNO
            </Link>
          )}
        </div>

        <h1 className="truncate text-center text-base font-bold tracking-wide">
          {title}
        </h1>

        <div className="flex min-w-11 items-center justify-end gap-1">
          {rightSlot ?? (
            <Link
              to="/settings"
              aria-label={t('home.settings')}
              className="inline-flex min-h-11 min-w-11 items-center justify-center rounded-xl px-2 text-sm font-medium transition hover:bg-surface-strong active:scale-95"
            >
              ⚙
            </Link>
          )}
        </div>
      </header>

      <main
        className="mx-auto flex w-full max-w-4xl flex-1 flex-col px-4 py-5"
        style={{
          paddingBottom: 'max(1.25rem, env(safe-area-inset-bottom))',
          paddingLeft: 'max(1rem, env(safe-area-inset-left))',
          paddingRight: 'max(1rem, env(safe-area-inset-right))',
        }}
      >
        {children}
      </main>
    </div>
  )
}
