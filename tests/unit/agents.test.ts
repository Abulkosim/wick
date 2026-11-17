import assert from 'node:assert/strict'
import test from 'node:test'
import { appStateSchema, defaultSettings, initialState, settingsSchema } from '../../src/lib/model.ts'

const hold = { id: 'claude-d442c2f9-7e86-4aa5-a5ba-c14469f84e18', agent: 'claude', name: 'Claude Code', label: 'wick', state: 'working', since: 1794940846 }

test('the snapshot carries agent holds', () => {
  assert.deepEqual(appStateSchema.parse(initialState()).agents, [])
  assert.equal(appStateSchema.parse({ ...initialState(), agents: [hold] }).agents[0]?.name, 'Claude Code')
  assert.ok(!appStateSchema.safeParse({ ...initialState(), agents: Array(17).fill(hold) }).success)
  assert.ok(!appStateSchema.safeParse({ ...initialState(), agents: [{ ...hold, id: '../x' }] }).success)
  assert.ok(!appStateSchema.safeParse({ ...initialState(), agents: [{ ...hold, state: 'sleeping' }] }).success)
})

test('agents keep the computer awake by default', () => {
  assert.equal(defaultSettings.agents.enabled, true)
  assert.ok(!settingsSchema.safeParse({ ...defaultSettings, agents: undefined }).success)
  const state = initialState()
  state.settings.agents.enabled = false
  assert.equal(defaultSettings.agents.enabled, true)
})
