import { access, readFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

/**
 * Checks what a release tag needs before CI builds it. Prerelease tags are refused: a published
 * release becomes `releases/latest`, which every installed copy reads for updates.
 */
export async function checkRelease(tag, root = '.') {
  if (!tag || !/^v\d+\.\d+\.\d+$/.test(tag)) {
    throw new Error('Expected a release tag such as v0.1.0, without a prerelease suffix.')
  }
  const read = (path) => readFile(join(root, path), 'utf8')
  const [packageText, tauriText, cargoText, changelog] = await Promise.all([
    read('package.json'),
    read('src-tauri/tauri.conf.json'),
    read('src-tauri/Cargo.toml'),
    read('CHANGELOG.md'),
  ])
  const packageConfig = JSON.parse(packageText)
  const tauriConfig = JSON.parse(tauriText)
  const cargoPackage = cargoText.split(/^\[package\]\s*$/m)[1]?.split(/^\[/m)[0]
  const cargoVersion = cargoPackage?.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1]
  const version = tag.slice(1)

  for (const [name, actual] of Object.entries({
    'package.json': packageConfig.version,
    'tauri.conf.json': tauriConfig.version,
    'Cargo.toml': cargoVersion,
  })) {
    if (actual !== version) throw new Error(`${name} version must match ${tag}.`)
  }

  for (const path of ['pnpm-lock.yaml', 'src-tauri/Cargo.lock', `changelogs/${tag}.md`]) {
    await access(join(root, path)).catch(() => {
      throw new Error(`Commit ${path} before tagging a release.`)
    })
  }
  if (!changelog.includes(`| [${tag}](changelogs/${tag}.md) |`)) {
    throw new Error(`Add a CHANGELOG.md row for ${tag}.`)
  }

  const updater = tauriConfig.plugins?.updater
  if (tauriConfig.bundle?.createUpdaterArtifacts !== true
    || updater?.requireSignedVersion !== true
    || updater?.endpoints?.length !== 1
    || updater.endpoints[0] !== 'https://github.com/Abulkosim/wick/releases/latest/download/latest.json'
    || typeof updater.pubkey !== 'string' || !updater.pubkey.startsWith('dW50cnVzdGVkIGNvbW1lbnQ6')) {
    throw new Error('Configure signed, version-bound updater artifacts and the public release endpoint before tagging.')
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const tag = process.argv[2]
  await checkRelease(tag)
  console.log(`Release inputs verified for ${tag}.`)
}
