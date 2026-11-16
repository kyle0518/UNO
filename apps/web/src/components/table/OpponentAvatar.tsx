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
  const previewBacks = player.hand_backs?.slice(0, 2) ?? null

  return (
    <div
      data-seat={player.seat}
      data-current={isCurrent}
      data-eliminated={player.eliminated}
      className={[
        'uno-opponent-chip relative flex shrink-0 flex-col gap-2 rounded-[18px]',
        compact ? 'min-w-[100px] px-2.5 py-2' : 'min-w-[130px] px-3 py-2.5',
        isTargetable ? 'ring-2 ring-accent' : '',
        isSkipped ? 'ring-2 ring-uno-red' : '',
      ].join(' ')}
    >
      <div className="flex items-center gap-2">
        <div
          className={[
            'relative flex h-9 w-9 shrink-0 items-center justify-center rounded-full border text-[11px] font-extrabold',
            isCurrent
              ? 'border-accent/60 bg-accent/20 text-accent'
              : 'border-white/10 bg-white/10 text-text-primary',
          ].join(' ')}
        >
          {name.slice(0, 2).toUpperCase()}
          {isCurrent && (
            <span
              className="absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full bg-accent ring-2 ring-[var(--app-bg)]"
              aria-hidden="true"
            />
          )}
        </div>
        <div className="min-w-0 flex-1 leading-tight">
          <p className="max-w-[78px] truncate text-xs font-bold text-text-primary">
            {name}
          </p>
          <p className="mt-1 whitespace-nowrap text-[11px] font-medium text-text-muted tabular-nums">
            {t('game.cardsCount', { count: player.hand_size })}
          </p>
        </div>
      </div>

      {previewBacks && previewBacks.length > 0 && !player.eliminated && (
        <div className="flex -space-x-5 pl-1" aria-label={t('game.cardsCount', { count: player.hand_size })}>
          {previewBacks.map((backFace, idx) => (
            <UnoCard
              key={idx}
              face={null}
              back={backFace}
              size="xs"
              className="pointer-events-none origin-top scale-[0.55]"
            />
          ))}
        </div>
      )}

      {player.team !== null && (
        <span className="self-start rounded-full bg-white/10 px-2 py-0.5 text-[10px] text-text-muted">
          {t('game.teamShort', { team: player.team + 1 })}
        </span>
      )}

      {player.uno_called && !player.eliminated && (
        <span className="self-start rounded-full bg-uno-red/90 px-2 py-0.5 text-[10px] font-extrabold tracking-wide text-white">
          {t('game.unoBadge')}
        </span>
      )}

      {player.eliminated && (
        <span className="self-start rounded-full bg-black/30 px-2 py-0.5 text-[10px] font-semibold text-text-muted">
          {t('game.eliminated')}
        </span>
      )}

      {isSkipped && (
        <span
          aria-label={t('game.skipPopup')}
          className="absolute -right-1 -top-1 flex h-5 w-5 items-center justify-center rounded-full bg-uno-red text-xs font-bold text-white shadow"
        >
          ⊘
        </span>
      )}

      {canCatch && onCatch && (
        <button
          type="button"
          onClick={() => onCatch(player.seat)}
          className="min-h-11 rounded-xl bg-uno-red px-3 py-2 text-xs font-extrabold text-white shadow-md transition hover:brightness-110 active:scale-95"
        >
          {t('game.catchButton')}
        </button>
      )}

      {isTargetable && onSelectTarget && (
        <button
          type="button"
          onClick={() => onSelectTarget(player.seat)}
          className="min-h-11 rounded-xl bg-accent px-3 py-2 text-sm font-extrabold text-neutral-950 transition hover:brightness-110 active:scale-95"
          aria-label={name}
        >
          ⇄
        </button>
      )}
    </div>
  )
}
