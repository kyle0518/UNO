import { Howl, Howler } from 'howler'
import { useSettings } from '@/settings/useSettings'

export type SoundEffect = 'play' | 'draw' | 'uno' | 'win' | 'turn' | 'error'

const SOUND_PATHS: Record<SoundEffect, string> = {
  play: `${import.meta.env.BASE_URL}sounds/play.wav`,
  draw: `${import.meta.env.BASE_URL}sounds/draw.wav`,
  uno: `${import.meta.env.BASE_URL}sounds/uno.wav`,
  win: `${import.meta.env.BASE_URL}sounds/win.wav`,
  turn: `${import.meta.env.BASE_URL}sounds/turn.wav`,
  error: `${import.meta.env.BASE_URL}sounds/error.wav`,
}

const cache = new Map<SoundEffect, Howl>()
let audioUnlocked = false
let listenersAttached = false

function handleFirstGesture() {
  unlockAudio()
}

export function attachAudioUnlockListeners(): void {
  if (listenersAttached || typeof window === 'undefined') return
  listenersAttached = true
  window.addEventListener('pointerdown', handleFirstGesture, {
    once: true,
    passive: true,
  })
  window.addEventListener('touchstart', handleFirstGesture, {
    once: true,
    passive: true,
  })
  window.addEventListener('keydown', handleFirstGesture, {
    once: true,
    passive: true,
  })
}

export function unlockAudio(): void {
  audioUnlocked = true
  const ctx = Howler.ctx
  if (ctx && ctx.state === 'suspended') {
    void ctx.resume()
  }
}

function getHowl(effect: SoundEffect): Howl {
  const existing = cache.get(effect)
  if (existing) return existing
  const howl = new Howl({
    src: [SOUND_PATHS[effect]],
    preload: true,
  })
  cache.set(effect, howl)
  return howl
}

export function playSound(effect: SoundEffect): boolean {
  const volume = useSettings.getState().volume
  if (volume <= 0) return false
  if (!audioUnlocked) return false

  const howl = getHowl(effect)
  howl.volume(volume)
  howl.play()
  return true
}

export function _resetSoundForTest(): void {
  for (const howl of cache.values()) {
    howl.unload()
  }
  cache.clear()
  audioUnlocked = false
  listenersAttached = false
}

attachAudioUnlockListeners()
