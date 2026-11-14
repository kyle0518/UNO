import { useEffect, useMemo, useRef, useState } from 'react'
import { motion } from 'motion/react'
import { UnoCard } from '@/components/card/UnoCard'
import { activeCardFace, backCardFace, sortHand } from '@/game/handSort'
import type { Action } from '@/generated/Action'
import type { Card } from '@/generated/Card'
import type { Side } from '@/generated/Side'
import { useSettings } from '@/settings/useSettings'

export interface HandProps {
  cards: Card[]
  side: Side
  legal: Action[]
  selectedCardId: number | null
  compact?: boolean
  cardSize?: 'sm' | 'md' | 'lg'
  onSelectCard: (cardId: number | null) => void
  onPlayAction: (action: Action) => void
  onDraw: () => void
  onCallUno: () => void
}

export function Hand({
  cards,
  side,
  legal,
  selectedCardId,
  compact = true,
  cardSize,
  onSelectCard,
  onPlayAction,
  onDraw,
  onCallUno,
}: HandProps) {
  const handSort = useSettings((s) => s.handSort)
  const animSpeed = useSettings((s) => s.animSpeed)
  const containerRef = useRef<HTMLDivElement | null>(null)
  const cardRefs = useRef<(HTMLDivElement | null)[]>([])
  const [containerWidth, setContainerWidth] = useState(360)
  const [focusedIdx, setFocusedIdx] = useState(0)
  const [kbdActive, setKbdActive] = useState(false)

  const pointerStartRef = useRef<{
    cardId: number
    pointerId: number
    x: number
    y: number
    capturing: boolean
  } | null>(null)
  const lastPointerTypeRef = useRef<string>('mouse')
  const swipedCardRef = useRef<number | null>(null)
  const [dragState, setDragState] = useState<{
    cardId: number
    offsetY: number
  } | null>(null)

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const updateWidth = () => {
      if (el.clientWidth > 0) setContainerWidth(el.clientWidth)
    }
    updateWidth()
    if (typeof ResizeObserver === 'undefined') return
    const observer = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width
      if (w && w > 0) setContainerWidth(w)
    })
    observer.observe(el)
    return () => observer.disconnect()
  }, [])

  const sortedCards = useMemo(
    () => sortHand(cards, side, handSort),
    [cards, side, handSort],
  )

  const legalCardAction = useMemo(() => {
    const map = new Map<number, Action>()
    for (const act of legal) {
      if (act.type === 'play') {
        map.set(act.card, act)
      } else if (act.type === 'jump_in' && !map.has(act.card)) {
        map.set(act.card, act)
      }
    }
    return map
  }, [legal])

  const canDraw = legal.some((a) => a.type === 'draw')
  const canCallUno = legal.some((a) => a.type === 'call_uno')

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null
      if (
        target &&
        (target.tagName === 'INPUT' ||
          target.tagName === 'TEXTAREA' ||
          target.tagName === 'SELECT')
      ) {
        return
      }
      if (document.querySelector('[role="dialog"][aria-modal="true"]')) return

      if (e.key === 'ArrowRight' && sortedCards.length > 0) {
        e.preventDefault()
        setKbdActive(true)
        const next = (focusedIdx + 1) % sortedCards.length
        setFocusedIdx(next)
        const card = sortedCards[next]
        if (card) {
          onSelectCard(legalCardAction.has(card.id) ? card.id : null)
          const btn = cardRefs.current[next]?.querySelector('button')
          btn?.focus()
        }
      } else if (e.key === 'ArrowLeft' && sortedCards.length > 0) {
        e.preventDefault()
        setKbdActive(true)
        const prev = (focusedIdx - 1 + sortedCards.length) % sortedCards.length
        setFocusedIdx(prev)
        const card = sortedCards[prev]
        if (card) {
          onSelectCard(legalCardAction.has(card.id) ? card.id : null)
          const btn = cardRefs.current[prev]?.querySelector('button')
          btn?.focus()
        }
      } else if (e.key === 'Enter') {
        const currentCard =
          sortedCards.find((c) => c.id === selectedCardId) ?? sortedCards[focusedIdx]
        if (!currentCard) return
        const act = legalCardAction.get(currentCard.id)
        if (act) {
          e.preventDefault()
          onPlayAction(act)
        }
      } else if ((e.key === 'd' || e.key === 'D') && canDraw) {
        e.preventDefault()
        onDraw()
      } else if ((e.key === 'u' || e.key === 'U') && canCallUno) {
        e.preventDefault()
        onCallUno()
      }
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [
    sortedCards,
    focusedIdx,
    selectedCardId,
    legalCardAction,
    canDraw,
    canCallUno,
    onSelectCard,
    onPlayAction,
    onDraw,
    onCallUno,
  ])

  const resolvedSize = cardSize ?? (compact ? 'md' : 'lg')
  const cardWidth = resolvedSize === 'sm' ? 56 : resolvedSize === 'md' ? 80 : 110
  const n = sortedCards.length
  const availWidth = Math.max(containerWidth - 24, cardWidth)
  const rawStep = n <= 1 ? cardWidth : (availWidth - cardWidth) / (n - 1)
  const minStep = cardWidth * 0.42
  const maxStep = cardWidth * 0.75
  const scrollable = n > 1 && rawStep < minStep
  const step =
    n <= 1 ? cardWidth : scrollable ? minStep : Math.min(maxStep, rawStep)
  const overlapMargin = step - cardWidth
  const useAnim = animSpeed !== 'off'

  return (
    <div
      ref={containerRef}
      data-testid="player-hand"
      onClick={(e) => {
        if (e.target === e.currentTarget && selectedCardId !== null) {
          onSelectCard(null)
        }
      }}
      className={[
        'relative flex w-full shrink-0 overflow-x-auto px-3 select-none',
        resolvedSize === 'sm' ? 'pb-1 pt-1' : 'pb-3 pt-7',
      ].join(' ')}
      style={{
        paddingBottom:
          resolvedSize === 'sm'
            ? 'max(0.25rem, env(safe-area-inset-bottom))'
            : 'max(0.75rem, env(safe-area-inset-bottom))',
      }}
    >
      <div
        className={[
          'flex items-end',
          scrollable ? 'justify-start' : 'mx-auto justify-center',
        ].join(' ')}
      >
        {sortedCards.map((card, idx) => {
          const action = legalCardAction.get(card.id)
          const isPlayable = Boolean(action)
          const isSelected = selectedCardId === card.id
          const state = !isPlayable
            ? 'disabled'
            : isSelected
              ? 'selected'
              : 'playable'

          const isDragging = dragState?.cardId === card.id
          const dragOffsetY = isDragging ? dragState.offsetY : 0

          const releaseCapture = (el: HTMLDivElement, pointerId: number) => {
            try {
              if (el.hasPointerCapture?.(pointerId)) {
                el.releasePointerCapture(pointerId)
              }
            } catch {
              // ignore in environments without pointer capture
            }
          }

          const handleTrigger = () => {
            if (!action) return
            if (swipedCardRef.current === card.id) {
              swipedCardRef.current = null
              return
            }
            const pointerType = lastPointerTypeRef.current
            pointerStartRef.current = null
            if (pointerType === 'touch' || pointerType === 'pen') {
              if (isSelected) {
                onPlayAction(action)
              } else {
                onSelectCard(card.id)
              }
            } else {
              onPlayAction(action)
            }
          }

          return (
            <motion.div
              key={card.id}
              ref={(el) => {
                cardRefs.current[idx] = el
              }}
              layoutId={useAnim ? `card-${card.id}` : undefined}
              animate={{ y: dragOffsetY }}
              transition={isDragging ? { duration: 0 } : undefined}
              data-card-id={card.id}
              data-playable={isPlayable ? 'true' : 'false'}
              style={{
                marginLeft: idx === 0 ? 0 : `${overlapMargin}px`,
                zIndex: isDragging ? 60 : isSelected ? 50 : idx + 1,
                touchAction: action ? 'pan-x' : undefined,
              }}
              onPointerDown={(e) => {
                setKbdActive(false)
                setFocusedIdx(idx)
                lastPointerTypeRef.current = e.pointerType || 'mouse'
                swipedCardRef.current = null
                if (!action) {
                  pointerStartRef.current = null
                  return
                }
                pointerStartRef.current = {
                  cardId: card.id,
                  pointerId: e.pointerId,
                  x: e.clientX,
                  y: e.clientY,
                  capturing: false,
                }
              }}
              onPointerMove={(e) => {
                const start = pointerStartRef.current
                if (!start || start.cardId !== card.id || !action) return
                const dy = start.y - e.clientY
                const dx = Math.abs(e.clientX - start.x)
                if (!start.capturing && dy > 10 && dy > dx) {
                  start.capturing = true
                  try {
                    e.currentTarget.setPointerCapture?.(e.pointerId)
                  } catch {
                    // ignore in environments without pointer capture
                  }
                }
                if (start.capturing) {
                  if (dy > 14) {
                    swipedCardRef.current = card.id
                  }
                  setDragState({
                    cardId: card.id,
                    offsetY: -Math.min(Math.max(0, dy), 72),
                  })
                }
                if (dy > 40) {
                  swipedCardRef.current = card.id
                  pointerStartRef.current = null
                  setDragState(null)
                  releaseCapture(e.currentTarget, e.pointerId)
                  onPlayAction(action)
                }
              }}
              onPointerUp={(e) => {
                const start = pointerStartRef.current
                if (start && start.cardId === card.id) {
                  if (start.capturing) {
                    releaseCapture(e.currentTarget, e.pointerId)
                  }
                  pointerStartRef.current = null
                }
                setDragState((prev) => (prev?.cardId === card.id ? null : prev))
              }}
              onPointerCancel={(e) => {
                const start = pointerStartRef.current
                if (start && start.cardId === card.id) {
                  if (start.capturing) {
                    releaseCapture(e.currentTarget, e.pointerId)
                  }
                  pointerStartRef.current = null
                }
                if (swipedCardRef.current === card.id) {
                  swipedCardRef.current = null
                }
                setDragState((prev) => (prev?.cardId === card.id ? null : prev))
              }}
              className="relative shrink-0"
            >
              <UnoCard
                face={activeCardFace(card, side)}
                back={backCardFace(card, side)}
                size={resolvedSize}
                state={state}
                className={
                  kbdActive && focusedIdx === idx
                    ? 'ring-2 ring-white/85'
                    : ''
                }
                onClick={handleTrigger}
              />
            </motion.div>
          )
        })}
      </div>
    </div>
  )
}
