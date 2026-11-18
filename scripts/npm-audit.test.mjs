import { test } from 'node:test'
import assert from 'node:assert/strict'
import { blocking, ignoredGhsas } from './npm-audit.mjs'

const via = (severity, ghsa) => ({ severity, name: 'pkg', title: 't', url: `https://github.com/advisories/${ghsa}` })

test('reads the ignore list from pnpm-workspace.yaml', () => {
  assert.deepEqual([...ignoredGhsas('auditConfig:\n  ignoreGhsas:\n    - GHSA-fc5q-39xc-vjpf\n')], ['GHSA-fc5q-39xc-vjpf'])
})

test('blocks high and critical advisories that are not ignored', () => {
  const report = { vulnerabilities: {
    a: { via: [via('high', 'GHSA-aaaa-bbbb-cccc'), 'b'] },
    b: { via: [via('moderate', 'GHSA-dddd-eeee-ffff')] },
    c: { via: [via('critical', 'GHSA-fc5q-39xc-vjpf')] },
  } }
  assert.equal(blocking(report, new Set(['GHSA-fc5q-39xc-vjpf'])).length, 1)
  assert.equal(blocking(report, new Set(['GHSA-fc5q-39xc-vjpf', 'GHSA-aaaa-bbbb-cccc'])).length, 0)
})
