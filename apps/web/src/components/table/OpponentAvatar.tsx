import { UnoCard } from '@/components/card/UnoCard'
import type { PublicPlayer } from '@/generated/PublicPlayer'
import { useTranslation } from '@/i18n'

export interface OpponentAvatarProps {
  player: PublicPlayer
  name: string
  isCurrent: boolean
  isNext?: boolean
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
  isNext = false,
  isSkipped,
  canCatch,
  isTargetable,
  compact = false,
  onCatch,
  onSelectTarget,
}: OpponentAvatarProps) {
  const { t } = useTranslation()
  const backPreview = player.hand_backs?.[0] ?? null

  return (
    <div
      className="uno-seat"
      data-seat={player.seat}
      data-current={isCurrent}
      data-next={isNext}
      data-eliminated={player.eliminated}
      data-compact={compact}
      data-targetable={isTargetable}
    >
      <div className="uno-seat-portrait">
        <span className="uno-seat-avatar" aria-hidden="true">
          {name.slice(0, 2).toUpperCase()}
        </span>
        <span
          className="uno-seat-hand-count"
          aria-label={t('game.cardsCount', { count: player.hand_size })}
        >
          {player.hand_size}
        </span>
        {backPreview && !player.eliminated && (
          <span className="uno-seat-back-preview" aria-hidden="true">
            <UnoCard face={null} back={backPreview} size="xs" />
          </span>
        )}
        {isCurrent && <span className="uno-seat-turn-dot" aria-hidden="true" />}
      </div>

      <div className="uno-seat-meta">
        <strong className="uno-seat-name" title={name}>{name}</strong>
        {player.team !== null && (
          <span className="uno-seat-team">
            {t('game.teamShort', { team: player.team + 1 })}
          </span>
        )}
        {player.uno_called && !player.eliminated && (
          <span className="uno-seat-uno">{t('game.unoBadge')}</span>
        )}
        {player.eliminated && (
          <span className="uno-seat-eliminated">{t('game.eliminated')}</span>
        )}
      </div>

      {isSkipped && (
        <span className="uno-seat-skipped" aria-label={t('game.skipPopup')}>⊘</span>
      )}

      {canCatch && onCatch && (
        <button
          type="button"
          className="uno-seat-action uno-seat-action--catch"
          onClick={() => onCatch(player.seat)}
        >
          {t('game.catchButton')}
        </button>
      )}

      {isTargetable && onSelectTarget && (
        <button
          type="button"
          className="uno-seat-action uno-seat-action--target"
          aria-label={name}
          onClick={() => onSelectTarget(player.seat)}
        >
          ⇄
        </button>
      )}
    </div>
  )
}
