import { beforeEach, describe, expect, it, vi } from 'vitest'
import { useSettings } from '@/settings/useSettings'

const { playSpy, volumeSpy, unloadSpy, resumeSpy, constructed } = vi.hoisted(
  () => ({
    playSpy: vi.fn(),
    volumeSpy: vi.fn(),
    unloadSpy: vi.fn(),
    resumeSpy: vi.fn(() => Promise.resolve()),
    constructed: [] as { src: string[] }[],
  }),
)

vi.mock('howler', () => {
  class MockHowl {
    constructor(opts: { src: string[] }) {
      constructed.push({ src: opts.src })
    }
    volume = volumeSpy
    play = playSpy
    unload = unloadSpy
  }
  return {
    Howl: MockHowl,
    Howler: {
      ctx: {
        state: 'suspended',
        resume: resumeSpy,
      },
    },
  }
})

import {
  _resetSoundForTest,
  attachAudioUnlockListeners,
  playSound,
  unlockAudio,
} from './sound'

describe('sound', () => {
  beforeEach(() => {
    localStorage.clear()
    useSettings.getState().resetSettings()
    _resetSoundForTest()
    playSpy.mockClear()
    volumeSpy.mockClear()
    unloadSpy.mockClear()
    resumeSpy.mockClear()
    constructed.length = 0
  })

  it('does not play before user interaction unlock', () => {
    expect(playSound('play')).toBe(false)
    expect(playSpy).not.toHaveBeenCalled()
    expect(constructed).toHaveLength(0)
  })

  it('unlocks on user interaction and skips playback when volume is 0', () => {
    attachAudioUnlockListeners()
    window.dispatchEvent(new Event('pointerdown'))
    expect(resumeSpy).toHaveBeenCalledTimes(1)

    useSettings.getState().setVolume(0)
    expect(playSound('play')).toBe(false)
    expect(playSpy).not.toHaveBeenCalled()

    useSettings.getState().setVolume(0.5)
    expect(playSound('uno')).toBe(true)
    expect(constructed).toEqual([{ src: ['/sounds/uno.wav'] }])
    expect(volumeSpy).toHaveBeenCalledWith(0.5)
    expect(playSpy).toHaveBeenCalledTimes(1)
  })

  it('reuses cached Howl instances after unlockAudio', () => {
    unlockAudio()
    expect(playSound('draw')).toBe(true)
    expect(playSound('draw')).toBe(true)
    expect(constructed).toHaveLength(1)
    expect(playSpy).toHaveBeenCalledTimes(2)
  })
})
