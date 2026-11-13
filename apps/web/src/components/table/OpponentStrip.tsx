import type { PublicPlayer } from '@/generated/PublicPlayer'
import { useTranslation } from '@/i18n'
import { OpponentAvatar } from './OpponentAvatar'

export interface OpponentItem {
  player: PublicPlayer
  name: string
}

export interface OpponentStripProps {
  opponents: OpponentItem[]
  currentSeat: number
  skippedSeat: number | null
  catchableSeats: Set<number>
  targetableSeats: Set<number>
  compact?: boolean
  onCatch: (seat: number) => void
  onSelectTarget: (seat: number) => void
}

export function OpponentStrip({
  opponents,
  currentSeat,
  skippedSeat,
  catchableSeats,
  targetableSeats,
  compact = false,
  onCatch,
  onSelectTarget,
}: OpponentStripProps) {
  const { t } = useTranslation()
  return (
    <section
      aria-label={t('game.opponents')}
      className={[
        'flex shrink-0 items-start gap-2 overflow-x-auto px-3',
        compact ? 'py-1' : 'py-2',
      ].join(' ')}
    >
      {opponents.map(({ player, name }) => (
        <OpponentAvatar
          key={player.seat}
          player={player}
          name={name}
          compact={compact}
          isCurrent={currentSeat === player.seat}
          isSkipped={skippedSeat === player.seat}
          canCatch={catchableSeats.has(player.seat)}
          isTargetable={targetableSeats.has(player.seat)}
          onCatch={onCatch}
          onSelectTarget={onSelectTarget}
        />
      ))}
    </section>
  )
}
