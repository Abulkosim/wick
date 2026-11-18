import { expect, test, type Page } from '@playwright/test'

const hero = (page: Page) => page.locator('.hero-copy')
const card = (page: Page) => page.getByRole('region', { name: 'Agents' })
const row = (page: Page, agent: string) => page.locator(`[data-agent="${agent}"]`)
const patch = (page: Page, change: object) => page.evaluate(change => (window as unknown as { wickDemo: { patch: (change: object) => void } }).wickDemo.patch(change), change)

test('the demo video script can hand a session over to an agent', async ({ page }) => {
  await page.goto('/?demo=1&state=timed')
  await expect(hero(page).getByRole('heading')).toHaveText(/Awake · \d+m left/)
  const since = await page.evaluate(() => Math.floor(Date.now() / 1000))
  await patch(page, { session: { mode: 'off' }, agents: [{ id: 'claude-d442c2f9', agent: 'claude', name: 'Claude Code', label: 'wick', state: 'working', since, paused: false }], heldBy: 'Claude Code works', heldByAgents: true })
  await expect(hero(page).getByRole('heading')).toHaveText('Awake')
  await expect(page.locator('.hero-detail')).toHaveText('While Claude Code works')
  await patch(page, { agents: [], heldBy: null, heldByAgents: false })
  await expect(hero(page).getByRole('heading')).toHaveText('Off')
  await expect(card(page)).toHaveCount(0)
})

test('a working agent keeps the Mac awake and its time ticks', async ({ page }) => {
  await page.clock.install()
  await page.goto('/?demo=1&state=agents-working')
  await expect(hero(page).getByRole('heading')).toHaveText('Awake')
  await expect(page.locator('.hero-detail')).toHaveText('While Claude Code works')
  await expect(page.getByRole('status')).toHaveText('Awake while Claude Code works in wick')
  await expect(card(page).locator('.trigger-row')).toHaveText([/Claude Code · wick\s*Working · 12m/])
  await page.clock.fastForward('01:00')
  await expect(card(page).locator('.trigger-status')).toHaveText('Working · 13m')
})

test('several agents are counted and listed working first', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-many')
  await expect(page.locator('.hero-detail')).toHaveText('While 2 agents work')
  await expect(card(page).locator('.trigger-row')).toHaveText([/Claude Code · wick\s*Working · 12m/, /Codex · margin\s*Working · 1h 23m/, /Claude Code · site\s*Waiting for you/])
  await expect(card(page).locator('.trigger-row.running')).toHaveCount(2)
})

test('the rest of a long list opens and closes from the same button', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-lots')
  const more = card(page).getByRole('button', { name: '3 more' })
  await expect(card(page).locator('.trigger-row')).toHaveCount(3)
  await more.focus()
  await page.keyboard.press('Enter')
  await expect(card(page).locator('.trigger-row')).toHaveCount(6)
  const fewer = card(page).getByRole('button', { name: 'Show fewer' })
  await expect(fewer).toBeFocused()
  await expect(fewer).toHaveAttribute('aria-expanded', 'true')
  await page.keyboard.press('Enter')
  await expect(card(page).locator('.trigger-row')).toHaveCount(3)
  await expect(more).toBeFocused()
})

test('an agent that starts after a Stop holds on its own', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-paused')
  await expect(page.locator('.hero-detail')).toHaveText('While Claude Code works')
  await expect(page.getByRole('status')).toHaveText('Awake while Claude Code works in site')
  await expect(page.locator('.agent-row').first()).toContainText('Working · 12m · paused')
  await expect(page.locator('.agent-row').nth(1)).not.toContainText('paused')
  await page.getByRole('button', { name: 'Stop', exact: true }).click()
  await expect(hero(page)).toContainText('Paused until the agents finish')
})

test('a waiting agent lets the Mac sleep', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-waiting')
  await expect(hero(page).getByRole('heading')).toHaveText('Off')
  await expect(hero(page)).toContainText('Your Mac sleeps as usual.')
  await expect(card(page).locator('.trigger-status')).toHaveText('Waiting for you')
  await expect(page.getByRole('button', { name: 'Start', exact: true })).toBeVisible()
})

test('agents are shown but don’t hold when the setting is off', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-off')
  await expect(hero(page).getByRole('heading')).toHaveText('Off')
  await expect(card(page)).toContainText('Agents don’t keep your Mac awake.')
  await expect(card(page).locator('.trigger-row.running')).toHaveCount(0)
  await card(page).getByRole('button', { name: 'Turn on' }).click()
  await expect(hero(page)).toContainText('While Claude Code works')
  await expect(card(page).getByRole('button', { name: 'Turn on' })).toHaveCount(0)
})

test('Stop pauses until the agent finishes, Resume holds again', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-working')
  await page.getByRole('button', { name: 'Stop', exact: true }).click()
  await expect(hero(page)).toContainText('Paused until Claude Code finishes')
  await hero(page).getByRole('button', { name: 'Resume' }).click()
  await expect(hero(page)).toContainText('While Claude Code works')
})

test('S stops an agent hold', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-working')
  await page.keyboard.press('s')
  await expect(hero(page)).toContainText('Paused until Claude Code finishes')
})

test('no agents card when no agent runs', async ({ page }) => {
  await page.goto('/?demo=1&state=off')
  await expect(page.getByRole('region', { name: 'Agents' })).toHaveCount(0)
})

test('other platforms say computer', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-off&platform=linux')
  await expect(card(page)).toContainText('Agents don’t keep your computer awake.')
  await page.goto('/?demo=1&state=agents-off&platform=linux&page=settings')
  await expect(page.locator('.agent-settings')).toContainText('Lets your computer sleep when they finish or wait for you.')
})

test('the switch in Settings turns agent holds off', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-working&page=settings')
  const toggle = page.getByRole('switch', { name: 'Keep awake while agents work' })
  await expect(toggle).toBeChecked()
  await toggle.click()
  await expect(toggle).not.toBeChecked()
  await page.getByRole('button', { name: 'Back to overview' }).click()
  await expect(hero(page).getByRole('heading')).toHaveText('Off')
})

test('Connect names the file and asks first; Disconnect removes', async ({ page }) => {
  await page.goto('/?demo=1&state=off&page=settings')
  const claude = row(page, 'claude')
  await expect(claude).toContainText('Not connected')
  await expect(row(page, 'codex')).toContainText('Not connected')
  await claude.getByRole('button', { name: 'Connect' }).click()
  await expect(claude).toContainText('Adds hooks to ~/.claude/settings.json. The file is backed up first.')
  await claude.getByRole('button', { name: 'Cancel' }).click()
  await expect(claude).toContainText('Not connected')
  await claude.getByRole('button', { name: 'Connect' }).click()
  await claude.getByRole('group', { name: 'Connect Claude Code' }).getByRole('button', { name: 'Connect' }).click()
  await expect(claude).toContainText('Connected')
  await expect(claude.getByRole('button', { name: 'Disconnect' })).toBeFocused()
  await claude.getByRole('button', { name: 'Disconnect' }).click()
  await expect(claude).toContainText('Not connected')
})

test('connecting works from the keyboard and Escape cancels without leaving Settings', async ({ page }) => {
  await page.goto('/?demo=1&state=off&page=settings')
  const codex = row(page, 'codex')
  await codex.getByRole('button', { name: 'Connect' }).focus()
  await page.keyboard.press('Enter')
  await expect(codex.locator('.confirm-connect')).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible()
  await expect(codex.getByRole('button', { name: 'Connect' })).toBeFocused()
  await page.keyboard.press('Enter')
  await expect(codex).toContainText('Adds hooks to ~/.codex/hooks.json and marks them trusted in ~/.codex/config.toml. Both files are backed up first.')
  await page.keyboard.press('Enter')
  await expect(codex).toContainText('Connected')
  await expect(codex.getByRole('button', { name: 'Disconnect' })).toBeFocused()
})

test('a moved Wick asks to reconnect', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-stale&page=settings')
  const claude = row(page, 'claude')
  await expect(claude).toContainText('Wick was moved')
  await claude.getByRole('button', { name: 'Reconnect' }).click()
  await expect(claude).toContainText('Connected')
})

test('an agent that isn’t installed can’t be connected', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-missing&page=settings')
  await expect(row(page, 'codex')).toContainText('Not installed')
  await expect(row(page, 'codex').getByRole('button', { name: 'Connect' })).toBeDisabled()
  await expect(row(page, 'claude').getByRole('button', { name: 'Disconnect' })).toBeEnabled()
})

test('Codex hooks turned off keep Disconnect and say why', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-hooks-off&page=settings')
  const codex = row(page, 'codex')
  await expect(codex.locator('.connection-status')).toHaveText('Hooks are turned off in ~/.codex/config.toml.')
  await codex.getByRole('button', { name: 'Disconnect' }).click()
  await expect(codex).toContainText('Not connected')
})

test('Connect says which environment it reads', async ({ page }) => {
  await page.goto('/?demo=1&state=off&page=settings')
  await row(page, 'claude').getByRole('button', { name: 'Connect' }).click()
  await expect(row(page, 'claude')).toContainText('Wick reads CLAUDE_CONFIG_DIR from its own environment, not from your shell profile.')
})

test('a file Wick can’t read is named inline and left alone', async ({ page }) => {
  await page.goto('/?demo=1&state=agents-error&page=settings')
  const claude = row(page, 'claude')
  await expect(claude.getByRole('alert')).toHaveText('~/.claude/settings.json isn’t valid JSON. Wick left it unchanged.')
  await claude.getByRole('button', { name: 'Connect' }).click()
  await claude.locator('.confirm-connect').click()
  await expect(claude.getByRole('alert')).toHaveText('~/.claude/settings.json isn’t valid JSON. Wick left it unchanged.')
  await expect(claude.getByRole('button', { name: 'Connect' })).toBeFocused()
})

for (const theme of ['light', 'dark'] as const) {
  for (const state of ['agents-working', 'agents-many', 'agents-lots', 'agents-waiting', 'agents-off']) {
    test(`screenshot ${state} ${theme}`, async ({ page }, testInfo) => {
      await page.goto(`/?demo=1&state=${state}&theme=${theme}`)
      await expect(card(page)).toBeVisible()
      await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`${state}-${theme}.png`) })
    })
  }
  for (const state of ['off', 'agents-connected', 'agents-stale', 'agents-missing', 'agents-error', 'agents-hooks-off', 'confirm', 'confirm-codex']) {
    test(`screenshot settings ${state} agents ${theme}`, async ({ page }, testInfo) => {
      await page.goto(`/?demo=1&state=${state.startsWith('confirm') ? 'off' : state}&theme=${theme}&page=settings`)
      const section = page.locator('.agent-settings')
      await expect(section.locator('.connection-row')).toHaveCount(2)
      if (state.startsWith('confirm')) await row(page, state === 'confirm' ? 'claude' : 'codex').getByRole('button', { name: 'Connect' }).click()
      await section.evaluate(element => element.scrollIntoView({ block: 'start' }))
      await page.locator('.app-shell').screenshot({ path: testInfo.outputPath(`settings-${state}-${theme}.png`) })
    })
  }
}
