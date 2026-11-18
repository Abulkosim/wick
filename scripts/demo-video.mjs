// Records the launch clip from the demo bridge: pnpm demo:video
// Needs ffmpeg with libx264 on PATH, or FFMPEG=/path/to/ffmpeg.
// Writes docs/images/demo.mp4 (H.264, 2x) and docs/images/demo.gif (1x, for the README).
import { chromium } from '@playwright/test'
import { spawn, spawnSync } from 'node:child_process'
import { linkSync, mkdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = fileURLToPath(new URL('..', import.meta.url))
const work = join(root, 'test-results', 'demo-video')
const frames = join(work, 'frames')
const images = join(root, 'docs', 'images')
const ffmpeg = process.env.FFMPEG ?? 'ffmpeg'
const port = Number(process.env.WICK_DEMO_PORT ?? 1437)
const base = process.env.WICK_DEMO_URL ?? `http://127.0.0.1:${port}`
const zoom = 2
// The page runs at half speed while recording, so the screencast catches every step of a transition.
const slow = 2
const viewport = { width: 1040 * zoom, height: 680 * zoom }

if (spawnSync(ffmpeg, ['-hide_banner', '-version'], { stdio: 'ignore' }).status !== 0) {
  console.error('ffmpeg not found. Install it (brew install ffmpeg, apt install ffmpeg) or set FFMPEG=/path/to/ffmpeg.')
  process.exit(1)
}

const server = process.env.WICK_DEMO_URL ? null : spawn('pnpm', ['exec', 'vite', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: root, stdio: 'ignore', detached: true })
const stopServer = () => { if (server?.pid) try { process.kill(-server.pid) } catch {} }
process.on('exit', stopServer)
for (let tries = 0; ; tries++) {
  try { if ((await fetch(base)).ok) break } catch {}
  if (tries > 100) throw new Error(`The demo bridge did not start at ${base}.`)
  await new Promise(resolve => setTimeout(resolve, 200))
}

rmSync(work, { recursive: true, force: true })
mkdirSync(frames, { recursive: true })

/** Menu bar, pointer and notification around the real popover; none of it is part of the app. */
function stage(zoom) {
  const css = `
    html { zoom: ${zoom} }
    .preview-ribbon { display: none !important }
    .preview-stage { place-items: start end; padding: 30px 340px 0 0; min-height: 680px }
    .stage-bar { position: fixed; inset: 0 0 auto; height: 26px; display: flex; align-items: center; justify-content: flex-end; gap: 18px; padding: 0 16px; font: 500 13px/1 Inter, sans-serif; color: #1d1d1f; background: rgba(246,246,246,.72); backdrop-filter: blur(20px); box-shadow: 0 .5px 0 #0000001f; z-index: 20; font-variant-numeric: tabular-nums }
    [data-theme=dark] .stage-bar { color: #f2f2f2; background: rgba(36,36,36,.72); box-shadow: 0 .5px 0 #ffffff1a }
    .stage-bar svg { display: block }
    .stage-tray { position: fixed; top: 2px; width: 22px; height: 22px; z-index: 21; color: #1d1d1f }
    [data-theme=dark] .stage-tray { color: #f2f2f2 }
    .stage-tray .status-candle { width: 22px; height: 22px }
    .stage-tray .candle-outline, .stage-tray .candle-wick { stroke: currentColor }
    .stage-tray .candle-body, .stage-tray .candle-flame { fill: currentColor }
    .stage-note { position: fixed; top: 34px; right: 10px; width: 344px; display: flex; gap: 10px; align-items: flex-start; padding: 11px 13px; border-radius: 16px; font: 13px/1.35 Inter, sans-serif; color: #1d1d1f; background: rgba(250,250,250,.86); backdrop-filter: blur(24px); box-shadow: 0 10px 34px -8px #0003, 0 0 0 .5px #0000001a; z-index: 30; transform: translateX(380px); transition: transform .25s ease }
    [data-theme=dark] .stage-note { color: #f2f2f2; background: rgba(44,44,44,.86); box-shadow: 0 10px 34px -8px #0008, 0 0 0 .5px #ffffff1a }
    .stage-note.shown { transform: none }
    .stage-note img { width: 34px; height: 34px; flex-shrink: 0 }
    .stage-note b { font-weight: 600 }
    .stage-note span { float: right; opacity: .55; font-size: 12px }
    .stage-note p { margin: 2px 0 0 }
    .stage-pointer { position: fixed; left: 0; top: 0; width: 22px; height: 30px; z-index: 40; pointer-events: none; transform-origin: 3px 3px; transition: scale .12s ease }
    .stage-pointer.down { scale: .86 }
    .stage-cover { position: fixed; inset: 0; z-index: 50; background: #101611; opacity: 0; pointer-events: none; transition: opacity .2s ease }
    .stage-cover.shown { opacity: 1 }
  `
  const wifi = '<svg width="17" height="13" viewBox="0 0 17 13"><path fill="currentColor" d="M8.5 2.2c2.5 0 4.8 1 6.5 2.6l1.2-1.3A10.9 10.9 0 0 0 8.5.4 10.9 10.9 0 0 0 .8 3.5L2 4.8a9.2 9.2 0 0 1 6.5-2.6Zm0 3.6c1.5 0 2.9.6 3.9 1.6l1.3-1.3a7.3 7.3 0 0 0-10.4 0l1.3 1.3c1-1 2.4-1.6 3.9-1.6Zm0 3.6c.6 0 1.2.3 1.6.7L8.5 12.3 6.9 10.1c.4-.4 1-.7 1.6-.7Z"/></svg>'
  const battery = '<svg width="26" height="12" viewBox="0 0 26 12"><rect x=".75" y=".75" width="21.5" height="10.5" rx="3" fill="none" stroke="currentColor" stroke-opacity=".45" stroke-width="1.5"/><rect x="2.5" y="2.5" width="14.5" height="7" rx="1.6" fill="currentColor"/><path d="M24 4v4c.8-.3 1.3-1 1.3-2S24.8 4.3 24 4Z" fill="currentColor" fill-opacity=".45"/></svg>'
  addEventListener('DOMContentLoaded', () => {
    const style = document.createElement('style')
    style.textContent = css
    document.head.append(style)
    document.body.insertAdjacentHTML('beforeend', `
      <div class="stage-bar">${battery}${wifi}<span class="stage-clock"></span></div>
      <div class="stage-tray"></div>
      <div class="stage-note"><img src="/docs/images/icon.png" alt=""><div><b>Wick</b><span>now</span><p></p></div></div>
      <svg class="stage-pointer" viewBox="0 0 22 30"><path d="M3 2.5v21.2l5-4.8 3.3 7.7 3.6-1.6-3.3-7.5h7.1z" fill="#111" stroke="#fff" stroke-width="1.6" stroke-linejoin="round"/></svg>
      <div class="stage-cover"></div>`)
    const tray = document.querySelector('.stage-tray')
    const clock = document.querySelector('.stage-clock')
    const pointer = document.querySelector('.stage-pointer')
    const sync = () => {
      const candle = document.querySelector('.hero .status-candle')
      const popover = document.querySelector('.popover')
      if (candle && tray.dataset.from !== candle.outerHTML) {
        tray.dataset.from = candle.outerHTML
        tray.innerHTML = candle.outerHTML
      }
      if (popover) {
        const box = popover.getBoundingClientRect()
        tray.style.left = `${(box.left + box.width / 2) / zoom - 11}px`
      }
      const now = new Date()
      const time = `${now.toLocaleDateString('en-GB', { weekday: 'short', day: 'numeric', month: 'short' }).replace(',', '')}  ${now.toLocaleTimeString('en-GB', { hour: '2-digit', minute: '2-digit' })}`
      if (clock.textContent !== time) clock.textContent = time
    }
    new MutationObserver(sync).observe(document.querySelector('#app'), { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ['class', 'y', 'height', 'd'] })
    sync()
    addEventListener('mousemove', event => { pointer.style.translate = `${event.clientX / zoom - 3}px ${event.clientY / zoom - 3}px` })
    addEventListener('mousedown', () => pointer.classList.add('down'))
    addEventListener('mouseup', () => pointer.classList.remove('down'))
    window.stage = {
      note(text) {
        const note = document.querySelector('.stage-note')
        note.querySelector('p').textContent = text ?? ''
        note.classList.toggle('shown', Boolean(text))
      },
      cover(shown) { document.querySelector('.stage-cover').classList.toggle('shown', shown) },
    }
  })
}

const browser = await chromium.launch()
const context = await browser.newContext({ viewport, deviceScaleFactor: 1, timezoneId: 'UTC', locale: 'en-GB', colorScheme: 'light', reducedMotion: 'no-preference' })
const page = await context.newPage()
await page.clock.install({ time: new Date('2026-11-18T16:30:00Z') })
await page.addInitScript(stage, zoom)
await page.goto(`${base}/?demo=1&state=off&theme=light`)
await page.evaluate(() => document.fonts.ready)
await page.mouse.move(820 * zoom, 560 * zoom)
await page.waitForTimeout(600)

const wait = ms => page.waitForTimeout(ms * slow)
const clip = () => Date.now() / slow
/** Calls frame(t) with t from 0 to 1 over ms of clip time, however long each call takes. */
async function animate(ms, frame) {
  const start = clip()
  for (let t = 0; t < 1;) {
    t = Math.min(1, (clip() - start) / ms)
    await frame(t)
    await page.waitForTimeout(10)
  }
}
let pointer = { x: 820 * zoom, y: 560 * zoom }
async function glide(x, y, ms) {
  const from = pointer
  await animate(ms, t => {
    const ease = t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2
    return page.mouse.move(from.x + (x - from.x) * ease, from.y + (y - from.y) * ease)
  })
  pointer = { x, y }
}
const patch = change => page.evaluate(change => window.wickDemo.patch(change), change)
const agent = (session, name, label, minutes) => page.evaluate(([session, name, label, minutes]) => ({ id: `${name === 'Codex' ? 'codex' : 'claude'}-${session}`, agent: name === 'Codex' ? 'codex' : 'claude', name, label, state: 'working', since: Math.floor(Date.now() / 1000) - minutes * 60, paused: false }), [session, name, label, minutes])
/** Runs the page clock through `minutes` in `ms` of clip time. */
async function lapse(minutes, ms, during = () => {}) {
  let done = 0
  await animate(ms, async t => {
    const target = Math.round(minutes * 60_000 * t)
    await page.clock.fastForward(target - done)
    done = target
    await during(t)
  })
}

const cdp = await context.newCDPSession(page)
const shots = []
cdp.on('Page.screencastFrame', ({ data, sessionId }) => {
  const file = `f${String(shots.length).padStart(5, '0')}.jpg`
  writeFileSync(join(frames, file), Buffer.from(data, 'base64'))
  shots.push({ file, at: Date.now() / 1000 / slow })
  cdp.send('Page.screencastFrameAck', { sessionId }).catch(() => {})
})
await cdp.send('Animation.enable')
await cdp.send('Animation.setPlaybackRate', { playbackRate: 1 / slow })
await cdp.send('Page.startScreencast', { format: 'jpeg', quality: 92, maxWidth: viewport.width, maxHeight: viewport.height })

await wait(1400)
const fifteen = page.getByRole('button', { name: '15 min' })
const target = await fifteen.boundingBox()
await glide(target.x + target.width / 2, target.y + target.height / 2 + 2, 700)
await wait(250)
await page.mouse.down()
await wait(110)
await page.mouse.up()
await wait(300)
await glide(190 * zoom, 500 * zoom, 800)
await wait(300)

let working = false
await lapse(14.5, 5000, async t => {
  if (working || t < 0.3) return
  working = true
  await patch({ agents: [await agent('d442c2f9', 'Claude Code', 'wick', 0)], heldBy: 'Claude Code works', heldByAgents: true })
})
await page.clock.fastForward(30_000)
await patch({ session: { mode: 'off' } })
await wait(1300)
await lapse(6, 1500)
await wait(1000)

await patch({ agents: [], heldBy: null, heldByAgents: false })
await page.evaluate(() => window.stage.note('Claude Code finished in wick. Your Mac can sleep again.'))
await wait(3200)

await page.evaluate(() => window.stage.cover(true))
await wait(260)
await page.evaluate(() => window.stage.note(null))
await page.clock.fastForward(9 * 60_000)
const agents = [await agent('d442c2f9', 'Claude Code', 'wick', 7), await agent('7f3a91c0', 'Codex', 'margin', 41)]
await page.evaluate(agents => {
  const { settings } = window.wickDemo.state()
  window.wickDemo.patch({ settings: { ...settings, theme: 'dark' }, agents, heldBy: '2 agents work', heldByAgents: true })
}, agents)
await wait(200)
await page.evaluate(() => window.stage.cover(false))
await wait(3200)

await cdp.send('Page.stopScreencast')
const ended = Date.now() / 1000 / slow
await browser.close()
stopServer()

const fps = 30
const sequence = join(work, 'sequence')
mkdirSync(sequence)
let shown = 0
for (let slot = 0, at = shots[0].at; at < ended; slot++, at = shots[0].at + slot / fps) {
  while (shots[shown + 1] && shots[shown + 1].at <= at) shown++
  linkSync(join(frames, shots[shown].file), join(sequence, `s${String(slot).padStart(5, '0')}.jpg`))
}

function encode(args) {
  const result = spawnSync(ffmpeg, ['-hide_banner', '-loglevel', 'error', '-y', '-framerate', String(fps), '-i', join(sequence, 's%05d.jpg'), ...args], { stdio: 'inherit' })
  if (result.status !== 0) throw new Error(`ffmpeg failed: ${args.at(-1)}`)
}
const mp4 = join(images, 'demo.mp4')
const gif = join(images, 'demo.gif')
encode(['-vf', 'scale=out_range=tv,format=yuv420p', '-c:v', 'libx264', '-preset', 'slow', '-crf', '23', '-tune', 'stillimage', '-movflags', '+faststart', '-an', mp4])
encode(['-vf', 'fps=15,scale=1040:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=full[p];[b][p]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle', '-loop', '0', gif])
const size = file => `${(statSync(file).size / 1e6).toFixed(2)} MB`
console.log(`${shots.length} frames, ${(ended - shots[0].at).toFixed(1)} s`)
console.log(`docs/images/demo.mp4 ${size(mp4)}`)
console.log(`docs/images/demo.gif ${size(gif)}`)
