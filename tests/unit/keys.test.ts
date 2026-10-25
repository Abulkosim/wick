import assert from 'node:assert/strict'
import test from 'node:test'
import { keyAction } from '../../src/lib/keys.ts'

const key = (k: string, mods: Partial<{ metaKey: boolean; ctrlKey: boolean; altKey: boolean }> = {}) => ({ key: k, metaKey: false, ctrlKey: false, altKey: false, ...mods })

test('number keys start presets, S toggles, + extends', () => {
  assert.deepEqual(keyAction(key('1'), false), { kind: 'start', request: { mode: 'for', minutes: 15 } })
  assert.deepEqual(keyAction(key('3'), false), { kind: 'start', request: { mode: 'for', minutes: 120 } })
  assert.deepEqual(keyAction(key('0'), false), { kind: 'start', request: { mode: 'indefinite' } })
  assert.deepEqual(keyAction(key('S'), false), { kind: 'toggle' })
  assert.deepEqual(keyAction(key('+'), false), { kind: 'extend', minutes: 15 })
})

test('typing and modifier chords are left alone', () => {
  assert.equal(keyAction(key('1'), true), null)
  assert.equal(keyAction(key('s', { metaKey: true }), false), null)
  assert.equal(keyAction(key('1', { altKey: true }), false), null)
  assert.equal(keyAction(key('x'), false), null)
})
