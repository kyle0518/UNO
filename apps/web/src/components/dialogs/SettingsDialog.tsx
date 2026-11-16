import { useEffect } from 'react'
import { SettingsPanel } from '@/components/settings/SettingsPanel'
import { useTranslation } from '@/i18n'

export interface SettingsDialogProps {
  open: boolean
  onClose: () => void
}

export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const { t } = useTranslation()

  useEffect(() => {
    if (!open) return
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault()
        onClose()
      }
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [open, onClose])

  if (!open) return null

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-dialog-title"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose()
      }}
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4 backdrop-blur-xs"
    >
      <div className="flex max-h-[85dvh] w-full max-w-lg flex-col rounded-3xl uno-surface p-5 text-text-primary shadow-2xl">
        <div className="mb-3 flex shrink-0 items-center justify-between">
          <h2 id="settings-dialog-title" className="text-lg font-bold">
            {t('game.openSettings')}
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="min-h-11 rounded-xl bg-white/10 px-4 text-sm font-semibold hover:bg-white/20"
          >
            {t('common.close')}
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto pr-1">
          <SettingsPanel />
        </div>
      </div>
    </div>
  )
}
