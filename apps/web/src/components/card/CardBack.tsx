import { useId } from 'react'
import type { CardFace } from '@/generated/CardFace'
import { useSettings } from '@/settings/useSettings'
import { COLOR_CSS_VAR, cornerLabel } from './cardMeta'

export interface CardBackProps {
  back?: CardFace
}

export function CardBack({ back }: CardBackProps) {
  const uid = useId()
  const cardBack = useSettings((s) => s.cardBack)
  const backColor = back?.color
    ? COLOR_CSS_VAR[back.color]
    : 'var(--uno-black)'

  return (
    <svg
      viewBox="0 0 120 180"
      data-card-back={cardBack}
      className="h-full w-full select-none"
      aria-hidden="true"
    >
      <rect
        x="0"
        y="0"
        width="120"
        height="180"
        rx="12"
        fill="var(--card-border-color, #ffffff)"
      />
      {cardBack === 'neon' ? (
        <>
          <rect x="8" y="8" width="104" height="164" rx="8" fill="#090c1a" />
          <rect
            x="14"
            y="14"
            width="92"
            height="152"
            rx="6"
            fill="none"
            stroke="#00e5ff"
            strokeWidth="2.5"
          />
          <polygon
            points="60,26 96,90 60,154 24,90"
            fill="#141838"
            stroke="#ff2a85"
            strokeWidth="2.5"
          />
          <text
            x="60"
            y="99"
            textAnchor="middle"
            fill="#00e5ff"
            stroke="#ff2a85"
            strokeWidth="1.5"
            paintOrder="stroke"
            fontFamily="var(--font-display)"
            fontSize="28"
            fontWeight="700"
            letterSpacing="2"
          >
            UNO
          </text>
        </>
      ) : cardBack === 'minimal' ? (
        <>
          <rect x="8" y="8" width="104" height="164" rx="8" fill="#1e293b" />
          <rect
            x="16"
            y="16"
            width="88"
            height="148"
            rx="5"
            fill="none"
            stroke="#94a3b8"
            strokeOpacity="0.45"
            strokeWidth="1.5"
          />
          <circle
            cx="60"
            cy="90"
            r="28"
            fill="none"
            stroke="#e2e8f0"
            strokeOpacity="0.25"
            strokeWidth="2"
          />
          <text
            x="60"
            y="97"
            textAnchor="middle"
            fill="#f8fafc"
            fontFamily="var(--font-display)"
            fontSize="24"
            fontWeight="700"
            letterSpacing="3"
          >
            UNO
          </text>
        </>
      ) : (
        <>
          <rect
            x="8"
            y="8"
            width="104"
            height="164"
            rx="8"
            fill="var(--uno-black)"
          />
          <ellipse
            cx="60"
            cy="90"
            rx="42"
            ry="64"
            transform="rotate(-30 60 90)"
            fill="var(--uno-red)"
          />
          <text
            x="60"
            y="101"
            textAnchor="middle"
            transform="rotate(-22 60 90)"
            fill="#ffffff"
            stroke="#212121"
            strokeWidth="3"
            paintOrder="stroke"
            fontFamily="var(--font-display)"
            fontSize="34"
            fontWeight="700"
            letterSpacing="1"
          >
            UNO
          </text>
        </>
      )}
      {back && (
        <g>
          <defs>
            <clipPath id={`flip-back-${uid}`}>
              <rect x="74" y="14" width="32" height="44" rx="5" />
            </clipPath>
          </defs>
          <rect
            x="74"
            y="14"
            width="32"
            height="44"
            rx="5"
            fill={backColor}
            stroke="#ffffff"
            strokeWidth="2.5"
          />
          <text
            x="90"
            y="42"
            textAnchor="middle"
            fill="#ffffff"
            fontFamily="var(--font-display)"
            fontSize="14"
            fontWeight="700"
            clipPath={`url(#flip-back-${uid})`}
          >
            {cornerLabel(back.face)}
          </text>
        </g>
      )}
    </svg>
  )
}
