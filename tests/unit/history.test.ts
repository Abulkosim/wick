import assert from 'node:assert/strict'
import test from 'node:test'
import { bars, hasHistory, minutesText, totalMinutes } from '../../src/lib/history.ts'

const week = [0, 30, 0, 125, 240, 3, 72].map((minutes, index) => ({ date: `2026-10-${String(20 + index).padStart(2, '0')}`, minutes }))

test('totals and emptiness', () => {
  assert.equal(totalMinutes(week), 470)
  assert.equal(hasHistory(week), true)
  assert.equal(hasHistory(week.map(day => ({ ...day, minutes: 0 }))), false)
  assert.equal(hasHistory([]), false)
})

test('durations read like the rest of Wick', () => {
  assert.equal(minutesText(0), '0m')
  assert.equal(minutesText(72), '1h 12m')
  assert.equal(minutesText(240), '4h 00m')
})

test('bars scale to the busiest day and mark today', () => {
  const result = bars(week)
  assert.equal(result.length, 7)
  assert.equal(result[4]!.height, 1)
  assert.equal(result[0]!.height, 0)
  assert.equal(result[5]!.height, 0.08)
  assert.equal(result[3]!.height, 125 / 240)
  assert.deepEqual(result.map(bar => bar.today), [false, false, false, false, false, false, true])
  assert.equal(result[6]!.duration, '1h 12m')
  assert.equal(result[6]!.initial, new Date(2026, 9, 26).toLocaleDateString(undefined, { weekday: 'narrow' }))
})
