import { useEffect, useRef, useState } from 'react'
import { Link, useNavigate } from 'react-router'
import { apiClient } from '@/api/client'
import { useAuth } from '@/auth/useAuth'
import { SettingsDialog } from '@/components/dialogs/SettingsDialog'
import { AppShell } from '@/components/layout/AppShell'
import { ChatDrawer, EmoteBubble } from '@/components/table/ChatDrawer'
import { GameTable } from '@/components/table/GameTable'
import { TutorialOverlay } from '@/components/tutorial/TutorialOverlay'
import { hasSeenTutorial } from '@/components/tutorial/tutorialStorage'
import type { GameClient } from '@/game/GameClient'
import { LocalGameClient } from '@/game/LocalGameClient'
import { RemoteGameClient } from '@/game/RemoteGameClient'
import { clearPendingRemoteRoom, loadPendingRemoteRoom } from '@/game/remoteSession'
import { gameSocket, type SocketStatus } from '@/net/socket'
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
  const [remoteCode] = useState<string | null>(() => loadPendingRemoteRoom())
  const [match] = useState<SoloMatchConfig | null>(() =>
    remoteCode ? null : loadPendingSoloMatch(),
  )
  const token = useAuth((s) => s.token)
  const [remoteNames, setRemoteNames] = useState<string[]>([])
  const [chatOpen, setChatOpen] = useState(false)
  const [socketStatus, setSocketStatus] = useState<SocketStatus>(gameSocket.status)
  const [tutorialOpen, setTutorialOpen] = useState(() => !remoteCode && !hasSeenTutorial())
  const [settingsOpen, setSettingsOpen] = useState(false)
  const [leaveConfirmOpen, setLeaveConfirmOpen] = useState(false)
  const clientRef = useRef<GameClient | null>(null)
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
  const chat = useGameStore((s) => s.chat)
  const emotes = useGameStore((s) => s.emotes)

  useEffect(() => {
    if (!remoteCode) return
    return gameSocket.subscribeStatus(setSocketStatus)
  }, [remoteCode])

  const active = Boolean((match || remoteCode) && view?.phase !== 'game_over')
  useWakeLock(active)
  useLeaveGuard(active)

  useEffect(() => {
    if (!remoteCode) return
    navigatedRef.current = false
    if (token) gameSocket.connect(token)
    // 一律重新 join：伺服器會回送最新 GameSnapshot，避免錯過開始時的快照。
    gameSocket.send({ type: 'join_room', code: remoteCode, as_spectator: false })
    const updateNames = () => {
      const room = gameSocket.room
      if (!room || room.code !== remoteCode) return
      setRemoteNames(
        room.seats.map((s) =>
          s.occupant.type === 'human' ? s.occupant.user.name : `AI ${s.seat}`,
        ),
      )
    }
    updateNames()
    const unsub = gameSocket.subscribe((msg) => {
      if (msg.type === 'room_state') updateNames()
    })
    const client = new RemoteGameClient()
    clientRef.current = client
    const disconnect = useGameStore.getState().connect(client)
    return () => {
      unsub()
      clientRef.current = null
      disconnect()
    }
  }, [remoteCode, token])

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
    if (!client || !(client instanceof LocalGameClient)) return
    if (pausedOverlay) {
      client.pause()
    } else {
      client.resume()
    }
  }, [pausedOverlay])

  useEffect(() => {
    if (!remoteCode || !view || view.phase !== 'game_over' || animating) return
    const id = window.setTimeout(() => {
      if (navigatedRef.current) return
      navigatedRef.current = true
      clearPendingRemoteRoom()
      void navigate(`/r/${remoteCode}`)
    }, 1500)
    return () => window.clearTimeout(id)
  }, [remoteCode, view, animating, navigate])

  useEffect(() => {
    if (!match || !view || view.phase !== 'game_over' || animating) return
    const id = window.setTimeout(() => {
      if (navigatedRef.current) return
      navigatedRef.current = true
      saveCompletedSoloResult(match, view, replayJsonRef.current)
      // 單機戰績上傳（T8.2）：失敗靜默，不影響結算流程。
      if (token && replayJsonRef.current) {
        try {
          const replay: unknown = JSON.parse(replayJsonRef.current)
          void apiClient.submitSoloMatch(replay).catch(() => undefined)
        } catch {
          // ignore malformed replay
        }
      }
      void navigate('/results')
    }, 250)
    return () => window.clearTimeout(id)
  }, [match, view, animating, navigate, token])

  if (!match && !remoteCode) {
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
        playerNames={match?.playerNames ?? remoteNames}
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
            void navigate(remoteCode ? `/r/${remoteCode}` : '/results')
          } else {
            setLeaveConfirmOpen(true)
          }
        }}
      />

      {remoteCode && socketStatus === 'reconnecting' && (
        <div
          role="alert"
          className="fixed inset-x-0 top-0 z-50 bg-amber-500 px-4 py-2 text-center text-sm font-bold text-black"
        >
          {t('game.reconnecting')}
        </div>
      )}
      {remoteCode && shownView.me === null && (
        <div
          role="status"
          className="fixed left-1/2 top-14 z-40 -translate-x-1/2 rounded-full bg-black/70 px-4 py-1.5 text-xs font-bold text-white"
        >
          {t('game.spectating')}
        </div>
      )}
      {remoteCode && <EmoteBubble emotes={emotes} playerNames={remoteNames} />}
      {remoteCode && (
        <>
          <button
            type="button"
            onClick={() => setChatOpen(true)}
            className="fixed bottom-24 right-3 z-40 min-h-11 rounded-full bg-btn-blue px-4 text-sm font-bold text-white shadow-lg"
            style={{ bottom: 'max(6rem, env(safe-area-inset-bottom))' }}
          >
            {t('game.chatOpen')}
          </button>
          <ChatDrawer
            open={chatOpen}
            chat={chat}
            onClose={() => setChatOpen(false)}
            onSend={(text) => {
              const c = clientRef.current
              if (c instanceof RemoteGameClient) c.sendChat(text)
            }}
            onEmote={(emote) => {
              const c = clientRef.current
              if (c instanceof RemoteGameClient) c.sendEmote(emote)
            }}
          />
        </>
      )}

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
          if (remoteCode) {
            gameSocket.send({ type: 'leave_room' })
            clearPendingRemoteRoom()
            void navigate(`/r/${remoteCode}`)
          } else {
            void navigate('/solo')
          }
        }}
      />
    </>
  )
}
