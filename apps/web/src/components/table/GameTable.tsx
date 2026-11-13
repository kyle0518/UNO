import { useMemo } from 'react'
import { AnimatePresence, MotionConfig, motion } from 'motion/react'
import { ChallengeDialog } from '@/components/dialogs/ChallengeDialog'
import { ColorPicker } from '@/components/dialogs/ColorPicker'
import { DrawnCardDialog } from '@/components/dialogs/DrawnCardDialog'
import { RoundResultDialog } from '@/components/dialogs/RoundResultDialog'
import { TargetPicker } from '@/components/dialogs/TargetPicker'
import { ConfettiCanvas } from '@/components/effects/ConfettiCanvas'
import type { LocalTurnTimer } from '@/game/GameClient'
import type { ActiveFx } from '@/game/store'
import type { Action } from '@/generated/Action'
import type { PlayerView } from '@/generated/PlayerView'
import { useMediaQuery } from '@/hooks/useMediaQuery'
import { useTranslation } from '@/i18n'
import { CenterPile } from './CenterPile'
import { DirectionRing } from './DirectionRing'
import { Hand } from './Hand'
import { OpponentAvatar } from './OpponentAvatar'
import { OpponentStrip } from './OpponentStrip'
import { StatusBar } from './StatusBar'
import { TopBar } from './TopBar'
import { UnoButton } from './UnoButton'

export interface GameTableProps {
  view: PlayerView
  playerNames: string[]
  selectedCardId: number | null
  timer: LocalTurnTimer | null
  lastError: string | null
  activeFx: ActiveFx | null
  unoPopup: { seat: number; caught: boolean; penalty?: number; seq: number } | null
  skippedSeat: number | null
  colorRipple: { color: NonNullable<PlayerView['active_color']>; seq: number } | null
  flipping: boolean
  confettiSeq: number
  onSelectCard: (cardId: number | null) => void
  onSendAction: (action: Action) => void
  onNextRound: () => void
  onLeaveRequest: () => void
}

export function GameTable({
  view,
  playerNames,
  selectedCardId,
  timer,
  lastError,
  activeFx,
  unoPopup,
  skippedSeat,
  colorRipple,
  flipping,
  confettiSeq,
  onSelectCard,
  onSendAction,
  onNextRound,
  onLeaveRequest,
}: GameTableProps) {
  const { t } = useTranslation()
  const isLandscapeDesktop = useMediaQuery(
    '(min-width: 768px) and (orientation: landscape)',
  )
  const isShortViewport = useMediaQuery('(max-height: 520px)')
  const tableCardSize = isLandscapeDesktop ? 'lg' : isShortViewport ? 'sm' : 'md'
  const unoLayout = isLandscapeDesktop
    ? 'desktop'
    : isShortViewport
      ? 'short'
      : 'portrait'

  const mySeat = view.me ?? 0
  const n = view.players.length
  const mePlayer = view.players.find((p) => p.seat === view.me) ?? null
  const meEliminated = mePlayer?.eliminated ?? false
  const myTeam = mePlayer?.team ?? null

  const opponents = useMemo(() => {
    const items = []
    for (let offset = 1; offset < n; offset += 1) {
      const seat = (mySeat + offset) % n
      const player = view.players.find((p) => p.seat === seat)
      if (!player) continue
      items.push({
        player,
        name: playerNames[seat] ?? t('game.playerAi', { seat }),
      })
    }
    return items
  }, [mySeat, n, view.players, playerNames, t])

  const catchableSeats = useMemo(
    () =>
      new Set(
        view.legal
          .filter((a): a is { type: 'catch_uno'; target: number } => a.type === 'catch_uno')
          .map((a) => a.target),
      ),
    [view.legal],
  )

  const targetableSeats = useMemo(
    () =>
      new Set(
        view.legal
          .filter(
            (a): a is { type: 'choose_target'; target: number } =>
              a.type === 'choose_target',
          )
          .map((a) => a.target),
      ),
    [view.legal],
  )

  const isMyDecision = view.me !== null && view.decision_player === view.me
  const canDraw = view.legal.some((a) => a.type === 'draw')
  const canCallUno = view.legal.some((a) => a.type === 'call_uno')
  const drawnCardObj =
    view.drawn_card !== null
      ? (view.my_hand.find((c) => c.id === view.drawn_card) ?? null)
      : null
  const canPlayDrawn =
    drawnCardObj !== null &&
    view.legal.some((a) => a.type === 'play' && a.card === drawnCardObj.id)
  const canKeepDrawn = view.legal.some((a) => a.type === 'pass')

  const activeName =
    view.decision_player !== null
      ? (playerNames[view.decision_player] ??
        t('game.playerAi', { seat: view.decision_player }))
      : ''

  const targetCandidates = opponents
    .filter((o) => targetableSeats.has(o.player.seat))
    .map((o) => ({
      seat: o.player.seat,
      name: o.name,
      player: o.player,
    }))

  const roundEvent =
    activeFx?.event.type === 'round_ended' ? activeFx.event : null

  return (
    <MotionConfig reducedMotion="user">
      <div
        className="uno-table-bg flex h-dvh w-full flex-col overflow-hidden text-text-primary select-none"
        style={{ touchAction: 'manipulation' }}
      >
        <TopBar
          modeId={view.config.mode_id}
          mode={view.config.mode}
          round={view.round}
          side={view.side}
          timer={timer}
          onLeaveRequest={onLeaveRequest}
        />

        {!isLandscapeDesktop && (
          <OpponentStrip
            opponents={opponents}
            currentSeat={view.current}
            skippedSeat={skippedSeat}
            catchableSeats={catchableSeats}
            targetableSeats={targetableSeats}
            compact={isShortViewport}
            onCatch={(target) => onSendAction({ type: 'catch_uno', target })}
            onSelectTarget={(target) =>
              onSendAction({ type: 'choose_target', target })
            }
          />
        )}

        <main
          onClick={(e) => {
            if (e.target === e.currentTarget && selectedCardId !== null) {
              onSelectCard(null)
            }
          }}
          className={[
            'relative flex min-h-0 flex-1 items-center justify-center overflow-hidden px-3',
            isShortViewport ? 'py-0.5' : 'py-2',
          ].join(' ')}
        >
          {isLandscapeDesktop && (
            <div
              aria-hidden="true"
              className="pointer-events-none absolute inset-x-10 inset-y-4 rounded-[999px] border border-white/15 bg-black/15 shadow-inner"
            />
          )}

          {isLandscapeDesktop &&
            opponents.map(({ player, name }, idx) => {
              const angleDeg = 180 + ((idx + 1) * 180) / n
              const rad = (angleDeg * Math.PI) / 180
              const leftPct = 50 + 39 * Math.cos(rad)
              const topPct = 54 + 36 * Math.sin(rad)
              return (
                <div
                  key={player.seat}
                  style={{ left: `${leftPct}%`, top: `${topPct}%` }}
                  className="absolute z-20 -translate-x-1/2 -translate-y-1/2"
                >
                  <OpponentAvatar
                    player={player}
                    name={name}
                    compact={n >= 7}
                    isCurrent={view.current === player.seat}
                    isSkipped={skippedSeat === player.seat}
                    canCatch={catchableSeats.has(player.seat)}
                    isTargetable={targetableSeats.has(player.seat)}
                    onCatch={(target) => onSendAction({ type: 'catch_uno', target })}
                    onSelectTarget={(target) =>
                      onSendAction({ type: 'choose_target', target })
                    }
                  />
                </div>
              )
            })}

          <DirectionRing
            direction={view.direction}
            large={isLandscapeDesktop}
          />
          <CenterPile
            topCard={view.top_card}
            side={view.side}
            activeColor={view.active_color}
            drawPileSize={view.draw_pile_size}
            drawPileTopBack={view.draw_pile_top_back}
            pendingDraw={view.pending_draw}
            canDraw={canDraw}
            colorRipple={colorRipple}
            flipping={flipping}
            large={isLandscapeDesktop}
            size={tableCardSize}
            onDraw={() => onSendAction({ type: 'draw' })}
          />

          <AnimatePresence>
            {unoPopup && (
              <motion.div
                key={unoPopup.seq}
                initial={{ scale: 0.4, opacity: 0 }}
                animate={{ scale: 1.1, opacity: 1 }}
                exit={{ scale: 0.9, opacity: 0 }}
                transition={{ type: 'spring', stiffness: 420, damping: 22 }}
                className={[
                  'pointer-events-none fixed left-1/2 top-1/3 z-50 -translate-x-1/2 rounded-3xl px-6 py-3 text-3xl font-black tracking-wide text-white shadow-2xl',
                  unoPopup.caught ? 'bg-uno-red' : 'bg-amber-500 text-neutral-950',
                ].join(' ')}
              >
                {unoPopup.caught
                  ? t('game.unoCaughtPopup', { penalty: unoPopup.penalty ?? 2 })
                  : t('game.unoPopup')}
              </motion.div>
            )}
          </AnimatePresence>
        </main>

        <StatusBar
          isMyTurn={isMyDecision}
          activePlayerName={activeName}
          phase={view.phase}
          lastError={lastError}
          showSwipeHint={selectedCardId !== null}
          meEliminated={meEliminated}
          myTeam={myTeam}
          compact={isShortViewport}
        />

        <Hand
          cards={view.my_hand}
          side={view.side}
          legal={view.legal}
          selectedCardId={selectedCardId}
          compact={!isLandscapeDesktop}
          cardSize={tableCardSize}
          onSelectCard={onSelectCard}
          onPlayAction={onSendAction}
          onDraw={() => onSendAction({ type: 'draw' })}
          onCallUno={() => onSendAction({ type: 'call_uno' })}
        />

        <UnoButton
          enabled={canCallUno}
          urgent={canCallUno && view.my_hand.length <= 2}
          layout={unoLayout}
          onClick={() => onSendAction({ type: 'call_uno' })}
        />

        <ConfettiCanvas triggerSeq={confettiSeq} />

        <ColorPicker
          open={isMyDecision && view.phase === 'choose_color'}
          side={view.side}
          onSelect={(color) => onSendAction({ type: 'choose_color', color })}
        />

        <DrawnCardDialog
          open={isMyDecision && view.phase === 'drawn_card_decision'}
          card={drawnCardObj}
          side={view.side}
          canPlay={canPlayDrawn}
          canKeep={canKeepDrawn}
          onPlay={(card) => onSendAction({ type: 'play', card })}
          onKeep={() => onSendAction({ type: 'pass' })}
        />

        <ChallengeDialog
          open={isMyDecision && view.phase === 'challenge'}
          timer={timer}
          topCard={view.top_card}
          side={view.side}
          onChallenge={() => onSendAction({ type: 'challenge' })}
          onAccept={() => onSendAction({ type: 'accept' })}
        />

        <TargetPicker
          open={isMyDecision && view.phase === 'choose_target'}
          candidates={targetCandidates}
          onSelect={(target) => onSendAction({ type: 'choose_target', target })}
        />

        <RoundResultDialog
          open={view.phase === 'round_over'}
          round={view.round}
          players={view.players}
          playerNames={playerNames}
          winnerSeat={roundEvent?.winner ?? null}
          roundDeltas={roundEvent?.scores}
          onNextRound={onNextRound}
        />
      </div>
    </MotionConfig>
  )
}
