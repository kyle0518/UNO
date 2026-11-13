import { useEffect, useState } from 'react'
import type { LocalTurnTimer } from '@/game/GameClient'

export interface TurnTimerProps {
  timer: LocalTurnTimer | null
  className?: string
}

export function TurnTimer({ timer, className = '' }: TurnTimerProps) {
  const [now, setNow] = useState(() => Date.now())

  useEffect(() => {
    if (!timer || timer.total_ms <= 0) return
    const id = window.setInterval(() => {
      setNow(Date.now())
    }, 200)
    return () => window.clearInterval(id)
  }, [timer])

  if (!timer || timer.total_ms <= 0) return null

  const remainingMs = Math.max(0, timer.deadline_ms - now)
  const seconds = Math.ceil(remainingMs / 1000)
  const ratio = Math.min(1, Math.max(0, remainingMs / timer.total_ms))
  const urgent = seconds <= 5

  const r = 13
  const c = 2 * Math.PI * r
  const dashOffset = c * (1 - ratio)

  return (
    <div
      className={[
        'inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-bold tabular-nums',
        urgent
          ? 'bg-red-500/20 text-red-200 ring-1 ring-red-400/60'
          : 'bg-white/10 text-text-primary',
        className,
      ].join(' ')}
      aria-label={`${seconds}s`}
    >
      <svg width="32" height="32" viewBox="0 0 32 32" className="h-5 w-5 -rotate-90" aria-hidden="true">
        <circle
          cx="16"
          cy="16"
          r={r}
          fill="none"
          stroke="currentColor"
          strokeWidth="3"
          opacity="0.2"
        />
        <circle
          cx="16"
          cy="16"
          r={r}
          fill="none"
          stroke={urgent ? 'var(--uno-red)' : 'var(--accent)'}
          strokeWidth="3"
          strokeDasharray={c}
          strokeDashoffset={dashOffset}
          strokeLinecap="round"
        />
      </svg>
      <span>{seconds}s</span>
    </div>
  )
}
