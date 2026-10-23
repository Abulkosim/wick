import { expect, test } from '@playwright/test'

test('lists other apps keeping the Mac awake while Wick is on', async ({ page }) => {
  await page.goto('/?demo=1&state=others')
  const card = page.getByRole('region', { name: 'Also keeping your Mac awake' })
  await expect(card.locator('.trigger-row')).toHaveText([/caffeinate\s*Started by claude/, /Google Chrome\s*Playing audio/, /zoom\.us\s*Zoom meeting · Display on/])
})

test('says who holds the Mac when Wick is off', async ({ page }) => {
  await page.goto('/?demo=1&state=others-off')
  await expect(page.getByRole('status')).toContainText('claude is keeping your Mac awake.')
  await expect(page.getByRole('region', { name: 'Keeping your Mac awake' })).toBeVisible()
})

test('no card when nothing else holds the Mac', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await expect(page.getByText(/keeping your Mac awake/i)).toHaveCount(0)
})

for (const theme of ['light', 'dark'] as const) {
  test(`screenshot others ${theme}`, async ({ page }, testInfo) => {
    await page.goto(`/?demo=1&state=others&theme=${theme}`)
    await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`others-${theme}.png`) })
  })
}

test('long reasons stay on one line inside the card', async ({ page }) => {
  await page.goto('/?demo=1&state=others')
  for (const row of await page.locator('.other-row .trigger-status').all()) {
    const box = await row.evaluate(element => ({ scroll: element.scrollHeight, client: element.clientHeight, right: element.getBoundingClientRect().right, card: element.closest('.card')!.getBoundingClientRect().right }))
    expect(box.scroll).toBeLessThanOrEqual(box.client + 1)
    expect(box.right).toBeLessThanOrEqual(box.card)
  }
})
