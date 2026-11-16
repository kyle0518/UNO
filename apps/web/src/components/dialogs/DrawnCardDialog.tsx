import { useEffect } from 'react'
import { UnoCard } from '@/components/card/UnoCard'
import { activeCardFace } from '@/game/handSort'
import type { Card } from '@/generated/Card'
import type { Side } from '@/generated/Side'
import { useTranslation } from '@/i18n'
import { DialogUnoButton } from './DialogUnoButton'

export interface DrawnCardDialogProps {
  open: boolean
  card: Card | null
  side: Side
  canPlay: boolean
  canKeep: boolean
  canCallUno?: boolean
  unoCalled?: boolean
  onCallUno?: () => void
  onPlay: (cardId: number) => void
  onKeep: () => void
}

export function DrawnCardDialog({
  open,
  card,
  side,
  canPlay,
  canKeep,
  canCallUno = false,
  unoCalled = false,
  onCallUno,
  onPlay,
  onKeep,
}: DrawnCardDialogProps) {
  const { t } = useTranslation()

  useEffect(() => {
    if (!open || !canCallUno || unoCalled || !onCallUno) return
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'u' || e.key === 'U') {
        e.preventDefault()
        onCallUno()
      }
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [open, canCallUno, unoCalled, onCallUno])

  if (!open || !card) return null

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="drawn-card-title"
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/65 p-4 backdrop-blur-xs"
    >
      <div className="flex w-full max-w-xs flex-col items-center gap-4 rounded-3xl uno-surface p-5 text-center text-text-primary shadow-2xl">
        <h2 id="drawn-card-title" className="text-lg font-bold">
          {t('game.drawnTitle')}
        </h2>
        <UnoCard face={activeCardFace(card, side)} size="lg" state="playable" />
        <DialogUnoButton
          visible={canCallUno}
          called={unoCalled}
          onCall={onCallUno}
        />
        <div className="flex w-full gap-3">
          {canKeep && (
            <button
              type="button"
              onClick={onKeep}
              className="min-h-11 flex-1 rounded-xl border border-white/20 bg-white/10 px-4 py-2.5 text-sm font-semibold text-text-primary transition hover:bg-white/20 active:scale-95"
            >
              {t('game.drawnKeep')}
            </button>
          )}
          <button
            type="button"
            disabled={!canPlay}
            onClick={() => onPlay(card.id)}
            className="min-h-11 flex-1 rounded-xl bg-btn-green px-4 py-2.5 text-sm font-bold text-white transition hover:brightness-110 disabled:opacity-50 active:scale-95"
          >
            {t('game.drawnPlay')}
          </button>
        </div>
      </div>
    </div>
  )
}
