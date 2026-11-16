import { mkdirSync } from 'node:fs'
import path from 'node:path'
import { expect, test, type Browser, type Page } from '@playwright/test'

const captures = path.resolve(process.cwd(), 'mobile-screenshots')

async function enterSolo(page: Page) {
  await page.addInitScript(() => {
    localStorage.setItem(
      'uno-settings',
      JSON.stringify({
        state: { animSpeed: 'off', volume: 0, vibration: false },
        version: 0,
      }),
    )
    localStorage.setItem('uno-tutorial-seen', '1')
  })
  await page.goto('/')
  await page.getByRole('link', { name: '快速 vs AI' }).click()
  await page.getByTestId('start-solo-game').click()
  await expect(page.getByTestId('player-hand')).toBeVisible({ timeout: 20_000 })
  await expect(page.getByTestId('draw-pile')).toBeVisible()
  await expect(page.getByTestId('uno-button')).toBeVisible()
}

async function checkPhoneGame(
  browser: Browser,
  label: string,
  width: number,
  height: number,
) {
  const context = await browser.newContext({
    viewport: { width, height },
    isMobile: true,
    hasTouch: true,
    deviceScaleFactor: 1,
  })
  const page = await context.newPage()
  const browserErrors: string[] = []
  page.on('pageerror', (error) => browserErrors.push(error.message))

  try {
    await enterSolo(page)
    mkdirSync(captures, { recursive: true })
    await page.screenshot({ path: path.join(captures, `${label}.png`) })

    // tsconfig for Playwright is Node-only (no DOM lib); browser code is a
    // string evaluated in Chromium, with the result typed on the Node side.
    const metrics = await page.evaluate(`(() => {
      const bounds = (selector) => {
        const element = document.querySelector(selector)
        if (!element) return null
        const rect = element.getBoundingClientRect()
        return { x: rect.x, y: rect.y, width: rect.width, height: rect.height, bottom: rect.bottom }
      }

      const cards = [...document.querySelectorAll('[data-testid="player-hand"] [data-card-id]')]
        .map((element) => element.getBoundingClientRect().top)

      return {
        width: innerWidth,
        height: innerHeight,
        scrollWidth: document.documentElement.scrollWidth,
        scrollHeight: document.documentElement.scrollHeight,
        arena: bounds('[data-testid="game-arena"]'),
        dock: bounds('[data-testid="player-dock"]'),
        hand: bounds('[data-testid="player-hand"]'),
        uno: bounds('[data-testid="uno-button"]'),
        draw: bounds('[data-testid="draw-pile"]'),
        cardTops: cards,
      }
    })()`) as {
      width: number
      height: number
      scrollWidth: number
      scrollHeight: number
      arena: { height: number } | null
      dock: { bottom: number } | null
      hand: { height: number } | null
      uno: { x: number; width: number; bottom: number } | null
      draw: { width: number; height: number } | null
      cardTops: number[]
    }

    expect(metrics.scrollWidth, `${label}: document has horizontal overflow`).toBeLessThanOrEqual(width + 1)
    expect(metrics.scrollHeight, `${label}: document has vertical overflow`).toBeLessThanOrEqual(height + 1)
    expect(metrics.dock, `${label}: missing dock`).not.toBeNull()
    expect(metrics.hand, `${label}: missing hand`).not.toBeNull()
    expect(metrics.arena, `${label}: missing arena`).not.toBeNull()
    expect(metrics.uno, `${label}: missing UNO button`).not.toBeNull()
    expect(metrics.draw, `${label}: missing draw pile`).not.toBeNull()
    expect(metrics.dock!.bottom, `${label}: hand dock off screen`).toBeLessThanOrEqual(height + 1)
    expect(metrics.arena!.height, `${label}: table squeezed out`).toBeGreaterThan(80)
    expect(metrics.hand!.height, `${label}: hand squeezed out`).toBeGreaterThan(70)
    expect(metrics.uno!.x + metrics.uno!.width, `${label}: UNO clipped`).toBeLessThanOrEqual(width + 1)
    expect(metrics.uno!.bottom, `${label}: UNO below viewport`).toBeLessThanOrEqual(height + 1)
    expect(metrics.draw!.width, `${label}: draw tap area too narrow`).toBeGreaterThanOrEqual(40)
    expect(metrics.draw!.height, `${label}: draw tap area too short`).toBeGreaterThanOrEqual(44)

    if (metrics.cardTops.length > 1) {
      const delta = Math.max(...metrics.cardTops) - Math.min(...metrics.cardTops)
      expect(delta, `${label}: uneven resting card heights`).toBeLessThanOrEqual(3)
    }

    // The playable cue must sit on the exposed edge of the card and remain
    // visible even when cards overlap. Non-playable cards get no false cue.
    const blocked = page.locator('[data-testid="player-hand"] [data-playable="false"]').first()
    if (await blocked.count()) {
      await expect(blocked.getByTestId('playable-indicator')).toHaveCount(0)
    }

    // An overlapping card is actionable by its exposed left strip. The new
    // cue is non-interactive, so it cannot steal touch events from the card.
    const playable = page.locator('[data-testid="player-hand"] [data-playable="true"]').first()
    if (await playable.count()) {
      const cue = playable.getByTestId('playable-indicator')
      await expect(cue).toBeVisible()
      await expect(cue).toHaveText(/可出|PLAY/)
      const cueBounds = await cue.boundingBox()
      expect(cueBounds, `${label}: playable cue has no layout`).not.toBeNull()
      expect(cueBounds!.width, `${label}: cue too small to recognize`).toBeGreaterThanOrEqual(25)

      await playable.locator('button').tap({
        position: { x: 12, y: 28 },
        timeout: 5_000,
      })
      await expect(playable).toHaveAttribute('data-selected', 'true')
    } else {
      const draw = page.getByTestId('draw-pile')
      await expect(draw).toBeEnabled({ timeout: 10_000 })
      await draw.tap({ timeout: 5_000 })
    }

    expect(browserErrors, `${label}: browser errors`).toEqual([])
  } finally {
    await context.close()
  }
}

test.describe('Real phone viewport gameplay checks', () => {
  test('iPhone 13 portrait: cards stay level and controls remain reachable', async ({ browser }) => {
    await checkPhoneGame(browser, 'iphone-13-portrait', 390, 844)
  })

  test('iPhone SE portrait: compact game fits on screen', async ({ browser }) => {
    await checkPhoneGame(browser, 'iphone-se-portrait', 375, 667)
  })

  test('Small Android portrait: no clipped hand or controls', async ({ browser }) => {
    await checkPhoneGame(browser, 'android-small-portrait', 360, 740)
  })

  test('Phone landscape: small cards and a usable draw pile', async ({ browser }) => {
    await checkPhoneGame(browser, 'phone-landscape', 844, 390)
  })
})
