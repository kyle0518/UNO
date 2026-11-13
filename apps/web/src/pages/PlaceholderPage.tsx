import { Link, useParams } from 'react-router'
import { AppShell } from '@/components/layout/AppShell'
import { useTranslation, type TranslationKey } from '@/i18n'

export interface PlaceholderPageProps {
  titleKey: TranslationKey
}

export function PlaceholderPage({ titleKey }: PlaceholderPageProps) {
  const { t } = useTranslation()
  const params = useParams()
  const vars: Record<string, string> = {}
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined) vars[k] = v
  }
  const title = t(titleKey, vars)

  return (
    <AppShell title={title}>
      <section className="mx-auto flex w-full max-w-md flex-1 flex-col items-center justify-center gap-4 text-center">
        <div className="w-full rounded-3xl uno-surface p-6">
          <h2 className="text-xl font-bold">{title}</h2>
          <p className="mt-2 text-sm text-text-muted">
            {t('common.comingSoon')}
          </p>
          <Link
            to="/"
            className="mt-5 inline-flex min-h-11 items-center justify-center rounded-xl bg-uno-blue px-5 text-sm font-semibold text-white"
          >
            {t('common.home')}
          </Link>
        </div>
      </section>
    </AppShell>
  )
}
