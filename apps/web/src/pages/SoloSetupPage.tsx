import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router'
import { useAuth } from '@/auth/useAuth'
import { AppShell } from '@/components/layout/AppShell'
import { SoloHouseRules } from '@/components/solo/SoloHouseRules'
import type { BotKind, SeatKind } from '@/game/GameClient'
import { unlockAudio } from '@/game/sound'
import {
  generateSeed,
  getDailySeed,
  savePendingSoloMatch,
} from '@/game/soloSession'
import type { RuleConfig } from '@/generated/RuleConfig'
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
          <SoloHouseRules
            config={config}
            open={drawerOpen}
            onToggleOpen={() => setDrawerOpen((o) => !o)}
            onUpdate={updateRule}
          />
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
