/// <reference types="node" />
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { fireEvent, render, screen, within } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Action } from '@/generated/Action'
import type { PlayerView } from '@/generated/PlayerView'
import type { Replay } from '@/generated/Replay'
import type { ReplayStep } from '@/generated/ReplayStep'
import type { RuleConfig } from '@/generated/RuleConfig'
import {
  resetSoloSessionForTest,
  type LocalReplayRecord,
} from '@/game/soloSession'
import { useSettings } from '@/settings/useSettings'
import { WasmBot, WasmGame, initSync, preset } from '@/wasm/pkg/uno_wasm'
import { ReplayPage } from './ReplayPage'

vi.mock('@/wasm/init', () => ({
  ensureWasmInit: () => Promise.resolve(),
}))

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const wasmBytes = readFileSync(
  path.join(__dirname, '../wasm/pkg/uno_wasm_bg.wasm'),
)
initSync({ module: wasmBytes })

function buildSampleReplayRecord(maxSteps = 10): LocalReplayRecord {
  const cfg = preset('classic', 4) as RuleConfig
  const seed = 123456
  const game = new WasmGame(cfg, BigInt(seed))
  const bots = [0, 1, 2, 3].map((i) => new WasmBot('heuristic', BigInt(seed + i)))
  const steps: ReplayStep[] = []

  for (let i = 0; i < maxSteps; i += 1) {
    const decision = game.decision_player()
    if (decision === undefined || decision === null) break
    const view = game.view(decision) as PlayerView
    if (view.phase === 'round_over' || view.phase === 'game_over') break
    const act = bots[decision]!.act(view) as Action
    game.apply(decision, act)
    steps.push({ seat: decision, action: act, t: (i + 1) * 500 })
  }

  const spectator = game.spectator_view() as PlayerView
  const playerNames = ['You', 'AI 1', 'AI 2', 'AI 3']
  const replay: Replay = {
    replay_version: 1,
    engine_version: '0.1.0',
    config: cfg,
    seed,
    players: playerNames.map((name, seat) => ({
      seat,
      name,
      user_id: null,
      kind: seat === 0 ? 'human' : 'bot',
      bot: seat === 0 ? undefined : 'heuristic',
    })),
    steps,
    created_at: '2026-11-13T00:00:00.000Z',
  }

  game.free()
  bots.forEach((b) => b.free())

  return {
    id: `local-test-${seed}`,
    replayJson: JSON.stringify(replay),
    config: cfg,
    seed,
    playerNames,
    seats: ['human', 'heuristic', 'heuristic', 'heuristic'],
    rankings: spectator.rankings ?? [0, 1, 2, 3],
    scores: spectator.players.map((p) => p.score),
    stepsCount: steps.length,
    savedAt: '2026-11-13T00:00:00.000Z',
  }
}

function renderReplayRoute(initialPath = '/replay/local') {
  return render(
    <MemoryRouter initialEntries={[initialPath]}>
      <Routes>
        <Route path="/replay/:id" element={<ReplayPage />} />
      </Routes>
    </MemoryRouter>,
  )
}

describe('ReplayPage', () => {
  beforeEach(() => {
    resetSoloSessionForTest()
    localStorage.clear()
    sessionStorage.clear()
    useSettings.getState().resetSettings()
  })

  it('shows empty state when no replay exists locally', () => {
    renderReplayRoute('/replay/local')
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(
      /重播檢視器 #local/i,
    )
    expect(screen.getByText(/找不到本地重播資料/i)).toBeInTheDocument()
  })

  it('steps forward/backward, scrubs timeline, and toggles God View', async () => {
    const record = buildSampleReplayRecord(10)
    sessionStorage.setItem('uno-solo-last-result', JSON.stringify(record))
    localStorage.setItem('uno-local-replays', JSON.stringify([record]))

    renderReplayRoute('/replay/local')

    const seat1Normal = await screen.findByTestId('replay-seat-hand-1')
    expect(
      within(seat1Normal).getAllByRole('img', { name: /UNO 牌背|UNO card back/i })
        .length,
    ).toBeGreaterThan(0)

    const seat0 = screen.getByTestId('replay-seat-hand-0')
    expect(
      within(seat0).queryAllByRole('img', { name: /UNO 牌背|UNO card back/i }),
    ).toHaveLength(0)

    expect(screen.getByText(/第 0 \/ 10 步|Step 0 \/ 10/i)).toBeInTheDocument()

    fireEvent.click(screen.getByTestId('replay-next'))
    expect(screen.getByText(/第 1 \/ 10 步|Step 1 \/ 10/i)).toBeInTheDocument()

    fireEvent.change(screen.getByTestId('replay-scrubber'), {
      target: { value: '6' },
    })
    expect(screen.getByText(/第 6 \/ 10 步|Step 6 \/ 10/i)).toBeInTheDocument()

    fireEvent.click(screen.getByTestId('replay-prev'))
    expect(screen.getByText(/第 5 \/ 10 步|Step 5 \/ 10/i)).toBeInTheDocument()

    const godToggle = screen.getByTestId('replay-god-view-toggle')
    expect(godToggle).toHaveAttribute('aria-pressed', 'false')
    fireEvent.click(godToggle)
    expect(godToggle).toHaveAttribute('aria-pressed', 'true')

    const seat1God = screen.getByTestId('replay-seat-hand-1')
    expect(
      within(seat1God).queryAllByRole('img', { name: /UNO 牌背|UNO card back/i }),
    ).toHaveLength(0)
  })

  it('loads a saved replay by explicit id from localStorage', async () => {
    const record = buildSampleReplayRecord(6)
    localStorage.setItem('uno-local-replays', JSON.stringify([record]))

    renderReplayRoute(`/replay/${record.id}`)

    await screen.findByTestId('replay-seat-hand-0')
    expect(screen.getByText(/第 0 \/ 6 步|Step 0 \/ 6/i)).toBeInTheDocument()
  })
})
