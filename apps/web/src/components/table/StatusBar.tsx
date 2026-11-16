import type { Phase } from '@/generated/Phase'
import { useTranslation } from '@/i18n'
import { useSettings } from '@/settings/useSettings'

export interface StatusBarProps {
  isMyTurn: boolean
  activePlayerName: string
  phase: Phase
  lastError: string | null
  showSwipeHint: boolean
  meEliminated?: boolean
  myTeam?: number | null
  dealingRound?: number | null
  compact?: boolean
}

export function StatusBar({
  isMyTurn,
  activePlayerName,
  phase,
  lastError,
  showSwipeHint,
  meEliminated = false,
  myTeam = null,
  dealingRound = null,
  compact = false,
}: StatusBarProps) {
  const { t } = useTranslation()
  const handSort = useSettings((s) => s.handSort)
  const setHandSort = useSettings((s) => s.setHandSort)

  let statusText = meEliminated
    ? `${t('game.youEliminated')} · ${t('game.waitingTurn', { name: activePlayerName })}`
    : t('game.waitingTurn', { name: activePlayerName })
  if (dealingRound !== null) {
    statusText = t('game.dealingRound', { round: dealingRound })
  } else if (phase === 'round_over') {
    statusText = t('game.phaseRoundOver')
  } else if (phase === 'game_over') {
    statusText = t('game.phaseGameOver')
  } else if (isMyTurn) {
    switch (phase) {
      case 'choose_color':
        statusText = t('game.phaseChooseColor')
        break
      case 'choose_target':
        statusText = t('game.phaseChooseTarget')
        break
      case 'challenge':
        statusText = t('game.phaseChallenge')
        break
      case 'drawn_card_decision':
        statusText = t('game.phaseDrawnDecision')
        break
      case 'play':
        statusText = showSwipeHint ? t('game.swipeHint') : t('game.yourTurn')
        break
    }
  }

  return (
    <div
      className={[
        'flex shrink-0 items-center justify-between gap-2 px-4',
        compact ? 'py-0' : 'py-1',
      ].join(' ')}
    >
      <div className="flex min-w-0 items-center gap-2">
        <p
          role="status"
          aria-live="polite"
          data-active={isMyTurn && !lastError}
          className={[
            'uno-status-pill min-w-0 truncate rounded-full px-3 py-1.5 text-xs font-semibold sm:text-sm',
            lastError
              ? 'border-uno-red bg-uno-red/20 text-text-primary'
              : 'text-text-primary',
          ].join(' ')}
        >
          {lastError ?? statusText}
        </p>
        {myTeam !== null && (
          <span className="shrink-0 rounded-full border border-white/15 bg-white/10 px-2.5 py-0.5 text-xs font-bold text-text-primary">
            {t('game.myTeamBadge', { team: myTeam + 1 })}
          </span>
        )}
      </div>

      <button
        type="button"
        onClick={() => setHandSort(handSort === 'color' ? 'number' : 'color')}
        className="flex min-h-11 shrink-0 items-center rounded-xl border border-white/10 bg-white/5 px-3 text-xs font-semibold text-text-primary transition hover:bg-white/10 sm:hidden"
      >
        {t('game.sortToggle', {
          mode:
            handSort === 'color'
              ? t('settings.sortColor')
              : t('settings.sortNumber'),
        })}
      </button>
    </div>
  )
}
