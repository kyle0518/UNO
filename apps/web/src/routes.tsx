import { Route, Routes } from 'react-router'
import type { TranslationKey } from '@/i18n'
import { DevCardsPage } from '@/pages/DevCardsPage'
import { GamePage } from '@/pages/GamePage'
import { HomePage } from '@/pages/HomePage'
import { LoginPage } from '@/pages/LoginPage'
import { PlaceholderPage } from '@/pages/PlaceholderPage'
import { RegisterPage } from '@/pages/RegisterPage'
import { ReplayPage } from '@/pages/ReplayPage'
import { ResultsPage } from '@/pages/ResultsPage'
import { RulesPage } from '@/pages/RulesPage'
import { SettingsPage } from '@/pages/SettingsPage'
import { SoloSetupPage } from '@/pages/SoloSetupPage'

const PLACEHOLDER_ROUTES: { path: string; titleKey: TranslationKey }[] = [
  { path: '/create', titleKey: 'route.createRoom' },
  { path: '/join', titleKey: 'route.joinRoom' },
  { path: '/r/:code', titleKey: 'route.room' },
  { path: '/stats', titleKey: 'route.stats' },
  { path: '/stats/:userId', titleKey: 'route.statsUser' },
  { path: '/leaderboard', titleKey: 'route.leaderboard' },
  { path: '/lab', titleKey: 'route.lab' },
  { path: '/daily', titleKey: 'route.daily' },
]

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<HomePage />} />
      <Route path="/login" element={<LoginPage />} />
      <Route path="/register" element={<RegisterPage />} />
      <Route path="/solo" element={<SoloSetupPage />} />
      <Route path="/game" element={<GamePage />} />
      <Route path="/results" element={<ResultsPage />} />
      <Route path="/replay/:id" element={<ReplayPage />} />
      <Route path="/rules" element={<RulesPage />} />
      <Route path="/settings" element={<SettingsPage />} />
      {import.meta.env.DEV && (
        <Route path="/dev/cards" element={<DevCardsPage />} />
      )}
      {PLACEHOLDER_ROUTES.map((route) => (
        <Route
          key={route.path}
          path={route.path}
          element={<PlaceholderPage titleKey={route.titleKey} />}
        />
      ))}
      <Route path="*" element={<PlaceholderPage titleKey="route.notFound" />} />
    </Routes>
  )
}
