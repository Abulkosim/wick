import type { UpdateState } from './updater.ts'
import { initialState, type AppState, type RunningApp, type Trigger } from './model.ts'

export const demoScenarios = ['off', 'timed', 'until', 'indefinite', 'display', 'battery', 'error', 'recovery', 'recovered', 'desktop', 'held', 'paused', 'triggers', 'others', 'others-off', 'clamshell', 'update-available', 'locked', 'conditions'] as const

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

export function createDemoState(scenario: string | null, now = Date.now()): AppState {
  const state = initialState()
  state.power = { onBattery: true, percent: 82, lowPowerMode: false }
  state.lidSleeps = true
  const minutes = (value: number): number => value * 60_000
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
      ]
      state.power = { onBattery: false, percent: 100, lowPowerMode: false }
      state.running = ['demo-power']
      state.heldBy = 'on power'
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
