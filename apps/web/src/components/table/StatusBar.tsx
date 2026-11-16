import type { Phase } from '@/generated/Phase'
import { useTranslation } from '@/i18n'
import { useSettings } from '@/settings/useSettings'

export interface StatusBarProps {
  isMyTurn: boolean
  activePlayerName: string
  playerName?: string
  handCount?: number
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
  playerName,
  handCount,
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

  const displayName = playerName || t('game.playerYou')

  return (
    <div
      className={[
        'uno-player-status flex min-h-14 shrink-0 items-center gap-2 px-3 sm:gap-4 sm:px-6',
        compact ? 'py-1' : 'py-2',
      ].join(' ')}
    >
      <div className="uno-player-identity flex shrink-0 items-center gap-2.5">
        <span
          className="uno-player-monogram inline-flex h-9 w-9 items-center justify-center rounded-full text-sm font-bold"
          aria-hidden="true"
        >
          {displayName.slice(0, 1).toUpperCase()}
        </span>
        <div className="uno-player-details flex min-w-0 flex-col leading-tight">
          <span className="max-w-24 truncate text-xs font-bold text-text-primary sm:text-sm">
            {displayName}
          </span>
          {handCount !== undefined && (
            <span className="mt-0.5 text-[11px] text-text-muted tabular-nums">
              {t('game.cardsCount', { count: handCount })}
            </span>
          )}
        </div>
      </div>

      <div className="flex min-w-0 flex-1 items-center justify-center gap-2">
        <p
          role="status"
          aria-live="polite"
          data-active={isMyTurn && !lastError}
          title={lastError ?? statusText}
          className={[
            'uno-status-pill min-w-0 truncate rounded-full px-3 py-1.5 text-center text-xs font-semibold sm:text-sm',
            lastError
              ? 'border-uno-red bg-uno-red/20 text-text-primary'
              : 'text-text-primary',
          ].join(' ')}
        >
          {lastError ?? statusText}
        </p>
        {myTeam !== null && (
          <span className="shrink-0 rounded-full border border-white/15 bg-white/10 px-2 py-0.5 text-[11px] font-bold text-text-primary">
            {t('game.myTeamBadge', { team: myTeam + 1 })}
          </span>
        )}
      </div>

      <button
        type="button"
        onClick={() => setHandSort(handSort === 'color' ? 'number' : 'color')}
        className="uno-sort-control flex min-h-11 shrink-0 items-center rounded-xl border border-white/10 bg-white/5 px-2.5 text-[11px] font-semibold text-text-primary transition hover:bg-white/10 sm:hidden"
      >
        {t('game.sortToggle', {
          mode: handSort === 'color'
            ? t('settings.sortColor')
            : t('settings.sortNumber'),
        })}
      </button>
    </div>
  )
}
