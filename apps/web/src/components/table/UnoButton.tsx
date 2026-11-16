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
        'fixed z-30 flex h-16 w-16 items-center justify-center rounded-full border border-white/45 text-base font-black tracking-tight text-white shadow-xl transition active:scale-95',
        POSITION_CLASSES[layout],
        enabled
          ? urgent
            ? 'bg-uno-red ring-2 ring-accent shadow-[0_0_20px_var(--accent)]'
            : 'bg-btn-red hover:scale-105'
          : 'cursor-not-allowed border-white/20 bg-black/30 opacity-45',
      ].join(' ')}
      aria-label={t('game.unoButton')}
    >
      {t('game.unoButton')}
    </button>
  )
}
