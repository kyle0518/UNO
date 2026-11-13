import { create } from 'zustand'
import { createJSONStorage, persist } from 'zustand/middleware'

export type ThemeId = 'classic' | 'neon' | 'minimal'
export type CardBackId = 'classic' | 'neon' | 'minimal'
export type AnimSpeed = '0.5x' | '1x' | '2x' | 'off'
export type Language = 'zh-TW' | 'en'
export type HandSort = 'color' | 'number'

export interface SettingsState {
  theme: ThemeId
  cardBack: CardBackId
  volume: number
  animSpeed: AnimSpeed
  colorBlind: boolean
  lang: Language
  vibration: boolean
  handSort: HandSort
  setTheme: (theme: ThemeId) => void
  setCardBack: (cardBack: CardBackId) => void
  setVolume: (volume: number) => void
  setAnimSpeed: (animSpeed: AnimSpeed) => void
  setColorBlind: (colorBlind: boolean) => void
  setLang: (lang: Language) => void
  setVibration: (vibration: boolean) => void
  setHandSort: (handSort: HandSort) => void
  resetSettings: () => void
}

export const DEFAULT_SETTINGS = {
  theme: 'classic' as ThemeId,
  cardBack: 'classic' as CardBackId,
  volume: 0.7,
  animSpeed: '1x' as AnimSpeed,
  colorBlind: false,
  lang: 'zh-TW' as Language,
  vibration: true,
  handSort: 'color' as HandSort,
}

export function clampVolume(value: number): number {
  if (!Number.isFinite(value)) return DEFAULT_SETTINGS.volume
  return Math.min(1, Math.max(0, value))
}

/** 動畫時長倍率（1x = 1；2x 快轉 = 0.5；0.5x 慢放 = 2；關閉 = 0）。 */
export function animDurationMultiplier(speed: AnimSpeed): number {
  switch (speed) {
    case '0.5x':
      return 2
    case '1x':
      return 1
    case '2x':
      return 0.5
    case 'off':
      return 0
  }
}

export function applyThemeToDocument(theme: ThemeId): void {
  if (typeof document !== 'undefined') {
    document.documentElement.dataset.theme = theme
  }
}

export function applyLangToDocument(lang: Language): void {
  if (typeof document !== 'undefined') {
    document.documentElement.lang = lang === 'zh-TW' ? 'zh-Hant' : 'en'
  }
}

export const useSettings = create<SettingsState>()(
  persist(
    (set) => ({
      ...DEFAULT_SETTINGS,
      setTheme: (theme) => {
        applyThemeToDocument(theme)
        set({ theme })
      },
      setCardBack: (cardBack) => set({ cardBack }),
      setVolume: (volume) => set({ volume: clampVolume(volume) }),
      setAnimSpeed: (animSpeed) => set({ animSpeed }),
      setColorBlind: (colorBlind) => set({ colorBlind }),
      setLang: (lang) => {
        applyLangToDocument(lang)
        set({ lang })
      },
      setVibration: (vibration) => set({ vibration }),
      setHandSort: (handSort) => set({ handSort }),
      resetSettings: () => {
        applyThemeToDocument(DEFAULT_SETTINGS.theme)
        applyLangToDocument(DEFAULT_SETTINGS.lang)
        set({ ...DEFAULT_SETTINGS })
      },
    }),
    {
      name: 'uno-settings',
      storage: createJSONStorage(() => localStorage),
      onRehydrateStorage: () => (state) => {
        if (state) {
          applyThemeToDocument(state.theme)
          applyLangToDocument(state.lang)
        }
      },
    },
  ),
)

applyThemeToDocument(useSettings.getState().theme)
applyLangToDocument(useSettings.getState().lang)
