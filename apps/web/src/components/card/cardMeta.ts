import type { CardFace } from '@/generated/CardFace'
import type { Color } from '@/generated/Color'
import type { Face } from '@/generated/Face'
import type { Language } from '@/settings/useSettings'

export const COLOR_CSS_VAR: Record<Color, string> = {
  red: 'var(--uno-red)',
  yellow: 'var(--uno-yellow)',
  green: 'var(--uno-green)',
  blue: 'var(--uno-blue)',
  pink: 'var(--flip-pink)',
  teal: 'var(--flip-teal)',
  orange: 'var(--flip-orange)',
  purple: 'var(--flip-purple)',
}

export const COLOR_BLIND_SYMBOL: Record<Color, string> = {
  red: '◆',
  yellow: '●',
  green: '▲',
  blue: '■',
  pink: '★',
  teal: '✚',
  orange: '⬟',
  purple: '⬢',
}

export function cornerLabel(face: Face): string {
  switch (face.k) {
    case 'number':
      return String(face.v)
    case 'skip':
      return '⊘'
    case 'reverse':
      return '⇄'
    case 'draw_two':
    case 'wild_draw_two':
      return '+2'
    case 'wild':
      return 'W'
    case 'wild_draw_four':
    case 'draw_four':
      return '+4'
    case 'draw_one':
      return '+1'
    case 'flip':
      return '↻'
    case 'draw_five':
      return '+5'
    case 'skip_everyone':
      return '⊘⊘'
    case 'wild_draw_color':
      return '+C'
    case 'discard_all':
      return 'ALL'
    case 'wild_reverse_draw_four':
      return '↺4'
    case 'wild_draw_six':
      return '+6'
    case 'wild_draw_ten':
      return '+10'
    case 'wild_color_roulette':
      return '?'
  }
}

export function isDarkSideCard(color: Color | null, face: Face): boolean {
  if (
    color === 'pink' ||
    color === 'teal' ||
    color === 'orange' ||
    color === 'purple'
  ) {
    return true
  }
  return (
    face.k === 'draw_five' ||
    face.k === 'skip_everyone' ||
    face.k === 'wild_draw_color'
  )
}

export const COLOR_LABELS: Record<Language, Record<Color, string>> = {
  'zh-TW': {
    red: '紅色',
    yellow: '黃色',
    green: '綠色',
    blue: '藍色',
    pink: '粉色',
    teal: '青色',
    orange: '橘色',
    purple: '紫色',
  },
  en: {
    red: 'Red',
    yellow: 'Yellow',
    green: 'Green',
    blue: 'Blue',
    pink: 'Pink',
    teal: 'Teal',
    orange: 'Orange',
    purple: 'Purple',
  },
}

function faceLabel(face: Face, lang: Language): string {
  if (lang === 'en') {
    switch (face.k) {
      case 'number':
        return String(face.v)
      case 'skip':
        return 'Skip'
      case 'reverse':
        return 'Reverse'
      case 'draw_two':
        return '+2'
      case 'wild':
        return ''
      case 'wild_draw_four':
        return '+4'
      case 'draw_one':
        return '+1'
      case 'flip':
        return 'Flip'
      case 'wild_draw_two':
        return '+2'
      case 'draw_five':
        return '+5'
      case 'skip_everyone':
        return 'Skip Everyone'
      case 'wild_draw_color':
        return 'Draw Color'
      case 'draw_four':
        return '+4'
      case 'discard_all':
        return 'Discard All'
      case 'wild_reverse_draw_four':
        return 'Reverse +4'
      case 'wild_draw_six':
        return '+6'
      case 'wild_draw_ten':
        return '+10'
      case 'wild_color_roulette':
        return 'Color Roulette'
    }
  }

  switch (face.k) {
    case 'number':
      return String(face.v)
    case 'skip':
      return '跳過'
    case 'reverse':
      return '迴轉'
    case 'draw_two':
      return '+2'
    case 'wild':
      return ''
    case 'wild_draw_four':
      return '+4'
    case 'draw_one':
      return '+1'
    case 'flip':
      return '翻面'
    case 'wild_draw_two':
      return '+2'
    case 'draw_five':
      return '+5'
    case 'skip_everyone':
      return '跳過所有人'
    case 'wild_draw_color':
      return '抽到同色'
    case 'draw_four':
      return '+4'
    case 'discard_all':
      return '全棄'
    case 'wild_reverse_draw_four':
      return '迴轉 +4'
    case 'wild_draw_six':
      return '+6'
    case 'wild_draw_ten':
      return '+10'
    case 'wild_color_roulette':
      return '色彩輪盤'
  }
}

export function formatCardLabel(
  face: CardFace | null,
  back: CardFace | undefined,
  lang: Language,
): string {
  if (!face) {
    const base = lang === 'en' ? 'UNO card back' : 'UNO 牌背'
    if (!back) return base
    const backText = formatCardLabel(back, undefined, lang)
    return lang === 'en' ? `${base} (${backText})` : `${base}（${backText}）`
  }

  const colorText = face.color
    ? COLOR_LABELS[lang][face.color]
    : lang === 'en'
      ? 'Wild'
      : '萬用'
  const symText = faceLabel(face.face, lang)
  return symText ? `${colorText} ${symText}` : colorText
}
