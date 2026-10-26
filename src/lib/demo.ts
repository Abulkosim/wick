import type { UpdateState } from './updater.ts'
import { initialState, type AppState, type Day, type RunningApp, type Trigger } from './model.ts'

export const demoScenarios = ['off', 'timed', 'until', 'indefinite', 'display', 'battery', 'error', 'recovery', 'recovered', 'desktop', 'held', 'paused', 'triggers', 'others', 'others-off', 'clamshell', 'update-available', 'locked', 'conditions', 'history'] as const

export const demoApps: RunningApp[] = [
  { name: 'Final Cut Pro', path: '/Applications/Final Cut Pro.app' },
  { name: 'Xcode', path: '/Applications/Xcode.app' },
  { name: 'zoom.us', path: '/Applications/zoom.us.app' },
]

export const demoTriggers = (): Trigger[] => [
  { id: 'demo-xcode', kind: 'app', name: 'Xcode', path: '/Applications/Xcode.app', enabled: true },
  { id: 'demo-claude', kind: 'process', name: 'claude', enabled: true },
  { id: 'demo-ffmpeg', kind: 'process', name: 'ffmpeg', enabled: false },
]

/** The last seven days ending today, oldest first. */
export function demoHistory(minutes: number[], now = Date.now()): Day[] {
  return minutes.map((value, index) => {
    const date = new Date(now)
    date.setDate(date.getDate() - (minutes.length - 1 - index))
    const pad = (part: number) => String(part).padStart(2, '0')
    return { date: `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`, minutes: value }
  })
}

export function createDemoState(scenario: string | null, now = Date.now()): AppState {
  const state = initialState()
  state.power = { onBattery: true, percent: 82, lowPowerMode: false }
  state.lidSleeps = true
  const minutes = (value: number): number => value * 60_000
  if (scenario && !['off', 'recovery', 'error', 'battery'].includes(scenario)) state.history = demoHistory([95, 210, 0, 160, 312, 48, 192], now)
  switch (scenario) {
    case 'timed':
      state.session = { mode: 'for', startedAt: now - minutes(18), endsAt: now + minutes(42) }
      break
    case 'until': {
      const end = new Date(now)
      end.setHours(end.getHours() + 3, 30, 0, 0)
      state.session = { mode: 'until', startedAt: now - minutes(25), endsAt: end.getTime() }
      state.settings.keepDisplayOn = true
      break
    }
    case 'indefinite':
      state.session = { mode: 'indefinite', startedAt: now - minutes(64) }
      break
    case 'display':
      state.session = { mode: 'indefinite', startedAt: now - minutes(5) }
      state.settings.keepDisplayOn = true
      break
    case 'battery':
      state.power = { onBattery: true, percent: 19, lowPowerMode: false }
      state.notice = 'Stopped: battery below 20%'
      break
    case 'desktop':
      state.power = { onBattery: false, percent: null, lowPowerMode: false }
      break
    case 'held':
      state.triggers = demoTriggers()
      state.running = ['demo-xcode']
      state.heldBy = 'Xcode is running'
      break
    case 'paused':
      state.triggers = demoTriggers()
      state.running = ['demo-claude']
      state.pausedBy = 'claude quits'
      break
    case 'triggers':
      state.triggers = demoTriggers()
      state.session = { mode: 'for', startedAt: now - minutes(10), endsAt: now + minutes(110) }
      state.running = ['demo-claude']
      break
    case 'others':
      state.session = { mode: 'indefinite', startedAt: now - minutes(12) }
      state.others = [
        { pid: 7710, name: 'caffeinate', display: false, reason: null, parent: 'claude' },
        { pid: 2585, name: 'Google Chrome', display: true, reason: 'Playing audio in a background tab with a very long title', parent: null },
        { pid: 812, name: 'zoom.us', display: true, reason: 'Zoom meeting', parent: null },
      ]
      break
    case 'others-off':
      state.others = [{ pid: 7710, name: 'caffeinate', display: false, reason: null, parent: 'claude' }]
      break
    case 'clamshell':
      state.session = { mode: 'indefinite', startedAt: now - minutes(30) }
      state.power = { onBattery: false, percent: 100, lowPowerMode: false }
      state.lidSleeps = false
      break
    case 'locked':
      state.session = { mode: 'indefinite', startedAt: now - minutes(40) }
      state.settings.pauseWhenLocked = true
      state.pausedByLock = true
      break
    case 'conditions':
      state.triggers = [
        { id: 'demo-power', kind: 'power', name: 'Power adapter', enabled: true },
        { id: 'demo-display', kind: 'display', name: 'External display', enabled: true },
        { id: 'demo-schedule', kind: 'schedule', name: 'Weekdays 09:00–18:00', schedule: { days: [0, 1, 2, 3, 4], start: '09:00', end: '18:00' }, enabled: true },
      ]
      state.power = { onBattery: false, percent: 100, lowPowerMode: false }
      state.running = ['demo-power']
      state.heldBy = 'on power'
      break
    case 'history':
      state.session = { mode: 'for', startedAt: now - minutes(25), endsAt: now + minutes(95) }
      break
    case 'error':
      state.error = 'macOS refused the sleep assertion (IOKit error 0xe00002c2).'
      break
    case 'recovery':
      state.recovery = "Wick's settings file is damaged or unsupported. Settings file: ~/Library/Application Support/com.abulkosim.wick/settings.json."
      break
    case 'recovered':
      state.recovered = ['Theme was reset to its default.']
      break
  }
  return state
}

export function createDemoUpdateState(scenario: string | null): UpdateState {
  return scenario === 'update-available' ? { kind: 'available', version: '0.2.0' } : { kind: 'idle' }
}
