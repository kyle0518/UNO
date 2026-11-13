import { useState } from 'react'
import { UnoCard } from '@/components/card/UnoCard'
import { AppShell } from '@/components/layout/AppShell'
import { TutorialOverlay } from '@/components/tutorial/TutorialOverlay'
import type { CardFace } from '@/generated/CardFace'
import { useTranslation, type TranslationKey } from '@/i18n'

interface RuleSection {
  titleKey: TranslationKey
  descKey: TranslationKey
  cards: CardFace[]
}

const RULE_SECTIONS: RuleSection[] = [
  {
    titleKey: 'rules.introTitle',
    descKey: 'rules.introDesc',
    cards: [
      { color: 'red', face: { k: 'number', v: 7 } },
      { color: 'blue', face: { k: 'number', v: 7 } },
      { color: null, face: { k: 'wild' } },
    ],
  },
  {
    titleKey: 'rules.classicTitle',
    descKey: 'rules.classicDesc',
    cards: [
      { color: 'yellow', face: { k: 'skip' } },
      { color: 'green', face: { k: 'reverse' } },
      { color: 'red', face: { k: 'draw_two' } },
      { color: null, face: { k: 'wild_draw_four' } },
    ],
  },
  {
    titleKey: 'rules.partyTitle',
    descKey: 'rules.partyDesc',
    cards: [
      { color: 'blue', face: { k: 'number', v: 7 } },
      { color: 'green', face: { k: 'number', v: 0 } },
      { color: 'yellow', face: { k: 'draw_two' } },
    ],
  },
  {
    titleKey: 'rules.flipTitle',
    descKey: 'rules.flipDesc',
    cards: [
      { color: 'blue', face: { k: 'flip' } },
      { color: 'teal', face: { k: 'draw_five' } },
      { color: 'pink', face: { k: 'skip_everyone' } },
      { color: null, face: { k: 'wild_draw_color' } },
    ],
  },
  {
    titleKey: 'rules.noMercyTitle',
    descKey: 'rules.noMercyDesc',
    cards: [
      { color: 'red', face: { k: 'discard_all' } },
      { color: null, face: { k: 'wild_reverse_draw_four' } },
      { color: null, face: { k: 'wild_draw_six' } },
      { color: null, face: { k: 'wild_draw_ten' } },
      { color: null, face: { k: 'wild_color_roulette' } },
    ],
  },
  {
    titleKey: 'rules.teamsTitle',
    descKey: 'rules.teamsDesc',
    cards: [
      { color: 'red', face: { k: 'number', v: 5 } },
      { color: 'blue', face: { k: 'number', v: 5 } },
    ],
  },
  {
    titleKey: 'rules.elimTitle',
    descKey: 'rules.elimDesc',
    cards: [{ color: null, face: { k: 'wild_draw_four' } }],
  },
  {
    titleKey: 'rules.blitzTitle',
    descKey: 'rules.blitzDesc',
    cards: [
      { color: 'yellow', face: { k: 'skip' } },
      { color: 'green', face: { k: 'draw_two' } },
    ],
  },
]

export function RulesPage() {
  const { t } = useTranslation()
  const [tutorialOpen, setTutorialOpen] = useState(false)

  return (
    <AppShell title={t('route.rules')}>
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-4">
        <div className="flex justify-end">
          <button
            type="button"
            onClick={() => setTutorialOpen(true)}
            className="min-h-11 rounded-xl bg-btn-blue px-4 text-sm font-semibold text-white transition hover:brightness-110"
          >
            {t('rules.replayTutorial')}
          </button>
        </div>

        {RULE_SECTIONS.map((sec) => (
          <article
            key={sec.titleKey}
            className="flex flex-col gap-3 rounded-2xl uno-surface p-4"
          >
            <h2 className="text-base font-bold text-accent">
              {t(sec.titleKey)}
            </h2>
            <p className="text-sm leading-relaxed text-text-primary">
              {t(sec.descKey)}
            </p>
            <div className="flex flex-wrap gap-2 pt-1">
              {sec.cards.map((cardFace, idx) => (
                <UnoCard key={idx} face={cardFace} size="sm" />
              ))}
            </div>
          </article>
        ))}
      </div>
      {tutorialOpen && (
        <TutorialOverlay forceOpen onClose={() => setTutorialOpen(false)} />
      )}
    </AppShell>
  )
}
