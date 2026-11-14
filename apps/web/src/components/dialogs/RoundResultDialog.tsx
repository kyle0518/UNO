import type { PublicPlayer } from '@/generated/PublicPlayer'
import { useTranslation } from '@/i18n'

export interface RoundResultDialogProps {
  open: boolean
  round: number
  players: PublicPlayer[]
  playerNames: string[]
  winnerSeat: number | null
  roundDeltas?: number[]
  nextRoundPending?: boolean
  onNextRound: () => void
}

export function RoundResultDialog({
  open,
  round,
  players,
  playerNames,
  winnerSeat,
  roundDeltas,
  nextRoundPending = false,
  onNextRound,
}: RoundResultDialogProps) {
  const { t } = useTranslation()
  if (!open) return null

  const inferredWinner =
    winnerSeat ?? players.find((p) => p.hand_size === 0)?.seat ?? null
  const sorted = [...players].sort((a, b) => b.score - a.score)

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="round-result-title"
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/70 p-4 backdrop-blur-xs"
    >
      <div className="flex w-full max-w-sm flex-col gap-4 rounded-3xl uno-surface p-5 text-text-primary shadow-2xl">
        <div className="text-center">
          <h2 id="round-result-title" className="text-xl font-bold">
            {t('game.roundEndTitle', { round })}
          </h2>
          <p className="mt-1 text-sm font-semibold text-accent">
            {inferredWinner !== null
              ? t('game.roundWinner', {
                  name:
                    playerNames[inferredWinner] ??
                    t('game.playerAi', { seat: inferredWinner }),
                })
              : t('game.roundNoWinner')}
          </p>
        </div>

        <ul className="flex flex-col gap-2">
          {sorted.map((p) => {
            const name =
              playerNames[p.seat] ?? t('game.playerAi', { seat: p.seat })
            const delta = roundDeltas?.[p.seat] ?? 0
            return (
              <li
                key={p.seat}
                className={[
                  'flex items-center justify-between rounded-xl px-3 py-2 text-sm',
                  p.seat === inferredWinner
                    ? 'bg-accent/20 ring-1 ring-accent'
                    : 'bg-white/5',
                  p.eliminated ? 'opacity-60' : '',
                ].join(' ')}
              >
                <span className="font-semibold">
                  {name}
                  {p.eliminated ? ` (${t('game.eliminated')})` : ''}
                </span>
                <span className="flex items-center gap-2 tabular-nums">
                  {delta > 0 && (
                    <span className="text-xs font-bold text-accent">
                      {t('game.roundScoreDelta', { delta })}
                    </span>
                  )}
                  <span className="font-bold">
                    {t('game.roundTotal', { total: p.score })}
                  </span>
                </span>
              </li>
            )
          })}
        </ul>

        <button
          type="button"
          disabled={nextRoundPending}
          onClick={onNextRound}
          className="min-h-11 w-full rounded-xl bg-btn-green px-4 py-3 text-base font-bold text-white shadow-lg transition hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-60 active:scale-95"
        >
          {t('game.nextRound')}
        </button>
      </div>
    </div>
  )
}
