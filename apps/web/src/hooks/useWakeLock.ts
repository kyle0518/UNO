import { useEffect } from 'react'

export function useWakeLock(enabled: boolean): void {
  useEffect(() => {
    if (!enabled) return
    if (
      typeof navigator === 'undefined' ||
      !('wakeLock' in navigator) ||
      typeof navigator.wakeLock?.request !== 'function'
    ) {
      return
    }

    let sentinel: WakeLockSentinel | null = null
    let cancelled = false

    const acquire = async () => {
      try {
        const req = await navigator.wakeLock.request('screen')
        if (cancelled) {
          void req.release()
        } else {
          sentinel = req
        }
      } catch {
        // Non-HTTPS or browser policy denied wake lock; ignore safely per SPEC §5.5.
      }
    }

    const handleVisibility = () => {
      if (document.visibilityState === 'visible' && !cancelled) {
        void acquire()
      }
    }

    void acquire()
    document.addEventListener('visibilitychange', handleVisibility)

    return () => {
      cancelled = true
      document.removeEventListener('visibilitychange', handleVisibility)
      if (sentinel) {
        void sentinel.release().catch(() => undefined)
        sentinel = null
      }
    }
  }, [enabled])
}
