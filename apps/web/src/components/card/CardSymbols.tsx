import type { Face } from '@/generated/Face'

interface CenterSymbolProps {
  face: Face
  ink: string
  darkPalette: boolean
}

function MiniCardStack({
  count,
  colors,
  stroke = '#ffffff',
}: {
  count: number
  colors: string[]
  stroke?: string
}) {
  const offsets =
    count === 1
      ? [{ x: 47, y: 66, r: 0 }]
      : count === 2
        ? [
            { x: 38, y: 72, r: -10 },
            { x: 56, y: 58, r: 10 },
          ]
        : [
            { x: 31, y: 74, r: -14 },
            { x: 43, y: 65, r: -4 },
            { x: 55, y: 60, r: 6 },
            { x: 65, y: 68, r: 15 },
          ]

  return (
    <g>
      {offsets.map((o, idx) => (
        <rect
          key={idx}
          x={o.x}
          y={o.y}
          width="24"
          height="36"
          rx="4"
          fill={colors[idx % colors.length]}
          stroke={stroke}
          strokeWidth="2.5"
          transform={`rotate(${o.r} ${o.x + 12} ${o.y + 18})`}
        />
      ))}
    </g>
  )
}

export function CenterSymbol({
  face,
  ink,
  darkPalette,
}: CenterSymbolProps) {
  const quadColors = darkPalette
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

  switch (face.k) {
    case 'number': {
      const underline = face.v === 6 || face.v === 9
      return (
        <g>
          <text
            x="60"
            y="108"
            textAnchor="middle"
            fill={ink}
            fontFamily="var(--font-display)"
            fontSize="58"
            fontWeight="700"
          >
            {face.v}
          </text>
          {underline && (
            <rect x="43" y="114" width="34" height="5" rx="2.5" fill={ink} />
          )}
        </g>
      )
    }

    case 'skip':
      return (
        <g fill="none" stroke={ink} strokeWidth="9" strokeLinecap="round">
          <circle cx="60" cy="90" r="26" />
          <line x1="42" y1="72" x2="78" y2="108" />
        </g>
      )

    case 'skip_everyone':
      return (
        <g fill="none" stroke={ink} strokeWidth="7" strokeLinecap="round">
          <circle cx="52" cy="94" r="20" />
          <line x1="38" y1="80" x2="66" y2="108" />
          <circle cx="70" cy="82" r="20" fill="#ffffff" />
          <circle cx="70" cy="82" r="20" />
          <line x1="56" y1="68" x2="84" y2="96" />
        </g>
      )

    case 'reverse':
      return (
        <g fill={ink}>
          <path d="M36 82 C44 64, 68 62, 80 74 L86 67 L88 86 L69 84 L74 79 C65 71, 49 73, 44 85 Z" />
          <path d="M84 98 C76 116, 52 118, 40 106 L34 113 L32 94 L51 96 L46 101 C55 109, 71 107, 76 95 Z" />
        </g>
      )

    case 'flip':
      return (
        <g>
          <rect
            x="46"
            y="68"
            width="28"
            height="42"
            rx="4"
            fill="none"
            stroke={ink}
            strokeWidth="4.5"
          />
          <path
            d="M34 86 C36 68, 52 58, 70 61 L67 55 L80 63 L67 70 L70 65 C55 63, 42 71, 40 86 Z"
            fill={ink}
          />
          <path
            d="M86 94 C84 112, 68 122, 50 119 L53 125 L40 117 L53 110 L50 115 C65 117, 78 109, 80 94 Z"
            fill={ink}
          />
        </g>
      )

    case 'draw_one':
      return <MiniCardStack count={1} colors={[ink]} />

    case 'draw_two':
      return <MiniCardStack count={2} colors={[ink, ink]} />

    case 'draw_four':
      return <MiniCardStack count={4} colors={[ink, ink, ink, ink]} />

    case 'draw_five':
      return (
        <g>
          <MiniCardStack count={2} colors={[ink, ink]} />
          <text
            x="60"
            y="128"
            textAnchor="middle"
            fill={ink}
            fontFamily="var(--font-display)"
            fontSize="24"
            fontWeight="700"
          >
            +5
          </text>
        </g>
      )

    case 'discard_all':
      return (
        <g>
          <MiniCardStack count={2} colors={[ink, ink]} />
          <text
            x="60"
            y="130"
            textAnchor="middle"
            fill={ink}
            fontFamily="var(--font-display)"
            fontSize="20"
            fontWeight="700"
          >
            ALL
          </text>
        </g>
      )

    case 'wild':
      return null

    case 'wild_draw_two':
      return <MiniCardStack count={2} colors={quadColors} stroke="#212121" />

    case 'wild_draw_four':
      return <MiniCardStack count={4} colors={quadColors} stroke="#212121" />

    case 'wild_draw_color':
      return (
        <g>
          <MiniCardStack count={4} colors={quadColors} stroke="#212121" />
          <circle
            cx="60"
            cy="90"
            r="14"
            fill="#ffffff"
            stroke="#212121"
            strokeWidth="3"
          />
          <text
            x="60"
            y="96"
            textAnchor="middle"
            fill="#212121"
            fontFamily="var(--font-display)"
            fontSize="16"
            fontWeight="700"
          >
            +C
          </text>
        </g>
      )

    case 'wild_reverse_draw_four':
      return (
        <g>
          <MiniCardStack count={4} colors={quadColors} stroke="#212121" />
          <circle
            cx="60"
            cy="90"
            r="17"
            fill="#ffffff"
            stroke="#212121"
            strokeWidth="3"
          />
          <text
            x="60"
            y="96"
            textAnchor="middle"
            fill="#212121"
            fontFamily="var(--font-display)"
            fontSize="16"
            fontWeight="700"
          >
            ↺+4
          </text>
        </g>
      )

    case 'wild_draw_six':
    case 'wild_draw_ten': {
      const label = face.k === 'wild_draw_six' ? '+6' : '+10'
      return (
        <g>
          <rect
            x="30"
            y="66"
            width="60"
            height="48"
            rx="12"
            fill="#212121"
            stroke="#ffffff"
            strokeWidth="3"
          />
          <text
            x="60"
            y="101"
            textAnchor="middle"
            fill="#ffffff"
            fontFamily="var(--font-display)"
            fontSize="30"
            fontWeight="700"
          >
            {label}
          </text>
        </g>
      )
    }

    case 'wild_color_roulette':
      return (
        <g>
          <circle
            cx="60"
            cy="90"
            r="26"
            fill="#ffffff"
            stroke="#212121"
            strokeWidth="4"
          />
          <text
            x="60"
            y="102"
            textAnchor="middle"
            fill="#212121"
            fontFamily="var(--font-display)"
            fontSize="34"
            fontWeight="700"
          >
            ?
          </text>
        </g>
      )
  }
}
