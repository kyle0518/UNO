import QRCode from 'qrcode'
import { useEffect, useRef } from 'react'

export interface QrCanvasProps {
  value: string
  size?: number
  className?: string
}

export function QrCanvas({ value, size = 180, className = '' }: QrCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas || !value) return
    let active = true
    void QRCode.toCanvas(canvas, value, {
      width: size,
      margin: 1,
      color: {
        dark: '#0f172a',
        light: '#ffffff',
      },
    }).catch(() => {
      if (active && canvas) {
        canvas.width = size
        canvas.height = size
      }
    })
    return () => {
      active = false
    }
  }, [value, size])

  return (
    <canvas
      ref={canvasRef}
      width={size}
      height={size}
      aria-label={value}
      className={`rounded-xl bg-white ${className}`}
    />
  )
}
