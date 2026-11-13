import { AppShell } from '@/components/layout/AppShell'
import { useTranslation, type TranslationKey } from '@/i18n'
import {
  useSettings,
  type AnimSpeed,
  type CardBackId,
  type HandSort,
  type Language,
  type ThemeId,
} from '@/settings/useSettings'

const THEME_OPTIONS: { id: ThemeId; labelKey: TranslationKey }[] = [
  { id: 'classic', labelKey: 'settings.themeClassic' },
  { id: 'neon', labelKey: 'settings.themeNeon' },
  { id: 'minimal', labelKey: 'settings.themeMinimal' },
]

const CARD_BACK_OPTIONS: { id: CardBackId; labelKey: TranslationKey }[] = [
  { id: 'classic', labelKey: 'settings.themeClassic' },
  { id: 'neon', labelKey: 'settings.themeNeon' },
  { id: 'minimal', labelKey: 'settings.themeMinimal' },
]

const LANG_OPTIONS: { id: Language; labelKey: TranslationKey }[] = [
  { id: 'zh-TW', labelKey: 'settings.langZh' },
  { id: 'en', labelKey: 'settings.langEn' },
]

const ANIM_OPTIONS: AnimSpeed[] = ['0.5x', '1x', '2x', 'off']

const SORT_OPTIONS: { id: HandSort; labelKey: TranslationKey }[] = [
  { id: 'color', labelKey: 'settings.sortColor' },
  { id: 'number', labelKey: 'settings.sortNumber' },
]

export function SettingsPage() {
  const { t } = useTranslation()
  const {
    theme,
    cardBack,
    volume,
    animSpeed,
    colorBlind,
    lang,
    vibration,
    handSort,
    setTheme,
    setCardBack,
    setVolume,
    setAnimSpeed,
    setColorBlind,
    setLang,
    setVibration,
    setHandSort,
    resetSettings,
  } = useSettings()

  return (
    <AppShell title={t('route.settings')}>
      <div className="mx-auto flex w-full max-w-lg flex-col gap-4">
        <section
          aria-labelledby="settings-theme-heading"
          className="rounded-2xl uno-surface p-4"
        >
          <h2
            id="settings-theme-heading"
            className="mb-3 text-sm font-semibold text-text-muted"
          >
            {t('settings.theme')}
          </h2>
          <div className="grid grid-cols-3 gap-2">
            {THEME_OPTIONS.map((opt) => (
              <button
                key={opt.id}
                type="button"
                aria-pressed={theme === opt.id}
                onClick={() => setTheme(opt.id)}
                className={`min-h-11 rounded-xl px-3 text-sm font-semibold transition ${
                  theme === opt.id
                    ? 'bg-btn-blue text-white'
                    : 'bg-surface-strong hover:brightness-110'
                }`}
              >
                {t(opt.labelKey)}
              </button>
            ))}
          </div>
        </section>

        <section className="rounded-2xl uno-surface p-4">
          <h2 className="mb-3 text-sm font-semibold text-text-muted">
            {t('settings.language')}
          </h2>
          <div className="grid grid-cols-2 gap-2">
            {LANG_OPTIONS.map((opt) => (
              <button
                key={opt.id}
                type="button"
                aria-pressed={lang === opt.id}
                onClick={() => setLang(opt.id)}
                className={`min-h-11 rounded-xl px-3 text-sm font-semibold transition ${
                  lang === opt.id
                    ? 'bg-btn-green text-white'
                    : 'bg-surface-strong hover:brightness-110'
                }`}
              >
                {t(opt.labelKey)}
              </button>
            ))}
          </div>
        </section>

        <section
          aria-labelledby="settings-cardback-heading"
          className="rounded-2xl uno-surface p-4"
        >
          <h2
            id="settings-cardback-heading"
            className="mb-3 text-sm font-semibold text-text-muted"
          >
            {t('settings.cardBack')}
          </h2>
          <div className="grid grid-cols-3 gap-2">
            {CARD_BACK_OPTIONS.map((opt) => (
              <button
                key={opt.id}
                type="button"
                aria-pressed={cardBack === opt.id}
                onClick={() => setCardBack(opt.id)}
                className={`min-h-11 rounded-xl px-3 text-sm font-semibold transition ${
                  cardBack === opt.id
                    ? 'bg-btn-red text-white'
                    : 'bg-surface-strong hover:brightness-110'
                }`}
              >
                {t(opt.labelKey)}
              </button>
            ))}
          </div>
        </section>

        <section className="rounded-2xl uno-surface p-4">
          <div className="mb-2 flex items-center justify-between">
            <label
              htmlFor="settings-volume"
              className="text-sm font-semibold text-text-muted"
            >
              {t('settings.volume')}
            </label>
            <span className="text-sm font-bold">
              {Math.round(volume * 100)}%
            </span>
          </div>
          <input
            id="settings-volume"
            type="range"
            min={0}
            max={1}
            step={0.05}
            value={volume}
            onChange={(e) => setVolume(Number(e.target.value))}
            className="h-11 w-full accent-uno-blue"
          />
        </section>

        <section className="rounded-2xl uno-surface p-4">
          <h2 className="mb-3 text-sm font-semibold text-text-muted">
            {t('settings.animSpeed')}
          </h2>
          <div className="grid grid-cols-4 gap-2">
            {ANIM_OPTIONS.map((speed) => (
              <button
                key={speed}
                type="button"
                aria-pressed={animSpeed === speed}
                onClick={() => setAnimSpeed(speed)}
                className={`min-h-11 rounded-xl px-2 text-sm font-semibold transition ${
                  animSpeed === speed
                    ? 'bg-btn-blue text-white'
                    : 'bg-surface-strong hover:brightness-110'
                }`}
              >
                {speed === 'off' ? t('settings.animOff') : speed}
              </button>
            ))}
          </div>
        </section>

        <section className="rounded-2xl uno-surface p-4">
          <h2 className="mb-3 text-sm font-semibold text-text-muted">
            {t('settings.handSort')}
          </h2>
          <div className="grid grid-cols-2 gap-2">
            {SORT_OPTIONS.map((opt) => (
              <button
                key={opt.id}
                type="button"
                aria-pressed={handSort === opt.id}
                onClick={() => setHandSort(opt.id)}
                className={`min-h-11 rounded-xl px-3 text-sm font-semibold transition ${
                  handSort === opt.id
                    ? 'bg-btn-green text-white'
                    : 'bg-surface-strong hover:brightness-110'
                }`}
              >
                {t(opt.labelKey)}
              </button>
            ))}
          </div>
        </section>

        <section className="flex flex-col gap-3 rounded-2xl uno-surface p-4">
          <button
            type="button"
            role="switch"
            aria-checked={colorBlind}
            onClick={() => setColorBlind(!colorBlind)}
            className="flex min-h-11 items-center justify-between rounded-xl bg-surface-strong px-4 text-sm font-semibold"
          >
            <span>{t('settings.colorBlind')}</span>
            <span>{colorBlind ? t('common.on') : t('common.off')}</span>
          </button>

          <button
            type="button"
            role="switch"
            aria-checked={vibration}
            onClick={() => setVibration(!vibration)}
            className="flex min-h-11 items-center justify-between rounded-xl bg-surface-strong px-4 text-sm font-semibold"
          >
            <span>{t('settings.vibration')}</span>
            <span>{vibration ? t('common.on') : t('common.off')}</span>
          </button>
        </section>

        <button
          type="button"
          onClick={resetSettings}
          className="min-h-11 rounded-2xl uno-surface px-4 text-sm font-semibold hover:bg-surface-strong"
        >
          {t('common.reset')}
        </button>
      </div>
    </AppShell>
  )
}
