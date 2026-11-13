import { useTranslation } from '@/i18n'

export interface UnoButtonProps {
  enabled: boolean
  urgent: boolean
  layout?: 'portrait' | 'short' | 'desktop'
  onClick: () => void
}

const POSITION_CLASSES: Record<'portrait' | 'short' | 'desktop', string> = {
  portrait: 'bottom-[216px] right-3',
  short: 'bottom-3 right-3',
  desktop: 'bottom-28 right-4',
}

export function UnoButton({
  enabled,
  urgent,
  layout = 'desktop',
  onClick,
}: UnoButtonProps) {
  const { t } = useTranslation()

  return (
    <button
      type="button"
      data-testid="uno-button"
      disabled={!enabled}
      onClick={onClick}
      className={[
        'fixed z-30 flex h-16 w-16 items-center justify-center rounded-full border-2 border-white/70 text-base font-black tracking-tight text-white shadow-2xl transition active:scale-95',
        POSITION_CLASSES[layout],
        enabled
          ? urgent
            ? 'bg-uno-red animate-pulse ring-4 ring-accent'
            : 'bg-btn-red hover:scale-105'
          : 'cursor-not-allowed bg-black/50 opacity-45',
      ].join(' ')}
      aria-label={t('game.unoButton')}
    >
      {t('game.unoButton')}
    </button>
  )
}
