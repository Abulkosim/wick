import { initialState, type AppState } from './model.ts'

export const demoScenarios = ['off', 'timed', 'until', 'indefinite', 'display', 'error', 'recovery', 'recovered'] as const

export function createDemoState(scenario: string | null, now = Date.now()): AppState {
  const state = initialState()
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
