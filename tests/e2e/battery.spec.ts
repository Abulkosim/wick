import { expect, test } from '@playwright/test'

test('a battery stop is explained and blocks a new start', async ({ page }) => {
  await page.goto('/?demo=1&state=battery')
  await expect(page.getByRole('status').getByText('Stopped: battery below 20%')).toBeVisible()
  await expect(page.getByText('On battery · 19%')).toBeVisible()
  await page.getByRole('button', { name: 'Start', exact: true }).click()
  await expect(page.getByRole('alert')).toContainText('Not started: battery below 20%. Plug in, or change it in Settings → Battery.')
  await expect(page.getByRole('status').getByRole('heading')).toHaveText('Off')
})

test('a lower threshold lets it start', async ({ page }) => {
  await page.goto('/?demo=1&state=battery&page=settings')
  await page.getByRole('button', { name: '10%' }).click()
  await expect(page.getByRole('button', { name: '10%' })).toHaveAttribute('aria-pressed', 'true')
  await page.getByRole('button', { name: 'Back' }).click()
  await page.getByRole('button', { name: '1 hour' }).click()
  await expect(page.getByRole('status').getByRole('heading')).toHaveText('Awake · 1h 00m left')
})

test('thresholds are off with the guard', async ({ page }) => {
  await page.goto('/?demo=1&state=off&page=settings')
  await page.getByRole('switch', { name: 'Stop on low battery' }).click()
  await expect(page.getByRole('button', { name: '20%' })).toBeDisabled()
})

test('desktops have no battery section or note', async ({ page }) => {
  await page.goto('/?demo=1&state=desktop&page=settings')
  await expect(page.getByRole('heading', { name: 'Battery' })).toHaveCount(0)
  await page.getByRole('button', { name: 'Back' }).click()
  await expect(page.getByText(/On battery/)).toHaveCount(0)
})

for (const theme of ['light', 'dark'] as const) {
  test(`screenshot battery stop ${theme}`, async ({ page }, testInfo) => {
    await page.goto(`/?demo=1&state=battery&theme=${theme}`)
    await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`battery-${theme}.png`) })
  })
}
