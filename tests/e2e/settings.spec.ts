import { expect, test } from '@playwright/test'

test('settings open from the footer and close with Back or Escape', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await page.getByRole('button', { name: 'Open settings' }).click()
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible()
  await expect(page.getByText('Saves automatically')).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(page.getByRole('status').getByRole('heading')).toHaveText('Off')
  await page.keyboard.press('ControlOrMeta+,')
  await page.getByRole('button', { name: 'Back to overview' }).click()
  await expect(page.getByRole('button', { name: 'Start', exact: true })).toBeVisible()
})

test('Start uses the chosen start duration', async ({ page }) => {
  await page.goto('/?demo=1&state=off&page=settings')
  await page.getByRole('combobox', { name: 'Start keeps awake' }).click()
  await page.getByRole('option', { name: '1 hour' }).click()
  await expect(page.getByRole('combobox', { name: 'Start keeps awake' })).toHaveText(/1 hour/)
  await page.getByRole('button', { name: 'Back to overview' }).click()
  await page.getByRole('button', { name: 'Start', exact: true }).click()
  await expect(page.getByRole('status').getByRole('heading')).toHaveText('Awake · 1h 00m left')
})

test('theme follows the choice', async ({ page }) => {
  await page.goto('/?demo=1&state=off&page=settings&theme=light')
  await page.getByRole('button', { name: 'Dark' }).click()
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark')
  await expect(page.getByRole('button', { name: 'Dark' })).toHaveAttribute('aria-pressed', 'true')
})

test('damaged settings explain and reset with a backup', async ({ page }) => {
  await page.goto('/?demo=1&state=recovery')
  await expect(page.getByRole('alert')).toContainText('Settings could not be read.')
  await page.getByRole('button', { name: 'Review in Settings' }).click()
  await page.getByRole('button', { name: 'Back up and reset' }).click()
  await expect(page.getByText('Settings reset. The old file was kept as a backup.')).toBeVisible()
  await expect(page.getByRole('alert')).toHaveCount(0)
})

test('recovered fields are listed once', async ({ page }) => {
  await page.goto('/?demo=1&state=recovered&page=settings')
  await expect(page.getByText('Theme was reset to its default.')).toBeVisible()
})

for (const theme of ['light', 'dark'] as const) {
  for (const state of ['off', 'recovery']) {
    test(`screenshot settings ${state} ${theme}`, async ({ page }, testInfo) => {
      await page.goto(`/?demo=1&state=${state}&theme=${theme}&page=settings`)
      await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible()
      await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`settings-${state}-${theme}.png`) })
    })
  }
}

test('records, rejects and removes a shortcut', async ({ page }) => {
  await page.goto('/?demo=1&state=off&page=settings')
  const field = page.getByRole('button', { name: 'Shortcut', exact: true })
  await field.click()
  await expect(field).toHaveText('Type shortcut…')
  await page.keyboard.press('Shift+KeyA')
  await expect(page.getByRole('alert')).toHaveText('Include Command, Control or Option.')
  await page.keyboard.press('Alt+Meta+KeyW')
  await expect(field).toHaveText('⌥⌘W')
  await page.getByRole('button', { name: 'Remove shortcut' }).click()
  await expect(field).toHaveText('Record shortcut')
})

test('trigger notifications are off until turned on', async ({ page }) => {
  await page.goto('/?demo=1&state=off&page=settings')
  const toggle = page.getByRole('switch', { name: 'When a trigger ends' })
  await expect(toggle).not.toBeChecked()
  await toggle.click()
  await expect(toggle).toBeChecked()
})

test('launch at login toggles', async ({ page }) => {
  await page.goto('/?demo=1&state=off&page=settings')
  const toggle = page.getByRole('switch', { name: 'Launch at login' })
  await expect(toggle).not.toBeChecked()
  await toggle.click()
  await expect(toggle).toBeChecked()
})

test('session notifications can be turned off', async ({ page }) => {
  await page.goto('/?demo=1&state=off&page=settings')
  const toggle = page.getByRole('switch', { name: 'When a session ends' })
  await expect(toggle).toBeChecked()
  await toggle.click()
  await expect(toggle).not.toBeChecked()
})

test('time left in the menu bar is off until turned on', async ({ page }) => {
  await page.goto('/?demo=1&state=timed&page=settings')
  const toggle = page.getByRole('switch', { name: 'Show time left in the menu bar' })
  await expect(toggle).not.toBeChecked()
  await toggle.click()
  await expect(toggle).toBeChecked()
})
