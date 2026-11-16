import type { RuleConfig } from '@/generated/RuleConfig'
import type { Scoring } from '@/generated/Scoring'
import type { StackDraw } from '@/generated/StackDraw'
import type { UnoRule } from '@/generated/UnoRule'
import type { Wild4Rule } from '@/generated/Wild4Rule'
import { useTranslation } from '@/i18n'

export interface SoloHouseRulesProps {
  config: RuleConfig
  open: boolean
  onToggleOpen: () => void
  onUpdate: (patch: Partial<RuleConfig>) => void
}

const HAND_SIZE_OPTIONS = [3, 5, 7, 10, 12, 15]
const DECK_COUNT_OPTIONS = [1, 2, 3]
const UNO_PENALTY_OPTIONS = [0, 1, 2, 4, 6]
const TURN_SECONDS_OPTIONS = [5, 15, 30, 60]
const GAME_SECONDS_OPTIONS = [120, 180, 300, 600]

function singleDeckCardCount(mode: RuleConfig['mode']): number {
  switch (mode) {
    case 'flip':
      return 112
    case 'no_mercy':
      return 168
    default:
      return 108
  }
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
          value ? 'bg-btn-green text-white' : 'bg-white/10 text-text-muted',
        ].join(' ')}
      >
        {value ? t('common.on') : t('common.off')}
      </button>
    </div>
  )
}

function ScoringControls({
  scoring,
  onUpdate,
}: {
  scoring: Scoring
  onUpdate: (scoring: Scoring) => void
}) {
  const { t } = useTranslation()
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="font-semibold">{t('solo.scoring')}</span>
        <div className="flex gap-2">
          {(['points', 'single_round'] as const).map((kind) => {
            const active = scoring.type === kind
            const nextScoring: Scoring =
              kind === 'points'
                ? {
                    type: 'points',
                    target: scoring.type === 'points' ? scoring.target : 500,
                  }
                : { type: 'single_round' }
            return (
              <button
                key={kind}
                type="button"
                onClick={() => onUpdate(nextScoring)}
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

      {scoring.type === 'points' && (
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="font-semibold">
            {t('solo.targetScore', { score: scoring.target })}
          </span>
          <div className="flex gap-1.5">
            {[200, 300, 500, 1000].map((target) => {
              const active = scoring.type === 'points' && scoring.target === target
              return (
                <button
                  key={target}
                  type="button"
                  onClick={() => onUpdate({ type: 'points', target })}
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
  )
}

function DeckAndTimerControls({
  config,
  onUpdate,
}: {
  config: RuleConfig
  onUpdate: (patch: Partial<RuleConfig>) => void
}) {
  const { t } = useTranslation()
  const perDeck = singleDeckCardCount(config.mode)
  const totalCards = perDeck * config.deck_count

  return (
    <>
      <label className="flex items-center justify-between gap-2">
        <span className="font-semibold">
          {t('solo.handSize', { count: config.hand_size })}
        </span>
        <select
          value={config.hand_size}
          onChange={(e) => onUpdate({ hand_size: Number(e.target.value) })}
          className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-3 text-xs font-semibold text-text-primary"
        >
          {HAND_SIZE_OPTIONS.map((size) => (
            <option
              key={size}
              value={size}
              disabled={config.num_players * size + 1 > totalCards}
            >
              {t('solo.cardsUnit', { count: size })}
            </option>
          ))}
        </select>
      </label>

      <label className="flex items-center justify-between gap-2">
        <span className="font-semibold">
          {t('solo.deckCount', { count: config.deck_count })}
        </span>
        <select
          value={config.deck_count}
          onChange={(e) => onUpdate({ deck_count: Number(e.target.value) })}
          className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-3 text-xs font-semibold text-text-primary"
        >
          {DECK_COUNT_OPTIONS.map((dc) => (
            <option
              key={dc}
              value={dc}
              disabled={config.num_players * config.hand_size + 1 > perDeck * dc}
            >
              {t('solo.decksUnit', { count: dc })}
            </option>
          ))}
        </select>
      </label>

      <label className="flex items-center justify-between gap-2">
        <span className="font-semibold">{t('solo.unoRule')}</span>
        <select
          value={config.uno_rule}
          onChange={(e) => onUpdate({ uno_rule: e.target.value as UnoRule })}
          className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-3 text-xs font-semibold text-text-primary"
        >
          <option value="manual">{t('solo.unoManual')}</option>
          <option value="auto">{t('solo.unoAuto')}</option>
        </select>
      </label>

      {config.uno_rule === 'manual' && (
        <label className="flex items-center justify-between gap-2">
          <span className="font-semibold">
            {t('solo.unoPenalty', { count: config.uno_penalty })}
          </span>
          <select
            value={config.uno_penalty}
            onChange={(e) => onUpdate({ uno_penalty: Number(e.target.value) })}
            className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-3 text-xs font-semibold text-text-primary"
          >
            {UNO_PENALTY_OPTIONS.map((count) => (
              <option key={count} value={count}>
                {t('solo.cardsUnit', { count })}
              </option>
            ))}
          </select>
        </label>
      )}

      <label className="flex items-center justify-between gap-2">
        <span className="font-semibold">{t('solo.turnSeconds')}</span>
        <select
          value={config.turn_seconds}
          onChange={(e) => onUpdate({ turn_seconds: Number(e.target.value) })}
          className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-3 text-xs font-semibold text-text-primary"
        >
          <option value={0}>{t('solo.noLimit')}</option>
          {TURN_SECONDS_OPTIONS.map((sec) => (
            <option key={sec} value={sec}>
              {t('solo.secondsUnit', { sec })}
            </option>
          ))}
        </select>
      </label>

      <label className="flex items-center justify-between gap-2">
        <span className="font-semibold">{t('solo.gameSeconds')}</span>
        <select
          value={config.game_seconds}
          onChange={(e) => onUpdate({ game_seconds: Number(e.target.value) })}
          className="min-h-11 rounded-xl border border-white/15 bg-black/40 px-3 text-xs font-semibold text-text-primary"
        >
          <option value={0}>{t('solo.noLimit')}</option>
          {GAME_SECONDS_OPTIONS.map((sec) => (
            <option key={sec} value={sec}>
              {t('solo.minutesUnit', { min: Math.round(sec / 60) })}
            </option>
          ))}
        </select>
      </label>
    </>
  )
}

export function SoloHouseRules({
  config,
  open,
  onToggleOpen,
  onUpdate,
}: SoloHouseRulesProps) {
  const { t } = useTranslation()
  const isDaily = config.mode_id === 'daily'
  const isNoMercy = config.mode === 'no_mercy'
  const isFlip = config.mode === 'flip'

  return (
    <section className="rounded-2xl uno-surface p-4">
      <button
        type="button"
        aria-expanded={open}
        onClick={onToggleOpen}
        className="flex min-h-11 w-full items-center justify-between text-left text-sm font-bold text-text-primary"
      >
        <span>{t('solo.houseRules')}</span>
        <span>{open ? '▲' : '▼'}</span>
      </button>

      {open && (
        <div className="mt-3 flex flex-col gap-3 border-t border-white/10 pt-3 text-sm">
          {(isFlip || isNoMercy || isDaily) && (
            <p className="text-xs text-text-muted">{t('solo.modeLockedNote')}</p>
          )}

          {!isNoMercy && !isDaily && (
            <ScoringControls
              scoring={config.scoring}
              onUpdate={(scoring) => onUpdate({ scoring })}
            />
          )}

          {!isFlip && !isNoMercy && (
            <label className="flex items-center justify-between gap-2">
              <span className="font-semibold">{t('solo.stackDraw')}</span>
              <select
                value={config.stack_draw}
                onChange={(e) =>
                  onUpdate({ stack_draw: e.target.value as StackDraw })
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
                  onUpdate({ wild4_rule: e.target.value as Wild4Rule })
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
              onChange={(seven_zero) => onUpdate({ seven_zero })}
            />
          )}

          {!isFlip && (
            <ToggleRow
              label={t('solo.jumpIn')}
              value={config.jump_in}
              onChange={(jump_in) => onUpdate({ jump_in })}
            />
          )}

          {!isNoMercy && (
            <>
              <ToggleRow
                label={t('solo.drawToMatch')}
                value={config.draw_to_match}
                onChange={(draw_to_match) => onUpdate({ draw_to_match })}
              />
              <ToggleRow
                label={t('solo.forcePlay')}
                value={config.force_play}
                onChange={(force_play) => onUpdate({ force_play })}
              />
            </>
          )}

          <DeckAndTimerControls config={config} onUpdate={onUpdate} />
        </div>
      )}
    </section>
  )
}
