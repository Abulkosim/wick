import { expect, test } from '@playwright/test'

test('Linux says computer, offers commands and keeps lock and display options', async ({ page }) => {
  await page.goto('/?demo=1&state=off&platform=linux')
  await expect(page.locator('html')).toHaveAttribute('data-platform', 'linux')
  await expect(page.locator('.popover-arrow')).toBeHidden()
  await expect(page.locator('.hero-copy')).toContainText('Your computer sleeps as usual.')
  await expect(page.getByText('A program is running, like ffmpeg or claude.')).toBeVisible()
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await expect(page.getByRole('radio', { name: 'App' })).toHaveCount(0)
  await expect(page.getByLabel('Command name')).toBeFocused()
  await page.getByRole('radio', { name: 'Condition' }).click()
  await expect(page.getByRole('radio', { name: 'External display connected' })).toBeVisible()
  await page.keyboard.press('Escape')
  await page.getByRole('button', { name: 'Open settings' }).click()
  await expect(page.getByText('Show time left in the menu bar')).toHaveCount(0)
  await expect(page.getByText('Pause while the screen is locked')).toBeVisible()
  await expect(page.getByText('Lets the computer sleep until you unlock it.')).toBeVisible()
  await expect(page.getByText('For example “ffmpeg quit. Your computer can sleep again.”')).toBeVisible()
  await expect(page.getByText('Starts or stops anywhere.')).toBeVisible()
})

test('Linux warns that the lid will suspend', async ({ page }) => {
  await page.goto('/?demo=1&state=indefinite&platform=linux')
  await expect(page.locator('.footer-text')).toHaveText('Closing the lid will suspend')
})

test('Linux shortcuts read Ctrl and Super', async ({ page }) => {
  await page.goto('/?demo=1&state=off&platform=linux&page=settings')
  const field = page.getByRole('button', { name: 'Shortcut', exact: true })
  await field.click()
  await page.keyboard.press('Shift+KeyA')
  await expect(page.getByRole('alert')).toHaveText('Include Ctrl, Alt or Super.')
  await page.keyboard.press('Control+Alt+KeyW')
  await expect(field).toHaveText('Ctrl+Alt+W')
})

test('Wayland explains how to add a shortcut instead', async ({ page }) => {
  await page.goto('/?demo=1&state=off&platform=linux&wayland=1&page=settings')
  await expect(page.getByText('Wayland doesn’t allow global shortcuts.')).toBeVisible()
  await expect(page.getByText('wick toggle')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Shortcut', exact: true })).toHaveCount(0)
})

test('without a tray Wick is a plain window', async ({ page }) => {
  await page.goto('/?demo=1&state=off&platform=linux&framed=1')
  await expect(page.locator('html')).toHaveAttribute('data-framed', 'true')
})

for (const theme of ['light', 'dark'] as const) {
  test(`screenshot Linux ${theme}`, async ({ page }, testInfo) => {
    await page.goto(`/?demo=1&state=activity&platform=linux&theme=${theme}`)
    await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`linux-${theme}.png`) })
  })
}
