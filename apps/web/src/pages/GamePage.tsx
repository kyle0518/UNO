import { useEffect, useRef, useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { AppShell } from '@/components/layout/AppShell'
import { GameTable } from '@/components/table/GameTable'
import { TutorialOverlay } from '@/components/tutorial/TutorialOverlay'
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

export function GamePage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const [match] = useState<SoloMatchConfig | null>(() => loadPendingSoloMatch())
  const [leaveConfirmOpen, setLeaveConfirmOpen] = useState(false)
  const replayJsonRef = useRef<string>('')
  const navigatedRef = useRef(false)

  const view = useGameStore((s) => s.view)
  const displayView = useGameStore((s) => s.displayView)
  const timer = useGameStore((s) => s.timer)
  const selectedCardId = useGameStore((s) => s.selectedCardId)
  const lastError = useGameStore((s) => s.lastError)
  const activeFx = useGameStore((s) => s.activeFx)
  const unoPopup = useGameStore((s) => s.unoPopup)
  const skippedSeat = useGameStore((s) => s.skippedSeat)
  const colorRipple = useGameStore((s) => s.colorRipple)
  const flipping = useGameStore((s) => s.flipping)
  const confettiSeq = useGameStore((s) => s.confettiSeq)
  const animating = useGameStore((s) => s.animating)
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
      onReplayReady: (json) => {
        replayJsonRef.current = json
      },
    })
    const disconnect = useGameStore.getState().connect(client)
    return () => {
      disconnect()
    }
  }, [match])

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

  return (
    <>
      <GameTable
        view={shownView}
        playerNames={match.playerNames}
        selectedCardId={selectedCardId}
        timer={timer}
        lastError={lastError}
        activeFx={activeFx}
        unoPopup={unoPopup}
        skippedSeat={skippedSeat}
        colorRipple={colorRipple}
        flipping={flipping}
        confettiSeq={confettiSeq}
        onSelectCard={setSelectedCardId}
        onSendAction={sendAction}
        onNextRound={nextRound}
        onLeaveRequest={() => {
          if (shownView.phase === 'game_over') {
            void navigate('/results')
          } else {
            setLeaveConfirmOpen(true)
          }
        }}
      />

      <TutorialOverlay />

      {leaveConfirmOpen && (
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
                onClick={() => setLeaveConfirmOpen(false)}
                className="min-h-11 flex-1 rounded-xl border border-white/20 bg-white/10 px-4 py-2 text-sm font-semibold"
              >
                {t('common.close')}
              </button>
              <button
                type="button"
                onClick={() => {
                  setLeaveConfirmOpen(false)
                  void navigate('/solo')
                }}
                className="min-h-11 flex-1 rounded-xl bg-btn-red px-4 py-2 text-sm font-bold text-white"
              >
                {t('game.menuLeave')}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  )
}
