import { useTranslation, type TranslationKey } from '@/i18n'
import {
  useSettings,
  type AnimSpeed,
  type CardBackId,
  type HandSort,
  type Language,
  type ThemeId,
} from '@/settings/useSettings'

interface OptionItem<T extends string> {
  id: T
  label: string
}

function OptionGroup<T extends string>({
  headingId,
  title,
  cols,
  value,
  options,
  activeClass,
  onSelect,
}: {
  headingId?: string
  title: string
  cols: 2 | 3 | 4
  value: T
  options: readonly OptionItem<T>[]
  activeClass: string
  onSelect: (next: T) => void
}) {
  const gridCols =
    cols === 4 ? 'grid-cols-4' : cols === 3 ? 'grid-cols-3' : 'grid-cols-2'
  return (
    <section
      aria-labelledby={headingId}
      className="rounded-2xl uno-surface p-4"
    >
      <h2
        id={headingId}
        className="mb-3 text-sm font-semibold text-text-muted"
      >
        {title}
      </h2>
      <div className={`grid gap-2 ${gridCols}`}>
        {options.map((opt) => (
          <button
            key={opt.id}
            type="button"
            aria-pressed={value === opt.id}
            onClick={() => onSelect(opt.id)}
            className={`min-h-11 rounded-xl px-2 text-sm font-semibold transition ${
              value === opt.id
                ? `${activeClass} text-white`
                : 'bg-surface-strong hover:brightness-110'
            }`}
          >
            {opt.label}
          </button>
        ))}
      </div>
    </section>
  )
}

function SwitchRow({
  label,
  checked,
  onToggle,
}: {
  label: string
  checked: boolean
  onToggle: () => void
}) {
  const { t } = useTranslation()
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={onToggle}
      className="flex min-h-11 items-center justify-between rounded-xl bg-surface-strong px-4 text-sm font-semibold"
    >
      <span>{label}</span>
      <span>{checked ? t('common.on') : t('common.off')}</span>
    </button>
  )
}

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

export function SettingsPanel() {
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
    <div className="mx-auto flex w-full max-w-lg flex-col gap-4">
      <OptionGroup
        headingId="settings-theme-heading"
        title={t('settings.theme')}
        cols={3}
        value={theme}
        options={THEME_OPTIONS.map((o) => ({ id: o.id, label: t(o.labelKey) }))}
        activeClass="bg-btn-blue"
        onSelect={setTheme}
      />

      <OptionGroup
        title={t('settings.language')}
        cols={2}
        value={lang}
        options={LANG_OPTIONS.map((o) => ({ id: o.id, label: t(o.labelKey) }))}
        activeClass="bg-btn-green"
        onSelect={setLang}
      />

      <OptionGroup
        headingId="settings-cardback-heading"
        title={t('settings.cardBack')}
        cols={3}
        value={cardBack}
        options={CARD_BACK_OPTIONS.map((o) => ({ id: o.id, label: t(o.labelKey) }))}
        activeClass="bg-btn-red"
        onSelect={setCardBack}
      />

      <section className="rounded-2xl uno-surface p-4">
        <div className="mb-2 flex items-center justify-between">
          <label
            htmlFor="settings-volume"
            className="text-sm font-semibold text-text-muted"
          >
            {t('settings.volume')}
          </label>
          <span className="text-sm font-bold">{Math.round(volume * 100)}%</span>
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

      <OptionGroup
        title={t('settings.animSpeed')}
        cols={4}
        value={animSpeed}
        options={ANIM_OPTIONS.map((s) => ({
          id: s,
          label: s === 'off' ? t('settings.animOff') : s,
        }))}
        activeClass="bg-btn-blue"
        onSelect={setAnimSpeed}
      />

      <OptionGroup
        title={t('settings.handSort')}
        cols={2}
        value={handSort}
        options={SORT_OPTIONS.map((o) => ({ id: o.id, label: t(o.labelKey) }))}
        activeClass="bg-btn-green"
        onSelect={setHandSort}
      />

      <section className="flex flex-col gap-3 rounded-2xl uno-surface p-4">
        <SwitchRow
          label={t('settings.colorBlind')}
          checked={colorBlind}
          onToggle={() => setColorBlind(!colorBlind)}
        />
        <SwitchRow
          label={t('settings.vibration')}
          checked={vibration}
          onToggle={() => setVibration(!vibration)}
        />
      </section>

      <button
        type="button"
        onClick={resetSettings}
        className="min-h-11 rounded-2xl uno-surface px-4 text-sm font-semibold hover:bg-surface-strong"
      >
        {t('common.reset')}
      </button>
    </div>
  )
}
