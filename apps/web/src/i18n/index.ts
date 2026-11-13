import { useCallback } from 'react'
import { useSettings, type Language } from '@/settings/useSettings'
import { en } from './en'
import { zhTW, type TranslationKey } from './zh-TW'

export type { TranslationKey }

const dictionaries: Record<Language, Record<TranslationKey, string>> = {
  'zh-TW': zhTW,
  en,
}

export function translate(
  lang: Language,
  key: TranslationKey,
  vars?: Record<string, string | number>,
): string {
  const template = dictionaries[lang][key]
  if (!vars) return template
  return template.replace(/\{(\w+)\}/g, (_, name: string) =>
    vars[name] !== undefined ? String(vars[name]) : `{${name}}`,
  )
}

export function t(
  key: TranslationKey,
  vars?: Record<string, string | number>,
): string {
  return translate(useSettings.getState().lang, key, vars)
}

export function useTranslation() {
  const lang = useSettings((s) => s.lang)
  const tr = useCallback(
    (key: TranslationKey, vars?: Record<string, string | number>) =>
      translate(lang, key, vars),
    [lang],
  )
  return { lang, t: tr }
}
