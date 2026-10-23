import assert from 'node:assert/strict'
import test from 'node:test'
import { activePreset, customMinutes, detail, fraction, parseClock, shortDuration, start, title } from '../../src/lib/session.ts'

const now = new Date(2026, 9, 23, 15, 0).getTime()

test('short durations round up to the minute', () => {
  assert.equal(shortDuration(0), '0m')
  assert.equal(shortDuration(1), '1m')
  assert.equal(shortDuration(42 * 60_000), '42m')
  assert.equal(shortDuration(65 * 60_000), '1h 05m')
  assert.equal(shortDuration(51 * 3_600_000), '2d 3h')
})

test('timed sessions read as time left', () => {
  const session = start({ mode: 'for', minutes: 60 }, now)
  assert.equal(title(session, now + 18 * 60_000), 'Awake · 42m left')
  assert.equal(fraction(session, now + 15 * 60_000), 0.75)
  assert.equal(fraction(session, now + 5 * 3_600_000), 0)
  assert.equal(activePreset(session), '1h')
  assert.equal(activePreset(start({ mode: 'for', minutes: 90 }, now)), 'custom')
  assert.match(detail(session, now, false), /^Until \S+/)
  assert.ok(!detail(session, now, false).includes('Display'))
})

test('until rolls to tomorrow once the time has passed', () => {
  const later = start({ mode: 'until', hour: 16, minute: 30 }, now)
  assert.equal(later.mode === 'until' && later.endsAt - now, 90 * 60_000)
  const tomorrow = start({ mode: 'until', hour: 9, minute: 0 }, now)
  assert.equal(tomorrow.mode === 'until' && tomorrow.endsAt - now, 18 * 3_600_000)
  assert.equal(detail(later, now, true), '1h 30m left · Display on')
})

test('off and indefinite', () => {
  assert.equal(title({ mode: 'off' }, now), 'Off')
  assert.equal(detail({ mode: 'off' }, now, true), 'Your Mac sleeps as usual.')
  const forever = start({ mode: 'indefinite' }, now)
  assert.equal(title(forever, now), 'Awake')
  assert.equal(fraction(forever, now), 1)
  assert.equal(detail(forever, now, false), 'Until you stop it')
})

test('clock and custom inputs are validated', () => {
  assert.deepEqual(parseClock('9:05'), { hour: 9, minute: 5 })
  assert.equal(parseClock('24:00'), null)
  assert.equal(parseClock('9.05'), null)
  assert.equal(customMinutes(1, 30), 90)
  assert.equal(customMinutes(0, 0), null)
  assert.equal(customMinutes(168, 1), null)
  assert.equal(customMinutes(1.5, 0), null)
})

test('held and paused by a trigger', () => {
  assert.equal(title({ mode: 'off' }, now, 'Xcode'), 'Awake')
  assert.equal(detail({ mode: 'off' }, now, true, 'Xcode is running'), 'While Xcode is running · Display on')
  assert.equal(detail({ mode: 'off' }, now, false, 'on power'), 'While on power')
  assert.equal(detail({ mode: 'off' }, now, false, null, 'claude quits'), 'Paused until claude quits')
  assert.equal(detail({ mode: 'off' }, now, false, null, 'you unplug'), 'Paused until you unplug')
  assert.equal(title(start({ mode: 'indefinite' }, now), now, 'Xcode'), 'Awake')
})
