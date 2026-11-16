import { useEffect, useMemo, useState } from 'react'
import { Link, useParams } from 'react-router'
import {
  apiClient,
  type UserMatchSummary,
  type UserStatsResponse,
} from '@/api/client'
import { useAuth } from '@/auth/useAuth'
import { AppShell } from '@/components/layout/AppShell'
import { useTranslation } from '@/i18n'

function EloChart({ stats, mode }: { stats: UserStatsResponse; mode: string }) {
  const points = useMemo(() => {
    const hist = stats.elo_history.filter((h) => h.mode_id === mode)
    if (hist.length === 0) return null
    const values = [1200, ...hist.map((h) => h.elo)]
    const min = Math.min(...values) - 20
    const max = Math.max(...values) + 20
    const w = 320
    const h = 96
    const stepX = values.length > 1 ? w / (values.length - 1) : w
    return values
      .map((v, i) => `${(i * stepX).toFixed(1)},${(h - ((v - min) / (max - min)) * h).toFixed(1)}`)
      .join(' ')
  }, [stats, mode])

  if (!points) return null
  return (
    <svg viewBox="0 0 320 96" className="w-full" role="img" aria-label={mode}>
      <polyline points={points} fill="none" stroke="currentColor" strokeWidth="2" className="text-accent" />
    </svg>
  )
}

export function StatsPage() {
  const { t } = useTranslation()
  const params = useParams()
  const authUser = useAuth((s) => s.user)
  const userId = params.userId ? Number(params.userId) : (authUser?.id ?? null)
  const [stats, setStats] = useState<UserStatsResponse | null>(null)
  const [matches, setMatches] = useState<UserMatchSummary[]>([])
  const [mode, setMode] = useState('classic')
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    if (userId === null || Number.isNaN(userId)) return
    let active = true
    void Promise.all([apiClient.userStats(userId), apiClient.userMatches(userId, 20)])
      .then(([s, m]) => {
        if (!active) return
        setStats(s)
        setMatches(m)
        const modes = Object.keys(s.by_mode)
        if (modes.length > 0 && !modes.includes('classic')) setMode(modes[0]!)
      })
      .catch(() => {
        if (active) setFailed(true)
      })
    return () => {
      active = false
    }
  }, [userId])

  const modes = stats ? Object.keys(stats.by_mode) : []

  return (
    <AppShell
      title={
        params.userId
          ? t('route.statsUser', { userId: params.userId })
          : t('route.stats')
      }
    >
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-5">
        {userId === null && <p className="text-sm text-text-muted">{t('stats.needLogin')}</p>}
        {failed && (
          <p role="alert" className="text-sm font-semibold text-red-300">
            {t('stats.loadFailed')}
          </p>
        )}
        {stats && (
          <>
            <section className="grid grid-cols-3 gap-3">
              {[
                { label: t('stats.totalGames'), value: String(stats.games) },
                { label: t('stats.totalWins'), value: String(stats.wins) },
                {
                  label: t('stats.winRate'),
                  value: stats.games > 0 ? `${Math.round((stats.wins / stats.games) * 100)}%` : '—',
                },
              ].map((c) => (
                <div key={c.label} className="uno-page-panel p-4 text-center">
                  <div className="font-display text-3xl font-bold">{c.value}</div>
                  <div className="text-xs text-text-muted">{c.label}</div>
                </div>
              ))}
            </section>

            <section className="uno-page-panel flex flex-col gap-3 p-5">
              <h2 className="text-sm font-bold">{t('stats.byMode')}</h2>
              {modes.length === 0 && <p className="text-sm text-text-muted">{t('stats.noData')}</p>}
              <div className="flex flex-wrap gap-2">
                {modes.map((m) => (
                  <button
                    key={m}
                    type="button"
                    onClick={() => setMode(m)}
                    className={[
                      'min-h-11 rounded-xl px-3 text-sm font-semibold',
                      mode === m ? 'bg-btn-blue text-white' : 'bg-white/10',
                    ].join(' ')}
                  >
                    {m}: {stats.by_mode[m]?.elo} ({stats.by_mode[m]?.games})
                  </button>
                ))}
              </div>
              <h3 className="text-xs font-bold text-text-muted">
                {t('stats.eloHistory')} · {mode}
              </h3>
              <EloChart stats={stats} mode={mode} />
            </section>

            <section className="uno-page-panel flex flex-col gap-2 p-5">
              <h2 className="text-sm font-bold">{t('stats.recentMatches')}</h2>
              {matches.length === 0 && (
                <p className="text-sm text-text-muted">{t('stats.noData')}</p>
              )}
              <ul className="flex flex-col gap-2">
                {matches.map((m) => (
                  <li
                    key={m.match_id}
                    className="flex items-center justify-between gap-2 rounded-xl bg-black/25 px-3 py-2 text-sm"
                  >
                    <span>
                      #{m.match_id} · {m.mode_id} · {t('stats.rankLabel', { rank: m.rank + 1 })} ·{' '}
                      {t('stats.scoreLabel', { score: m.score })} ·{' '}
                      {t('stats.playersLabel', { count: m.num_players })}
                    </span>
                    <Link
                      to={`/replay/${m.match_id}`}
                      className="min-h-11 shrink-0 rounded-xl bg-btn-blue px-3 py-2 text-xs font-bold text-white"
                    >
                      {t('stats.watchReplay')}
                    </Link>
                  </li>
                ))}
              </ul>
            </section>
          </>
        )}
      </div>
    </AppShell>
  )
}
