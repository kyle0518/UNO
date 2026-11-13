import path from 'node:path'
import { fileURLToPath } from 'node:url'
import AxeBuilder from '@axe-core/playwright'
import { devices, expect, test, type Page } from '@playwright/test'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const SCREENSHOTS_DIR = path.resolve(__dirname, '../../../docs/screenshots')

async function startClassicSoloGame(
  page: Page,
  skipTutorial = true,
  targetScore?: 200 | 300 | 500 | 1000,
) {
  const settingsJson = JSON.stringify({
    state: { animSpeed: 'off', volume: 0, vibration: false },
    version: 0,
  })
  const initScript = [
    `globalThis.localStorage.setItem('uno-settings', ${JSON.stringify(settingsJson)});`,
    skipTutorial
      ? `globalThis.localStorage.setItem('uno-tutorial-seen', '1');`
      : `globalThis.localStorage.removeItem('uno-tutorial-seen');`,
  ].join('\n')
  await page.addInitScript(initScript)

  await page.goto('/')
  await page.getByRole('link', { name: '快速 vs AI' }).click()
  await expect(page).toHaveURL(/\/solo$/)

  const startBtn = page.getByTestId('start-solo-game')
  await expect(startBtn).toBeEnabled({ timeout: 15_000 })
  if (targetScore !== undefined) {
    await page.getByRole('button', { name: /房規與細節設定/ }).click()
    await page.getByRole('button', { name: String(targetScore), exact: true }).click()
    await expect(page.getByText(`目標分數：${targetScore} 分`)).toBeVisible()
  }
  await startBtn.click()
  await expect(page).toHaveURL(/\/game$/)
  await expect(page.getByTestId('player-hand')).toBeVisible({ timeout: 15_000 })
}

test.describe('Solo UNO game layouts & full match E2E', () => {
  test('captures portrait screenshots on iPhone 13 and Pixel 7 without overflow', async ({
    browser,
  }) => {
    const iphoneCtx = await browser.newContext({ ...devices['iPhone 13'] })
    const iphonePage = await iphoneCtx.newPage()
    await startClassicSoloGame(iphonePage, true)
    await expect(iphonePage.getByTestId('draw-pile')).toBeVisible()
    await iphonePage.screenshot({
      path: path.join(SCREENSHOTS_DIR, 'game-iphone13.png'),
    })
    const iphoneScroll = await iphonePage.evaluate(
      'document.documentElement.scrollWidth <= window.innerWidth',
    )
    expect(iphoneScroll).toBe(true)
    await iphoneCtx.close()

    const pixelCtx = await browser.newContext({ ...devices['Pixel 7'] })
    const pixelPage = await pixelCtx.newPage()
    await startClassicSoloGame(pixelPage, true)
    await pixelPage.screenshot({
      path: path.join(SCREENSHOTS_DIR, 'game-pixel7.png'),
    })
    const pixelScroll = await pixelPage.evaluate(
      'document.documentElement.scrollWidth <= window.innerWidth',
    )
    expect(pixelScroll).toBe(true)

    const axeResults = await new AxeBuilder({ page: pixelPage }).analyze()
    const severe = axeResults.violations.filter(
      (v) => v.impact === 'serious' || v.impact === 'critical',
    )
    expect(severe).toEqual([])
    await pixelCtx.close()
  })

  test('captures landscape screenshots on 1280x800 desktop and landscape iPhone', async ({
    browser,
  }) => {
    const desktopCtx = await browser.newContext({
      viewport: { width: 1280, height: 800 },
    })
    const desktopPage = await desktopCtx.newPage()
    await startClassicSoloGame(desktopPage, true)
    await desktopPage.screenshot({
      path: path.join(SCREENSHOTS_DIR, 'game-desktop-1280x800.png'),
    })
    const ovalOpponents = await desktopPage.locator('[data-seat]').count()
    expect(ovalOpponents).toBe(3)
    await desktopCtx.close()

    const iphoneLandCtx = await browser.newContext({
      ...devices['iPhone 13 landscape'],
    })
    const iphoneLandPage = await iphoneLandCtx.newPage()
    await startClassicSoloGame(iphoneLandPage, true)
    await iphoneLandPage.screenshot({
      path: path.join(SCREENSHOTS_DIR, 'game-iphone-landscape.png'),
    })
    await iphoneLandCtx.close()
  })

  test('plays a full classic solo match on mobile until /results appears', async ({
    browser,
  }) => {
    test.setTimeout(360_000)
    const context = await browser.newContext({ ...devices['Pixel 7'] })
    const page = await context.newPage()

    const errors: string[] = []
    page.on('pageerror', (err) => errors.push(`pageerror: ${err.message}`))
    page.on('console', (msg) => {
      if (msg.type() === 'error') errors.push(`console.error: ${msg.text()}`)
    })

    await startClassicSoloGame(page, false, 200)

    // First-visit tutorial appears on /game; dismiss it
    const tutorialSkip = page.getByRole('button', { name: '跳過教學' })
    if (await tutorialSkip.isVisible().catch(() => false)) {
      await tutorialSkip.click()
    }

    const deadline = Date.now() + 340_000
    while (!page.url().includes('/results') && Date.now() < deadline) {
      const roundBtn = page.getByRole('button', { name: '進入下一輪' })
      if (await roundBtn.isVisible().catch(() => false)) {
        await roundBtn.click({ timeout: 2_000 }).catch(() => undefined)
        await page.waitForTimeout(80)
        continue
      }

      const redChoice = page.locator('[data-color="red"]')
      if (await redChoice.isVisible().catch(() => false)) {
        await redChoice.click({ timeout: 1_000 }).catch(() => undefined)
        continue
      }

      const playDrawn = page.getByRole('button', { name: '立即打出' })
      if (await playDrawn.isVisible().catch(() => false)) {
        await playDrawn.click({ timeout: 1_000 }).catch(() => undefined)
        continue
      }

      const acceptChallenge = page.getByRole('button', { name: '接受 (+4)' })
      if (await acceptChallenge.isVisible().catch(() => false)) {
        await acceptChallenge.click({ timeout: 1_000 }).catch(() => undefined)
        continue
      }

      const targetBtn = page
        .locator('[aria-labelledby="target-picker-title"] button')
        .first()
      if (await targetBtn.isVisible().catch(() => false)) {
        await targetBtn.click({ timeout: 1_000 }).catch(() => undefined)
        continue
      }

      const unoBtn = page.getByTestId('uno-button')
      if (await unoBtn.isEnabled().catch(() => false)) {
        await unoBtn.click({ timeout: 1_000 }).catch(() => undefined)
      }

      const playableBtn = page
        .locator('[data-testid="player-hand"] [data-playable="true"] button')
        .first()
      if (await playableBtn.isVisible().catch(() => false)) {
        await playableBtn.scrollIntoViewIfNeeded({ timeout: 500 }).catch(() => undefined)
        await playableBtn
          .click({ position: { x: 8, y: 24 }, timeout: 1_000 })
          .catch(() => undefined)
      } else {
        const drawPile = page.getByTestId('draw-pile')
        if (await drawPile.isEnabled().catch(() => false)) {
          await drawPile.click({ timeout: 1_000 }).catch(() => undefined)
        }
      }

      if (page.url().includes('/results')) break
      await page.waitForTimeout(15)
    }

    expect(errors).toEqual([])
    await expect(page).toHaveURL(/\/results$/, { timeout: 10_000 })
    await expect(page.getByText(/冠軍：/)).toBeVisible()
    await expect(page.getByRole('button', { name: '再來一局' })).toBeVisible()
    const replayLink = page.getByRole('link', { name: '看重播' })
    await expect(replayLink).toBeVisible()

    const resultsAxe = await new AxeBuilder({ page }).analyze()
    expect(
      resultsAxe.violations.filter(
        (v) => v.impact === 'serious' || v.impact === 'critical',
      ),
    ).toEqual([])

    await replayLink.click()
    await expect(page).toHaveURL(/\/replay\/local$/)
    await expect(page.getByTestId('replay-seat-hand-1')).toBeVisible({
      timeout: 15_000,
    })

    await page.getByTestId('replay-next').click()
    await expect(page.getByTestId('replay-step-summary')).toBeVisible()

    const scrubber = page.getByTestId('replay-scrubber')
    const maxSteps = await scrubber.getAttribute('max')
    if (maxSteps && Number(maxSteps) >= 2) {
      await scrubber.fill('2')
    }

    const godToggle = page.getByTestId('replay-god-view-toggle')
    await godToggle.click()
    await expect(godToggle).toHaveAttribute('aria-pressed', 'true')

    const replayAxe = await new AxeBuilder({ page }).analyze()
    expect(
      replayAxe.violations.filter(
        (v) => v.impact === 'serious' || v.impact === 'critical',
      ),
    ).toEqual([])
    expect(errors).toEqual([])

    await context.close()
  })
})
