import { useCallback, useEffect, useState } from 'react'
import { UnoCard } from '@/components/card/UnoCard'
import type { CardFace } from '@/generated/CardFace'
import { useTranslation, type TranslationKey } from '@/i18n'
import { hasSeenTutorial, markTutorialSeen } from './tutorialStorage'

interface TutorialStep {
  titleKey: TranslationKey
  descKey: TranslationKey
  cards?: { face: CardFace; state?: 'normal' | 'playable' }[]
  showUnoBadge?: boolean
}

const TUTORIAL_STEPS: TutorialStep[] = [
  {
    titleKey: 'tutorial.step1Title',
    descKey: 'tutorial.step1Desc',
    cards: [
      { face: { color: 'red', face: { k: 'number', v: 7 } } },
      { face: { color: 'blue', face: { k: 'number', v: 7 } }, state: 'playable' },
      { face: { color: null, face: { k: 'wild' } }, state: 'playable' },
    ],
  },
  {
    titleKey: 'tutorial.step2Title',
    descKey: 'tutorial.step2Desc',
    cards: [
      { face: { color: 'green', face: { k: 'skip' } }, state: 'playable' },
      { face: { color: 'yellow', face: { k: 'number', v: 2 } } },
    ],
  },
  {
    titleKey: 'tutorial.step3Title',
    descKey: 'tutorial.step3Desc',
    cards: [{ face: { color: 'red', face: { k: 'draw_two' } }, state: 'playable' }],
    showUnoBadge: true,
  },
  {
    titleKey: 'tutorial.step4Title',
    descKey: 'tutorial.step4Desc',
  },
]

export interface TutorialOverlayProps {
  forceOpen?: boolean
  onClose?: () => void
}

export function TutorialOverlay({
  forceOpen = false,
  onClose,
}: TutorialOverlayProps) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(() => forceOpen || !hasSeenTutorial())
  const [stepIndex, setStepIndex] = useState(0)

  const closeAndRemember = useCallback(() => {
    markTutorialSeen()
    setOpen(false)
    onClose?.()
  }, [onClose])

  useEffect(() => {
    if (!open) return
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        closeAndRemember()
      }
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [open, closeAndRemember])

  if (!open) return null

  const step = TUTORIAL_STEPS[stepIndex]
  if (!step) return null
  const isFirst = stepIndex === 0
  const isLast = stepIndex === TUTORIAL_STEPS.length - 1

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="tutorial-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4"
    >
      <div className="flex w-full max-w-md flex-col gap-4 rounded-3xl uno-surface p-6 text-text-primary shadow-2xl">
        <div className="flex items-center justify-between gap-2">
          <span className="rounded-full bg-white/10 px-3 py-1 text-xs font-semibold text-accent">
            {t('tutorial.stepBadge', {
              current: stepIndex + 1,
              total: TUTORIAL_STEPS.length,
            })}
          </span>
          <button
            type="button"
            onClick={closeAndRemember}
            className="min-h-11 rounded-xl px-3 text-sm font-medium text-text-muted transition hover:bg-white/10 hover:text-text-primary"
          >
            {t('tutorial.skip')}
          </button>
        </div>

        <div className="flex min-h-24 items-center justify-center gap-3 rounded-2xl bg-black/25 p-4">
          {step.cards ? (
            step.cards.map((item, idx) => (
              <UnoCard
                key={idx}
                face={item.face}
                size="sm"
                state={item.state ?? 'normal'}
              />
            ))
          ) : (
            <div className="flex flex-wrap justify-center gap-2 text-xs font-semibold">
              {['← / →', 'Enter', 'D', 'U', 'Esc'].map((k) => (
                <kbd
                  key={k}
                  className="rounded-lg border border-surface-border bg-white/10 px-2.5 py-1.5 text-text-primary"
                >
                  {k}
                </kbd>
              ))}
            </div>
          )}
          {step.showUnoBadge && (
            <span className="inline-flex min-h-11 min-w-14 items-center justify-center rounded-2xl bg-btn-red px-4 text-base font-extrabold tracking-wide text-white shadow-lg">
              UNO!
            </span>
          )}
        </div>

        <div className="flex flex-col gap-1.5">
          <h2 id="tutorial-title" className="text-lg font-bold text-accent">
            {t(step.titleKey)}
          </h2>
          <p className="text-sm leading-relaxed text-text-primary">
            {t(step.descKey)}
          </p>
        </div>

        <div className="flex justify-center gap-1.5" aria-hidden="true">
          {TUTORIAL_STEPS.map((_, idx) => (
            <span
              key={idx}
              className={[
                'h-2 rounded-full transition-all',
                idx === stepIndex ? 'w-6 bg-accent' : 'w-2 bg-white/25',
              ].join(' ')}
            />
          ))}
        </div>

        <div className="flex items-center justify-between gap-2 pt-1">
          <button
            type="button"
            disabled={isFirst}
            onClick={() => setStepIndex((i) => Math.max(0, i - 1))}
            className="min-h-11 min-w-22 rounded-xl border border-surface-border px-4 text-sm font-semibold transition hover:bg-white/10 disabled:opacity-40"
          >
            {t('tutorial.prev')}
          </button>

          {isLast ? (
            <button
              type="button"
              onClick={closeAndRemember}
              className="min-h-11 flex-1 rounded-xl bg-btn-green px-4 text-sm font-bold text-white transition hover:brightness-110"
            >
              {t('tutorial.finish')}
            </button>
          ) : (
            <button
              type="button"
              onClick={() =>
                setStepIndex((i) => Math.min(TUTORIAL_STEPS.length - 1, i + 1))
              }
              className="min-h-11 flex-1 rounded-xl bg-btn-blue px-4 text-sm font-bold text-white transition hover:brightness-110"
            >
              {t('tutorial.next')}
            </button>
          )}
        </div>
      </div>
    </div>
  )
}
