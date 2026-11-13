import { useId, type CSSProperties } from 'react'
import type { CardFace } from '@/generated/CardFace'
import { useSettings } from '@/settings/useSettings'
import { CardBack } from './CardBack'
import {
  COLOR_BLIND_SYMBOL,
  COLOR_CSS_VAR,
  cornerLabel,
  formatCardLabel,
  isDarkSideCard,
} from './cardMeta'
import { CenterSymbol } from './CardSymbols'

export type CardSize = 'xs' | 'sm' | 'md' | 'lg'
export type CardState = 'normal' | 'playable' | 'selected' | 'disabled'

export interface UnoCardProps {
  face: CardFace | null
  back?: CardFace
  size: CardSize
  state?: CardState
  colorBlind?: boolean
  onClick?: () => void
  className?: string
}

const SIZE_CLASSES: Record<CardSize, string> = {
  xs: 'w-9 h-[54px]',
  sm: 'w-14 h-[84px]',
  md: 'w-20 h-[120px]',
  lg: 'w-[110px] h-[165px]',
}

const STATE_CLASSES: Record<CardState, string> = {
  normal: '',
  playable:
    '-translate-y-2 ring-2 ring-accent shadow-[0_0_16px_var(--accent)]',
  selected:
    '-translate-y-6 ring-3 ring-accent shadow-[0_0_20px_var(--accent)]',
  disabled: 'cursor-not-allowed bg-[#151a17]',
}

export function UnoCard({
  face,
  back,
  size,
  state = 'normal',
  colorBlind,
  onClick,
  className = '',
}: UnoCardProps) {
  const uid = useId()
  const lang = useSettings((s) => s.lang)
  const settingsColorBlind = useSettings((s) => s.colorBlind)
  const showColorBlind = colorBlind ?? settingsColorBlind
  const ariaLabel = formatCardLabel(face, back, lang)

  const glowColor = face?.color ? COLOR_CSS_VAR[face.color] : 'var(--accent)'
  const style = {
    '--uno-card-glow': glowColor,
  } as CSSProperties

  const wrapperClasses = [
    'relative inline-block shrink-0 select-none transition-transform duration-150',
    'rounded-[10%/6.67%] uno-card-shadow',
    SIZE_CLASSES[size],
    STATE_CLASSES[state],
    onClick && state !== 'disabled' ? 'cursor-pointer active:scale-95' : '',
    className,
  ]
    .filter(Boolean)
    .join(' ')

  const contentInner = face ? (
    <CardFront
      uid={uid}
      cardFace={face}
      showColorBlind={showColorBlind}
    />
  ) : (
    <CardBack back={back} />
  )

  const content =
    state === 'disabled' ? (
      <span className="block h-full w-full opacity-[0.58] saturate-[0.4]">
        {contentInner}
      </span>
    ) : (
      contentInner
    )

  if (onClick) {
    return (
      <button
        type="button"
        disabled={state === 'disabled'}
        onClick={onClick}
        style={style}
        className={wrapperClasses}
      >
        <span className="sr-only">{ariaLabel}</span>
        <span aria-hidden="true" className="pointer-events-none block h-full w-full">
          {content}
        </span>
      </button>
    )
  }

  return (
    <div
      role="img"
      aria-label={ariaLabel}
      style={style}
      className={wrapperClasses}
    >
      {content}
    </div>
  )
}

function CardFront({
  uid,
  cardFace,
  showColorBlind,
}: {
  uid: string
  cardFace: CardFace
  showColorBlind: boolean
}) {
  const { color, face } = cardFace
  const bgFill = color ? COLOR_CSS_VAR[color] : 'var(--uno-black)'
  const darkPalette = isDarkSideCard(color, face)
  const corner = cornerLabel(face)
  const cornerFontSize = corner.length >= 3 ? 13 : corner.length === 2 ? 16 : 19
  const cbSymbol = color ? COLOR_BLIND_SYMBOL[color] : null
  const ink = color ? COLOR_CSS_VAR[color] : '#212121'
  const clipId = `oval-${uid}`

  const quad = darkPalette
    ? [
        'var(--flip-pink)',
        'var(--flip-teal)',
        'var(--flip-orange)',
        'var(--flip-purple)',
      ]
    : [
        'var(--uno-red)',
        'var(--uno-blue)',
        'var(--uno-yellow)',
        'var(--uno-green)',
      ]

  return (
    <svg
      viewBox="0 0 120 180"
      className="h-full w-full select-none"
      aria-hidden="true"
    >
      <defs>
        <clipPath id={clipId}>
          <ellipse
            cx="60"
            cy="90"
            rx="42"
            ry="64"
            transform="rotate(-30 60 90)"
          />
        </clipPath>
      </defs>

      <rect
        x="0"
        y="0"
        width="120"
        height="180"
        rx="12"
        fill="var(--card-border-color, #ffffff)"
      />
      <rect x="8" y="8" width="104" height="164" rx="8" fill={bgFill} />

      {color ? (
        <ellipse
          cx="60"
          cy="90"
          rx="42"
          ry="64"
          transform="rotate(-30 60 90)"
          fill="#ffffff"
        />
      ) : (
        <g clipPath={`url(#${clipId})`}>
          <rect x="0" y="0" width="60" height="90" fill={quad[0]} />
          <rect x="60" y="0" width="60" height="90" fill={quad[1]} />
          <rect x="0" y="90" width="60" height="90" fill={quad[2]} />
          <rect x="60" y="90" width="60" height="90" fill={quad[3]} />
        </g>
      )}

      <CenterSymbol face={face} ink={ink} darkPalette={darkPalette} />

      <g fill="#ffffff" stroke="#111111" strokeWidth="1.2" paintOrder="stroke">
        <text
          x="16"
          y="30"
          fontFamily="var(--font-display)"
          fontSize={cornerFontSize}
          fontWeight="700"
        >
          {corner}
        </text>
        {showColorBlind && cbSymbol && (
          <text x="16" y="47" fontSize="13" fontWeight="700">
            {cbSymbol}
          </text>
        )}
      </g>

      <g
        transform="rotate(180 60 90)"
        fill="#ffffff"
        stroke="#111111"
        strokeWidth="1.2"
        paintOrder="stroke"
      >
        <text
          x="16"
          y="30"
          fontFamily="var(--font-display)"
          fontSize={cornerFontSize}
          fontWeight="700"
        >
          {corner}
        </text>
        {showColorBlind && cbSymbol && (
          <text x="16" y="47" fontSize="13" fontWeight="700">
            {cbSymbol}
          </text>
        )}
      </g>
    </svg>
  )
}
