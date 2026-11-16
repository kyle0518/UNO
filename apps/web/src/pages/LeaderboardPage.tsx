import { useEffect, useState } from 'react'
import { apiClient, type LeaderboardEntry } from '@/api/client'
import { AppShell } from '@/components/layout/AppShell'
import { useTranslation } from '@/i18n'

const MODES = ['classic', 'party', 'flip', 'no_mercy', 'teams', 'elimination', 'blitz', 'daily'] as const

export function LeaderboardPage() {
  const { t } = useTranslation()
  const [mode, setMode] = useState<string>('classic')
  const [entries, setEntries] = useState<LeaderboardEntry[]>([])
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    let active = true
    void apiClient
      .leaderboard(mode, 50)
      .then((list) => {
        if (active) setEntries(list)
      })
      .catch(() => {
        if (active) setFailed(true)
      })
    return () => {
      active = false
    }
  }, [mode])

  return (
    <AppShell title={t('leaderboard.title')}>
      <div className="mx-auto flex w-full max-w-2xl flex-col gap-5">
        <div className="flex flex-wrap gap-2">
          {MODES.map((m) => (
            <button
              key={m}
              type="button"
              onClick={() => setMode(m)}
              className={[
                'min-h-11 rounded-xl px-3 text-sm font-semibold',
                mode === m ? 'bg-btn-blue text-white' : 'bg-white/10',
              ].join(' ')}
            >
              {m}
            </button>
          ))}
        </div>
        {failed && (
          <p role="alert" className="text-sm font-semibold text-red-300">
            {t('stats.loadFailed')}
          </p>
        )}
        {entries.length === 0 && !failed && (
          <p className="text-sm text-text-muted">{t('stats.noData')}</p>
        )}
        <ol className="flex flex-col gap-2">
          {entries.map((e, idx) => (
            <li
              key={e.user.id}
              className="flex items-center justify-between gap-2 rounded-2xl bg-black/25 px-4 py-3 text-sm"
            >
              <span className="font-bold">
                #{idx + 1} {e.user.name}
                {e.user.is_guest && <span className="text-text-muted"> · {t('home.guestBadge')}</span>}
              </span>
              <span className="flex gap-4 text-text-muted">
                <span>
                  {t('leaderboard.elo')}: <b className="text-text-primary">{e.elo}</b>
                </span>
                <span>
                  {t('leaderboard.games')}: {e.games}
                </span>
                <span>
                  {t('leaderboard.winsCol')}: {e.wins}
                </span>
              </span>
            </li>
          ))}
        </ol>
      </div>
    </AppShell>
  )
}
