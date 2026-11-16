import { useEffect, useState } from 'react'
import { Link } from 'react-router'
import { TurnTimer } from '@/components/dialogs/TurnTimer'
import type { LocalGameClock, LocalTurnTimer } from '@/game/GameClient'
import type { ModeKind } from '@/generated/ModeKind'
import type { Side } from '@/generated/Side'
import { useTranslation, type TranslationKey } from '@/i18n'
import { useSettings } from '@/settings/useSettings'

export interface TopBarProps {
  modeId: string
  mode: ModeKind
  round: number
  side: Side
  timer: LocalTurnTimer | null
  gameTimer?: LocalGameClock | null
  onLeaveRequest: () => void
  onOpenSettings?: () => void
}

function formatClock(ms: number): string {
  const totalSec = Math.ceil(Math.max(0, ms) / 1000)
  const mins = Math.floor(totalSec / 60)
  const secs = totalSec % 60
  return `${mins}:${String(secs).padStart(2, '0')}`
}

function GameClockBadge({ gameTimer }: { gameTimer: LocalGameClock }) {
  const { t } = useTranslation()
  const [now, setNow] = useState(() => Date.now())

  useEffect(() => {
    if (gameTimer.paused_remaining_ms !== undefined) return
    const id = window.setInterval(() => {
      setNow(Date.now())
    }, 250)
    return () => window.clearInterval(id)
  }, [gameTimer])

  const remainingMs = Math.max(
    0,
    gameTimer.paused_remaining_ms ?? gameTimer.deadline_ms - now,
  )
  const urgent = remainingMs <= 30_000
  const formatted = formatClock(remainingMs)

  return (
    <span
      aria-label={`${t('game.gameClockLabel')}: ${formatted}`}
      className={[
        'inline-flex items-center rounded-full px-2.5 py-1 text-xs font-bold tabular-nums',
        urgent
          ? 'bg-red-500/20 text-red-200 ring-1 ring-red-400/60'
          : 'bg-white/10 text-text-primary',
      ].join(' ')}
    >
      ⏱ {formatted}
    </span>
  )
}

export function TopBar({
  modeId,
  mode,
  round,
  side,
  timer,
  gameTimer = null,
  onLeaveRequest,
  onOpenSettings,
}: TopBarProps) {
  const { t } = useTranslation()
  const handSort = useSettings((s) => s.handSort)
  const setHandSort = useSettings((s) => s.setHandSort)

  const modeKey = `mode.${modeId}` as TranslationKey
  const modeLabel = t(modeKey)

  return (
    <header
      className="uno-game-header relative z-20 flex min-h-14 shrink-0 items-center justify-between gap-2 px-3 py-1.5 sm:px-5"
      style={{ paddingTop: 'max(0.25rem, env(safe-area-inset-top))' }}
    >
      <div className="flex min-w-0 items-center gap-2">
        <button
          type="button"
          onClick={onLeaveRequest}
          className="flex min-h-11 min-w-11 items-center justify-center rounded-xl border border-white/10 bg-white/5 px-2.5 text-lg font-bold text-text-primary transition hover:bg-white/10 active:scale-95"
          aria-label={t('game.menuLeave')}
        >
          ←
        </button>
        <div className="flex min-w-0 flex-wrap items-center gap-1.5">
          <span className="truncate text-sm font-bold tracking-[0.025em] text-text-primary sm:text-base">
            {modeLabel}
          </span>
          <span className="rounded-full border border-white/10 bg-white/5 px-2.5 py-0.5 text-[11px] font-semibold text-text-muted">
            {t('game.roundLabel', { round })}
          </span>
          {mode === 'flip' && (
            <span
              className={[
                'rounded-full px-2 py-0.5 text-xs font-bold',
                side === 'dark'
                  ? 'bg-purple-600/80 text-white'
                  : 'bg-amber-300/90 text-neutral-950',
              ].join(' ')}
            >
              {side === 'dark' ? t('game.sideDark') : t('game.sideLight')}
            </span>
          )}
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-1.5">
        {gameTimer && gameTimer.total_ms > 0 && (
          <GameClockBadge gameTimer={gameTimer} />
        )}
        <TurnTimer timer={timer} />
        <button
          type="button"
          onClick={() => setHandSort(handSort === 'color' ? 'number' : 'color')}
          className="hidden min-h-11 items-center rounded-xl border border-white/10 bg-white/5 px-3 text-xs font-semibold text-text-primary transition hover:bg-white/10 sm:flex"
        >
          {t('game.sortToggle', {
            mode:
              handSort === 'color'
                ? t('settings.sortColor')
                : t('settings.sortNumber'),
          })}
        </button>
        {onOpenSettings ? (
          <button
            type="button"
            onClick={onOpenSettings}
            className="flex min-h-11 min-w-11 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-lg transition hover:bg-white/10"
            aria-label={t('home.settings')}
          >
            ⚙
          </button>
        ) : (
          <Link
            to="/settings"
            className="flex min-h-11 min-w-11 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-lg transition hover:bg-white/10"
            aria-label={t('home.settings')}
          >
            ⚙
          </Link>
        )}
      </div>
    </header>
  )
}
