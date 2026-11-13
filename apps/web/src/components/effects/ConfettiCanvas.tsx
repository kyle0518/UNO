import { useEffect, useRef } from 'react'
import { useReducedMotion } from 'motion/react'
import { useSettings } from '@/settings/useSettings'

interface Particle {
  x: number
  y: number
  vx: number
  vy: number
  size: number
  color: string
  rotation: number
  vr: number
}

const COLORS = ['#E53935', '#FDD835', '#43A047', '#1E88E5', '#FFC107', '#EC407A']

export interface ConfettiCanvasProps {
  triggerSeq: number
}

export function ConfettiCanvas({ triggerSeq }: ConfettiCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null)
  const animSpeed = useSettings((s) => s.animSpeed)
  const reducedMotion = useReducedMotion()

  useEffect(() => {
    if (triggerSeq <= 0 || animSpeed === 'off' || reducedMotion) return
    const canvas = canvasRef.current
    if (!canvas) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    const width = (canvas.width = window.innerWidth)
    const height = (canvas.height = window.innerHeight)

    const particles: Particle[] = Array.from({ length: 70 }, (_, idx) => ({
      x: (width * ((idx * 17) % 100)) / 100,
      y: -20 - (idx % 12) * 18,
      vx: ((idx % 7) - 3) * 1.4,
      vy: 3 + (idx % 5) * 1.1,
      size: 6 + (idx % 4) * 2,
      color: COLORS[idx % COLORS.length] ?? '#FFC107',
      rotation: idx * 0.4,
      vr: ((idx % 5) - 2) * 0.08,
    }))

    let rafId = 0
    const start = performance.now()
    const duration = animSpeed === '2x' ? 900 : animSpeed === '0.5x' ? 2200 : 1500

    const frame = (now: number) => {
      const elapsed = now - start
      if (elapsed >= duration) {
        ctx.clearRect(0, 0, width, height)
        return
      }
      ctx.clearRect(0, 0, width, height)
      const alpha = elapsed > duration * 0.7 ? 1 - (elapsed - duration * 0.7) / (duration * 0.3) : 1
      ctx.globalAlpha = Math.max(0, alpha)

      for (const p of particles) {
        p.x += p.vx
        p.y += p.vy
        p.rotation += p.vr
        ctx.save()
        ctx.translate(p.x, p.y)
        ctx.rotate(p.rotation)
        ctx.fillStyle = p.color
        ctx.fillRect(-p.size / 2, -p.size / 4, p.size, p.size * 0.5)
        ctx.restore()
      }
      rafId = requestAnimationFrame(frame)
    }

    rafId = requestAnimationFrame(frame)
    return () => {
      cancelAnimationFrame(rafId)
      ctx.clearRect(0, 0, width, height)
    }
  }, [triggerSeq, animSpeed, reducedMotion])

  if (triggerSeq <= 0 || animSpeed === 'off' || reducedMotion) return null

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      className="pointer-events-none fixed inset-0 z-50"
    />
  )
}
