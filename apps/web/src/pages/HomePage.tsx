import { Link } from 'react-router'
import { AppShell } from '@/components/layout/AppShell'
import { useTranslation, type TranslationKey } from '@/i18n'

interface NavAction {
  to: string
  labelKey: TranslationKey
  accent?: boolean
}

const PRIMARY_ACTIONS: NavAction[] = [
  { to: '/solo', labelKey: 'home.quickSolo', accent: true },
  { to: '/rules', labelKey: 'home.rules' },
  { to: '/settings', labelKey: 'home.settings' },
]

export function HomePage() {
  const { t } = useTranslation()

  const rightSlot = (
    <span className="inline-block min-h-11 min-w-11" aria-hidden="true" />
  )

  return (
    <AppShell title={t('app.title')} showBack={false} rightSlot={rightSlot}>
      <section className="mx-auto flex w-full max-w-md flex-1 flex-col items-center justify-center gap-6 py-4">
        <div className="flex flex-col items-center gap-2 text-center">
          <div className="inline-flex h-20 w-20 items-center justify-center rounded-3xl bg-btn-red font-display text-3xl font-bold tracking-wider text-white uno-card-shadow">
            UNO
          </div>
          <p className="text-sm text-text-muted">{t('app.subtitle')}</p>
        </div>

        <nav
          aria-label={t('app.title')}
          className="grid w-full grid-cols-2 gap-3"
        >
          {PRIMARY_ACTIONS.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              className={`flex min-h-14 items-center justify-center rounded-2xl px-4 py-3 text-center text-base font-semibold transition active:scale-[0.98] ${
                item.accent
                  ? 'col-span-2 bg-btn-red text-white shadow-md hover:brightness-110'
                  : 'uno-surface hover:bg-surface-strong'
              }`}
            >
              {t(item.labelKey)}
            </Link>
          ))}
        </nav>
      </section>
    </AppShell>
  )
}
