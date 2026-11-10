import assert from 'node:assert/strict'
import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { checkRelease } from './check-release.mjs'

const version = JSON.parse(await readFile('package.json', 'utf8')).version
const tag = `v${version}`

async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'wick-release-'))
  t.after(() => rm(root, { recursive: true, force: true }))
  await mkdir(join(root, 'src-tauri'), { recursive: true })
  await mkdir(join(root, 'changelogs'), { recursive: true })
  for (const path of ['package.json', 'pnpm-lock.yaml', 'src-tauri/tauri.conf.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock']) {
    await cp(path, join(root, path))
  }
  await writeFile(join(root, 'changelogs', `${tag}.md`), `# ${tag}\n`)
  await writeFile(join(root, 'CHANGELOG.md'), `| Version | Date (UTC) | Changes |\n| --- | --- | --- |\n| [${tag}](changelogs/${tag}.md) | 2026-11-10 | Test. |\n`)
  return root
}

test('accepts a tag whose versions, changelog and updater match', async (t) => {
  await checkRelease(tag, await fixture(t))
})

test('refuses prerelease and malformed tags', async (t) => {
  const root = await fixture(t)
  for (const bad of [`${tag}-beta.1`, version, 'v1.2', '']) {
    await assert.rejects(checkRelease(bad, root), /Expected a release tag/)
  }
})

test('refuses a version mismatch, a missing changelog row or file', async (t) => {
  const root = await fixture(t)
  await assert.rejects(checkRelease('v99.0.0', root), /package.json version must match/)
  await writeFile(join(root, 'CHANGELOG.md'), '| Version | Date (UTC) | Changes |\n')
  await assert.rejects(checkRelease(tag, root), /CHANGELOG.md row/)
  await rm(join(root, 'changelogs', `${tag}.md`))
  await assert.rejects(checkRelease(tag, root), /Commit changelogs/)
})

test('refuses an updater without signed, version-bound artifacts', async (t) => {
  const root = await fixture(t)
  const config = JSON.parse(await readFile(join(root, 'src-tauri/tauri.conf.json'), 'utf8'))
  config.plugins.updater.requireSignedVersion = false
  await writeFile(join(root, 'src-tauri/tauri.conf.json'), JSON.stringify(config))
  await assert.rejects(checkRelease(tag, root), /signed, version-bound/)
})
