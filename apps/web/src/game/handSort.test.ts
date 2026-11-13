import { describe, expect, it } from 'vitest'
import type { Card } from '@/generated/Card'
import { sortHand } from './handSort'

const SAMPLE_CARDS: Card[] = [
  { id: 1, light: { color: null, face: { k: 'wild' } }, dark: null },
  { id: 2, light: { color: 'blue', face: { k: 'number', v: 3 } }, dark: null },
  { id: 3, light: { color: 'red', face: { k: 'number', v: 9 } }, dark: null },
  { id: 4, light: { color: 'red', face: { k: 'number', v: 1 } }, dark: null },
  { id: 5, light: { color: 'green', face: { k: 'skip' } }, dark: null },
]

describe('sortHand', () => {
  it('sorts by color then rank in color mode with wild last', () => {
    const sorted = sortHand(SAMPLE_CARDS, 'light', 'color')
    expect(sorted.map((c) => c.id)).toEqual([4, 3, 5, 2, 1])
  })

  it('sorts by face rank first in number mode', () => {
    const sorted = sortHand(SAMPLE_CARDS, 'light', 'number')
    expect(sorted.map((c) => c.id)).toEqual([4, 2, 3, 5, 1])
  })

  it('uses dark side faces in Flip dark side mode', () => {
    const flipCards: Card[] = [
      {
        id: 10,
        light: { color: 'red', face: { k: 'number', v: 2 } },
        dark: { color: 'purple', face: { k: 'draw_five' } },
      },
      {
        id: 11,
        light: { color: 'blue', face: { k: 'number', v: 8 } },
        dark: { color: 'pink', face: { k: 'number', v: 1 } },
      },
    ]
    const sorted = sortHand(flipCards, 'dark', 'color')
    expect(sorted.map((c) => c.id)).toEqual([11, 10])
  })
})
