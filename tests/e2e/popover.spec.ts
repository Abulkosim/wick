import { expect, test } from '@playwright/test'

test('renders the popover preview', async ({ page }) => {
  await page.goto('/?demo=1')
  await expect(page.getByText('Interface preview')).toBeVisible()
})
