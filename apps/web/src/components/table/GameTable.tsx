import { useMemo } from 'react'
import { AnimatePresence, MotionConfig, motion } from 'motion/react'
import { ChallengeDialog } from '@/components/dialogs/ChallengeDialog'
import { ColorPicker } from '@/components/dialogs/ColorPicker'
import { DrawnCardDialog } from '@/components/dialogs/DrawnCardDialog'
import { RoundResultDialog } from '@/components/dialogs/RoundResultDialog'
import { TargetPicker } from '@/components/dialogs/TargetPicker'
import { ConfettiCanvas } from '@/components/effects/ConfettiCanvas'
import type { LocalGameClock, LocalTurnTimer } from '@/game/GameClient'
import type { ActiveFx, RoundResultSummary } from '@/game/store'
import type { Action } from '@/generated/Action'
import type { PlayerView } from '@/generated/PlayerView'
import { useMediaQuery } from '@/hooks/useMediaQuery'
import { useTranslation } from '@/i18n'
import { CenterPile } from './CenterPile'
import { DirectionRing } from './DirectionRing'
import { Hand } from './Hand'
import { OpponentStrip } from './OpponentStrip'
import { StatusBar } from './StatusBar'
import { TopBar } from './TopBar'
import { UnoButton } from './UnoButton'

export interface GameTableProps {
  view: PlayerView
  playerNames: string[]
  selectedCardId: number | null
  timer: LocalTurnTimer | null
  gameTimer?: LocalGameClock | null
  lastError: string | null
  activeFx: ActiveFx | null
  lastRoundResult?: RoundResultSummary | null
  canCallUnoOverride?: boolean
  unoPopup: { seat: number; caught: boolean; penalty?: number; seq: number } | null
  skippedSeat: number | null
  colorRipple: { color: NonNullable<PlayerView['active_color']>; seq: number } | null
  flipping: boolean
  confettiSeq: number
  nextRoundPending?: boolean
  onSelectCard: (cardId: number | null) => void
  onSendAction: (action: Action) => void
  onNextRound: () => void
  onLeaveRequest: () => void
  onOpenSettings?: () => void
}

export function GameTable({
  view,
  playerNames,
  selectedCardId,
  timer,
  gameTimer = null,
  lastError,
  activeFx,
  lastRoundResult = null,
  canCallUnoOverride,
  unoPopup,
  skippedSeat,
  colorRipple,
  flipping,
  confettiSeq,
  nextRoundPending = false,
  onSelectCard,
  onSendAction,
  onNextRound,
  onLeaveRequest,
  onOpenSettings,
}: GameTableProps) {
  const { t } = useTranslation()
  const isLandscapeDesktop = useMediaQuery(
    '(min-width: 768px) and (orientation: landscape)',
  )
  const isShortViewport = useMediaQuery('(max-height: 520px)')
  const tableCardSize = isLandscapeDesktop ? 'lg' : isShortViewport ? 'sm' : 'md'

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
  const canCallUno =
    canCallUnoOverride ?? view.legal.some((a) => a.type === 'call_uno')
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
  const resolvedWinner = roundEvent
    ? roundEvent.winner
    : (lastRoundResult?.winner ?? null)
  const resolvedDeltas = roundEvent?.scores ?? lastRoundResult?.scores
  const dealingRound =
    activeFx?.event.type === 'round_started' ? activeFx.event.round : null

  return (
    <MotionConfig reducedMotion="user">
      <div
        className="uno-game-screen relative flex h-dvh w-full flex-col overflow-hidden text-text-primary select-none"
        style={{ touchAction: 'manipulation' }}
      >
        <TopBar
          modeId={view.config.mode_id}
          mode={view.config.mode}
          round={view.round}
          side={view.side}
          timer={timer}
          gameTimer={gameTimer}
          onLeaveRequest={onLeaveRequest}
          onOpenSettings={onOpenSettings}
        />


        <main
          onClick={(e) => {
            if (e.target === e.currentTarget && selectedCardId !== null) {
              onSelectCard(null)
            }
          }}
          data-testid="game-arena"
          className="uno-arena uno-arena-v3 relative min-h-0 flex-1 overflow-hidden"
        >
          <div className="uno-arena-layout">
            <section className="uno-play-canvas" aria-label={t('game.table')}>
              <header className="uno-play-canvas-header">
                <div className="uno-play-canvas-heading">
                  <span className="uno-play-canvas-kicker">{t('game.table')}</span>
                  <span className="uno-play-canvas-round">{t('game.roundLabel', { round: view.round })}</span>
                </div>
                <div className="uno-play-canvas-player-count">
                  <span className="uno-live-dot" aria-hidden="true" />
                  {view.players.filter((p) => !p.eliminated).length} / {n}
                </div>
              </header>

              <div className="uno-play-canvas-seats">
                <OpponentStrip
                  opponents={opponents}
                  currentSeat={view.current}
                  skippedSeat={skippedSeat}
                  catchableSeats={catchableSeats}
                  targetableSeats={targetableSeats}
                  compact={isShortViewport || n > 5}
                  onCatch={(target) => onSendAction({ type: 'catch_uno', target })}
                  onSelectTarget={(target) => onSendAction({ type: 'choose_target', target })}
                />
              </div>

              <div className="uno-play-field">
                <div className="uno-play-field-orbit" aria-hidden="true" />
                <div className="uno-play-field-orbit uno-play-field-orbit--outer" aria-hidden="true" />
                <div className="uno-play-field-caption">{t('game.playZone')}</div>
                <div className="uno-arena-pile relative z-10">
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
                </div>
              </div>
            </section>

            <DirectionRing
              direction={view.direction}
              currentSeat={view.current}
              mySeat={mySeat}
              players={view.players}
              playerNames={playerNames}
            />
          </div>

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

        <section className="uno-player-dock relative z-20 shrink-0" data-testid="player-dock">
          <StatusBar
            isMyTurn={isMyDecision}
            activePlayerName={activeName}
            playerName={playerNames[mySeat] ?? t('game.playerYou')}
            handCount={view.my_hand.length}
            phase={view.phase}
            lastError={lastError}
            showSwipeHint={selectedCardId !== null}
            meEliminated={meEliminated}
            myTeam={myTeam}
            dealingRound={dealingRound}
            compact={isShortViewport}
          />

          <div className="uno-hand-dock relative flex items-center gap-2 px-2 sm:gap-4 sm:px-5">
            <div className="min-w-0 flex-1">
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
            </div>
            <UnoButton
              enabled={canCallUno}
              urgent={canCallUno && view.my_hand.length <= 2}
              layout={isLandscapeDesktop ? 'desktop' : isShortViewport ? 'short' : 'portrait'}
              onClick={() => onSendAction({ type: 'call_uno' })}
            />
          </div>
        </section>

        <ConfettiCanvas triggerSeq={confettiSeq} />

        <ColorPicker
          open={isMyDecision && view.phase === 'choose_color'}
          side={view.side}
          canCallUno={canCallUno}
          unoCalled={mePlayer?.uno_called ?? false}
          onCallUno={() => onSendAction({ type: 'call_uno' })}
          onSelect={(color) => onSendAction({ type: 'choose_color', color })}
        />

        <DrawnCardDialog
          open={isMyDecision && view.phase === 'drawn_card_decision'}
          card={drawnCardObj}
          side={view.side}
          canPlay={canPlayDrawn}
          canKeep={canKeepDrawn}
          canCallUno={canCallUno}
          unoCalled={mePlayer?.uno_called ?? false}
          onCallUno={() => onSendAction({ type: 'call_uno' })}
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
          winnerSeat={resolvedWinner}
          roundDeltas={resolvedDeltas}
          nextRoundPending={nextRoundPending}
          onNextRound={onNextRound}
        />
      </div>
    </MotionConfig>
  )
}
