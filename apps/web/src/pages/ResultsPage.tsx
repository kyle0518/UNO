import { useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { AppShell } from '@/components/layout/AppShell'
import {
  generateSeed,
  loadLastSoloResult,
  loadLocalReplays,
  savePendingSoloMatch,
} from '@/game/soloSession'
import { useTranslation, type TranslationKey } from '@/i18n'

export function ResultsPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [result] = useState(() => loadLastSoloResult())
  const [recentReplays] = useState(() => loadLocalReplays())
  const [replayOpen, setReplayOpen] = useState(false)
  const [copied, setCopied] = useState(false)

  if (!result) {
    return (
      <AppShell title={t('results.title')} showBack>
        <div className="mx-auto flex max-w-md flex-col items-center gap-4 rounded-3xl uno-surface p-6 text-center">
          <p className="text-sm text-text-muted">{t('results.noResults')}</p>
          <Link
            to="/solo"
            className="min-h-11 inline-flex items-center rounded-xl bg-btn-green px-5 py-2.5 text-sm font-bold text-white"
          >
            {t('game.backToSolo')}
          </Link>
        </div>
      </AppShell>
    )
  }

  const winnerSeat = result.rankings[0] ?? 0
  const winnerName =
    result.playerNames[winnerSeat] ?? t('game.playerAi', { seat: winnerSeat })
  const modeLabel = t(`mode.${result.config.mode_id}` as TranslationKey)

  const handlePlayAgain = () => {
    const seats =
      result.seats && result.seats.length === result.playerNames.length
        ? result.seats
        : result.playerNames.map((_, idx) =>
            idx === 0 ? ('human' as const) : ('heuristic' as const),
          )
    savePendingSoloMatch({
      config: result.config,
      seed: generateSeed(),
      seats,
      playerNames: result.playerNames,
    })
    void navigate('/game')
  }

  const handleCopyReplay = async () => {
    try {
      await navigator.clipboard.writeText(result.replayJson)
      setCopied(true)
      window.setTimeout(() => setCopied(false), 2000)
    } catch {
      setCopied(false)
    }
  }

  const handleDownloadReplay = () => {
    const blob = new Blob([result.replayJson], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `uno-solo-${result.seed}.json`
    a.click()
    URL.revokeObjectURL(url)
  }

  const replayCount = recentReplays.length

  return (
    <AppShell title={t('results.title')} showBack>
      <div className="mx-auto flex max-w-lg flex-col gap-5 pb-8">
        <section className="rounded-3xl uno-surface p-6 text-center">
          <p className="text-xs font-bold uppercase tracking-wider text-text-muted">
            {modeLabel}
          </p>
          <h2 className="mt-1 text-2xl font-black text-accent">
            {t('results.winnerBanner', { name: winnerName })}
          </h2>
          <p className="mt-2 text-xs text-text-muted">
            {t('results.eloPlaceholder')}
          </p>
        </section>

        <section className="rounded-3xl uno-surface p-4">
          <ol className="flex flex-col gap-2">
            {result.rankings.map((seat, idx) => {
              const name =
                result.playerNames[seat] ?? t('game.playerAi', { seat })
              const score = result.scores[seat] ?? 0
              return (
                <li
                  key={seat}
                  className={[
                    'flex items-center justify-between rounded-2xl px-4 py-3 text-sm',
                    idx === 0
                      ? 'bg-accent/20 ring-1 ring-accent'
                      : 'bg-white/5',
                  ].join(' ')}
                >
                  <div className="flex items-center gap-3">
                    <span className="inline-flex h-7 w-7 items-center justify-center rounded-full bg-black/30 text-xs font-black">
                      {t('results.rankLabel', { rank: idx + 1 })}
                    </span>
                    <span className="font-bold text-text-primary">{name}</span>
                  </div>
                  <span className="font-extrabold tabular-nums text-text-primary">
                    {t('results.totalScore', { score })}
                  </span>
                </li>
              )
            })}
          </ol>
        </section>

        <div className="flex flex-wrap items-center justify-center gap-2 text-center text-xs text-text-muted">
          <span>
            {t('results.replaySaved', { steps: result.stepsCount })} ({replayCount})
          </span>
          <button
            type="button"
            onClick={() => setReplayOpen(true)}
            className="min-h-8 rounded-lg border border-white/15 bg-white/10 px-2.5 py-1 text-xs font-semibold text-text-primary hover:bg-white/20"
          >
            {t('results.replayTools')}
          </button>
        </div>

        <div className="flex flex-col gap-3 sm:flex-row">
          <button
            type="button"
            onClick={handlePlayAgain}
            className="min-h-12 flex-1 rounded-2xl bg-btn-green px-4 py-3 text-base font-bold text-white shadow-lg transition hover:brightness-110 active:scale-95"
          >
            {t('results.playAgain')}
          </button>
          <Link
            to="/replay/local"
            className="min-h-12 inline-flex flex-1 items-center justify-center rounded-2xl bg-btn-blue px-4 py-3 text-base font-bold text-white shadow-lg transition hover:brightness-110 active:scale-95"
          >
            {t('results.viewReplay')}
          </Link>
          <Link
            to="/"
            className="min-h-12 inline-flex flex-1 items-center justify-center rounded-2xl border border-white/20 bg-white/10 px-4 py-3 text-base font-semibold text-text-primary transition hover:bg-white/20"
          >
            {t('results.backLobby')}
          </Link>
        </div>

        {recentReplays.length > 1 && (
          <section className="rounded-3xl uno-surface p-4">
            <h3 className="mb-2 text-xs font-bold uppercase tracking-wider text-text-muted">
              {t('results.recentReplays')}
            </h3>
            <ul className="flex flex-col gap-2">
              {recentReplays.slice(0, 5).map((item) => {
                const topSeat = item.rankings[0] ?? 0
                const topName =
                  item.playerNames[topSeat] ?? t('game.playerAi', { seat: topSeat })
                const itemModeKey = `mode.${item.config.mode}` as TranslationKey
                return (
                  <li
                    key={item.id}
                    className="flex items-center justify-between gap-2 rounded-xl bg-white/5 px-3 py-2 text-xs"
                  >
                    <div className="min-w-0">
                      <p className="truncate font-semibold text-text-primary">
                        {t(itemModeKey)} · {topName}
                      </p>
                      <p className="text-text-muted tabular-nums">
                        {t('results.replaySaved', { steps: item.stepsCount })}
                      </p>
                    </div>
                    <Link
                      to={`/replay/${item.id}`}
                      className="min-h-9 inline-flex shrink-0 items-center rounded-lg bg-btn-blue px-3 py-1.5 text-xs font-bold text-white hover:brightness-110"
                    >
                      {t('results.watchReplayItem')}
                    </Link>
                  </li>
                )
              })}
            </ul>
          </section>
        )}

        {replayOpen && (
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="local-replay-title"
            className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4"
          >
            <div className="flex max-h-[85dvh] w-full max-w-lg flex-col gap-4 rounded-3xl uno-surface p-5 text-text-primary shadow-2xl">
              <div className="flex items-center justify-between">
                <h2 id="local-replay-title" className="text-lg font-bold">
                  {t('results.replayModalTitle')}
                </h2>
                <button
                  type="button"
                  onClick={() => setReplayOpen(false)}
                  className="min-h-11 min-w-11 rounded-xl bg-white/10 px-3 text-sm font-bold hover:bg-white/20"
                >
                  {t('common.close')}
                </button>
              </div>
              <p className="text-xs text-text-muted">
                {t('results.replaySaved', { steps: result.stepsCount })} ·{' '}
                {t('results.seedLabel', { seed: result.seed })}
              </p>
              <pre className="max-h-64 overflow-auto rounded-xl bg-black/40 p-3 text-xs text-text-muted">
                {result.replayJson}
              </pre>
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={() => void handleCopyReplay()}
                  className="min-h-11 flex-1 rounded-xl bg-btn-blue px-4 py-2 text-sm font-bold text-white"
                >
                  {copied ? t('results.replayCopied') : t('results.replayCopy')}
                </button>
                <button
                  type="button"
                  onClick={handleDownloadReplay}
                  className="min-h-11 flex-1 rounded-xl bg-white/15 px-4 py-2 text-sm font-bold text-text-primary"
                >
                  {t('results.replayDownload')}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </AppShell>
  )
}
