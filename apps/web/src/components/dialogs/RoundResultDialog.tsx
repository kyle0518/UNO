import type { PublicPlayer } from '@/generated/PublicPlayer'
import { useTranslation } from '@/i18n'

export interface RoundResultDialogProps {
  open: boolean
  round: number
  players: PublicPlayer[]
  playerNames: string[]
  winnerSeat: number | null
  roundDeltas?: number[]
  targetScore?: number | null
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
  targetScore = null,
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
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/75 p-3 backdrop-blur-xs sm:p-4"
    >
      <div className="uno-surface flex w-full max-w-md flex-col gap-4 rounded-3xl p-4 text-text-primary shadow-2xl sm:p-5">
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

        <div className="flex flex-col gap-1">
          <div className="grid grid-cols-[minmax(0,1fr)_84px_92px] items-center gap-1 border-b border-white/15 px-2 pb-2 text-[11px] font-semibold text-text-muted sm:grid-cols-[minmax(0,1fr)_100px_108px] sm:px-3">
            <span>{t('game.roundColumnPlayer')}</span>
            <span className="text-right">{t('game.roundColumnEarned')}</span>
            <span className="text-right">{t('game.roundColumnTotal')}</span>
          </div>
          <ul className="flex max-h-[45dvh] flex-col gap-1.5 overflow-y-auto">
            {sorted.map((p) => {
              const name =
                playerNames[p.seat] ?? t('game.playerAi', { seat: p.seat })
              const delta = roundDeltas?.[p.seat] ?? 0
              return (
                <li
                  key={p.seat}
                  className={[
                    'grid grid-cols-[minmax(0,1fr)_84px_92px] items-center gap-1 rounded-xl px-2 py-2.5 text-sm sm:grid-cols-[minmax(0,1fr)_100px_108px] sm:px-3',
                    p.seat === inferredWinner
                      ? 'bg-accent/15 ring-1 ring-accent/70'
                      : 'bg-white/5',
                    p.eliminated ? 'opacity-60' : '',
                  ].join(' ')}
                >
                  <span className="min-w-0 truncate font-semibold" title={name}>
                    {name}
                    {p.eliminated ? ` (${t('game.eliminated')})` : ''}
                  </span>
                  <span
                    data-testid={`round-earned-${p.seat}`}
                    className={`text-right font-bold tabular-nums ${delta > 0 ? 'text-accent' : 'text-text-muted'}`}
                  >
                    {delta > 0
                      ? t('game.roundScoreDelta', { delta })
                      : t('game.scoreLabel', { score: delta })}
                  </span>
                  <span
                    data-testid={`round-total-${p.seat}`}
                    className="text-right font-extrabold tabular-nums text-text-primary"
                  >
                    {t('game.scoreLabel', { score: p.score })}
                  </span>
                </li>
              )
            })}
          </ul>
        </div>

        {targetScore !== null && (
          <p className="rounded-xl border border-white/10 bg-black/15 px-3 py-2 text-xs leading-relaxed text-text-muted">
            {t('game.roundScoringNote', { target: targetScore })}
          </p>
        )}

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
