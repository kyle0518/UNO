import type { PublicPlayer } from '@/generated/PublicPlayer'
import { useTranslation } from '@/i18n'

export interface TargetCandidate {
  seat: number
  name: string
  player: PublicPlayer
}

export interface TargetPickerProps {
  open: boolean
  candidates: TargetCandidate[]
  onSelect: (seat: number) => void
}

export function TargetPicker({ open, candidates, onSelect }: TargetPickerProps) {
  const { t } = useTranslation()
  if (!open || candidates.length === 0) return null

  return (
    <div
      role="dialog"
      aria-modal="false"
      aria-labelledby="target-picker-title"
      className="fixed inset-x-4 bottom-32 z-40 mx-auto max-w-md rounded-2xl border-2 border-accent uno-surface p-4 text-text-primary shadow-2xl"
    >
      <h2 id="target-picker-title" className="text-sm font-bold text-accent">
        {t('game.targetTitle')}
      </h2>
      <p className="mt-1 text-xs text-text-muted">{t('game.targetHint')}</p>
      <div className="mt-3 flex flex-wrap gap-2">
        {candidates.map((cand) => (
          <button
            key={cand.seat}
            type="button"
            onClick={() => onSelect(cand.seat)}
            className="min-h-11 flex-1 rounded-xl border border-accent/60 bg-accent/15 px-3 py-2 text-sm font-bold text-text-primary transition hover:bg-accent/25 active:scale-95"
          >
            <span>{cand.name}</span>
            <span className="ml-1.5 text-xs text-text-muted">
              ({t('game.cardsCount', { count: cand.player.hand_size })})
            </span>
          </button>
        ))}
      </div>
    </div>
  )
}
