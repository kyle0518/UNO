import { useEffect, useRef, useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { SettingsDialog } from '@/components/dialogs/SettingsDialog'
import { AppShell } from '@/components/layout/AppShell'
import { GameTable } from '@/components/table/GameTable'
import { TutorialOverlay } from '@/components/tutorial/TutorialOverlay'
import { hasSeenTutorial } from '@/components/tutorial/tutorialStorage'
import { LocalGameClient } from '@/game/LocalGameClient'
import {
  loadPendingSoloMatch,
  saveCompletedSoloResult,
  type SoloMatchConfig,
} from '@/game/soloSession'
import { useGameStore } from '@/game/store'
import { useLeaveGuard } from '@/hooks/useLeaveGuard'
import { useWakeLock } from '@/hooks/useWakeLock'
import { useTranslation } from '@/i18n'

function LeaveConfirmDialog({
  open,
  onCancel,
  onConfirm,
}: {
  open: boolean
  onCancel: () => void
  onConfirm: () => void
}) {
  const { t } = useTranslation()
  if (!open) return null

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="leave-confirm-title"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"
    >
      <div className="flex w-full max-w-xs flex-col gap-4 rounded-3xl uno-surface p-5 text-text-primary shadow-2xl">
        <h2 id="leave-confirm-title" className="text-base font-bold">
          {t('game.leaveConfirm')}
        </h2>
        <div className="flex gap-3">
          <button
            type="button"
            onClick={onCancel}
            className="min-h-11 flex-1 rounded-xl border border-white/20 bg-white/10 px-4 py-2 text-sm font-semibold"
          >
            {t('common.close')}
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className="min-h-11 flex-1 rounded-xl bg-btn-red px-4 py-2 text-sm font-bold text-white"
          >
            {t('game.menuLeave')}
          </button>
        </div>
      </div>
    </div>
  )
}

export function GamePage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [match] = useState<SoloMatchConfig | null>(() => loadPendingSoloMatch())
  const [tutorialOpen, setTutorialOpen] = useState(() => !hasSeenTutorial())
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [leaveConfirmOpen, setLeaveConfirmOpen] = useState(false)
  const clientRef = useRef<LocalGameClient | null>(null)
  const replayJsonRef = useRef<string>('')
  const navigatedRef = useRef(false)

  const pausedOverlay = tutorialOpen || settingsOpen || leaveConfirmOpen
  const prevPausedRef = useRef(pausedOverlay)

  const view = useGameStore((s) => s.view)
  const displayView = useGameStore((s) => s.displayView)
  const timer = useGameStore((s) => s.timer)
  const gameTimer = useGameStore((s) => s.gameTimer)
  const selectedCardId = useGameStore((s) => s.selectedCardId)
  const lastError = useGameStore((s) => s.lastError)
  const activeFx = useGameStore((s) => s.activeFx)
  const lastRoundResult = useGameStore((s) => s.lastRoundResult)
  const unoPopup = useGameStore((s) => s.unoPopup)
  const skippedSeat = useGameStore((s) => s.skippedSeat)
  const colorRipple = useGameStore((s) => s.colorRipple)
  const flipping = useGameStore((s) => s.flipping)
  const confettiSeq = useGameStore((s) => s.confettiSeq)
  const animating = useGameStore((s) => s.animating)
  const nextRoundPending = useGameStore((s) => s.nextRoundPending)
  const setSelectedCardId = useGameStore((s) => s.setSelectedCardId)
  const sendAction = useGameStore((s) => s.sendAction)
  const nextRound = useGameStore((s) => s.nextRound)

  const active = Boolean(match && view?.phase !== 'game_over')
  useWakeLock(active)
  useLeaveGuard(active)

  useEffect(() => {
    if (!match) return
    navigatedRef.current = false
    replayJsonRef.current = ''
    const client = new LocalGameClient({
      config: match.config,
      seed: match.seed,
      seats: match.seats,
      playerNames: match.playerNames,
      paused: !hasSeenTutorial(),
      onReplayReady: (json) => {
        replayJsonRef.current = json
      },
    })
    clientRef.current = client
    const disconnect = useGameStore.getState().connect(client)
    return () => {
      clientRef.current = null
      disconnect()
    }
  }, [match])

  useEffect(() => {
    if (prevPausedRef.current === pausedOverlay) return
    prevPausedRef.current = pausedOverlay
    const client = clientRef.current
    if (!client) return
    if (pausedOverlay) {
      client.pause()
    } else {
      client.resume()
    }
  }, [pausedOverlay])

  useEffect(() => {
    if (!match || !view || view.phase !== 'game_over' || animating) return
    const id = window.setTimeout(() => {
      if (navigatedRef.current) return
      navigatedRef.current = true
      saveCompletedSoloResult(match, view, replayJsonRef.current)
      void navigate('/results')
    }, 250)
    return () => window.clearTimeout(id)
  }, [match, view, animating, navigate])

  if (!match) {
    return (
      <AppShell title={t('route.game')} showBack>
        <div className="mx-auto flex max-w-md flex-col items-center gap-4 rounded-3xl uno-surface p-6 text-center">
          <p className="text-sm text-text-muted">{t('game.noActiveMatch')}</p>
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

  const shownView = displayView ?? view
  if (!shownView) {
    return (
      <div className="uno-table-bg flex h-dvh w-full items-center justify-center text-text-primary">
        <p className="text-base font-bold">{t('common.loading')}</p>
      </div>
    )
  }

  const canCallUnoAuthoritative = view
    ? view.legal.some((a) => a.type === 'call_uno')
    : undefined

  return (
    <>
      <GameTable
        view={shownView}
        playerNames={match.playerNames}
        selectedCardId={selectedCardId}
        timer={timer}
        gameTimer={gameTimer}
        lastError={lastError}
        activeFx={activeFx}
        lastRoundResult={lastRoundResult}
        canCallUnoOverride={canCallUnoAuthoritative}
        unoPopup={unoPopup}
        skippedSeat={skippedSeat}
        colorRipple={colorRipple}
        flipping={flipping}
        confettiSeq={confettiSeq}
        nextRoundPending={nextRoundPending}
        onSelectCard={setSelectedCardId}
        onSendAction={sendAction}
        onNextRound={nextRound}
        onOpenSettings={() => setSettingsOpen(true)}
        onLeaveRequest={() => {
          if (shownView.phase === 'game_over') {
            void navigate('/results')
          } else {
            setLeaveConfirmOpen(true)
          }
        }}
      />

      <TutorialOverlay onClose={() => setTutorialOpen(false)} />

      <SettingsDialog
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />

      <LeaveConfirmDialog
        open={leaveConfirmOpen}
        onCancel={() => setLeaveConfirmOpen(false)}
        onConfirm={() => {
          setLeaveConfirmOpen(false)
          void navigate('/solo')
        }}
      />
    </>
  )
}
