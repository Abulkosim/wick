import { expect, test } from '@playwright/test'

test('empty card explains triggers', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await expect(page.getByText('An app or command is running, like Xcode or claude.')).toBeVisible()
})

test('adds a running app and a command', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await page.getByRole('combobox', { name: 'Running app' }).click()
  await page.getByRole('option', { name: 'Xcode' }).click()
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await expect(page.locator('.trigger-row')).toHaveText([/Xcode is running/])
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await page.getByRole('radio', { name: 'Command' }).click()
  await page.getByRole('button', { name: 'claude' }).click()
  await expect(page.getByLabel('Command name')).toHaveValue('claude')
  await page.getByLabel('Command name').press('Enter')
  await expect(page.locator('.trigger-row')).toHaveText([/Xcode is running/, /claude is running/])
})

test('rejects a path as a command', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await page.getByRole('radio', { name: 'Command' }).click()
  await page.getByLabel('Command name').fill('/usr/bin/ffmpeg')
  await expect(page.getByRole('alert')).toHaveText('Type a command name, like ffmpeg.')
  await expect(page.locator('.trigger-form').getByRole('button', { name: 'Add' })).toBeDisabled()
})

test('a running trigger holds the Mac awake; Stop pauses until it quits', async ({ page }) => {
  await page.goto('/?demo=1&state=held')
  await expect(page.getByRole('status').getByRole('heading')).toHaveText('Awake')
  await expect(page.getByRole('status')).toContainText('While Xcode is running')
  await expect(page.locator('.trigger-row').first()).toContainText('Running')
  await page.getByRole('button', { name: 'Stop' }).click()
  await expect(page.getByRole('status')).toContainText('Paused until Xcode quits')
  await page.getByRole('button', { name: 'Resume' }).click()
  await expect(page.getByRole('status')).toContainText('While Xcode is running')
})

test('turning a trigger off or removing it', async ({ page }) => {
  await page.goto('/?demo=1&state=held')
  await page.getByRole('switch', { name: 'Stay awake while Xcode is running' }).click()
  await expect(page.getByRole('status').getByRole('heading')).toHaveText('Off')
  await page.locator('.trigger-row').first().hover()
  await page.getByRole('button', { name: 'Remove Xcode' }).click()
  await expect(page.locator('.trigger-row')).toHaveCount(2)
})

for (const theme of ['light', 'dark'] as const) {
  for (const state of ['held', 'paused', 'triggers']) {
    test(`screenshot ${state} ${theme}`, async ({ page }, testInfo) => {
      await page.goto(`/?demo=1&state=${state}&theme=${theme}`)
      await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`${state}-${theme}.png`) })
    })
  }
}

test('adds power and display conditions once each', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await page.getByRole('radio', { name: 'Condition' }).click()
  await page.getByRole('radio', { name: 'On power' }).click()
  await page.locator('.trigger-form').getByRole('button', { name: 'Add' }).click()
  await expect(page.locator('.trigger-row')).toHaveText([/On power/])
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await page.getByRole('radio', { name: 'Condition' }).click()
  await expect(page.getByRole('radio', { name: 'On power' })).toHaveCount(0)
  await page.getByRole('radio', { name: 'External display connected' }).click()
  await page.locator('.trigger-form').getByRole('button', { name: 'Add' }).click()
  await expect(page.locator('.trigger-row')).toHaveText([/On power/, /External display connected/])
})

test('a condition holds the Mac awake and pauses until it ends', async ({ page }) => {
  await page.goto('/?demo=1&state=conditions')
  await expect(page.getByRole('status')).toContainText('While on power')
  await expect(page.locator('.trigger-row').first()).toContainText('Plugged in')
  await page.getByRole('button', { name: 'Stop' }).click()
  await expect(page.getByRole('status')).toContainText('Paused until you unplug')
})

test('adds a weekday schedule and an overnight one', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await page.getByRole('radio', { name: 'Condition' }).click()
  await page.getByRole('radio', { name: 'Schedule', exact: true }).click()
  await expect(page.getByRole('button', { name: 'Monday' })).toHaveAttribute('aria-pressed', 'true')
  await expect(page.getByRole('button', { name: 'Saturday' })).toHaveAttribute('aria-pressed', 'false')
  await page.locator('.trigger-form').getByRole('button', { name: 'Add' }).click()
  await expect(page.locator('.trigger-row')).toHaveText([/Weekdays 09:00–18:00/])
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await page.getByRole('radio', { name: 'Condition' }).click()
  await page.getByRole('radio', { name: 'Schedule', exact: true }).click()
  for (const day of ['Monday', 'Tuesday', 'Wednesday', 'Thursday']) await page.getByRole('button', { name: day }).click()
  await page.getByLabel('From').fill('22:00')
  await page.getByLabel('to', { exact: true }).fill('06:00')
  await expect(page.getByText('Runs overnight into the next day.')).toBeVisible()
  await page.locator('.trigger-form').getByRole('button', { name: 'Add' }).click()
  await expect(page.locator('.trigger-row')).toHaveText([/Weekdays 09:00–18:00/, /Fri 22:00–06:00/])
})

test('a schedule with no days cannot be added', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await page.getByRole('button', { name: 'Add', exact: true }).click()
  await page.getByRole('radio', { name: 'Condition' }).click()
  await page.getByRole('radio', { name: 'Schedule', exact: true }).click()
  for (const day of ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday']) await page.getByRole('button', { name: day }).click()
  await expect(page.getByRole('alert')).toHaveText('Choose at least one day.')
  await expect(page.locator('.trigger-form').getByRole('button', { name: 'Add' })).toBeDisabled()
})
