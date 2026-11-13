import { beforeEach, describe, expect, it } from 'vitest'
import {
  DEFAULT_SETTINGS,
  animDurationMultiplier,
  clampVolume,
  useSettings,
} from './useSettings'

describe('useSettings', () => {
  beforeEach(() => {
    localStorage.clear()
    useSettings.getState().resetSettings()
  })

  it('uses expected default settings and applies classic theme', () => {
    const state = useSettings.getState()
    expect(state.theme).toBe(DEFAULT_SETTINGS.theme)
    expect(state.lang).toBe('zh-TW')
    expect(state.volume).toBe(0.7)
    expect(document.documentElement.dataset.theme).toBe('classic')
    expect(document.documentElement.lang).toBe('zh-Hant')
  })

  it('updates document data-theme and persists changes to localStorage', () => {
    useSettings.getState().setTheme('neon')
    expect(document.documentElement.dataset.theme).toBe('neon')

    useSettings.getState().setTheme('minimal')
    expect(document.documentElement.dataset.theme).toBe('minimal')

    useSettings.getState().setLang('en')
    expect(document.documentElement.lang).toBe('en')

    const raw = localStorage.getItem('uno-settings')
    expect(raw).not.toBeNull()
    expect(raw).toContain('"theme":"minimal"')
    expect(raw).toContain('"lang":"en"')
  })

  it('clamps volume and computes animation duration multipliers', () => {
    expect(clampVolume(-0.2)).toBe(0)
    expect(clampVolume(1.5)).toBe(1)
    expect(clampVolume(Number.NaN)).toBe(DEFAULT_SETTINGS.volume)

    useSettings.getState().setVolume(2)
    expect(useSettings.getState().volume).toBe(1)

    expect(animDurationMultiplier('0.5x')).toBe(2)
    expect(animDurationMultiplier('1x')).toBe(1)
    expect(animDurationMultiplier('2x')).toBe(0.5)
    expect(animDurationMultiplier('off')).toBe(0)
  })
})
