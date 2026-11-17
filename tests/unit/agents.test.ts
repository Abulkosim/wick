import assert from 'node:assert/strict'
import test from 'node:test'
import { agentHolder, agentPause, agentPlace, agentStatus, agentsHold, connectionAction, connectionStatus, elapsed } from '../../src/lib/agents.ts'
import type { Agent, AgentConnection } from '../../src/lib/model.ts'

const now = 1_794_940_000_000
const seconds = now / 1000
const agent = (extra: Partial<Agent> = {}): Agent => ({ id: 'claude-a', agent: 'claude', name: 'Claude Code', label: 'wick', state: 'working', since: seconds - 12 * 60, ...extra })
const connection = (status: AgentConnection['status'], error: string | null = null): AgentConnection => ({ agent: 'claude', name: 'Claude Code', status, file: '~/.claude/settings.json', error })

test('only a working agent holds, and only while enabled', () => {
  assert.equal(agentsHold([agent()], true), true)
  assert.equal(agentsHold([agent()], false), false)
  assert.equal(agentsHold([agent({ state: 'waiting' })], true), false)
  assert.equal(agentsHold([], true), false)
})

test('the holder names one agent or counts several working ones', () => {
  assert.equal(agentHolder([]), null)
  assert.equal(agentHolder([agent({ state: 'waiting' })]), null)
  assert.equal(agentHolder([agent()]), 'Claude Code works')
  assert.equal(agentHolder([agent()], true), 'Claude Code works in wick')
  assert.equal(agentHolder([agent(), agent({ id: 'codex-b', name: 'Codex' }), agent({ id: 'c', state: 'waiting' })]), '2 agents work')
  assert.equal(agentHolder([agent({ agent: 'other', name: 'render', label: 'render' })]), 'render works')
})

test('a label equal to the name is shown once', () => {
  assert.equal(agentPlace(agent()), 'wick')
  assert.equal(agentPlace(agent({ agent: 'other', name: 'render', label: 'render' })), null)
  assert.equal(agentPlace(agent({ label: '' })), null)
})

test('a pause waits for the one working agent or for all of them', () => {
  assert.equal(agentPause([agent(), agent({ id: 'w', state: 'waiting' })]), 'Claude Code finishes')
  assert.equal(agentPause([agent(), agent({ id: 'b', name: 'Codex' })]), 'the agents finish')
})

test('elapsed time counts whole minutes, at least one', () => {
  assert.equal(elapsed(seconds - 10, now), '1m')
  assert.equal(elapsed(seconds + 30, now), '1m')
  assert.equal(elapsed(seconds - 12 * 60 - 59, now), '12m')
  assert.equal(elapsed(seconds - 65 * 60, now), '1h 05m')
  assert.equal(elapsed(seconds - 26 * 3600, now), '1d 2h')
})

test('status says how long an agent works or that it waits', () => {
  assert.equal(agentStatus(agent(), now), 'Working · 12m')
  assert.equal(agentStatus(agent({ state: 'waiting' }), now), 'Waiting for you')
})

test('each connection offers one action and says where it stands', () => {
  assert.deepEqual((['connected', 'disconnected', 'stale', 'missing', 'error'] as const).map(status => connectionAction(connection(status))), ['disconnect', 'connect', 'reconnect', 'connect', 'connect'])
  assert.equal(connectionStatus(connection('connected')), 'Connected')
  assert.equal(connectionStatus(connection('disconnected')), 'Not connected')
  assert.equal(connectionStatus(connection('missing')), 'Not installed')
  assert.equal(connectionStatus(connection('stale')), 'Wick was moved')
  assert.equal(connectionStatus(connection('error', 'Not valid JSON.')), 'Not valid JSON.')
  assert.equal(connectionStatus(connection('error')), 'Could not read ~/.claude/settings.json.')
})

test('agent payloads are validated and clipped', async () => {
  const { agentSchema, appStateSchema, initialState } = await import('../../src/lib/model.ts')
  assert.equal(agentSchema.parse({ ...agent(), label: 'x'.repeat(300) }).label.length, 256)
  assert.ok(!agentSchema.safeParse({ ...agent(), state: 'sleeping' }).success)
  assert.ok(!agentSchema.safeParse({ ...agent(), agent: 'gemini' }).success)
  assert.ok(!appStateSchema.safeParse({ ...initialState(), agents: Array.from({ length: 17 }, () => agent()) }).success)
  assert.deepEqual(initialState().settings.agents, { enabled: true })
})
