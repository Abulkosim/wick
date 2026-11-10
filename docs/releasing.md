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
- Windows: checks Rust formatting, runs Clippy and the Rust tests, and builds x64 NSIS and MSI installers.
- Linux x64 and arm64 (Ubuntu 22.04, for glibc 2.35+): each checks formatting, runs Clippy and the Rust tests under a private session bus against the runner's logind, builds the AppImage, `.deb` and `.rpm`, and smoke-tests the built binary (`scripts/linux-smoke.sh`: start, logind inhibitor, status, stop, `kill -9`).
- On a release tag in `Abulkosim/wick` only, the native jobs sign the updater artifacts (Mac archive, Windows installers, AppImage, deb, rpm) with `TAURI_SIGNING_PRIVATE_KEY`, and each Linux job runs the AppImage updater install test against its signed AppImage. The release job then:
  - verifies the signature and its signed version, and generates `latest.json`;
  - writes `SHA256SUMS.txt`;
  - generates an SPDX SBOM;
  - records GitHub build provenance for every installer and package, the archive, `latest.json` and checksums, and attests the SBOM;
  - creates or updates a **draft** release.

Actions are pinned to commit SHAs. Builds have read-only repository access; only the release job can write, plus the OIDC and attestation permissions provenance needs. Pull requests, `main` builds and manual runs build without updater artifacts and receive no signing secrets.

## Publish

1. Download the draft's assets into `.cache/release-vX.Y.Z/` with `gh release download vX.Y.Z -D .cache/release-vX.Y.Z`.
2. Verify `shasum -a 256 -c SHA256SUMS.txt` and the attestations:

   ```sh
   for f in Wick_X.Y.Z_universal.dmg Wick.app.tar.gz Wick_X.Y.Z_x64-setup.exe Wick_X.Y.Z_x64_en-US.msi \
     Wick_X.Y.Z_amd64.AppImage Wick_X.Y.Z_amd64.deb Wick-X.Y.Z-1.x86_64.rpm \
     Wick_X.Y.Z_aarch64.AppImage Wick_X.Y.Z_arm64.deb Wick-X.Y.Z-1.aarch64.rpm latest.json SHA256SUMS.txt; do
     gh attestation verify "$f" --repo Abulkosim/wick
   done
   gh attestation verify Wick_X.Y.Z_universal.dmg --repo Abulkosim/wick --predicate-type https://spdx.dev/Document/v2.3
   ```

3. Run the updater install test against that exact archive (below). Both Linux jobs already ran its AppImage counterpart (`tests/updater_install_linux.rs`) on their signed AppImages; check their logs.
4. Install the DMG's app, launch it, start and stop a session, and check `pmset -g assertions`. For Linux, run the draft AppImage in a desktop session (or the dev container in CONTRIBUTING.md) and check `systemd-inhibit --list`.
5. Record what passed in [verification.md](verification.md), newest release first.
6. Publish the draft and mark it latest: `gh release edit vX.Y.Z --draft=false --latest`. Fix the changelog date to the publish date (UTC) if it differs.

The workflow refuses to replace assets on a published release. Never edit a published release's files or reuse a version; ship a new patch.

## Homebrew

`packaging/homebrew/Casks/wick.rb` is the cask; [Abulkosim/homebrew-tap](https://github.com/Abulkosim/homebrew-tap) serves it as `abulkosim/tap/wick`. Publishing a release runs `.github/workflows/homebrew.yml`, which reads the DMG checksum from the release's `SHA256SUMS.txt`, rewrites `version` and `sha256` with `packaging/homebrew/update-cask.mjs`, and pushes to the tap. Drafts and prereleases are refused, since their assets are not public.

The workflow pushes with a write deploy key on the tap only (secret `HOMEBREW_TAP_DEPLOY_KEY`), gated by the variable `HOMEBREW_TAP_ENABLED=true`. To rotate it, delete the deploy key in the tap's settings, add a new one with write access, and replace the secret. Re-run for a tag with `gh workflow run homebrew.yml -f tag=vX.Y.Z`.

After it runs, check with `brew update && brew livecheck --cask abulkosim/tap/wick` and `brew audit --cask --online abulkosim/tap/wick`. Change stanzas in `packaging/` first, then copy the cask to the tap; `brew style --cask` must be clean.

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

After publishing, `WICK_UPDATE_TARGET=/Applications/Wick.app cargo test --manifest-path src-tauri/Cargo.toml --test public_update -- --ignored --nocapture` updates an installed older Wick from the live endpoint, the same way Settings → Updates does.

`latest.json` is served from the latest published release. Both Mac architectures use the universal archive. Windows entries keep the installer family (NSIS or MSI), with NSIS as the generic fallback. To stop offering a bad release, mark it as a prerelease so the endpoint falls back to the previous release, then ship a fixed patch.
