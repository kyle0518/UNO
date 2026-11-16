import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router'
import { apiClient, type DailyInfo } from '@/api/client'
import { useAuth } from '@/auth/useAuth'
import { AppShell } from '@/components/layout/AppShell'
import type { SeatKind } from '@/game/GameClient'
import { saveDailyPending, savePendingSoloMatch } from '@/game/soloSession'
import { useTranslation } from '@/i18n'

export function DailyPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const user = useAuth((s) => s.user)
  const [info, setInfo] = useState<DailyInfo | null>(null)
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    let active = true
    void apiClient
      .daily()
      .then((d) => {
        if (active) setInfo(d)
      })
      .catch(() => {
        if (active) setFailed(true)
      })
    return () => {
      active = false
    }
  }, [])

  const handlePlay = () => {
    if (!info) return
    const seats: SeatKind[] = ['human', 'hard', 'hard', 'hard']
    const playerNames = [
      user?.name ?? t('game.playerYou'),
      t('game.playerAi', { seat: 1 }),
      t('game.playerAi', { seat: 2 }),
      t('game.playerAi', { seat: 3 }),
    ]
    saveDailyPending(info.date)
    savePendingSoloMatch({
      config: info.config,
      seed: info.seed,
      seats,
      playerNames,
    })
    void navigate('/game')
  }

  return (
    <AppShell title={t('route.daily')}>
      <div className="mx-auto flex w-full max-w-md flex-col gap-5">
        <section className="uno-page-panel flex flex-col gap-3 p-6 text-center">
          <h2 className="font-display text-2xl font-bold">{t('daily.today')}</h2>
          {info && <p className="text-sm text-text-muted">{info.date}</p>}
          {info &&
            (info.my_best !== null ? (
              <p className="text-base font-bold">{t('daily.myBest', { score: info.my_best })}</p>
            ) : (
              <p className="text-sm text-text-muted">{t('daily.noBest')}</p>
            ))}
          {failed && (
            <p role="alert" className="text-sm font-semibold text-red-300">
              {t('stats.loadFailed')}
            </p>
          )}
          <button
            type="button"
            disabled={!info}
            onClick={handlePlay}
            className="min-h-12 rounded-2xl bg-btn-green px-5 text-base font-bold text-white disabled:opacity-40"
          >
            {t('daily.play')}
          </button>
        </section>
      </div>
    </AppShell>
  )
}
