import { AppShell } from '@/components/layout/AppShell'
import { SettingsPanel } from '@/components/settings/SettingsPanel'
import { useTranslation } from '@/i18n'

export function SettingsPage() {
  const { t } = useTranslation()
  return (
    <AppShell title={t('route.settings')}>
      <SettingsPanel />
    </AppShell>
  )
}
