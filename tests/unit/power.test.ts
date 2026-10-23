import assert from 'node:assert/strict'
import test from 'node:test'
import { powerNote, stopReason } from '../../src/lib/power.ts'

const guard = { batteryGuard: true, batteryThreshold: 20, stopInLowPowerMode: false }
const battery = (percent: number, lowPowerMode = false) => ({ onBattery: true, percent, lowPowerMode })

test('matches the Rust guard', () => {
  assert.equal(stopReason(battery(20), guard), null)
  assert.equal(stopReason(battery(19), guard), 'Stopped: battery below 20%')
  assert.equal(stopReason({ ...battery(5), onBattery: false }, guard), null)
  assert.equal(stopReason(battery(5), { ...guard, batteryGuard: false }), null)
  assert.equal(stopReason(battery(80, true), guard), null)
  assert.equal(stopReason(battery(80, true), { ...guard, stopInLowPowerMode: true }), 'Stopped: Low Power Mode is on')
})

test('footer note stays quiet on power', () => {
  assert.equal(powerNote(battery(82)), 'On battery · 82%')
  assert.equal(powerNote({ onBattery: false, percent: 100, lowPowerMode: false }), null)
  assert.equal(powerNote({ onBattery: false, percent: null, lowPowerMode: false }), null)
  assert.equal(powerNote(battery(50, true)), 'Low Power Mode is on')
})
