import assert from 'node:assert/strict'
import test from 'node:test'
import { otherDetail, othersSummary } from '../../src/lib/others.ts'

const other = (name: string, extra: Partial<{ display: boolean; reason: string; parent: string }> = {}) => ({ pid: 1, name, display: false, reason: null, parent: null, ...extra })

test('details prefer the starter, then the reason', () => {
  assert.equal(otherDetail(other('caffeinate', { parent: 'claude' })), 'Started by claude')
  assert.equal(otherDetail(other('zoom.us', { reason: 'Zoom meeting', display: true })), 'Zoom meeting · Display on')
  assert.equal(otherDetail(other('Music')), 'Keeps the Mac awake')
  assert.equal(otherDetail(other('VLC', { display: true })), 'Keeps the display on')
})

test('summary names one holder or counts several', () => {
  assert.equal(othersSummary([]), null)
  assert.equal(othersSummary([other('caffeinate', { parent: 'claude' })]), 'claude is keeping your Mac awake.')
  assert.equal(othersSummary([other('zoom.us'), other('Music')]), '2 apps are keeping your Mac awake.')
})
