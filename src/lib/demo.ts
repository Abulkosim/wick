import { initialState, type AppState } from './model.ts'

export const demoScenarios = ['off', 'timed', 'until', 'indefinite', 'display', 'error'] as const

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
      state.display = true
      break
    }
    case 'indefinite':
      state.session = { mode: 'indefinite', startedAt: now - minutes(64) }
      break
    case 'display':
      state.session = { mode: 'indefinite', startedAt: now - minutes(5) }
      state.display = true
      break
    case 'error':
      state.error = 'macOS refused the sleep assertion (IOKit error 0xe00002c2).'
      break
  }
  return state
}
