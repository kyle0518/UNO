import { activeCardFace } from '@/game/handSort'
import type { Card } from '@/generated/Card'
import type { Side } from '@/generated/Side'
import type { LocalTurnTimer } from '@/game/GameClient'
import { useTranslation } from '@/i18n'
import { TurnTimer } from './TurnTimer'

export interface ChallengeDialogProps {
  open: boolean
  timer: LocalTurnTimer | null
  topCard?: Card | null
  side?: Side
  onChallenge: () => void
  onAccept: () => void
}

export function ChallengeDialog({
  open,
  timer,
  topCard = null,
  side = 'light',
  onChallenge,
  onAccept,
}: ChallengeDialogProps) {
  const { t } = useTranslation()
  if (!open) return null

  const faceKind = topCard ? activeCardFace(topCard, side).face.k : 'wild_draw_four'
  const title =
    faceKind === 'wild_draw_two'
      ? t('game.challengeTitleTwo')
      : faceKind === 'wild_draw_color'
        ? t('game.challengeTitleColor')
        : t('game.challengeTitle')
  const desc =
    faceKind === 'wild_draw_two'
      ? t('game.challengeDescTwo')
      : faceKind === 'wild_draw_color'
        ? t('game.challengeDescColor')
        : t('game.challengeDesc')
  const acceptLabel =
    faceKind === 'wild_draw_two'
      ? t('game.acceptBtnTwo')
      : faceKind === 'wild_draw_color'
        ? t('game.acceptBtnColor')
        : t('game.acceptBtn')

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="challenge-title"
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/65 p-4 backdrop-blur-xs"
    >
      <div className="flex w-full max-w-sm flex-col gap-4 rounded-3xl uno-surface p-5 text-text-primary shadow-2xl">
        <div className="flex items-center justify-between gap-2">
          <h2 id="challenge-title" className="text-lg font-bold">
            {title}
          </h2>
          <TurnTimer timer={timer} />
        </div>
        <p className="text-sm leading-relaxed text-text-muted">
          {desc}
        </p>
        <div className="flex gap-3">
          <button
            type="button"
            onClick={onAccept}
            className="min-h-11 flex-1 rounded-xl border border-white/20 bg-white/10 px-4 py-2.5 text-sm font-semibold text-text-primary transition hover:bg-white/20 active:scale-95"
          >
            {acceptLabel}
          </button>
          <button
            type="button"
            onClick={onChallenge}
            className="min-h-11 flex-1 rounded-xl bg-btn-red px-4 py-2.5 text-sm font-bold text-white transition hover:brightness-110 active:scale-95"
          >
            {t('game.challengeBtn')}
          </button>
        </div>
      </div>
    </div>
  )
}
