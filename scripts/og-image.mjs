// Renders the site's first screen as the 1280×640 social image: node scripts/og-image.mjs site/index.html site/og.png
import { chromium } from '@playwright/test'
import { pathToFileURL } from 'node:url'
const [site, out, theme] = process.argv.slice(2)
const browser = await chromium.launch()
const page = await browser.newPage({ viewport: { width: 1280, height: 640 }, deviceScaleFactor: 1, colorScheme: theme ?? 'light' })
await page.goto(pathToFileURL(site).href)
await page.waitForLoadState('networkidle')
await page.evaluate(() => document.fonts.ready)
await page.screenshot({ path: out })
await browser.close()
