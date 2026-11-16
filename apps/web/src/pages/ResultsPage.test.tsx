import { fireEvent, render, screen } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { beforeEach, describe, expect, it } from 'vitest'
import {
  loadPendingSoloMatch,
  resetSoloSessionForTest,
  type LocalReplayRecord,
} from '@/game/soloSession'
import type { RuleConfig } from '@/generated/RuleConfig'
import { useSettings } from '@/settings/useSettings'
import { ResultsPage } from './ResultsPage'

function mockConfig(): RuleConfig {
  return {
    mode_id: 'classic',
    mode: 'classic',
    num_players: 4,
    hand_size: 7,
    deck_count: 1,
    stack_draw: 'none',
    seven_zero: false,
    jump_in: false,
    draw_to_match: false,
    force_play: false,
    wild4_rule: 'challenge',
    uno_rule: 'manual',
    uno_penalty: 2,
    scoring: { type: 'points', target: 500 },
    teams: null,
    elimination: false,
    turn_seconds: 30,
    game_seconds: 0,
  }
}

describe('ResultsPage', () => {
  beforeEach(() => {
    resetSoloSessionForTest()
    localStorage.clear()
    sessionStorage.clear()
    useSettings.getState().resetSettings()
  })

  it('preserves chosen bot seats on Play Again and links to local replay viewer', () => {
    const record: LocalReplayRecord = {
      id: 'local-1-99',
      replayJson: '{"steps":[]}',
      config: mockConfig(),
      seed: 99,
      playerNames: ['You', 'Bot A', 'Bot B', 'Bot C'],
      seats: ['human', 'hard', 'random', 'heuristic'],
      rankings: [0, 1, 2, 3],
      scores: [500, 120, 80, 40],
      stepsCount: 18,
      savedAt: '2026-11-13T00:00:00.000Z',
    }
    sessionStorage.setItem('uno-solo-last-result', JSON.stringify(record))
    localStorage.setItem('uno-local-replays', JSON.stringify([record]))

    render(
      <MemoryRouter initialEntries={['/results']}>
        <Routes>
          <Route path="/results" element={<ResultsPage />} />
          <Route path="/game" element={<div>Game Route</div>} />
        </Routes>
      </MemoryRouter>,
    )

    const replayLink = screen.getByRole('link', { name: /看重播|View Replay/i })
    expect(replayLink).toHaveAttribute('href', '/replay/local')

    fireEvent.click(screen.getByRole('button', { name: /再來一局|Play Again/i }))
    const pending = loadPendingSoloMatch()
    expect(pending?.seats).toEqual(['human', 'hard', 'random', 'heuristic'])
    expect(pending?.playerNames).toEqual(['You', 'Bot A', 'Bot B', 'Bot C'])
  })

  it('uses mode_id for recent replay labels so party/blitz presets show accurately', () => {
    const blitzConfig: RuleConfig = {
      ...mockConfig(),
      mode_id: 'blitz',
      mode: 'classic',
      turn_seconds: 5,
      game_seconds: 180,
      uno_rule: 'auto',
    }
    const partyConfig: RuleConfig = {
      ...mockConfig(),
      mode_id: 'party',
      mode: 'classic',
      seven_zero: true,
      jump_in: true,
    }
    const baseRecord: LocalReplayRecord = {
      id: 'local-1-1',
      replayJson: '{"steps":[]}',
      config: blitzConfig,
      seed: 1,
      playerNames: ['You', 'Bot 1'],
      seats: ['human', 'heuristic'],
      rankings: [0, 1],
      scores: [100, 0],
      stepsCount: 12,
      savedAt: '2026-11-14T00:00:00.000Z',
    }
    const partyRecord: LocalReplayRecord = {
      ...baseRecord,
      id: 'local-2-2',
      config: partyConfig,
    }
    sessionStorage.setItem('uno-solo-last-result', JSON.stringify(baseRecord))
    localStorage.setItem('uno-local-replays', JSON.stringify([baseRecord, partyRecord]))

    render(
      <MemoryRouter initialEntries={['/results']}>
        <Routes>
          <Route path="/results" element={<ResultsPage />} />
        </Routes>
      </MemoryRouter>,
    )

    expect(screen.getAllByText(/閃電 Blitz|Blitz/i).length).toBeGreaterThanOrEqual(2)
    expect(screen.getByText(/派對 Party|Party/i)).toBeInTheDocument()
  })
})
