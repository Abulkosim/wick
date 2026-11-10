import assert from 'node:assert/strict'
import test from 'node:test'
import { defaultSettings } from '../../src/lib/model.ts'

test('outgoing payloads are checked before they reach Rust', async () => {
  const { outgoing } = await import('../../src/lib/wick.ts')
  assert.ok(outgoing.start_session?.safeParse({ request: { mode: 'for', minutes: 60 } }).success)
  assert.ok(!outgoing.start_session?.safeParse({ request: { mode: 'for', minutes: -1 } }).success)
  assert.ok(outgoing.extend_session?.safeParse({ minutes: 15 }).success)
  assert.ok(!outgoing.extend_session?.safeParse({ minutes: 1.5 }).success)
  assert.ok(outgoing.save_settings?.safeParse({ settings: defaultSettings }).success)
  assert.ok(!outgoing.save_settings?.safeParse({ settings: { ...defaultSettings, batteryThreshold: 'low' } }).success)
  assert.ok(!outgoing.set_display?.safeParse({ display: 'yes' }).success)
})
