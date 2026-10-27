import { expect, test, type Page } from '@playwright/test'
import { createDemoState } from '../../src/lib/demo.ts'

declare global { interface Window { __FIT__?: number[] } }

async function desktop(page: Page, scenario: string): Promise<void> {
  const state = createDemoState(scenario)
  await page.addInitScript(({ state }) => {
    window.__FIT__ = []
    window.__TAURI__ = {
      core: { async invoke(command, args) {
        if (command === 'get_state') return state
        if (command === 'get_popover_placement') return { revision: 0, arrowX: null }
        if (command === 'get_update_status') return { revision: 0, state: { kind: 'idle' } }
        if (command === 'fit_popover') window.__FIT__!.push(Number(args?.height))
        return undefined
      } },
      event: { async listen() { return () => {} } },
    }
  }, { state })
}

for (const scenario of ['off', 'held', 'others']) {
  test(`the native window gets the full content height (${scenario})`, async ({ page }) => {
    await page.setViewportSize({ width: 360, height: 300 })
    await desktop(page, scenario)
    await page.goto('/')
    await expect.poll(() => page.evaluate(() => window.__FIT__?.at(-1) ?? 0)).toBeGreaterThan(400)
    const height = await page.evaluate(() => window.__FIT__!.at(-1)!)
    expect(height).toBeLessThanOrEqual(720)
    await page.setViewportSize({ width: 360, height })
    const body = page.locator('.popover-body')
    if (height < 720) await expect.poll(() => body.evaluate(element => element.scrollHeight - element.clientHeight)).toBeLessThanOrEqual(1)
    else await expect.poll(() => body.evaluate(element => element.scrollHeight > element.clientHeight)).toBe(true)
  })
}

test('adding a trigger asks for a taller window', async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 640 })
  await desktop(page, 'off')
  await page.goto('/')
  await expect.poll(() => page.evaluate(() => window.__FIT__?.length ?? 0)).toBeGreaterThan(0)
  const before = await page.evaluate(() => window.__FIT__!.at(-1)!)
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await expect.poll(() => page.evaluate(() => window.__FIT__!.at(-1)!)).toBeGreaterThan(before)
})
