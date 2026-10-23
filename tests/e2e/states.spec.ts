import { expect, test } from '@playwright/test'
import { demoScenarios } from '../../src/lib/demo.ts'

for (const theme of ['light', 'dark'] as const) {
  for (const state of demoScenarios) {
    test(`state ${state} renders cleanly in ${theme}`, async ({ page }, testInfo) => {
      const errors: string[] = []
      page.on('console', message => { if (message.type() === 'error') errors.push(message.text()) })
      page.on('pageerror', error => errors.push(error.message))
      await page.goto(`/?demo=1&state=${state}&theme=${theme}`)
      await expect(page.locator('.hero-title')).not.toBeEmpty()
      await expect(page.locator('html')).toHaveAttribute('data-theme', theme)
      const overflow = await page.locator('.popover').evaluate(element => element.scrollWidth > element.clientWidth)
      expect(overflow).toBe(false)
      await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`${state}-${theme}.png`) })
      expect(errors).toEqual([])
    })
  }
}

test('narrow phone-width preview has no horizontal scroll', async ({ page }) => {
  await page.setViewportSize({ width: 340, height: 700 })
  await page.goto('/?demo=1&state=held')
  const scroll = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth)
  expect(scroll).toBe(false)
})
