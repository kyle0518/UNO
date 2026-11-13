import { useEffect } from 'react'

export function useLeaveGuard(enabled: boolean): void {
  useEffect(() => {
    if (!enabled || typeof window === 'undefined') return
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      e.preventDefault()
      e.returnValue = ''
    }
    window.addEventListener('beforeunload', handleBeforeUnload)
    return () => window.removeEventListener('beforeunload', handleBeforeUnload)
  }, [enabled])
}
