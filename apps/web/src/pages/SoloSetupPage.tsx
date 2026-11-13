import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router'
import { useAuth } from '@/auth/useAuth'
import { AppShell } from '@/components/layout/AppShell'
import type { BotKind, SeatKind } from '@/game/GameClient'
import { unlockAudio } from '@/game/sound'
import {
  generateSeed,
  getDailySeed,
  savePendingSoloMatch,
} from '@/game/soloSession'
import type { RuleConfig } from '@/generated/RuleConfig'
import type { Scoring } from '@/generated/Scoring'
import type { StackDraw } from '@/generated/StackDraw'
import type { UnoRule } from '@/generated/UnoRule'
import type { Wild4Rule } from '@/generated/Wild4Rule'
import { useTranslation, type TranslationKey } from '@/i18n'
import { ensureWasmInit } from '@/wasm/init'
import { preset, validate_config } from '@/wasm/pkg/uno_wasm'

interface ModeMeta {
  id: string
  icon: string
  descKey: TranslationKey
  minPlayers: number
  maxPlayers: number
  step: number
  defaultPlayers: number
}

const MODES: ModeMeta[] = [
  {
    id: 'classic',
    icon: '🎴',
    descKey: 'solo.modeClassicDesc',
    minPlayers: 2,
    maxPlayers: 10,
    step: 1,
    defaultPlayers: 4,
  },
  {
    id: 'party',
    icon: '🎉',
    descKey: 'solo.modePartyDesc',
    minPlayers: 2,
    maxPlayers: 10,
    step: 1,
    defaultPlayers: 4,
  },
  {
    id: 'flip',
    icon: '🔄',
    descKey: 'solo.modeFlipDesc',
    minPlayers: 2,
    maxPlayers: 10,
    step: 1,
    defaultPlayers: 4,
  },
  {
    id: 'no_mercy',
    icon: '🔥',
    descKey: 'solo.modeNoMercyDesc',
    minPlayers: 2,
    maxPlayers: 6,
    step: 1,
    defaultPlayers: 4,
  },
  {
    id: 'teams',
    icon: '🤝',
    descKey: 'solo.modeTeamsDesc',
    minPlayers: 4,
    maxPlayers: 6,
    step: 2,
    defaultPlayers: 4,
  },
  {
    id: 'elimination',
    icon: '🏆',
    descKey: 'solo.modeEliminationDesc',
    minPlayers: 3,
    maxPlayers: 10,
    step: 1,
    defaultPlayers: 4,
  },
  {
    id: 'blitz',
    icon: '⚡',
    descKey: 'solo.modeBlitzDesc',
    minPlayers: 2,
    maxPlayers: 10,
    step: 1,
    defaultPlayers: 4,
  },
  {
    id: 'daily',
    icon: '📅',
    descKey: 'solo.modeDailyDesc',
    minPlayers: 4,
    maxPlayers: 4,
    step: 1,
    defaultPlayers: 4,
  },
]

function clampPlayersForMode(mode: ModeMeta, count: number): number {
  const clamped = Math.min(mode.maxPlayers, Math.max(mode.minPlayers, count))
  if (mode.step > 1) {
    return (
      mode.minPlayers +
      Math.round((clamped - mode.minPlayers) / mode.step) * mode.step
    )
  }
  return clamped
}

export function SoloSetupPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const user = useAuth((s) => s.user)

  const [modeId, setModeId] = useState<string>('classic')
  const [numPlayers, setNumPlayers] = useState<number>(4)
  const [botKinds, setBotKinds] = useState<BotKind[]>(() =>
    Array.from({ length: 10 }, () => 'heuristic' as BotKind),
  )
  const [drawerOpen, setDrawerOpen] = useState(false)
  const [config, setConfig] = useState<RuleConfig | null>(null)
  const [wasmReady, setWasmReady] = useState(false)
  const [errorMsg, setErrorMsg] = useState<string | null>(null)

  const currentMode = MODES.find((m) => m.id === modeId) ?? MODES[0]!

  useEffect(() => {
    let active = true
    void ensureWasmInit()
      .then(() => {
        if (!active) return
        setWasmReady(true)
        const clamped = clampPlayersForMode(currentMode, numPlayers)
        const fresh = preset(modeId, clamped) as RuleConfig
        setConfig((prev) => {
          if (!prev || prev.mode_id !== modeId) {
            return fresh
          }
          if (prev.num_players === clamped) {
            return prev
          }
          try {
            return validate_config({
              ...prev,
              num_players: clamped,
              teams: fresh.teams,
            }) as RuleConfig
          } catch {
            return fresh
          }
        })
        setErrorMsg(null)
      })
      .catch(() => {
        if (active) setWasmReady(false)
      })
    return () => {
      active = false
    }
  }, [modeId, numPlayers, currentMode])

  const handleSelectMode = (nextMode: ModeMeta) => {
    setModeId(nextMode.id)
    setNumPlayers(nextMode.defaultPlayers)
    if (nextMode.id === 'daily') {
      setBotKinds(Array.from({ length: 10 }, () => 'hard' as BotKind))
    }
  }

  const updateRule = (patch: Partial<RuleConfig>) => {
    if (!config || !wasmReady) return
    try {
      const next = validate_config({ ...config, ...patch }) as RuleConfig
      setConfig(next)
      setErrorMsg(null)
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err))
    }
  }

  const handleStart = () => {
    if (!config || !wasmReady) return
    unlockAudio()
    const seats: SeatKind[] = Array.from({ length: config.num_players }, (_, idx) => {
      if (idx === 0) return 'human'
      if (config.mode_id === 'daily') return 'hard'
      return botKinds[idx] ?? 'heuristic'
    })
    const playerNames = Array.from({ length: config.num_players }, (_, idx) =>
      idx === 0
        ? (user?.name ?? t('game.playerYou'))
        : t('game.playerAi', { seat: idx }),
    )
    const seed = config.mode_id === 'daily' ? getDailySeed() : generateSeed()
    savePendingSoloMatch({ config, seed, seats, playerNames })
    void navigate('/game')
  }

  const isDaily = config?.mode_id === 'daily'
  const isNoMercy = config?.mode === 'no_mercy'
  const isFlip = config?.mode === 'flip'

  return (
    <AppShell title={t('solo.title')} showBack>
      <div className="mx-auto flex max-w-2xl flex-col gap-5 pb-10">
        <section>
          <h2 className="mb-2.5 text-sm font-bold uppercase tracking-wider text-text-muted">
            {t('solo.chooseMode')}
          </h2>
          <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
            {MODES.map((m) => {
              const selected = m.id === modeId
              return (
                <button
                  key={m.id}
                  type="button"
                  data-mode={m.id}
                  onClick={() => handleSelectMode(m)}
                  className={[
                    'flex min-h-16 items-start gap-3 rounded-2xl p-3.5 text-left transition',
                    selected
                      ? 'border-2 border-accent bg-accent/15 shadow-lg'
                      : 'uno-surface hover:bg-white/15',
                  ].join(' ')}
                >
                  <span className="text-2xl" aria-hidden="true">
                    {m.icon}
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center justify-between gap-2">
                      <span className="font-bold text-text-primary">
                        {t(`mode.${m.id}` as TranslationKey)}
                      </span>
                      <span className="rounded-full bg-white/10 px-2 py-0.5 text-[11px] font-semibold text-text-muted">
                        {t('solo.playerRange', {
                          min: m.minPlayers,
                          max: m.maxPlayers,
                        })}
                      </span>
                    </div>
                    <p className="mt-1 text-xs leading-relaxed text-text-muted">
                      {t(m.descKey)}
                    </p>
                  </div>
                </button>
              )
            })}
          </div>
        </section>

        <section className="rounded-2xl uno-surface p-4">
          <div className="flex items-center justify-between gap-3">
            <label htmlFor="solo-players" className="text-sm font-bold">
              {t('solo.players', { count: numPlayers })}
            </label>
            <input
              id="solo-players"
              type="range"
              min={currentMode.minPlayers}
              max={currentMode.maxPlayers}
              step={currentMode.step}
              disabled={currentMode.minPlayers === currentMode.maxPlayers}
              value={numPlayers}
              onChange={(e) => setNumPlayers(Number(e.target.value))}
              className="w-44 accent-accent"
            />
          </div>

          <ul className="mt-4 flex flex-col gap-2">
            {Array.from({ length: numPlayers }, (_, seat) => (
              <li
                key={seat}
                className="flex items-center justify-between gap-2 rounded-xl bg-black/20 px-3 py-2 text-sm"
              >
                <span className="font-semibold text-text-primary">
                  {seat === 0
                    ? t('solo.seatYou', { seat: 1 })
                    : t('solo.seatBot', { seat: seat + 1, num: seat })}
                </span>
                {seat === 0 ? (
                  <span className="rounded-full bg-accent/25 px-2.5 py-0.5 text-xs font-bold text-accent">
                    {user?.name ?? t('game.playerYou')}
                  </span>
                ) : (
                  <select
                    aria-label={t('solo.seatBot', { seat: seat + 1, num: seat })}
                    disabled={modeId === 'daily'}
                    value={modeId === 'daily' ? 'hard' : (botKinds[seat] ?? 'heuristic')}
                    onChange={(e) => {
                      const val = e.target.value as BotKind
                      setBotKinds((prev) => {
                        const next = [...prev]
                        next[seat] = val
                        return next
                      })
                    }}
                    className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-3 py-1 text-xs font-semibold text-text-primary"
                  >
                    <option value="random">{t('solo.botRandom')}</option>
                    <option value="heuristic">{t('solo.botHeuristic')}</option>
                    <option value="hard">{t('solo.botHard')}</option>
                  </select>
                )}
              </li>
            ))}
          </ul>
        </section>

        {config && (
          <section className="rounded-2xl uno-surface p-4">
            <button
              type="button"
              aria-expanded={drawerOpen}
              onClick={() => setDrawerOpen((o) => !o)}
              className="flex min-h-11 w-full items-center justify-between text-left text-sm font-bold text-text-primary"
            >
              <span>{t('solo.houseRules')}</span>
              <span>{drawerOpen ? '▲' : '▼'}</span>
            </button>

            {drawerOpen && (
              <div className="mt-3 flex flex-col gap-3 border-t border-white/10 pt-3 text-sm">
                {(isFlip || isNoMercy || isDaily) && (
                  <p className="text-xs text-text-muted">
                    {t('solo.modeLockedNote')}
                  </p>
                )}

                {!isNoMercy && !isDaily && (
                  <div className="flex flex-col gap-3">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <span className="font-semibold">{t('solo.scoring')}</span>
                      <div className="flex gap-2">
                        {(['points', 'single_round'] as const).map((kind) => {
                          const active = config.scoring.type === kind
                          const scoring: Scoring =
                            kind === 'points'
                              ? {
                                  type: 'points',
                                  target:
                                    config.scoring.type === 'points'
                                      ? config.scoring.target
                                      : 500,
                                }
                              : { type: 'single_round' }
                          return (
                            <button
                              key={kind}
                              type="button"
                              onClick={() => updateRule({ scoring })}
                              className={[
                                'min-h-11 rounded-xl px-3 text-xs font-bold',
                                active
                                  ? 'bg-accent text-neutral-950'
                                  : 'bg-white/10 text-text-primary',
                              ].join(' ')}
                            >
                              {kind === 'points'
                                ? t('solo.scoringPoints')
                                : t('solo.scoringSingle')}
                            </button>
                          )
                        })}
                      </div>
                    </div>

                    {config.scoring.type === 'points' && (
                      <div className="flex flex-wrap items-center justify-between gap-2">
                        <span className="font-semibold">
                          {t('solo.targetScore', { score: config.scoring.target })}
                        </span>
                        <div className="flex gap-1.5">
                          {[200, 300, 500, 1000].map((target) => {
                            const active =
                              config.scoring.type === 'points' &&
                              config.scoring.target === target
                            return (
                              <button
                                key={target}
                                type="button"
                                onClick={() =>
                                  updateRule({ scoring: { type: 'points', target } })
                                }
                                className={[
                                  'min-h-11 min-w-11 rounded-xl px-2.5 text-xs font-bold tabular-nums',
                                  active
                                    ? 'bg-accent text-neutral-950'
                                    : 'bg-white/10 text-text-primary',
                                ].join(' ')}
                              >
                                {target}
                              </button>
                            )
                          })}
                        </div>
                      </div>
                    )}
                  </div>
                )}

                {!isFlip && !isNoMercy && (
                  <label className="flex items-center justify-between gap-2">
                    <span className="font-semibold">{t('solo.stackDraw')}</span>
                    <select
                      value={config.stack_draw}
                      onChange={(e) =>
                        updateRule({ stack_draw: e.target.value as StackDraw })
                      }
                      className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-3 text-xs font-semibold text-text-primary"
                    >
                      <option value="none">{t('solo.stackNone')}</option>
                      <option value="same">{t('solo.stackSame')}</option>
                      <option value="progressive">{t('solo.stackProgressive')}</option>
                      <option value="no_mercy">{t('solo.stackNoMercy')}</option>
                    </select>
                  </label>
                )}

                {!isNoMercy && (
                  <label className="flex items-center justify-between gap-2">
                    <span className="font-semibold">{t('solo.wild4Rule')}</span>
                    <select
                      value={config.wild4_rule}
                      onChange={(e) =>
                        updateRule({ wild4_rule: e.target.value as Wild4Rule })
                      }
                      className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-3 text-xs font-semibold text-text-primary"
                    >
                      {config.stack_draw === 'none' && (
                        <option value="challenge">{t('solo.wild4Challenge')}</option>
                      )}
                      <option value="strict">{t('solo.wild4Strict')}</option>
                      <option value="free">{t('solo.wild4Free')}</option>
                    </select>
                  </label>
                )}

                {!isFlip && !isNoMercy && (
                  <ToggleRow
                    label={t('solo.sevenZero')}
                    value={config.seven_zero}
                    onChange={(seven_zero) => updateRule({ seven_zero })}
                  />
                )}

                {!isFlip && (
                  <ToggleRow
                    label={t('solo.jumpIn')}
                    value={config.jump_in}
                    onChange={(jump_in) => updateRule({ jump_in })}
                  />
                )}

                {!isNoMercy && (
                  <>
                    <ToggleRow
                      label={t('solo.drawToMatch')}
                      value={config.draw_to_match}
                      onChange={(draw_to_match) => updateRule({ draw_to_match })}
                    />
                    <ToggleRow
                      label={t('solo.forcePlay')}
                      value={config.force_play}
                      onChange={(force_play) => updateRule({ force_play })}
                    />
                  </>
                )}

                <label className="flex items-center justify-between gap-2">
                  <span className="font-semibold">{t('solo.unoRule')}</span>
                  <select
                    value={config.uno_rule}
                    onChange={(e) =>
                      updateRule({ uno_rule: e.target.value as UnoRule })
                    }
                    className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-3 text-xs font-semibold text-text-primary"
                  >
                    <option value="manual">{t('solo.unoManual')}</option>
                    <option value="auto">{t('solo.unoAuto')}</option>
                  </select>
                </label>

                <label className="flex items-center justify-between gap-2">
                  <span className="font-semibold">{t('solo.turnSeconds')}</span>
                  <select
                    value={config.turn_seconds}
                    onChange={(e) =>
                      updateRule({ turn_seconds: Number(e.target.value) })
                    }
                    className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-3 text-xs font-semibold text-text-primary"
                  >
                    <option value={0}>{t('solo.noLimit')}</option>
                    {[5, 15, 30, 60].map((sec) => (
                      <option key={sec} value={sec}>
                        {t('solo.secondsUnit', { sec })}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
            )}
          </section>
        )}

        {errorMsg && (
          <p role="alert" className="text-center text-sm font-semibold text-text-error">
            {errorMsg}
          </p>
        )}

        <button
          type="button"
          data-testid="start-solo-game"
          disabled={!config || !wasmReady}
          onClick={handleStart}
          className="min-h-12 w-full rounded-2xl bg-btn-green px-6 py-3.5 text-lg font-black text-white shadow-xl transition hover:brightness-110 disabled:opacity-50 active:scale-95"
        >
          {config ? t('solo.startGame') : t('solo.starting')}
        </button>
      </div>
    </AppShell>
  )
}

function ToggleRow({
  label,
  value,
  onChange,
}: {
  label: string
  value: boolean
  onChange: (next: boolean) => void
}) {
  const { t } = useTranslation()
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="font-semibold">{label}</span>
      <button
        type="button"
        role="switch"
        aria-checked={value}
        onClick={() => onChange(!value)}
        className={[
          'min-h-11 min-w-20 rounded-xl px-3 text-xs font-bold transition',
          value
            ? 'bg-btn-green text-white'
            : 'bg-white/10 text-text-muted',
        ].join(' ')}
      >
        {value ? t('common.on') : t('common.off')}
      </button>
    </div>
  )
}
