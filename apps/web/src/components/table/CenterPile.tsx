import { motion } from 'motion/react'
import { COLOR_BLIND_SYMBOL, COLOR_CSS_VAR } from '@/components/card/cardMeta'
import { UnoCard } from '@/components/card/UnoCard'
import { activeCardFace, backCardFace } from '@/game/handSort'
import type { Card } from '@/generated/Card'
import type { CardFace } from '@/generated/CardFace'
import type { Color } from '@/generated/Color'
import type { Side } from '@/generated/Side'
import { useTranslation, type TranslationKey } from '@/i18n'
import { animDurationMultiplier, useSettings } from '@/settings/useSettings'

export interface CenterPileProps {
  topCard: Card
  side: Side
  activeColor: Color | null
  drawPileSize: number
  drawPileTopBack: CardFace | null
  pendingDraw: number
  canDraw: boolean
  colorRipple: { color: Color; seq: number } | null
  flipping: boolean
  large?: boolean
  size?: 'sm' | 'md' | 'lg'
  onDraw: () => void
}

export function CenterPile({
  topCard,
  side,
  activeColor,
  drawPileSize,
  drawPileTopBack,
  pendingDraw,
  canDraw,
  colorRipple,
  flipping,
  large = false,
  size,
  onDraw,
}: CenterPileProps) {
  const { t } = useTranslation()
  const animSpeed = useSettings((s) => s.animSpeed)
  const colorBlind = useSettings((s) => s.colorBlind)
  const useAnim = animSpeed !== 'off'
  const flipSec = 0.7 * animDurationMultiplier(animSpeed)

  const topFace = activeCardFace(topCard, side)
  const topBack = backCardFace(topCard, side)
  const effectiveColor = activeColor ?? topFace.color
  const glowColor = effectiveColor ? COLOR_CSS_VAR[effectiveColor] : 'var(--accent)'
  const cardSize = size ?? (large ? 'lg' : 'md')

  return (
    <motion.div
      animate={flipping && useAnim ? { rotateY: [0, 90, 0] } : { rotateY: 0 }}
      transition={{ duration: flipSec || 0.01 }}
      className={[
        'relative z-10 flex flex-col items-center',
        cardSize === 'sm' ? 'gap-1' : 'gap-2.5',
      ].join(' ')}
    >
      <div className="flex items-center gap-5 sm:gap-8">
        <div className="relative flex flex-col items-center gap-1">
          <button
            type="button"
            data-testid="draw-pile"
            disabled={!canDraw}
            onClick={onDraw}
            className={[
              'relative rounded-[14%] p-0.5 transition focus-visible:outline-none',
              canDraw
                ? 'cursor-pointer ring-3 ring-accent shadow-[0_0_18px_var(--accent)] hover:scale-105 active:scale-95'
                : 'cursor-default opacity-80',
            ].join(' ')}
          >
            <span className="sr-only">
              {t('game.drawPile', { count: drawPileSize })}
            </span>
            <span aria-hidden="true" className="pointer-events-none block">
              <UnoCard
                face={null}
                back={drawPileTopBack ?? undefined}
                size={cardSize}
                state="normal"
              />
            </span>
          </button>
          <span className="rounded-full bg-black/45 px-2.5 py-0.5 text-xs font-bold text-text-primary tabular-nums">
            {drawPileSize}
          </span>
        </div>

        <div className="relative flex flex-col items-center gap-1">
          <div
            className="pointer-events-none absolute -inset-4 rounded-3xl opacity-55 blur-xl transition-colors duration-300"
            style={{ backgroundColor: glowColor }}
            aria-hidden="true"
          />

          {colorRipple && useAnim && (
            <motion.div
              key={colorRipple.seq}
              initial={{ scale: 0.7, opacity: 0.85 }}
              animate={{ scale: 1.9, opacity: 0 }}
              transition={{ duration: 0.4 * animDurationMultiplier(animSpeed) }}
              className="pointer-events-none absolute inset-0 rounded-2xl border-4"
              style={{ borderColor: COLOR_CSS_VAR[colorRipple.color] }}
              aria-hidden="true"
            />
          )}

          <motion.div
            key={`discard-${topCard.id}`}
            layoutId={useAnim ? `card-${topCard.id}` : undefined}
            initial={useAnim ? { scale: 1.12, rotate: (topCard.id % 2 === 0 ? 1 : -1) * 8 } : false}
            animate={{ scale: 1, rotate: ((topCard.id % 7) - 3) * 2 }}
            transition={{ duration: 0.35 * animDurationMultiplier(animSpeed) }}
            role="group"
            aria-label={t('game.discardPile')}
            className="relative"
          >
            <UnoCard face={topFace} back={topBack} size={cardSize} />
          </motion.div>

          {effectiveColor && (
            <div className="relative z-10 flex items-center gap-1.5 rounded-full bg-black/55 px-2.5 py-0.5 text-xs font-bold text-white shadow">
              <span
                className="inline-block h-3 w-3 rounded-full ring-1 ring-white/50"
                style={{ backgroundColor: glowColor }}
                aria-hidden="true"
              />
              {colorBlind && (
                <span aria-hidden="true">{COLOR_BLIND_SYMBOL[effectiveColor]}</span>
              )}
              <span>{t(`color.${effectiveColor}` as TranslationKey)}</span>
            </div>
          )}
        </div>
      </div>

      {pendingDraw > 0 && (
        <div
          data-testid="pending-draw-badge"
          className="animate-bounce rounded-full bg-uno-red px-4 py-1 text-sm font-black text-white shadow-lg ring-2 ring-white/50"
        >
          {t('game.pendingDraw', { count: pendingDraw })}
        </div>
      )}
    </motion.div>
  )
}
