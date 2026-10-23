# Releasing Wick

Releases are not signed by a verified publisher. macOS builds use ad-hoc signing for Apple Silicon compatibility; this does not provide notarization or remove Gatekeeper prompts. See [Tauri's signing guidance](https://v2.tauri.app/distribute/sign/macos/#ad-hoc-signing).

## Before tagging

1. Run the full check suite (below) and the native checks in [verification.md](verification.md).
2. Commit both `pnpm-lock.yaml` and `src-tauri/Cargo.lock`. Frontend installs use `--frozen-lockfile`; Rust checks and builds use `--locked`.
3. Set the same version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
4. Write `changelogs/vX.Y.Z.md` and add a row to `CHANGELOG.md`: the release date and a few short, user-facing bullets that say where a setting lives. The release workflow uses this file for the GitHub release notes and the update manifest.
5. Run `node --test scripts/*.test.mjs` and `node scripts/check-release.mjs vX.Y.Z`. It requires matching versions, both lockfiles, the changelog, the public updater key, the GitHub manifest URL and signed-version enforcement.
6. Push the release commit and confirm all checks on `main` pass. Create and push the matching annotated tag: `git tag -a vX.Y.Z -m "Wick vX.Y.Z" && git push origin vX.Y.Z`.

```sh
pnpm typecheck && pnpm test && pnpm test:e2e && node --test scripts/*.test.mjs
cd src-tauri && cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked
```

## What CI does

- Ubuntu: installs the pinned pnpm, checks types, builds the interface, runs unit, release-script and browser tests. On a tag it validates the release inputs first.
- macOS: checks Rust formatting, runs Clippy and the Rust tests (including live IOKit assertion tests), and builds the universal Apple Silicon/Intel app and DMG.
- On a release tag in `Abulkosim/wick` only, the macOS job signs the updater archive with `TAURI_SIGNING_PRIVATE_KEY`. The release job then:
  - verifies the signature and its signed version, and generates `latest.json`;
  - writes `SHA256SUMS.txt`;
  - generates an SPDX SBOM;
  - records GitHub build provenance for the DMG, archive, `latest.json` and checksums, and attests the SBOM;
  - creates or updates a **draft** release.

Actions are pinned to commit SHAs. Builds have read-only repository access; only the release job can write, plus the OIDC and attestation permissions provenance needs. Pull requests, `main` builds and manual runs build without updater artifacts and receive no signing secrets.

## Publish

1. Download the draft's assets into `.cache/release-vX.Y.Z/` with `gh release download vX.Y.Z -D .cache/release-vX.Y.Z`.
2. Verify `shasum -a 256 -c SHA256SUMS.txt` and the attestations:

   ```sh
   for f in Wick_X.Y.Z_universal.dmg Wick.app.tar.gz latest.json SHA256SUMS.txt; do
     gh attestation verify "$f" --repo Abulkosim/wick
   done
   gh attestation verify Wick_X.Y.Z_universal.dmg --repo Abulkosim/wick --predicate-type https://spdx.dev/Document/v2.3
   ```

3. Run the updater install test against that exact archive (below).
4. Install the DMG's app, launch it, start and stop a session, and check `pmset -g assertions`.
5. Record what passed in [verification.md](verification.md), newest release first.
6. Publish the draft and mark it latest: `gh release edit vX.Y.Z --draft=false --latest`. Fix the changelog date to the publish date (UTC) if it differs.

The workflow refuses to replace assets on a published release. Never edit a published release's files or reuse a version; ship a new patch.

## Updates

Tauri updater signatures are free. They authenticate downloads against the public key embedded in Wick; they do not replace Apple notarization.

The key lives outside the repository in `~/.tauri/wick/` (`wick.key`, `wick.key.pub`, `wick.key.password`, all 0600), with a backup kept by the owner. Its contents are in the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` Actions secrets. Losing it means installed copies can never trust another update.

Run the install test against a release archive before publishing:

```sh
WICK_UPDATE_ARTIFACT_DIR="$PWD/.cache/release-vX.Y.Z" WICK_REQUIRE_UNIVERSAL=1 \
  cargo test --manifest-path src-tauri/Cargo.toml --test updater_install --locked -- --ignored --nocapture
```

It uses the pinned updater plugin and the real public key with a loopback-only server. It checks that altered bytes and a mismatched signed version are rejected before replacing a disposable old app, then that the install has the right version, ad-hoc signature and both Mac architectures. It never opens the installed copy.

Local release builds need the signing environment, or `--config '{"bundle":{"createUpdaterArtifacts":false}}'`.

`latest.json` is served from the latest published release. Both Mac architectures use the universal archive. To stop offering a bad release, mark it as a prerelease so the endpoint falls back to the previous release, then ship a fixed patch.
