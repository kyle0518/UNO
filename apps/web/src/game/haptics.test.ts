import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useSettings } from '@/settings/useSettings'
import { vibratePenalty, vibrateTurn } from './haptics'

describe('haptics', () => {
  const originalVibrate = navigator.vibrate

  beforeEach(() => {
    localStorage.clear()
    useSettings.getState().resetSettings()
  })

  afterEach(() => {
    Object.defineProperty(navigator, 'vibrate', {
      configurable: true,
      value: originalVibrate,
    })
  })

  it('triggers turn and penalty patterns when enabled', () => {
    const spy = vi.fn(() => true)
    Object.defineProperty(navigator, 'vibrate', {
      configurable: true,
      value: spy,
    })

    expect(vibrateTurn()).toBe(true)
    expect(vibratePenalty()).toBe(true)
    expect(spy).toHaveBeenNthCalledWith(1, 30)
    expect(spy).toHaveBeenNthCalledWith(2, [40, 30, 40])
  })

  it('does nothing when disabled or navigator.vibrate is unavailable', () => {
    const spy = vi.fn(() => true)
    Object.defineProperty(navigator, 'vibrate', {
      configurable: true,
      value: spy,
    })

    useSettings.getState().setVibration(false)
    expect(vibrateTurn()).toBe(false)
    expect(spy).not.toHaveBeenCalled()

    useSettings.getState().setVibration(true)
    Object.defineProperty(navigator, 'vibrate', {
      configurable: true,
      value: undefined,
    })
    expect(() => vibratePenalty()).not.toThrow()
    expect(vibratePenalty()).toBe(false)
  })
})
