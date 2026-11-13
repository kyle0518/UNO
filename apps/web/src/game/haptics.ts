import { useSettings } from '@/settings/useSettings'

export function vibrate(pattern: number | number[]): boolean {
  if (!useSettings.getState().vibration) return false
  if (typeof navigator === 'undefined' || typeof navigator.vibrate !== 'function') {
    return false
  }
  return navigator.vibrate(pattern)
}

export function vibrateTurn(): boolean {
  return vibrate(30)
}

export function vibratePenalty(): boolean {
  return vibrate([40, 30, 40])
}
