export const TUTORIAL_SEEN_KEY = 'uno-tutorial-seen'

export function hasSeenTutorial(): boolean {
  if (typeof window === 'undefined') return true
  try {
    return window.localStorage.getItem(TUTORIAL_SEEN_KEY) === '1'
  } catch {
    return false
  }
}

export function markTutorialSeen(): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(TUTORIAL_SEEN_KEY, '1')
  } catch {
    // ignore storage failures
  }
}

export function resetTutorialSeen(): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.removeItem(TUTORIAL_SEEN_KEY)
  } catch {
    // ignore storage failures
  }
}
