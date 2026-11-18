#!/usr/bin/env node
// `npm audit` for the Raycast extension, honouring the GHSA ignore list in pnpm-workspace.yaml.
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const LEVELS = ['info', 'low', 'moderate', 'high', 'critical']
const GHSA = /GHSA(-[0-9a-z]{4}){3}/

export function ignoredGhsas(yaml) {
  return new Set(yaml.match(new RegExp(GHSA, 'g')) ?? [])
}

export function blocking(report, ignored, level = 'high') {
  const min = LEVELS.indexOf(level)
  const found = new Map()
  for (const vulnerability of Object.values(report.vulnerabilities ?? {})) {
    for (const via of vulnerability.via ?? []) {
      if (typeof via !== 'object' || LEVELS.indexOf(via.severity) < min) continue
      const id = via.url?.match(GHSA)?.[0] ?? via.url ?? via.title
      if (!ignored.has(id)) found.set(id, `${via.severity} ${via.name}: ${via.title} (${via.url})`)
    }
  }
  return [...found.values()]
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const ignored = ignoredGhsas(readFileSync(new URL('../pnpm-workspace.yaml', import.meta.url), 'utf8'))
  let output
  try {
    output = execFileSync('npm', ['audit', '--omit=dev', '--json'], { cwd: process.argv[2] ?? '.', encoding: 'utf8' })
  } catch (error) {
    output = error.stdout
  }
  const problems = blocking(JSON.parse(output), ignored)
  for (const problem of problems) console.error(problem)
  console.log(problems.length ? `${problems.length} advisories at high or above.` : `No advisories at high or above (ignored: ${[...ignored].join(', ') || 'none'}).`)
  process.exit(problems.length ? 1 : 0)
}
