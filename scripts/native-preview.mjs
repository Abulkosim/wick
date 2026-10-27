// Renders the desktop popover (fake bridge, fitted height) over a light or dark backdrop.
// Usage: node --experimental-strip-types scripts/native-preview.mjs <out-dir> <scenario>:<theme>:<light|dark|none>[:settings] ...
import { webkit } from '@playwright/test'
import { createDemoState } from '../src/lib/demo.ts'
const [,, out, ...specs] = process.argv
const browser = await webkit.launch()
for (const spec of specs) {
  const [scenario, theme, backdrop, page_] = spec.split(':')
  const state = createDemoState(scenario); state.settings.theme = theme
  const page = await browser.newPage({ viewport: { width: 372, height: 300 }, deviceScaleFactor: 2, colorScheme: theme === 'dark' ? 'dark' : 'light' })
  await page.addInitScript(({ state, backdrop }) => {
    window.__FIT__ = []
    if (backdrop !== 'none') document.addEventListener('DOMContentLoaded', () => { document.documentElement.style.background = backdrop === 'dark' ? '#1e1e1e' : '#ececec' })
    window.__TAURI__ = { core: { async invoke(c, a) {
      if (c === 'get_state') return state
      if (c === 'get_popover_placement') return { revision: 0, arrowX: null }
      if (c === 'get_update_status') return { revision: 0, state: { kind: 'idle' } }
      if (c === 'running_apps') return []
      if (c === 'fit_popover') window.__FIT__.push(a.height)
    } }, event: { async listen() { return () => {} } } }
  }, { state, backdrop })
  await page.goto('http://127.0.0.1:1420/')
  await page.waitForTimeout(400)
  if (page_ === 'settings') { await page.getByRole('button', { name: 'Open settings' }).click(); await page.waitForTimeout(300) }
  const h = page_ === 'settings' ? 720 : await page.evaluate(() => window.__FIT__.at(-1))
  await page.setViewportSize({ width: 372, height: h })
  await page.waitForTimeout(200)
  await page.screenshot({ path: `${out}/n-${scenario}-${theme}-${backdrop}${page_ ? '-' + page_ : ''}.png`, omitBackground: backdrop === 'none' })
  await page.close()
}
await browser.close()
