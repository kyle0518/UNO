import { Link } from 'react-router'
import { AppShell } from '@/components/layout/AppShell'
import { useTranslation } from '@/i18n'

export function HomePage() {
  const { t } = useTranslation()

  return (
    <AppShell
      title={t('app.title')}
      showBack={false}
      rightSlot={<span className="inline-block min-h-11 min-w-11" aria-hidden="true" />}
    >
      <section className="mx-auto flex w-full max-w-5xl flex-1 items-center py-5 sm:py-10">
        <div className="uno-home-hero grid w-full md:grid-cols-[1.08fr_0.92fr]">
          <div className="flex flex-col justify-center gap-6 p-7 sm:gap-8 sm:p-12 lg:p-14">
            <span className="text-[11px] font-bold tracking-[0.28em] text-accent">
              UNO / WEB EDITION
            </span>

            <div>
              <h2 className="font-display text-[clamp(4.5rem,12vw,7.5rem)] leading-[0.9] font-bold tracking-[-0.07em] text-text-primary">
                UNO<span className="text-accent">.</span>
              </h2>
              <p className="mt-6 max-w-sm text-sm leading-7 text-text-muted sm:text-base">
                {t('app.subtitle')}
              </p>
            </div>

            <nav aria-label={t('app.title')} className="grid gap-3">
              <Link
                to="/solo"
                className="uno-primary-action flex min-h-14 items-center justify-between rounded-2xl px-5 py-3.5 text-base font-bold active:scale-[0.99]"
              >
                {t('home.quickSolo')}
                <span aria-hidden="true" className="text-xl">↗</span>
              </Link>
              <div className="grid grid-cols-2 gap-3">
                <Link
                  to="/rules"
                  className="uno-quiet-action inline-flex min-h-12 items-center justify-center rounded-2xl px-4 text-sm font-semibold"
                >
                  {t('home.rules')}
                </Link>
                <Link
                  to="/settings"
                  className="uno-quiet-action inline-flex min-h-12 items-center justify-center rounded-2xl px-4 text-sm font-semibold"
                >
                  {t('home.settings')}
                </Link>
              </div>
            </nav>
          </div>

          <div className="uno-home-art flex items-center justify-center" aria-hidden="true">
            <span className="uno-home-card uno-home-card--left" />
            <span className="uno-home-card uno-home-card--right" />
            <span className="uno-home-card uno-home-card--front">
              <span>UNO</span>
            </span>
          </div>
        </div>
      </section>
    </AppShell>
  )
}
