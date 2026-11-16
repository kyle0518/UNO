import { useTranslation } from '@/i18n'

export interface UnoButtonProps {
  enabled: boolean
  urgent: boolean
  layout?: 'portrait' | 'short' | 'desktop'
  onClick: () => void
}

const POSITION_CLASSES: Record<'portrait' | 'short' | 'desktop', string> = {
  portrait: 'h-16 w-16',
  short: 'h-14 w-14',
  desktop: 'h-[72px] w-[72px]',
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
        'uno-action-button relative z-30 flex shrink-0 items-center justify-center rounded-[22px] border text-base font-black tracking-tight transition-transform duration-150 active:scale-95',
        POSITION_CLASSES[layout],
        enabled
          ? urgent
            ? 'uno-action-button--urgent border-accent bg-uno-red text-white'
            : 'border-white/15 bg-btn-red text-white hover:-translate-y-1'
          : 'cursor-not-allowed border-white/10 bg-black/20 text-text-muted opacity-60',
      ].join(' ')}
      aria-label={t('game.unoButton')}
    >
      {t('game.unoButton')}
    </button>
  )
}
