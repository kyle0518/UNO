import type { PlayerView } from '@/generated/PlayerView'
import type { RuleConfig } from '@/generated/RuleConfig'
import type { SeatKind } from './GameClient'

export interface SoloMatchConfig {
  config: RuleConfig
  seed: number
  seats: SeatKind[]
  playerNames: string[]
}

export interface LocalReplayRecord {
  id: string
  replayJson: string
  config: RuleConfig
  seed: number
  playerNames: string[]
  seats?: SeatKind[]
  rankings: number[]
  scores: number[]
  stepsCount: number
  savedAt: string
}

const SOLO_CONFIG_KEY = 'uno-solo-pending-match'
const LOCAL_REPLAYS_KEY = 'uno-local-replays'
const LAST_RESULT_KEY = 'uno-solo-last-result'

let inMemoryPending: SoloMatchConfig | null = null
let inMemoryLastResult: LocalReplayRecord | null = null

export function savePendingSoloMatch(match: SoloMatchConfig): void {
  inMemoryPending = match
  try {
    sessionStorage.setItem(SOLO_CONFIG_KEY, JSON.stringify(match))
  } catch {
    // Fallback to in-memory only when storage is unavailable
  }
}

export function loadPendingSoloMatch(): SoloMatchConfig | null {
  if (inMemoryPending) return inMemoryPending
  try {
    const raw = sessionStorage.getItem(SOLO_CONFIG_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as SoloMatchConfig
    inMemoryPending = parsed
    return parsed
  } catch {
    return null
  }
}

export function generateSeed(): number {
  if (typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function') {
    const buf = new Uint32Array(2)
    crypto.getRandomValues(buf)
    // Keep within IEEE-754 safe integer range (0..2^53 - 1) for serde_wasm_bindgen json_compatible u64
    const hi = (buf[0] ?? 0) & 0x001f_ffff
    const lo = buf[1] ?? 0
    return hi * 0x1_0000_0000 + lo
  }
  return Math.floor(Math.random() * Number.MAX_SAFE_INTEGER)
}

export function getDailySeed(date = new Date()): number {
  const y = date.getUTCFullYear()
  const m = date.getUTCMonth() + 1
  const d = date.getUTCDate()
  return y * 10_000 + m * 100 + d
}

export function saveCompletedSoloResult(
  match: SoloMatchConfig,
  finalView: PlayerView,
  replayJson: string,
): LocalReplayRecord {
  let stepsCount: number
  try {
    const parsed = JSON.parse(replayJson) as { steps?: unknown[] }
    stepsCount = Array.isArray(parsed.steps) ? parsed.steps.length : 0
  } catch {
    stepsCount = finalView.turn
  }

  const record: LocalReplayRecord = {
    id: `local-${Date.now()}-${match.seed}`,
    replayJson,
    config: match.config,
    seed: match.seed,
    playerNames: match.playerNames,
    seats: match.seats,
    rankings: finalView.rankings ?? finalView.players.map((p) => p.seat),
    scores: finalView.players.map((p) => p.score),
    stepsCount,
    savedAt: new Date().toISOString(),
  }

  inMemoryLastResult = record
  try {
    sessionStorage.setItem(LAST_RESULT_KEY, JSON.stringify(record))
    const prevRaw = localStorage.getItem(LOCAL_REPLAYS_KEY)
    const prev = prevRaw ? (JSON.parse(prevRaw) as LocalReplayRecord[]) : []
    const next = [record, ...prev.filter((r) => r.id !== record.id)].slice(0, 10)
    localStorage.setItem(LOCAL_REPLAYS_KEY, JSON.stringify(next))
  } catch {
    // Ignore quota errors
  }

  return record
}

export function loadLastSoloResult(): LocalReplayRecord | null {
  if (inMemoryLastResult) return inMemoryLastResult
  try {
    const raw = sessionStorage.getItem(LAST_RESULT_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as LocalReplayRecord
    inMemoryLastResult = parsed
    return parsed
  } catch {
    return null
  }
}

export function loadLocalReplays(): LocalReplayRecord[] {
  try {
    const raw = localStorage.getItem(LOCAL_REPLAYS_KEY)
    if (!raw) return inMemoryLastResult ? [inMemoryLastResult] : []
    return JSON.parse(raw) as LocalReplayRecord[]
  } catch {
    return inMemoryLastResult ? [inMemoryLastResult] : []
  }
}

export function loadReplayById(id: string): LocalReplayRecord | null {
  if (id === 'local') {
    return loadLastSoloResult() ?? loadLocalReplays()[0] ?? null
  }
  const found = loadLocalReplays().find((item) => item.id === id)
  if (found) return found
  const last = loadLastSoloResult()
  return last && last.id === id ? last : null
}

export function resetSoloSessionForTest(): void {
  inMemoryPending = null
  inMemoryLastResult = null
}
