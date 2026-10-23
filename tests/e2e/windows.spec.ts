import { expect, test } from '@playwright/test'

test('Windows hides the Mac-only parts', async ({ page }) => {
  await page.goto('/?demo=1&state=off&platform=windows')
  await expect(page.locator('html')).toHaveAttribute('data-platform', 'windows')
  await expect(page.locator('.popover-arrow')).toBeHidden()
  await expect(page.getByText('A program is running, like ffmpeg or claude.')).toBeVisible()
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await expect(page.getByRole('radio', { name: 'App' })).toHaveCount(0)
  await expect(page.getByLabel('Command name')).toBeFocused()
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: 'Open settings' }).click()
  await expect(page.getByText('Show time left in the menu bar')).toHaveCount(0)
  await expect(page.getByText('Pause while the screen is locked')).toHaveCount(0)
  await expect(page.getByText('Launch at login')).toBeVisible()
})

for (const theme of ['light', 'dark'] as const) {
  test(`screenshot Windows ${theme}`, async ({ page }, testInfo) => {
    await page.goto(`/?demo=1&state=triggers&platform=windows&theme=${theme}`)
    await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`windows-${theme}.png`) })
  })
}
