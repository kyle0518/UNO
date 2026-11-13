import { motion } from 'motion/react'
import { useTranslation } from '@/i18n'
import { animDurationMultiplier, useSettings } from '@/settings/useSettings'

export interface DirectionRingProps {
  direction: number
  large?: boolean
}

export function DirectionRing({
  direction,
  large = false,
}: DirectionRingProps) {
  const { t } = useTranslation()
  const animSpeed = useSettings((s) => s.animSpeed)
  const isCounter = direction < 0
  const durationSec = 0.5 * animDurationMultiplier(animSpeed)

  return (
    <motion.div
      role="img"
      aria-label={isCounter ? t('game.reverseDir') : t('game.forwardDir')}
      animate={{ rotateY: isCounter ? 180 : 0 }}
      transition={{ duration: durationSec, ease: 'easeInOut' }}
      className="pointer-events-none absolute inset-0 flex items-center justify-center"
    >
      <svg
        viewBox="0 0 260 190"
        className={[
          'h-full w-full opacity-40',
          large ? 'max-w-[380px]' : 'max-w-[300px]',
        ].join(' ')}
        aria-hidden="true"
      >
        <ellipse
          cx="130"
          cy="95"
          rx="118"
          ry="82"
          fill="none"
          stroke="var(--accent)"
          strokeWidth="2.5"
          strokeDasharray="10 8"
        />
        <path
          d="M 238 80 L 248 95 L 232 100"
          fill="none"
          stroke="var(--accent)"
          strokeWidth="3"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
        <path
          d="M 22 110 L 12 95 L 28 90"
          fill="none"
          stroke="var(--accent)"
          strokeWidth="3"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    </motion.div>
  )
}
