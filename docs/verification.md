# Verification

What was checked for each release, newest first.

## v0.2.0 — draft 2026-10-23

Tag `v0.2.0` (`b543d78`), CI run [39999738063](https://github.com/Abulkosim/wick/actions/runs/39999738063): frontend and browser checks, macOS universal, Windows x64 and the draft release all passed. `main` run [39998556494](https://github.com/Abulkosim/wick/actions/runs/39998556494) passed first; its browser install stalled once and the new retry recovered it.

- Local suite: 227 browser tests (Chromium + WebKit), 29 unit, 14 release-script, 75 Rust.
- Draft assets: DMG, `Wick.app.tar.gz`, NSIS `.exe`, MSI, three `.sig`, `latest.json`, `SHA256SUMS.txt`, SBOM. `shasum -a 256 -c`: all OK.
- `gh attestation verify`: SLSA provenance for the DMG, archive, both Windows installers, `latest.json`, `SHA256SUMS.txt`; SPDX SBOM attestations verified on the DMG and setup `.exe`.
- `latest.json`: 0.2.0; `darwin-aarch64`/`darwin-x86_64` → `Wick.app.tar.gz`, `windows-x86_64`/`-nsis` → setup `.exe`, `windows-x86_64-msi` → `.msi`.
- `updater_install` against the exact draft archive, `WICK_REQUIRE_UNIVERSAL=1`: tampered bytes and mismatched signed version rejected; 0.2.0 installed, codesign valid, `x86_64 arm64`.
- DMG app on macOS 26.6.2: codesign valid, universal, 0.2.0, registers `wick://`. `wick start 30` → `Wick: awake until 23:40`; `wick status` JSON correct; 0 assertions after `wick stop` and after `kill -9`.
- New features checked live during development: schedule trigger (`awake while on schedule until 21:54`), `wick://start?minutes=1` and `wick://toggle`, power trigger on battery held nothing.
- Pending 👤: in-app update from the installed v0.1.0 after publish; Windows install, tray and `powercfg /requests` in a VM; pause-while-locked with a real lock; tray countdown text on screen.

Checksums:

    bbc6eec7457221f4784b12c2b0eb1d9d631c7bba59b6e3f4149efbd692c3c1bc  latest.json
    3ca90681f56cdb18057d718ae59d42018417eabbd90c16d2497e3c9bb3ff3841  Wick_0.2.0_universal.dmg
    c039b2092b344db1e3e7af32b0605429a1964d61c5af3776228218d999abc72c  Wick_0.2.0_x64_en-US.msi
    9d76da936cb47200b6afae608df453d07de8b232c12fff270b80cf59db261a19  Wick_0.2.0_x64_en-US.msi.sig
    3ff2ff8b208b159e95851f58ea35f4f05ffed1d3ded8d8e73d5a6761c6d6bf6e  Wick_0.2.0_x64-setup.exe
    b2bdd2e54e72d8c85d11911d4f146e6d47ac96671103a0b19830b40f461b2b3f  Wick_0.2.0_x64-setup.exe.sig
    d8652b7a030d7cd39aaab646d69375748eac3624aff1fbcba59fd71e57412646  Wick.app.tar.gz
    be6b4c660eeb67f923776d274519c8d8f8ba50101ae80a33b1f9a8932011a2e4  Wick.app.tar.gz.sig

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
