import { devices, expect, test, type Page } from '@playwright/test'

// 連線對局 E2E（PLAN T7.4 驗收）：需要 uno-server 在 8787 運行
//（vite dev 會 proxy /api 與 /ws 到 8787）。未運行時整組 skip。
const SERVER_INFO = 'http://127.0.0.1:8787/api/server-info'

async function serverOnline(): Promise<boolean> {
  try {
    const res = await fetch(SERVER_INFO)
    return res.ok
  } catch {
    return false
  }
}

async function prep(page: Page) {
  const settingsJson = JSON.stringify({
    state: { animSpeed: 'off', volume: 0, vibration: false },
    version: 0,
  })
  await page.addInitScript(
    `globalThis.localStorage.setItem('uno-settings', ${JSON.stringify(settingsJson)});` +
      `globalThis.localStorage.setItem('uno-tutorial-seen', '1');`,
  )
}

async function guestEnter(page: Page, nickname: string) {
  const nick = page.getByPlaceholder('暱稱')
  await nick.fill(nickname)
  await page.getByRole('button', { name: '進入', exact: true }).click()
}

async function driveOnce(page: Page): Promise<boolean> {
  const redChoice = page.locator('[data-color="red"]')
  if (await redChoice.isVisible().catch(() => false)) {
    await redChoice.click({ timeout: 1_000 }).catch(() => undefined)
    return true
  }
  const playDrawn = page.getByRole('button', { name: '立即打出' })
  if (await playDrawn.isVisible().catch(() => false)) {
    await playDrawn.click({ timeout: 1_000 }).catch(() => undefined)
    return true
  }
  const keepDrawn = page.getByRole('button', { name: '保留' })
  if (await keepDrawn.isVisible().catch(() => false)) {
    await keepDrawn.click({ timeout: 1_000 }).catch(() => undefined)
    return true
  }
  const acceptChallenge = page.getByRole('button', { name: '接受 (+4)' })
  if (await acceptChallenge.isVisible().catch(() => false)) {
    await acceptChallenge.click({ timeout: 1_000 }).catch(() => undefined)
    return true
  }
  const targetBtn = page.locator('[aria-labelledby="target-picker-title"] button').first()
  if (await targetBtn.isVisible().catch(() => false)) {
    await targetBtn.click({ timeout: 1_000 }).catch(() => undefined)
    return true
  }
  const unoBtn = page.getByTestId('uno-button')
  if (await unoBtn.isEnabled().catch(() => false)) {
    await unoBtn.click({ timeout: 1_000 }).catch(() => undefined)
    return true
  }
  const playableBtn = page
    .locator('[data-testid="player-hand"] [data-playable="true"] button')
    .first()
  if (await playableBtn.isVisible().catch(() => false)) {
    await playableBtn.click({ timeout: 1_000 }).catch(() => undefined)
    return true
  }
  const drawPile = page.getByTestId('draw-pile')
  if (await drawPile.isEnabled().catch(() => false)) {
    await drawPile.click({ timeout: 1_000 }).catch(() => undefined)
    return true
  }
  return false
}

async function handCount(page: Page): Promise<number> {
  return page.locator('[data-testid="player-hand"] button').count()
}

async function setupStartedGame(browser: import('@playwright/test').Browser) {
  const ctxA = await browser.newContext({ viewport: { width: 1280, height: 800 } })
  const ctxB = await browser.newContext({ ...devices['iPhone 13'] })
  const a = await ctxA.newPage()
  const b = await ctxB.newPage()
  await prep(a)
  await prep(b)
  await a.goto('/create')
  await guestEnter(a, '房主A')
  const createBtn = a.getByRole('button', { name: '建立房間' })
  await expect(createBtn).toBeEnabled({ timeout: 15_000 })
  await createBtn.click()
  await expect(a).toHaveURL(/\/r\/[A-Z0-9]{6}$/, { timeout: 15_000 })
  const code = a.url().split('/r/')[1]!
  await b.goto(`/r/${code}`)
  await guestEnter(b, '玩家B')
  await expect(b.getByText('座位 1')).toBeVisible({ timeout: 15_000 })
  await a.getByRole('button', { name: '入座' }).first().click()
  await b.getByRole('button', { name: '入座' }).first().click()
  await expect(a.getByText('玩家B')).toBeVisible({ timeout: 15_000 })
  for (let i = 0; i < 2; i++) {
    await a.getByRole('button', { name: '加電腦' }).first().click()
    await a.waitForTimeout(400)
  }
  await a.getByRole('button', { name: /準備 \/ 取消/ }).click()
  await b.getByRole('button', { name: /準備 \/ 取消/ }).click()
  await a.waitForTimeout(400)
  await a.getByRole('button', { name: '開始遊戲' }).click()
  const enterA = a.getByRole('button', { name: '進入遊戲桌' })
  const enterB = b.getByRole('button', { name: '進入遊戲桌' })
  await expect(enterA).toBeVisible({ timeout: 20_000 })
  await expect(enterB).toBeVisible({ timeout: 20_000 })
  await enterA.click()
  await enterB.click()
  await expect(a.getByTestId('player-hand')).toBeVisible({ timeout: 20_000 })
  await expect(b.getByTestId('player-hand')).toBeVisible({ timeout: 20_000 })
  return { ctxA, ctxB, a, b, code }
}

test.describe('Multiplayer room E2E', () => {
  test('two browsers create/join, start, and both see the table; guest B can act', async ({
    browser,
  }) => {
    test.skip(!(await serverOnline()), 'uno-server not running on 8787')
    test.setTimeout(240_000)

    const ctxA = await browser.newContext({ viewport: { width: 1280, height: 800 } })
    const ctxB = await browser.newContext({ ...devices['iPhone 13'] })
    const a = await ctxA.newPage()
    const b = await ctxB.newPage()
    await prep(a)
    await prep(b)

    // A 建房
    await a.goto('/create')
    await guestEnter(a, '房主A')
    const createBtn = a.getByRole('button', { name: '建立房間' })
    await expect(createBtn).toBeEnabled({ timeout: 15_000 })
    await createBtn.click()
    await expect(a).toHaveURL(/\/r\/[A-Z0-9]{6}$/, { timeout: 15_000 })
    const code = a.url().split('/r/')[1]!
    expect(code).toHaveLength(6)

    // B 用 /r/:code 直接開啟，訪客登入後自動加入
    await b.goto(`/r/${code}`)
    await guestEnter(b, '玩家B')
    await expect(b.getByText('座位 1')).toBeVisible({ timeout: 15_000 })

    // 入座
    await a.getByRole('button', { name: '入座' }).first().click()
    await b.getByRole('button', { name: '入座' }).first().click()
    await expect(a.getByText('玩家B')).toBeVisible({ timeout: 15_000 })

    // A 補 2 個 bot
    for (let i = 0; i < 2; i++) {
      await a.getByRole('button', { name: '加電腦' }).first().click()
      await a.waitForTimeout(400)
    }

    // 雙方準備，A 開始
    await a.getByRole('button', { name: /準備 \/ 取消/ }).click()
    await b.getByRole('button', { name: /準備 \/ 取消/ }).click()
    await a.waitForTimeout(400)
    await a.getByRole('button', { name: '開始遊戲' }).click()

    const enterA = a.getByRole('button', { name: '進入遊戲桌' })
    const enterB = b.getByRole('button', { name: '進入遊戲桌' })
    await expect(enterA).toBeVisible({ timeout: 20_000 })
    await expect(enterB).toBeVisible({ timeout: 20_000 })
    await enterA.click()
    await enterB.click()
    await expect(a).toHaveURL(/\/game$/)
    await expect(b).toHaveURL(/\/game$/)
    await expect(a.getByTestId('player-hand')).toBeVisible({ timeout: 20_000 })
    await expect(b.getByTestId('player-hand')).toBeVisible({ timeout: 20_000 })

    // 輪流驅動，直到 B 成功做過一次動作（手牌數改變）或回合推進至多 90 秒
    const initialB = await handCount(b)
    let bActed = false
    const deadline = Date.now() + 90_000
    while (Date.now() < deadline && !bActed) {
      await driveOnce(a)
      const before = await handCount(b)
      const acted = await driveOnce(b)
      if (acted) {
        await b.waitForTimeout(600)
        const after = await handCount(b)
        if (after !== before || before !== initialB) bActed = true
      }
      await a.waitForTimeout(120)
    }
    expect(bActed).toBe(true)

    await ctxA.close()
    await ctxB.close()
  })

  test('guest B goes offline 3s, reconnects, and can keep playing', async ({ browser }) => {
    test.skip(!(await serverOnline()), 'uno-server not running on 8787')
    test.setTimeout(240_000)
    const { ctxA, ctxB, a, b } = await setupStartedGame(browser)

    // B 離線 3 秒再恢復，應自動重連並回到遊戲
    await ctxB.setOffline(true)
    await b.waitForTimeout(3000)
    await ctxB.setOffline(false)
    await expect(b.getByTestId('player-hand')).toBeVisible({ timeout: 30_000 })

    // 重連後 B 仍能行動（手牌數改變）
    let acted = false
    const deadline = Date.now() + 90_000
    while (Date.now() < deadline && !acted) {
      await driveOnce(a)
      const before = await handCount(b)
      if (await driveOnce(b)) {
        await b.waitForTimeout(600)
        if ((await handCount(b)) !== before) acted = true
      }
      await a.waitForTimeout(120)
    }
    expect(acted).toBe(true)
    await ctxA.close()
    await ctxB.close()
  })

  test('in-game chat drawer sends a message visible to the other player', async ({
    browser,
  }) => {
    test.skip(!(await serverOnline()), 'uno-server not running on 8787')
    test.setTimeout(240_000)
    const { ctxA, ctxB, a, b } = await setupStartedGame(browser)
    await b.getByRole('button', { name: '聊天' }).click()
    await b.getByPlaceholder('輸入訊息…').fill('哈囉A')
    await b.getByRole('button', { name: '送出' }).click()
    await a.getByRole('button', { name: '聊天' }).click()
    await expect(a.getByText('哈囉A')).toBeVisible({ timeout: 15_000 })
    await ctxA.close()
    await ctxB.close()
  })
})
