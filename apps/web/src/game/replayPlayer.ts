import { COLOR_LABELS, formatCardLabel } from '@/components/card/cardMeta'
import { activeCardFace } from '@/game/handSort'
import type { Action } from '@/generated/Action'
import type { GodView } from '@/generated/GodView'
import type { PlayerView } from '@/generated/PlayerView'
import type { Replay } from '@/generated/Replay'
import type { ReplayStep } from '@/generated/ReplayStep'
import type { RuleConfig } from '@/generated/RuleConfig'
import type { TranslationKey } from '@/i18n'
import type { Language } from '@/settings/useSettings'
import { WasmGame } from '@/wasm/pkg/uno_wasm'

export interface ReplayGameHandle {
  apply(seat: number, action: Action): unknown
  next_round(): unknown
  god_view(): unknown
  spectator_view(): unknown
  free?(): void
}

export type ReplayGameFactory = (config: RuleConfig, seed: number) => ReplayGameHandle

export interface ReplayFrame {
  stepIndex: number
  lastStep: ReplayStep | null
  godView: GodView
  spectatorView: PlayerView
}

export function parseReplay(json: string): Replay {
  const parsed = JSON.parse(json) as Partial<Replay>
  if (
    !parsed ||
    typeof parsed !== 'object' ||
    !parsed.config ||
    typeof parsed.seed !== 'number' ||
    !Array.isArray(parsed.steps) ||
    !Array.isArray(parsed.players)
  ) {
    throw new Error('Invalid replay payload')
  }
  return parsed as Replay
}

export class ReplayTimeline {
  readonly replay: Replay
  private readonly createGame: ReplayGameFactory
  private game: ReplayGameHandle | null = null
  private appliedCount = -1

  constructor(
    replay: Replay,
    createGame: ReplayGameFactory = (config, seed) =>
      new WasmGame(config, BigInt(seed)),
  ) {
    this.replay = replay
    this.createGame = createGame
  }

  frameAt(targetStep: number): ReplayFrame {
    const total = this.replay.steps.length
    const clamped = Math.max(0, Math.min(total, Math.trunc(targetStep)))

    if (this.game === null || clamped < this.appliedCount) {
      this.game?.free?.()
      this.game = this.createGame(this.replay.config, this.replay.seed)
      this.appliedCount = 0
    }

    while (this.appliedCount < clamped) {
      const step = this.replay.steps[this.appliedCount]
      if (!step) break
      if (step.action === 'next_round') {
        this.game.next_round()
      } else {
        this.game.apply(step.seat ?? 0, step.action)
      }
      this.appliedCount += 1
    }

    const godView = this.game.god_view() as GodView
    const spectatorView = this.game.spectator_view() as PlayerView
    const lastStep =
      this.appliedCount > 0 ? (this.replay.steps[this.appliedCount - 1] ?? null) : null

    return {
      stepIndex: this.appliedCount,
      lastStep,
      godView,
      spectatorView,
    }
  }

  dispose(): void {
    this.game?.free?.()
    this.game = null
    this.appliedCount = -1
  }
}

export function formatReplayTime(ms: number): string {
  const safeMs = Math.max(0, Math.round(ms))
  const totalSec = Math.floor(safeMs / 1000)
  const mins = Math.floor(totalSec / 60)
  const secs = totalSec % 60
  const tenths = Math.floor((safeMs % 1000) / 100)
  return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}.${tenths}`
}

export type ReplayTranslator = (
  key: TranslationKey,
  params?: Record<string, string | number>,
) => string

export function describeReplayStep(
  step: ReplayStep | null,
  playerNames: string[],
  godView: GodView,
  t: ReplayTranslator,
  lang: Language,
): string {
  if (!step) {
    return t('replay.initialState')
  }
  if (step.action === 'next_round') {
    return t('replay.nextRoundAction')
  }

  const seat = step.seat ?? 0
  const name = playerNames[seat] ?? t('game.playerAi', { seat })
  const action = step.action

  switch (action.type) {
    case 'play':
    case 'jump_in': {
      const cardObj = godView.discard.find((c) => c.id === action.card) ?? null
      const cardLabel = cardObj
        ? formatCardLabel(activeCardFace(cardObj, godView.side), undefined, lang)
        : `#${action.card}`
      return t('replay.playCardAction', { name, card: cardLabel })
    }
    case 'draw':
      return t('replay.drawAction', { name })
    case 'pass':
      return t('replay.keepAction', { name })
    case 'call_uno':
      return t('replay.unoAction', { name })
    case 'catch_uno':
      return t('replay.catchUnoAction', {
        name,
        target: playerNames[action.target] ?? t('game.playerAi', { seat: action.target }),
      })
    case 'challenge':
      return t('replay.challengeAction', { name })
    case 'accept':
      return t('replay.acceptAction', { name })
    case 'choose_target':
      return t('replay.swapAction', {
        name,
        target: playerNames[action.target] ?? t('game.playerAi', { seat: action.target }),
      })
    case 'choose_color':
      return t('replay.chooseColorAction', {
        name,
        color: COLOR_LABELS[lang][action.color],
      })
    case 'timeout':
      return t('replay.timeoutAction', { name })
    case 'end_by_time':
      return t('replay.endByTimeAction')
  }
}
