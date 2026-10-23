import assert from 'node:assert/strict'
import test from 'node:test'
import { formatShortcut, recordShortcut } from '../../src/lib/shortcut.ts'

const key = (code: string, mods: Partial<{ ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean }> = {}) => ({ code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods })

test('records modifiers in a fixed order with the physical key', () => {
  assert.deepEqual(recordShortcut(key('KeyW', { metaKey: true, altKey: true })), { kind: 'shortcut', value: 'Alt+Super+KeyW' })
  assert.deepEqual(recordShortcut(key('Digit1', { ctrlKey: true, shiftKey: true })), { kind: 'shortcut', value: 'Control+Shift+Digit1' })
  assert.deepEqual(recordShortcut(key('F5', { metaKey: true })), { kind: 'shortcut', value: 'Super+F5' })
})

test('waits on modifiers and rejects unusable combinations', () => {
  assert.deepEqual(recordShortcut(key('MetaLeft', { metaKey: true })), { kind: 'pending' })
  assert.equal(recordShortcut(key('KeyA', { shiftKey: true })).kind, 'error')
  assert.equal(recordShortcut(key('KeyA')).kind, 'error')
  assert.equal(recordShortcut(key('IntlRo', { metaKey: true })).kind, 'error')
})

test('formats with Mac symbols', () => {
  assert.equal(formatShortcut('Alt+Super+KeyW'), '⌥⌘W')
  assert.equal(formatShortcut('Control+Shift+Digit1'), '⌃⇧1')
  assert.equal(formatShortcut('Super+Shift+Space'), '⇧⌘Space')
  assert.equal(formatShortcut('Alt+ArrowUp'), '⌥↑')
})
