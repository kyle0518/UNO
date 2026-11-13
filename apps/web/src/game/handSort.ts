import type { Card } from '@/generated/Card'
import type { CardFace } from '@/generated/CardFace'
import type { Color } from '@/generated/Color'
import type { Face } from '@/generated/Face'
import type { Side } from '@/generated/Side'
import type { HandSort } from '@/settings/useSettings'

const COLOR_RANK: Record<Color, number> = {
  red: 0,
  yellow: 1,
  green: 2,
  blue: 3,
  pink: 4,
  teal: 5,
  orange: 6,
  purple: 7,
}

export function activeCardFace(card: Card, side: Side): CardFace {
  return side === 'dark' && card.dark ? card.dark : card.light
}

export function backCardFace(card: Card, side: Side): CardFace | undefined {
  if (!card.dark) return undefined
  return side === 'dark' ? card.light : card.dark
}

function faceRank(face: Face): number {
  switch (face.k) {
    case 'number':
      return face.v
    case 'draw_one':
      return 10
    case 'skip':
      return 11
    case 'skip_everyone':
      return 12
    case 'reverse':
      return 13
    case 'draw_two':
      return 14
    case 'draw_four':
      return 15
    case 'draw_five':
      return 16
    case 'flip':
      return 17
    case 'discard_all':
      return 18
    case 'wild':
      return 19
    case 'wild_draw_two':
      return 20
    case 'wild_draw_four':
      return 21
    case 'wild_reverse_draw_four':
      return 22
    case 'wild_draw_six':
      return 23
    case 'wild_draw_ten':
      return 24
    case 'wild_draw_color':
      return 25
    case 'wild_color_roulette':
      return 26
  }
}

export function sortHand(cards: readonly Card[], side: Side, mode: HandSort): Card[] {
  return [...cards].sort((a, b) => {
    const fa = activeCardFace(a, side)
    const fb = activeCardFace(b, side)
    const ca = fa.color ? COLOR_RANK[fa.color] : 99
    const cb = fb.color ? COLOR_RANK[fb.color] : 99
    const ra = faceRank(fa.face)
    const rb = faceRank(fb.face)

    if (mode === 'color') {
      if (ca !== cb) return ca - cb
      if (ra !== rb) return ra - rb
    } else {
      if (ra !== rb) return ra - rb
      if (ca !== cb) return ca - cb
    }
    return a.id - b.id
  })
}
