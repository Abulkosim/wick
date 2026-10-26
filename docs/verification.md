# Verification

What was checked for each release, newest first.

## v0.4.0 — 2026-10-26

Tag `v0.4.0`, CI run [40265194461](https://github.com/Abulkosim/wick/actions/runs/40265194461): frontend, macOS universal, Windows x64 and the draft release passed. PR [#4](https://github.com/Abulkosim/wick/pull/4) checks passed before the merge; `main` run [40264596436](https://github.com/Abulkosim/wick/actions/runs/40264596436) passed before tagging.

- Local suite (`scripts/check-all.sh`): all passed, including new tests for history (midnight split, 30-day prune, damaged file set aside, sleep excluded by the monotonic clock, save and reload), the history card, Settings → History and the cask updater.
- Live on a local release build: `wick start 2` wrote `history.json` with 60 039 ms after a minute (save while awake) and 119 996 ms once the session ended; `wick status` showed `awakeTodayMinutes`; no assertion left after quit.
- Draft: 8/8 checksums OK; provenance for DMG, archive, both Windows installers, `latest.json`, `SHA256SUMS.txt`; SBOM attestation on the DMG; `latest.json` 0.4.0 with all five platform entries. DMG app universal, codesign valid, 0.4.0.
- `updater_install` against the draft archive: tampered and wrong-version rejected; 0.4.0 universal installed, codesign valid.
- Published 2026-10-26 03:53 UTC, marked latest. Homebrew workflow run [40266282448](https://github.com/Abulkosim/wick/actions/runs/40266282448) pushed `wick 0.4.0` to the tap with the DMG checksum; `brew livecheck` 0.4.0, `brew audit --online` passed. Before release, a real `brew install --cask` (0.3.0) into a scratch appdir linked `wick`, ran `wick status` once quarantine was cleared, and uninstalled cleanly.
- Owner's Mac updated 0.3.0 → 0.4.0 from the live endpoint (`tests/public_update.rs`); its indefinite session restarted with display on; `wick status` answers with `awakeTodayMinutes`.
- Pending 👤: the history card on screen after a day of use; Windows VM.

Checksums:

    36058d8a8b41779ef8d3cac66c8b37fb5e39383583f6cbcca09550545b9e46a6  latest.json
    569549a2c8ba7208bde4b147e1f16d694d7fe06fda9da18771aa72a9ecb2b828  Wick_0.4.0_universal.dmg
    f961906c05e52a8f54e5c2856119c062ee0ba6273ace38c5eff07399295f2de3  Wick_0.4.0_x64_en-US.msi
    0b37e383bb4b406bb7845d2a4861e8a1293775c4b8687525a9111f94e659b439  Wick_0.4.0_x64_en-US.msi.sig
    423500852dcb87b2e2f31199384f73f2080f46c503624e1872747bcb469b4eab  Wick_0.4.0_x64-setup.exe
    41521d1f552d13e41d3c41904290ece394d30efc35c34223ea986950619ebcf3  Wick_0.4.0_x64-setup.exe.sig
    e118c951ddb68ba24b636fc615ffe196392a59894894560ec448c3ae60e58cce  Wick.app.tar.gz
    ba2738c0f531f53b524f797b4427383ff70d0654160bf8ea3ed6ec3a8e1ca26f  Wick.app.tar.gz.sig

## v0.3.0 — 2026-10-25

Tag `v0.3.0`, CI run [40154976385](https://github.com/Abulkosim/wick/actions/runs/40154976385): frontend, macOS universal, Windows x64 and the draft release passed. `main` run [40154543608](https://github.com/Abulkosim/wick/actions/runs/40154543608) passed first.

- Local suite (`scripts/check-all.sh`): all passed, including new tests for the commit order, pause handling, tooltips, resume marker, DST and popover keys.
- Draft: 8/8 checksums OK; provenance for DMG, archive, both Windows installers, `latest.json`, `SHA256SUMS.txt`; SBOM attestation on the DMG; `latest.json` 0.3.0 with all five platform entries.
- `updater_install` against the draft archive: tampered and wrong-version rejected; 0.3.0 universal installed, codesign valid.
- Live on a debug build before tagging: a timed session expired on time; a uniquely named process trigger held and released; `kill -9` does not resume a session.
- Published 2026-10-25 02:42 UTC, marked latest. Owner's Mac updated 0.2.0 → 0.3.0 from the live endpoint (`tests/public_update.rs`); settings kept, `wick status` answers.
- Pending 👤: tray right-click menu and popover keys by hand; Windows VM.

Checksums:

    1b71536ccdbb7edb5470b518a3934cbe32ea50a25f0f42685f70cd8048811f34  latest.json
    d7e2bb5e5781f036abfdbbe1093e61d5247af3c8dd3b852ec0418531ba83abc0  Wick_0.3.0_universal.dmg
    034e469a340718db4923b880c4cb9117fb4a3b45d3c0d481bfbccbca8f9b6cb4  Wick_0.3.0_x64_en-US.msi
    5a2bfe43f0cb2bc218b496f930419d04e6ed420141c25355d0784543d38e153c  Wick_0.3.0_x64_en-US.msi.sig
    732fc87cf97f669ebc9effeba45010ba02024f301e3fc416733da0a3047f5f77  Wick_0.3.0_x64-setup.exe
    7708c992bbbfdf457a25732b84d6bec03285f37001501094f195f3f8721afaad  Wick_0.3.0_x64-setup.exe.sig
    20554ab27940882b8baddc03bfeb79fcd9189ea14771b66e2b4d95f080dcd82f  Wick.app.tar.gz
    bf2bbf01fa50a567c1c68c8843e83ebb41fb4bab5a86dee66c29b3ec626bebfd  Wick.app.tar.gz.sig

## v0.2.0 — 2026-10-23

Tag `v0.2.0` (`b543d78`), CI run [39999738063](https://github.com/Abulkosim/wick/actions/runs/39999738063): frontend and browser checks, macOS universal, Windows x64 and the draft release all passed. `main` run [39998556494](https://github.com/Abulkosim/wick/actions/runs/39998556494) passed first; its browser install stalled once and the new retry recovered it.

- Local suite: 227 browser tests (Chromium + WebKit), 29 unit, 14 release-script, 75 Rust.
- Draft assets: DMG, `Wick.app.tar.gz`, NSIS `.exe`, MSI, three `.sig`, `latest.json`, `SHA256SUMS.txt`, SBOM. `shasum -a 256 -c`: all OK.
- `gh attestation verify`: SLSA provenance for the DMG, archive, both Windows installers, `latest.json`, `SHA256SUMS.txt`; SPDX SBOM attestations verified on the DMG and setup `.exe`.
- `latest.json`: 0.2.0; `darwin-aarch64`/`darwin-x86_64` → `Wick.app.tar.gz`, `windows-x86_64`/`-nsis` → setup `.exe`, `windows-x86_64-msi` → `.msi`.
- `updater_install` against the exact draft archive, `WICK_REQUIRE_UNIVERSAL=1`: tampered bytes and mismatched signed version rejected; 0.2.0 installed, codesign valid, `x86_64 arm64`.
- DMG app on macOS 26.6.2: codesign valid, universal, 0.2.0, registers `wick://`. `wick start 30` → `Wick: awake until 23:40`; `wick status` JSON correct; 0 assertions after `wick stop` and after `kill -9`.
- New features checked live during development: schedule trigger (`awake while on schedule until 21:54`), `wick://start?minutes=1` and `wick://toggle`, power trigger on battery held nothing.
- Published 2026-10-23 18:32 UTC and marked latest; `latest.json` (0.2.0), DMG, both Windows installers and the archive resolve publicly.
- Owner's Mac updated 0.1.0 → 0.2.0 through the updater plugin with the public endpoint and key (`tests/public_update.rs`, the same path as Settings → Updates minus the click): installed universal, codesign valid, settings kept, `wick status` answers.
- Pending 👤: Windows install, tray and `powercfg /requests` in a VM; pause-while-locked with a real lock; tray countdown text on screen.

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
