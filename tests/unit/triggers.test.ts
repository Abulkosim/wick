import assert from 'node:assert/strict'
import test from 'node:test'
import { daysLabel, triggerId, validProcessName, validSchedule } from '../../src/lib/triggers.ts'

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

test('schedule labels and checks match Rust', () => {
  assert.equal(daysLabel([4, 0, 1, 2, 3]), 'Weekdays')
  assert.equal(daysLabel([0, 1, 2, 3, 4, 5, 6]), 'Every day')
  assert.equal(daysLabel([5, 6]), 'Weekends')
  assert.equal(daysLabel([2, 0]), 'Mon, Wed')
  assert.equal(validSchedule([0], '09:00', '18:00'), null)
  assert.equal(validSchedule([0], '22:00', '06:00'), null)
  assert.equal(validSchedule([], '09:00', '18:00'), 'Choose at least one day.')
  assert.equal(validSchedule([0], '9am', '18:00'), 'Use times like 09:00.')
  assert.equal(validSchedule([0], '09:00', '09:00'), 'Start and end can’t be the same time.')
})
