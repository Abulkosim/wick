import assert from 'node:assert/strict'
import test from 'node:test'
import { triggerId, validProcessName } from '../../src/lib/triggers.ts'

test('process names match the Rust rules', () => {
  assert.ok(validProcessName('claude'))
  assert.ok(validProcessName(' ffmpeg '))
  assert.ok(!validProcessName(''))
  assert.ok(!validProcessName('/usr/bin/claude'))
  assert.ok(!validProcessName('x'.repeat(65)))
})

test('ids fit the stored format', () => {
  assert.match(triggerId(), /^[A-Za-z0-9-]{1,64}$/)
  assert.notEqual(triggerId(), triggerId())
})
