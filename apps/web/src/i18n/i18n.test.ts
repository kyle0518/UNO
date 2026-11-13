import { beforeEach, describe, expect, it } from 'vitest'
import { useSettings } from '@/settings/useSettings'
import { en } from './en'
import { t, translate } from './index'
import { zhTW } from './zh-TW'

describe('i18n', () => {
  beforeEach(() => {
    localStorage.clear()
    useSettings.getState().resetSettings()
  })

  it('has identical translation keys between zh-TW and en', () => {
    expect(Object.keys(zhTW).sort()).toEqual(Object.keys(en).sort())
  })

  it('interpolates parameters and switches languages', () => {
    expect(translate('zh-TW', 'route.room', { code: 'AB12CD' })).toBe(
      '房間大廳 #AB12CD',
    )
    expect(translate('en', 'route.room', { code: 'AB12CD' })).toBe(
      'Room Lobby #AB12CD',
    )

    expect(t('home.settings')).toBe('設定')
    useSettings.getState().setLang('en')
    expect(t('home.settings')).toBe('Settings')
  })
})
