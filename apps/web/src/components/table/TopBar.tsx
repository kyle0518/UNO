import { Link } from 'react-router'
import { TurnTimer } from '@/components/dialogs/TurnTimer'
import type { LocalTurnTimer } from '@/game/GameClient'
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
  onLeaveRequest: () => void
}

export function TopBar({
  modeId,
  mode,
  round,
  side,
  timer,
  onLeaveRequest,
}: TopBarProps) {
  const { t } = useTranslation()
  const handSort = useSettings((s) => s.handSort)
  const setHandSort = useSettings((s) => s.setHandSort)

  const modeKey = `mode.${modeId}` as TranslationKey
  const modeLabel = t(modeKey)

  return (
    <header
      className="flex min-h-12 shrink-0 items-center justify-between gap-2 border-b border-white/10 bg-black/25 px-3 py-1 backdrop-blur-md"
      style={{ paddingTop: 'max(0.25rem, env(safe-area-inset-top))' }}
    >
      <div className="flex items-center gap-2 min-w-0">
        <button
          type="button"
          onClick={onLeaveRequest}
          className="flex min-h-11 min-w-11 items-center justify-center rounded-xl bg-white/10 px-2.5 text-sm font-bold text-text-primary transition hover:bg-white/20 active:scale-95"
          aria-label={t('game.menuLeave')}
        >
          ←
        </button>
        <div className="flex flex-wrap items-center gap-1.5 min-w-0">
          <span className="truncate text-sm font-bold text-text-primary">
            {modeLabel}
          </span>
          <span className="rounded-full bg-white/10 px-2 py-0.5 text-xs font-semibold text-text-muted">
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

      <div className="flex items-center gap-1.5 shrink-0">
        <TurnTimer timer={timer} />
        <button
          type="button"
          onClick={() => setHandSort(handSort === 'color' ? 'number' : 'color')}
          className="hidden min-h-11 items-center rounded-xl bg-white/10 px-2.5 text-xs font-semibold text-text-primary transition hover:bg-white/20 sm:flex"
        >
          {t('game.sortToggle', {
            mode:
              handSort === 'color'
                ? t('settings.sortColor')
                : t('settings.sortNumber'),
          })}
        </button>
        <Link
          to="/settings"
          className="flex min-h-11 min-w-11 items-center justify-center rounded-xl bg-white/10 text-base transition hover:bg-white/20"
          aria-label={t('home.settings')}
        >
          ⚙
        </Link>
      </div>
    </header>
  )
}
