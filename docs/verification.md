# Verification

What was checked for each release, newest first.

## v0.1.0 — 2026-10-23

Tag `v0.1.0` (`d1c08f4`), CI run [39981899989](https://github.com/Abulkosim/wick/actions/runs/39981899989): frontend and browser checks, macOS universal build and draft release all passed. `main` run [39980619248](https://github.com/Abulkosim/wick/actions/runs/39980619248) passed before tagging.

- Local suite: 205 browser tests (Chromium + WebKit), 26 unit, 14 release-script, 62 Rust (including live IOKit assertion, process scan, caffeinate and ioreg lid tests).
- Draft assets: DMG, `Wick.app.tar.gz`, `.sig`, `latest.json`, `SHA256SUMS.txt`, SBOM. `shasum -a 256 -c SHA256SUMS.txt`: all OK.
- `gh attestation verify`: SLSA provenance for the DMG, archive, `latest.json` and `SHA256SUMS.txt`; SPDX SBOM attestation for the DMG.
- `latest.json`: version 0.1.0, `darwin-aarch64` and `darwin-x86_64` → `https://github.com/Abulkosim/wick/releases/download/v0.1.0/Wick.app.tar.gz`.
- `updater_install` against the exact draft archive with `WICK_REQUIRE_UNIVERSAL=1`: tampered bytes and mismatched signed version rejected; 0.1.0 installed, codesign valid, `x86_64 arm64`.
- DMG app on macOS 26.6.2 (Apple Silicon): codesign valid, universal, version 0.1.0. `wick start 30` → `Wick: awake until 21:33`; 0 assertions after `wick stop` and after `kill -9`.
- Published 2026-10-23 16:04 UTC and marked latest; `latest.json` and the DMG resolve publicly.
- Pending 👤: in-app update from a later release; tray icon and notification banner seen on the owner's screen (no screen capture here).

Checksums:

    95090a6166bc580daa7014b44f672cd8ad51ac6849c0940b13081fff222c1f2a  latest.json
    cdec47df15a36ff4373e4af3dfafa9d44d1eb490b91cff222b7c0a60d219566a  Wick_0.1.0_universal.dmg
    123f66a3bd82432e794d61e945242d24d017a16c5040280dde03a8e4695bbe46  Wick.app.tar.gz
    9970d88b16494c223c3ba70a9196d2ba417ca8e2e75513d3873b6c08ef697e4d  Wick.app.tar.gz.sig

## Native checks (every release)

- `wick start 30` → `pmset -g assertions` shows `Wick: awake until HH:MM`; `wick stop` removes it.
- Quit and `kill -9` while awake leave no `Wick:` assertion.
- A process trigger holds while the process runs, Stop pauses it, and it releases on exit.
- Popover opens at full height in light and dark, with the theme following Wick's setting.
