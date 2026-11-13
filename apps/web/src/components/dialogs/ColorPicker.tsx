import { useEffect } from 'react'
import { COLOR_BLIND_SYMBOL, COLOR_CSS_VAR } from '@/components/card/cardMeta'
import type { Color } from '@/generated/Color'
import type { Side } from '@/generated/Side'
import { useTranslation, type TranslationKey } from '@/i18n'
import { useSettings } from '@/settings/useSettings'

export interface ColorPickerProps {
  open: boolean
  side: Side
  colorBlind?: boolean
  onSelect: (color: Color) => void
}

const LIGHT_COLORS: Color[] = ['red', 'yellow', 'green', 'blue']
const DARK_COLORS: Color[] = ['pink', 'teal', 'orange', 'purple']

const COLOR_TEXT_CLASS: Record<Color, string> = {
  red: 'text-white',
  yellow: 'text-neutral-950',
  green: 'text-white',
  blue: 'text-white',
  pink: 'text-white',
  teal: 'text-neutral-950',
  orange: 'text-neutral-950',
  purple: 'text-white',
}

export function ColorPicker({
  open,
  side,
  colorBlind,
  onSelect,
}: ColorPickerProps) {
  const { t } = useTranslation()
  const settingsColorBlind = useSettings((s) => s.colorBlind)
  const showSymbols = colorBlind ?? settingsColorBlind
  const choices = side === 'dark' ? DARK_COLORS : LIGHT_COLORS

  useEffect(() => {
    if (!open) return
    const handleKey = (e: KeyboardEvent) => {
      if (e.key >= '1' && e.key <= '4') {
        const picked = choices[Number(e.key) - 1]
        if (picked) {
          e.preventDefault()
          onSelect(picked)
        }
      }
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [open, choices, onSelect])

  if (!open) return null

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="color-picker-title"
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/65 p-4 backdrop-blur-xs"
    >
      <div className="w-full max-w-sm rounded-3xl uno-surface p-5 text-text-primary shadow-2xl">
        <h2 id="color-picker-title" className="mb-4 text-center text-lg font-bold">
          {t('game.colorPickerTitle')}
        </h2>
        <div className="grid grid-cols-2 gap-3">
          {choices.map((color, idx) => {
            const label = t(`color.${color}` as TranslationKey)
            return (
              <button
                key={color}
                type="button"
                data-color={color}
                onClick={() => onSelect(color)}
                style={{ backgroundColor: COLOR_CSS_VAR[color] }}
                className={[
                  'flex min-h-[76px] flex-col items-center justify-center gap-1 rounded-2xl px-4 py-3 font-bold shadow-lg transition hover:brightness-105 active:scale-95',
                  COLOR_TEXT_CLASS[color],
                ].join(' ')}
              >
                <span className="flex items-center gap-2 text-lg">
                  {showSymbols && (
                    <span aria-hidden="true">{COLOR_BLIND_SYMBOL[color]}</span>
                  )}
                  <span>{label}</span>
                </span>
                <span className="rounded bg-black/75 px-1.5 py-0.5 text-xs text-white">
                  {idx + 1}
                </span>
              </button>
            )
          })}
        </div>
      </div>
    </div>
  )
}
