import { motion } from 'motion/react'
import type { PublicPlayer } from '@/generated/PublicPlayer'
import { useTranslation } from '@/i18n'
import { animDurationMultiplier, useSettings } from '@/settings/useSettings'
import { getNextActiveSeat } from './direction'

export interface DirectionRingProps {
  direction: number
  currentSeat: number
  mySeat: number
  players: PublicPlayer[]
  playerNames: string[]
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
