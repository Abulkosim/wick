import { expect, test } from '@playwright/test'

test('off: one click starts indefinitely, one click stops', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await expect(page.getByRole('status').getByRole('heading')).toHaveText('Off')
  await expect(page.getByText('Your Mac sleeps as usual.')).toBeVisible()
  await page.getByRole('button', { name: 'Start', exact: true }).click()
  await expect(page.getByRole('status').getByRole('heading')).toHaveText('Awake')
  await expect(page.getByText('Until you stop it')).toBeVisible()
  await expect(page.getByRole('button', { name: 'Indefinitely' })).toHaveAttribute('aria-pressed', 'true')
  await page.getByRole('button', { name: 'Stop' }).click()
  await expect(page.getByRole('status').getByRole('heading')).toHaveText('Off')
})

test('a preset starts a timed session in one click', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await page.getByRole('button', { name: '1 hour' }).click()
  await expect(page.getByRole('status').getByRole('heading')).toHaveText('Awake · 1h 00m left')
  await expect(page.getByRole('button', { name: '1 hour' })).toHaveAttribute('aria-pressed', 'true')
})

test('timed demo counts down with the end time', async ({ page }) => {
  await page.goto('/?demo=1&state=timed')
  await expect(page.getByRole('status').getByRole('heading')).toHaveText(/^Awake · 4[12]m left$/)
  await expect(page.getByRole('status').getByText(/^Until \d/)).toBeVisible()
  await expect(page.getByRole('button', { name: 'Stop' })).toBeVisible()
})

test('until and custom take typed values', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await page.getByRole('button', { name: 'Until…' }).click()
  await page.getByLabel('Until').fill('23:59')
  await page.locator('#duration-editor').getByRole('button', { name: 'Start' }).click()
  await expect(page.getByRole('status').getByRole('heading')).toHaveText(/^Awake until /)
  await expect(page.getByRole('button', { name: 'Until…' })).toHaveAttribute('aria-pressed', 'true')
  await page.getByRole('button', { name: 'Custom…' }).click()
  await page.getByLabel('Hours').fill('0')
  await page.getByLabel('Minutes').fill('0')
  await expect(page.locator('#duration-editor').getByRole('button', { name: 'Start' })).toBeDisabled()
  await page.getByLabel('Minutes').fill('45')
  await page.getByLabel('Minutes').press('Enter')
  await expect(page.getByRole('status').getByRole('heading')).toHaveText('Awake · 45m left')
  await expect(page.getByRole('button', { name: 'Custom…' })).toHaveAttribute('aria-pressed', 'true')
})

test('display switch shows in the status', async ({ page }) => {
  await page.goto('/?demo=1&state=indefinite')
  await page.getByRole('switch', { name: 'Keep display on' }).click()
  await expect(page.getByRole('switch', { name: 'Keep display on' })).toBeChecked()
  await expect(page.getByText('Until you stop it · Display on')).toBeVisible()
})

test('error state explains what failed', async ({ page }) => {
  await page.goto('/?demo=1&state=error')
  await expect(page.getByRole('alert')).toContainText('Could not keep your Mac awake.')
})

test('keyboard reaches every control in order', async ({ page, browserName }) => {
  test.skip(browserName === 'webkit', 'WebKit on macOS tabs only to text fields by default')
  await page.goto('/?demo=1&state=off')
  await page.keyboard.press('Tab')
  await expect(page.getByRole('button', { name: 'Start', exact: true })).toBeFocused()
  await page.keyboard.press('Tab')
  await expect(page.getByRole('button', { name: '15 min' })).toBeFocused()
})

for (const theme of ['light', 'dark'] as const) {
  for (const state of ['off', 'timed', 'until', 'indefinite', 'error']) {
    test(`screenshot ${state} ${theme}`, async ({ page }, testInfo) => {
      await page.goto(`/?demo=1&state=${state}&theme=${theme}`)
      await expect(page.locator('.hero-title')).toBeVisible()
      await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`${state}-${theme}.png`) })
    })
  }
}
