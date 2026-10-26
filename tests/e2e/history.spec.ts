import { expect, test } from '@playwright/test'

test('the history card shows today and the last seven days', async ({ page }) => {
  await page.goto('/?demo=1&state=history')
  const card = page.getByRole('region', { name: 'Awake today' })
  await expect(card).toContainText('3h 12m')
  await expect(card).toContainText('16h 57m in the last 7 days')
  await expect(card.getByRole('list', { name: 'Last 7 days' }).getByRole('listitem')).toHaveCount(7)
})

test('a first run shows no history card', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await expect(page.getByRole('heading', { name: 'Keep awake for' })).toBeVisible()
  await expect(page.getByRole('region', { name: 'Awake today' })).toHaveCount(0)
})

test('history can be turned off and cleared in Settings', async ({ page }) => {
  await page.goto('/?demo=1&state=history&page=settings')
  await expect(page.getByText('16h 57m awake')).toBeVisible()
  await page.getByRole('button', { name: 'Clear history' }).click()
  await expect(page.getByText('History cleared.')).toBeVisible()
  await expect(page.getByText('Nothing recorded yet.')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Clear history' })).toBeDisabled()
  const toggle = page.getByRole('switch', { name: 'Keep a history' })
  await expect(toggle).toBeChecked()
  await toggle.click()
  await expect(toggle).not.toBeChecked()
})

test('turning history off hides the card', async ({ page }) => {
  await page.goto('/?demo=1&state=history&page=settings')
  await page.getByRole('switch', { name: 'Keep a history' }).click()
  await page.getByRole('button', { name: 'Back to overview' }).click()
  await expect(page.getByRole('heading', { name: 'Keep awake for' })).toBeVisible()
  await expect(page.getByRole('region', { name: 'Awake today' })).toHaveCount(0)
})

for (const theme of ['light', 'dark'] as const) {
  test(`screenshot history ${theme}`, async ({ page }, testInfo) => {
    await page.goto(`/?demo=1&state=history&theme=${theme}`)
    await expect(page.getByRole('region', { name: 'Awake today' })).toBeVisible()
    await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`history-${theme}.png`) })
    await page.goto(`/?demo=1&state=history&theme=${theme}&page=settings`)
    await page.getByRole('heading', { name: 'History' }).scrollIntoViewIfNeeded()
    await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`history-settings-${theme}.png`) })
  })
}
