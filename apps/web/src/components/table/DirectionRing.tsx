import { motion } from 'motion/react'
import type { PublicPlayer } from '@/generated/PublicPlayer'
import { useTranslation } from '@/i18n'
import { animDurationMultiplier, useSettings } from '@/settings/useSettings'

export interface DirectionRingProps {
  direction: number
  currentSeat: number
  mySeat: number
  players: PublicPlayer[]
  playerNames: string[]
}

/** Find the next occupied seat in the actual playing order, skipping eliminations. */
export function getNextActiveSeat(
  players: PublicPlayer[],
  currentSeat: number,
  direction: number,
): number | null {
  const seats = players
    .filter((player) => !player.eliminated)
    .map((player) => player.seat)
    .sort((a, b) => a - b)

  if (seats.length <= 1) return null
  const index = seats.indexOf(currentSeat)
  if (index >= 0) {
    return seats[(index + (direction >= 0 ? 1 : -1) + seats.length) % seats.length] ?? null
  }

  // The current player can be eliminated as a round ends. Choose the next
  // surviving seat in the same direction rather than displaying a dead seat.
  if (direction >= 0) return seats.find((seat) => seat > currentSeat) ?? seats[0] ?? null
  return [...seats].reverse().find((seat) => seat < currentSeat) ?? seats[seats.length - 1] ?? null
}

/**
 * Direction is gameplay context, not a separate dashboard.
 * A compact chip remains visible beside the piles on every viewport.
 */
export function DirectionRing({
  direction,
  currentSeat,
  mySeat,
  players,
  playerNames,
}: DirectionRingProps) {
  const { t } = useTranslation()
  const animSpeed = useSettings((s) => s.animSpeed)
  const clockwise = direction >= 0
  const nextSeat = getNextActiveSeat(players, currentSeat, direction)
  const nextName = nextSeat === null
    ? null
    : nextSeat === mySeat
      ? t('game.playerYou')
      : playerNames[nextSeat] ?? t('game.playerAi', { seat: nextSeat })

  return (
    <aside
      className="uno-turn-cue"
      data-direction={clockwise ? 'clockwise' : 'counter-clockwise'}
      data-next-seat={nextSeat ?? undefined}
      aria-label={t('game.turnOrder')}
    >
      <motion.span
        key={clockwise ? 'clockwise' : 'counter-clockwise'}
        className="uno-turn-cue-arrow"
        aria-hidden="true"
        initial={{ scale: 0.65, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        transition={{
          duration: 0.25 * animDurationMultiplier(animSpeed),
          ease: 'easeOut',
        }}
      >
        {clockwise ? '↻' : '↺'}
      </motion.span>
      <span className="uno-turn-cue-copy">
        <strong>{clockwise ? t('game.forwardDir') : t('game.reverseDir')}</strong>
        {nextName && <small>{t('game.nextInOrder')} · {nextName}</small>}
      </span>
    </aside>
  )
}
