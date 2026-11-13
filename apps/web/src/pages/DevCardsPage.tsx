import { useState } from 'react'
import { UnoCard } from '@/components/card/UnoCard'
import { AppShell } from '@/components/layout/AppShell'
import type { CardFace } from '@/generated/CardFace'

const CLASSIC_SAMPLES: CardFace[] = [
  { color: 'red', face: { k: 'number', v: 0 } },
  { color: 'yellow', face: { k: 'number', v: 6 } },
  { color: 'green', face: { k: 'number', v: 9 } },
  { color: 'blue', face: { k: 'skip' } },
  { color: 'red', face: { k: 'reverse' } },
  { color: 'green', face: { k: 'draw_two' } },
  { color: null, face: { k: 'wild' } },
  { color: null, face: { k: 'wild_draw_four' } },
]

const FLIP_SAMPLES: CardFace[] = [
  { color: 'yellow', face: { k: 'draw_one' } },
  { color: 'blue', face: { k: 'flip' } },
  { color: null, face: { k: 'wild_draw_two' } },
  { color: 'pink', face: { k: 'number', v: 5 } },
  { color: 'teal', face: { k: 'draw_five' } },
  { color: 'orange', face: { k: 'skip_everyone' } },
  { color: 'purple', face: { k: 'reverse' } },
  { color: null, face: { k: 'wild_draw_color' } },
]

const NO_MERCY_SAMPLES: CardFace[] = [
  { color: 'red', face: { k: 'draw_four' } },
  { color: 'blue', face: { k: 'discard_all' } },
  { color: null, face: { k: 'wild_reverse_draw_four' } },
  { color: null, face: { k: 'wild_draw_six' } },
  { color: null, face: { k: 'wild_draw_ten' } },
  { color: null, face: { k: 'wild_color_roulette' } },
]

export function DevCardsPage() {
  const [colorBlind, setColorBlind] = useState(false)

  return (
    <AppShell title="Dev: All Cards">
      <div className="flex flex-col gap-6">
        <div className="flex flex-wrap items-center gap-3">
          <button
            type="button"
            onClick={() => setColorBlind((v) => !v)}
            className="min-h-11 rounded-xl uno-surface px-4 text-sm font-semibold"
          >
            Color-Blind: {colorBlind ? 'ON' : 'OFF'}
          </button>
        </div>

        <section className="flex flex-col gap-3">
          <h2 className="text-sm font-bold text-text-muted">Classic</h2>
          <div className="flex flex-wrap gap-3">
            {CLASSIC_SAMPLES.map((cf, i) => (
              <UnoCard
                key={i}
                face={cf}
                size="md"
                colorBlind={colorBlind}
              />
            ))}
          </div>
        </section>

        <section className="flex flex-col gap-3">
          <h2 className="text-sm font-bold text-text-muted">
            Flip (Light + Dark)
          </h2>
          <div className="flex flex-wrap gap-3">
            {FLIP_SAMPLES.map((cf, i) => (
              <UnoCard
                key={i}
                face={cf}
                size="md"
                colorBlind={colorBlind}
              />
            ))}
          </div>
        </section>

        <section className="flex flex-col gap-3">
          <h2 className="text-sm font-bold text-text-muted">No Mercy</h2>
          <div className="flex flex-wrap gap-3">
            {NO_MERCY_SAMPLES.map((cf, i) => (
              <UnoCard
                key={i}
                face={cf}
                size="md"
                colorBlind={colorBlind}
              />
            ))}
          </div>
        </section>

        <section className="flex flex-col gap-3">
          <h2 className="text-sm font-bold text-text-muted">
            Backs, Sizes & States
          </h2>
          <div className="flex flex-wrap items-end gap-4 pt-6">
            <UnoCard face={null} size="xs" />
            <UnoCard
              face={null}
              back={{ color: 'pink', face: { k: 'draw_five' } }}
              size="sm"
            />
            <UnoCard
              face={{ color: 'red', face: { k: 'number', v: 7 } }}
              size="md"
              state="playable"
              colorBlind={colorBlind}
            />
            <UnoCard
              face={{ color: 'green', face: { k: 'skip' } }}
              size="md"
              state="selected"
              colorBlind={colorBlind}
            />
            <UnoCard
              face={{ color: 'blue', face: { k: 'reverse' } }}
              size="lg"
              state="disabled"
              colorBlind={colorBlind}
            />
          </div>
        </section>
      </div>
    </AppShell>
  )
}
