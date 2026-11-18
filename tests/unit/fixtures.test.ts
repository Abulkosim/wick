import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { agentHolder } from '../../src/lib/agents.ts'
import { agentConnectionsSchema, appStateSchema, defaultSettings } from '../../src/lib/model.ts'

const fixture = (name: string): unknown => JSON.parse(readFileSync(new URL(`../fixtures/${name}`, import.meta.url), 'utf8'))

test('the Rust snapshot parses with every field kept', () => {
  const raw = fixture('state.json')
  const state = appStateSchema.parse(raw)
  assert.deepEqual(state, raw)
  assert.deepEqual(state.settings, defaultSettings)
  assert.equal(state.heldBy, agentHolder(state.agents))
})

test('Rust agent connections parse with every field kept', () => {
  const raw = fixture('connections.json')
  assert.deepEqual(agentConnectionsSchema.parse(raw), raw)
})
