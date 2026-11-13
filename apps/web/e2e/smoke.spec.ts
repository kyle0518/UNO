import AxeBuilder from '@axe-core/playwright'
import { expect, test } from '@playwright/test'

test.describe('UNO web smoke & accessibility', () => {
  test('home page presents streamlined solo entry without server clutter', async ({
    page,
  }) => {
    await page.goto('/')
    await expect(page.getByRole('link', { name: '快速 vs AI' })).toBeVisible()
    await expect(page.getByRole('link', { name: '規則說明' })).toBeVisible()
    await expect(page.getByRole('link', { name: '設定' })).toBeVisible()
    await expect(page.getByRole('link', { name: '建立房間' })).toHaveCount(0)
    await expect(page.getByRole('link', { name: 'AI 實驗室' })).toHaveCount(0)
  })

  test('rules page displays cards and completes 4-step tutorial', async ({
    page,
  }) => {
    await page.goto('/rules')
    await expect(page.getByText('基本玩法')).toBeVisible()

    await page.getByRole('button', { name: '查看 4 步新手教學' }).click()
    await expect(page.getByRole('dialog')).toBeVisible()
    await expect(page.getByText('教學 1 / 4')).toBeVisible()

    await page.getByRole('button', { name: '下一步' }).click()
    await page.getByRole('button', { name: '下一步' }).click()
    await page.getByRole('button', { name: '下一步' }).click()
    await expect(page.getByText('教學 4 / 4')).toBeVisible()

    await page.getByRole('button', { name: '開始遊戲' }).click()
    await expect(page.getByRole('dialog')).toHaveCount(0)

    const storage = await page.context().storageState()
    const originStorage = storage.origins.find((o) =>
      o.origin.includes('127.0.0.1:4173'),
    )
    expect(originStorage?.localStorage).toContainEqual({
      name: 'uno-tutorial-seen',
      value: '1',
    })
  })

  test('dev cards page renders all card faces', async ({ page }) => {
    await page.goto('/dev/cards')
    const imgs = page.locator('[role="img"]')
    await expect(imgs.first()).toBeVisible()
    expect(await imgs.count()).toBeGreaterThanOrEqual(25)
  })

  for (const path of ['/', '/solo', '/settings', '/rules']) {
    test(`axe accessibility check passes on ${path}`, async ({ page }) => {
      await page.goto(path)
      const results = await new AxeBuilder({ page }).analyze()
      const severe = results.violations.filter(
        (v) => v.impact === 'serious' || v.impact === 'critical',
      )
      expect(severe).toEqual([])
    })
  }
})
