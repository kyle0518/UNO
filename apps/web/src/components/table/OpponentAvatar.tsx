import { UnoCard } from '@/components/card/UnoCard'
import type { PublicPlayer } from '@/generated/PublicPlayer'
import { useTranslation } from '@/i18n'

export interface OpponentAvatarProps {
  player: PublicPlayer
  name: string
  isCurrent: boolean
  isSkipped: boolean
  canCatch: boolean
  isTargetable: boolean
  compact?: boolean
  onCatch?: (seat: number) => void
  onSelectTarget?: (seat: number) => void
}

export function OpponentAvatar({
  player,
  name,
  isCurrent,
  isSkipped,
  canCatch,
  isTargetable,
  compact = false,
  onCatch,
  onSelectTarget,
}: OpponentAvatarProps) {
  const { t } = useTranslation()
  const previewBacks = player.hand_backs?.slice(0, 3) ?? null
  const fanCount = Math.min(player.hand_size, 3)

  return (
    <div
      data-seat={player.seat}
      className={[
        'relative flex shrink-0 flex-col items-center gap-1 rounded-2xl transition',
        compact ? 'min-w-[78px] px-2 py-1' : 'min-w-[92px] px-2.5 py-2',
        player.eliminated ? 'opacity-45 grayscale' : 'uno-surface',
        isCurrent ? 'ring-2 ring-accent shadow-[0_0_14px_var(--accent)]' : '',
        isTargetable ? 'ring-2 ring-uno-yellow animate-pulse' : '',
        isSkipped ? 'animate-bounce ring-2 ring-uno-red' : '',
      ].join(' ')}
    >
      <div className="flex items-center gap-1.5">
        <div
          className={[
            'flex h-7 w-7 items-center justify-center rounded-full text-xs font-extrabold',
            isCurrent
              ? 'bg-accent text-neutral-950'
              : 'bg-white/15 text-text-primary',
          ].join(' ')}
        >
          {name.slice(0, 2).toUpperCase()}
        </div>
        <div className="flex flex-col leading-tight">
          <span className="max-w-[68px] truncate text-xs font-bold text-text-primary">
            {name}
          </span>
          <span className="text-[11px] font-semibold text-text-muted tabular-nums">
            {t('game.cardsCount', { count: player.hand_size })}
            {player.team !== null
              ? ` · ${t('game.teamShort', { team: player.team + 1 })}`
              : ''}
          </span>
        </div>
      </div>

      {!compact && !player.eliminated && fanCount > 0 && (
        <div className="flex items-center -space-x-6 pt-0.5">
          {previewBacks
            ? previewBacks.map((backFace, idx) => (
                <UnoCard
                  key={idx}
                  face={null}
                  back={backFace}
                  size="xs"
                  className="pointer-events-none scale-[0.72] origin-top"
                />
              ))
            : Array.from({ length: fanCount }, (_, idx) => (
                <UnoCard
                  key={idx}
                  face={null}
                  size="xs"
                  className="pointer-events-none scale-[0.65] origin-top"
                />
              ))}
        </div>
      )}

      {player.uno_called && !player.eliminated && (
        <span className="rounded-full bg-uno-red px-2 py-0.5 text-[10px] font-black tracking-wider text-white shadow">
          {t('game.unoBadge')}
        </span>
      )}

      {player.eliminated && (
        <span className="rounded-full bg-black/50 px-2 py-0.5 text-[10px] font-bold text-text-muted">
          {t('game.eliminated')}
        </span>
      )}

      {isSkipped && (
        <span
          aria-label={t('game.skipPopup')}
          className="absolute -right-1 -top-1 flex h-6 w-6 items-center justify-center rounded-full bg-uno-red text-xs font-bold text-white shadow"
        >
          ⊘
        </span>
      )}

      {canCatch && onCatch && (
        <button
          type="button"
          onClick={() => onCatch(player.seat)}
          className="mt-0.5 min-h-11 min-w-11 rounded-xl bg-btn-red px-3 py-1 text-xs font-black text-white shadow-lg transition hover:brightness-110 active:scale-95"
        >
          {t('game.catchButton')}
        </button>
      )}

      {isTargetable && onSelectTarget && (
        <button
          type="button"
          onClick={() => onSelectTarget(player.seat)}
          className="mt-0.5 min-h-11 min-w-11 rounded-xl bg-accent px-3 py-1 text-xs font-black text-neutral-950 shadow-lg transition hover:brightness-110 active:scale-95"
        >
          ⇄
        </button>
      )}
    </div>
  )
}
