import { useEffect, useMemo, useState } from 'react'
import { Link, useParams } from 'react-router'
import { UnoCard } from '@/components/card/UnoCard'
import { COLOR_CSS_VAR, COLOR_LABELS } from '@/components/card/cardMeta'
import { AppShell } from '@/components/layout/AppShell'
import { activeCardFace, backCardFace } from '@/game/handSort'
import {
  ReplayTimeline,
  describeReplayStep,
  formatReplayTime,
  parseReplay,
} from '@/game/replayPlayer'
import { loadReplayById, type LocalReplayRecord } from '@/game/soloSession'
import type { Color } from '@/generated/Color'
import { useTranslation } from '@/i18n'
import { ensureWasmInit } from '@/wasm/init'

const SPEED_OPTIONS = [0.5, 1, 2, 4] as const
const BASE_STEP_MS = 850

export function ReplayPage() {
  const { id } = useParams<{ id: string }>()
  const replayId = id ?? 'local'
  const { t, lang } = useTranslation()

  const [record] = useState<LocalReplayRecord | null>(() =>
    loadReplayById(replayId),
  )
  const [stepIndex, setStepIndex] = useState(0)
  const [playing, setPlaying] = useState(false)
  const [speed, setSpeed] = useState<(typeof SPEED_OPTIONS)[number]>(1)
  const [godViewEnabled, setGodViewEnabled] = useState(false)
  const [timeline, setTimeline] = useState<ReplayTimeline | null>(null)
  const [wasmError, setWasmError] = useState<string | null>(null)

  const parsedResult = useMemo(() => {
    if (!record) return null
    try {
      return { replay: parseReplay(record.replayJson), error: null as string | null }
    } catch (err) {
      return {
        replay: null,
        error: err instanceof Error ? err.message : String(err),
      }
    }
  }, [record])

  const parsedReplay = parsedResult?.replay ?? null
  const errorMsg = parsedResult?.error ?? wasmError
  const totalSteps = parsedReplay ? parsedReplay.steps.length : 0

  useEffect(() => {
    if (!parsedReplay) return
    let active = true
    let created: ReplayTimeline | null = null

    void ensureWasmInit()
      .then(() => {
        if (!active) return
        created = new ReplayTimeline(parsedReplay)
        setTimeline(created)
        setStepIndex(0)
        setWasmError(null)
      })
      .catch((err) => {
        if (active) {
          setWasmError(err instanceof Error ? err.message : String(err))
        }
      })

    return () => {
      active = false
      created?.dispose()
    }
  }, [parsedReplay])

  const frame = useMemo(
    () => (timeline ? timeline.frameAt(stepIndex) : null),
    [timeline, stepIndex],
  )

  const isPlaying = playing && stepIndex < totalSteps

  useEffect(() => {
    if (!isPlaying || totalSteps === 0) return
    const timer = window.setInterval(() => {
      setStepIndex((prev) => (prev < totalSteps ? prev + 1 : prev))
    }, Math.round(BASE_STEP_MS / speed))
    return () => window.clearInterval(timer)
  }, [isPlaying, speed, totalSteps])

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null
      if (
        target &&
        (target.tagName === 'INPUT' ||
          target.tagName === 'BUTTON' ||
          target.tagName === 'SELECT')
      ) {
        return
      }
      if (e.key === ' ' && totalSteps > 0) {
        e.preventDefault()
        setPlaying((prev) => !prev)
      } else if (e.key === 'ArrowLeft') {
        e.preventDefault()
        setPlaying(false)
        setStepIndex((prev) => Math.max(0, prev - 1))
      } else if (e.key === 'ArrowRight') {
        e.preventDefault()
        setPlaying(false)
        setStepIndex((prev) => Math.min(totalSteps, prev + 1))
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [totalSteps])

  const title = t('route.replay', { id: replayId })

  if (!record) {
    return (
      <AppShell title={title} showBack>
        <div className="mx-auto flex max-w-md flex-col items-center gap-4 rounded-3xl uno-surface p-6 text-center">
          <p className="text-sm text-text-muted">{t('replay.noReplay')}</p>
          <div className="flex flex-wrap justify-center gap-2">
            <Link
              to="/solo"
              className="min-h-11 inline-flex items-center rounded-xl bg-btn-green px-5 py-2.5 text-sm font-bold text-white"
            >
              {t('replay.openSolo')}
            </Link>
            <Link
              to="/"
              className="min-h-11 inline-flex items-center rounded-xl border border-white/20 bg-white/10 px-5 py-2.5 text-sm font-semibold text-text-primary"
            >
              {t('results.backLobby')}
            </Link>
          </div>
        </div>
      </AppShell>
    )
  }

  const playerNames = parsedReplay
    ? parsedReplay.players.map((p, idx) => record.playerNames[idx] ?? p.name)
    : record.playerNames

  const godView = frame?.godView ?? null
  const topDiscard =
    godView && godView.discard.length > 0
      ? godView.discard[godView.discard.length - 1]!
      : null
  const activeColor: Color | null = godView?.active_color ?? null
  const side = godView?.side ?? 'light'
  const stepSummary = godView
    ? describeReplayStep(frame?.lastStep ?? null, playerNames, godView, t, lang)
    : t('common.loading')

  const handleTogglePlay = () => {
    if (totalSteps === 0) return
    if (stepIndex >= totalSteps) {
      setStepIndex(0)
      setPlaying(true)
    } else {
      setPlaying(!isPlaying)
    }
  }

  return (
    <AppShell title={title} showBack>
      <div className="mx-auto flex max-w-4xl flex-col gap-4 pb-8">
        {errorMsg && (
          <div
            role="alert"
            className="rounded-2xl border border-red-400/40 bg-red-950/60 px-4 py-3 text-sm font-semibold text-red-100"
          >
            {errorMsg}
          </div>
        )}

        <section className="flex flex-col gap-3 rounded-3xl uno-surface p-4">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex flex-wrap items-center gap-2 text-xs text-text-muted tabular-nums">
              <span className="rounded-lg bg-white/10 px-2.5 py-1 font-bold text-text-primary">
                {t('replay.stepOf', { step: stepIndex, total: totalSteps })}
              </span>
              {godView && (
                <span>
                  {t('replay.roundTurn', {
                    round: godView.round,
                    turn: godView.turn,
                  })}
                </span>
              )}
              <span>
                {t('replay.elapsedTime', {
                  time: formatReplayTime(frame?.lastStep?.t ?? 0),
                })}
              </span>
            </div>

            <button
              type="button"
              data-testid="replay-god-view-toggle"
              aria-pressed={godViewEnabled}
              onClick={() => setGodViewEnabled((prev) => !prev)}
              className={[
                'min-h-11 rounded-xl px-4 text-xs font-bold transition',
                godViewEnabled
                  ? 'bg-accent text-neutral-950 shadow'
                  : 'border border-white/20 bg-white/10 text-text-primary hover:bg-white/20',
              ].join(' ')}
            >
              {godViewEnabled ? t('replay.godView') : t('replay.normalView')}
            </button>
          </div>

          <p
            data-testid="replay-step-summary"
            className="rounded-xl bg-black/30 px-3 py-2 text-sm font-semibold text-text-primary"
          >
            {stepSummary}
          </p>

          <div className="flex items-center gap-3">
            <input
              type="range"
              min={0}
              max={totalSteps}
              value={stepIndex}
              disabled={!godView}
              aria-label={t('replay.stepOf', { step: stepIndex, total: totalSteps })}
              data-testid="replay-scrubber"
              onChange={(e) => {
                setPlaying(false)
                setStepIndex(Number(e.target.value))
              }}
              className="h-3 min-w-0 flex-1 cursor-pointer accent-amber-400"
            />
          </div>

          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <button
                type="button"
                disabled={!godView || stepIndex === 0}
                onClick={() => {
                  setPlaying(false)
                  setStepIndex(0)
                }}
                className="min-h-11 rounded-xl bg-white/10 px-3 text-xs font-bold text-text-primary transition hover:bg-white/20 disabled:opacity-40"
              >
                {t('replay.reset')}
              </button>
              <button
                type="button"
                data-testid="replay-prev"
                disabled={!godView || stepIndex <= 0}
                onClick={() => {
                  setPlaying(false)
                  setStepIndex((prev) => Math.max(0, prev - 1))
                }}
                className="min-h-11 rounded-xl bg-white/10 px-4 text-xs font-bold text-text-primary transition hover:bg-white/20 disabled:opacity-40"
              >
                {t('replay.prev')}
              </button>
              <button
                type="button"
                data-testid="replay-play-pause"
                disabled={!godView || totalSteps === 0}
                onClick={handleTogglePlay}
                className="min-h-11 min-w-20 rounded-xl bg-btn-green px-4 text-xs font-bold text-white transition hover:brightness-110 disabled:opacity-40"
              >
                {isPlaying ? t('replay.pause') : t('replay.play')}
              </button>
              <button
                type="button"
                data-testid="replay-next"
                disabled={!godView || stepIndex >= totalSteps}
                onClick={() => {
                  setPlaying(false)
                  setStepIndex((prev) => Math.min(totalSteps, prev + 1))
                }}
                className="min-h-11 rounded-xl bg-white/10 px-4 text-xs font-bold text-text-primary transition hover:bg-white/20 disabled:opacity-40"
              >
                {t('replay.next')}
              </button>
            </div>

            <div className="flex items-center gap-1.5">
              <span className="text-xs font-semibold text-text-muted">
                {t('replay.speed')}
              </span>
              {SPEED_OPTIONS.map((opt) => (
                <button
                  key={opt}
                  type="button"
                  aria-pressed={speed === opt}
                  onClick={() => setSpeed(opt)}
                  className={[
                    'min-h-11 min-w-11 rounded-xl px-2.5 text-xs font-bold tabular-nums transition',
                    speed === opt
                      ? 'bg-accent text-neutral-950'
                      : 'bg-white/10 text-text-primary hover:bg-white/20',
                  ].join(' ')}
                >
                  {opt}x
                </button>
              ))}
            </div>
          </div>
        </section>

        {godView && (
          <>
            <section className="flex flex-wrap items-center justify-between gap-4 rounded-3xl uno-surface p-4">
              <div className="flex items-center gap-4">
                <UnoCard
                  face={topDiscard ? activeCardFace(topDiscard, side) : null}
                  back={topDiscard ? backCardFace(topDiscard, side) : undefined}
                  size="md"
                />
                <div className="flex flex-col gap-1.5 text-xs">
                  <div className="flex items-center gap-2">
                    <span className="text-text-muted">
                      {t('replay.activeColorLabel')}:
                    </span>
                    {activeColor ? (
                      <span className="inline-flex items-center gap-1.5 font-bold text-text-primary">
                        <span
                          aria-hidden="true"
                          className="inline-block h-3.5 w-3.5 rounded-full border border-white/60"
                          style={{ backgroundColor: COLOR_CSS_VAR[activeColor] }}
                        />
                        {COLOR_LABELS[lang][activeColor]}
                      </span>
                    ) : (
                      <span className="text-text-muted">—</span>
                    )}
                  </div>
                  <p className="text-text-muted tabular-nums">
                    {t('replay.drawPileLeft', {
                      count: godView.draw_pile.length,
                    })}{' '}
                    ·{' '}
                    {t('replay.discardCount', {
                      count: godView.discard.length,
                    })}
                  </p>
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="rounded-lg bg-white/10 px-2 py-0.5 font-semibold text-text-primary">
                      {godView.direction < 0
                        ? t('game.reverseDir')
                        : t('game.forwardDir')}
                    </span>
                    {godView.config.mode === 'flip' && (
                      <span className="rounded-lg bg-white/10 px-2 py-0.5 font-semibold text-text-primary">
                        {side === 'dark'
                          ? t('game.sideDark')
                          : t('game.sideLight')}
                      </span>
                    )}
                    {godView.pending_draw > 0 && (
                      <span className="rounded-lg bg-uno-red px-2 py-0.5 font-black text-white">
                        {t('game.pendingDraw', { count: godView.pending_draw })}
                      </span>
                    )}
                  </div>
                </div>
              </div>

              <Link
                to="/results"
                className="min-h-11 inline-flex items-center rounded-xl border border-white/20 bg-white/10 px-4 text-xs font-bold text-text-primary hover:bg-white/20"
              >
                {t('replay.backToResults')}
              </Link>
            </section>

            <section className="flex flex-col gap-3 rounded-3xl uno-surface p-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <h2 className="text-sm font-bold text-text-primary">
                  {t('replay.handsLabel')}
                </h2>
                {!godViewEnabled && (
                  <p className="text-xs text-text-muted">
                    {t('replay.hiddenCardsLabel')}
                  </p>
                )}
              </div>

              <div className="grid gap-3 sm:grid-cols-2">
                {godView.hands.map((hand, seat) => {
                  const name =
                    playerNames[seat] ?? t('game.playerAi', { seat })
                  const isCurrent = godView.current === seat
                  const isEliminated = godView.eliminated[seat] ?? false
                  const hasUno = godView.uno_called[seat] ?? false
                  const score = godView.scores[seat] ?? 0
                  const showFaces = godViewEnabled || seat === 0

                  return (
                    <div
                      key={seat}
                      data-testid={`replay-seat-hand-${seat}`}
                      className={[
                        'flex flex-col gap-2 rounded-2xl border p-3',
                        isCurrent
                          ? 'border-accent bg-accent/10'
                          : 'border-white/10 bg-black/25',
                      ].join(' ')}
                    >
                      <div className="flex items-center justify-between gap-2 text-xs">
                        <div className="flex min-w-0 items-center gap-2">
                          <span className="truncate font-bold text-text-primary">
                            {name}
                          </span>
                          {hasUno && (
                            <span className="rounded bg-amber-400 px-1.5 py-0.5 text-[10px] font-black text-neutral-950">
                              {t('game.unoBadge')}
                            </span>
                          )}
                          {isEliminated && (
                            <span className="rounded bg-red-600 px-1.5 py-0.5 text-[10px] font-bold text-white">
                              {t('game.eliminated')}
                            </span>
                          )}
                        </div>
                        <span className="shrink-0 font-semibold text-text-muted tabular-nums">
                          {t('game.cardsCount', { count: hand.length })} ·{' '}
                          {t('game.scoreLabel', { score })}
                        </span>
                      </div>

                      <div className="flex flex-wrap gap-1">
                        {hand.map((card) => (
                          <UnoCard
                            key={card.id}
                            face={
                              showFaces
                                ? activeCardFace(card, side)
                                : null
                            }
                            back={backCardFace(card, side)}
                            size="xs"
                          />
                        ))}
                      </div>
                    </div>
                  )
                })}
              </div>
            </section>
          </>
        )}
      </div>
    </AppShell>
  )
}
