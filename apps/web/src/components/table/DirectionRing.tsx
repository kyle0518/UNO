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

/**
 * Turn route, not a decorative arrow: the sequence comes from the game
 * direction and skips eliminated seats. The current seat anchors the route.
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
  const duration = 0.35 * animDurationMultiplier(animSpeed)

  const activeSeats = players.filter((player) => !player.eliminated)
  const step = clockwise ? 1 : -1
  const ordered: PublicPlayer[] = []
  if (players.length > 0) {
    for (let i = 0; i < players.length; i += 1) {
      const seat = (currentSeat + i * step + players.length * 2) % players.length
      const player = players.find((p) => p.seat === seat)
      if (player && !player.eliminated) ordered.push(player)
    }
  }
  const first = ordered[0] ?? activeSeats[0]
  const next = ordered[1] ?? null
  const seatName = (seat: number) =>
    seat === mySeat
      ? t('game.playerYou')
      : playerNames[seat] ?? t('game.playerAi', { seat })

  return (
    <aside
      className="uno-route-panel"
      aria-label={t('game.turnOrder')}
      data-direction={clockwise ? 'clockwise' : 'counter-clockwise'}
    >
      <div className="uno-route-head">
        <div>
          <p className="uno-eyebrow">{t('game.turnOrder')}</p>
          <div className="uno-route-direction" aria-live="polite">
            <motion.span
              className="uno-route-glyph"
              aria-hidden="true"
              animate={{ rotate: clockwise ? 0 : -180 }}
              transition={{ duration, ease: 'easeInOut' }}
            >
              {clockwise ? '↻' : '↺'}
            </motion.span>
            <span>{clockwise ? t('game.forwardDir') : t('game.reverseDir')}</span>
          </div>
        </div>
        <div className="uno-route-flow" aria-hidden="true">
          <span className="uno-route-flow-dot" />
          <span className="uno-route-flow-line" />
          <span className="uno-route-flow-arrow">›</span>
        </div>
      </div>

      <div className="uno-route-players">
        {ordered.slice(0, 5).map((player, idx) => (
          <div
            key={player.seat}
            className="uno-route-item"
            data-active={idx === 0}
            data-next={idx === 1}
          >
            <span className="uno-route-node">{idx + 1}</span>
            <span className="uno-route-name">{seatName(player.seat)}</span>
            {idx < Math.min(ordered.length, 5) - 1 && (
              <span className="uno-route-separator" aria-hidden="true">›</span>
            )}
          </div>
        ))}
        {ordered.length > 5 && (
          <span className="uno-route-more">+{ordered.length - 5}</span>
        )}
      </div>

      <div className="uno-route-summary">
        <div>
          <span className="uno-route-summary-label">{t('game.onTurn')}</span>
          <strong className="uno-route-summary-name">
            {first ? seatName(first.seat) : '—'}
          </strong>
        </div>
        <span className="uno-route-summary-arrow" aria-hidden="true">→</span>
        <div>
          <span className="uno-route-summary-label">{t('game.nextInOrder')}</span>
          <strong className="uno-route-summary-name">
            {next ? seatName(next.seat) : '—'}
          </strong>
        </div>
      </div>
    </aside>
  )
}
