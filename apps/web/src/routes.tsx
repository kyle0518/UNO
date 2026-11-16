import { Route, Routes } from 'react-router'
import type { TranslationKey } from '@/i18n'
import { CreateRoomPage } from '@/pages/CreateRoomPage'
import { DailyPage } from '@/pages/DailyPage'
import { DevCardsPage } from '@/pages/DevCardsPage'
import { GamePage } from '@/pages/GamePage'
import { HomePage } from '@/pages/HomePage'
import { JoinRoomPage } from '@/pages/JoinRoomPage'
import { LeaderboardPage } from '@/pages/LeaderboardPage'
import { LoginPage } from '@/pages/LoginPage'
import { PlaceholderPage } from '@/pages/PlaceholderPage'
import { RegisterPage } from '@/pages/RegisterPage'
import { ReplayPage } from '@/pages/ReplayPage'
import { ResultsPage } from '@/pages/ResultsPage'
import { RoomPage } from '@/pages/RoomPage'
import { RulesPage } from '@/pages/RulesPage'
import { SettingsPage } from '@/pages/SettingsPage'
import { SoloSetupPage } from '@/pages/SoloSetupPage'
import { StatsPage } from '@/pages/StatsPage'

const PLACEHOLDER_ROUTES: { path: string; titleKey: TranslationKey }[] = [
  { path: '/lab', titleKey: 'route.lab' },
]

export function AppRoutes() {
  return (
    <Routes>
      <Route path="/" element={<HomePage />} />
      <Route path="/login" element={<LoginPage />} />
      <Route path="/register" element={<RegisterPage />} />
      <Route path="/create" element={<CreateRoomPage />} />
      <Route path="/join" element={<JoinRoomPage />} />
      <Route path="/r/:code" element={<RoomPage />} />
      <Route path="/daily" element={<DailyPage />} />
      <Route path="/stats" element={<StatsPage />} />
      <Route path="/stats/:userId" element={<StatsPage />} />
      <Route path="/leaderboard" element={<LeaderboardPage />} />
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
