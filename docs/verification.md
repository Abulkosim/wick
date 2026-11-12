# Verification

What was checked for each release, newest first.

## v0.8.0 — 2026-11-10

Linux arm64 and the Phase 10 sweep: PRs [#24](https://github.com/Abulkosim/wick/pull/24) (arm64 pipeline) and [#27](https://github.com/Abulkosim/wick/pull/27)–[#44](https://github.com/Abulkosim/wick/pull/44), each green before merging. `main` run [42891935312](https://github.com/Abulkosim/wick/actions/runs/42891935312) passed before tagging. Tag `v0.8.0`, CI run [42894982317](https://github.com/Abulkosim/wick/actions/runs/42894982317): frontend, Windows x64, Linux x64 and Linux arm64 passed first time; macOS failed in `hdiutil create` ("Resource busy", a runner flake, the same build passed on every PR) and passed on re-running the failed jobs, then the draft release passed.

- Locally (x86_64 Linux): Rust lib tests 106/106, unit 40/40, release-script tests 23/23, e2e 160/160 in Chromium and the changed specs in WebKit, Clippy clean.
- Tag build, both Linux runners (x86_64 and arm64): `updater_install_linux` on the signed AppImage passed (tampered bytes and an altered archive rejected, then installed); the smoke test passed on the binary and the AppImage.
- Draft: 20/20 checksums OK; provenance verified for all 12 files and the SBOM attestation on the DMG; `latest.json` 0.8.0 with 13 platform entries (`linux-aarch64*` new), every URL on the v0.8.0 tag.
- Mac archive, checked without a Mac: its signature verifies for 0.8.0 with the app's public key; flipped bytes and version 0.8.1 are rejected; `Info.plist` says 0.8.0. The `updater_install` test against it needs macOS.
- The tag build used the repository-level signing secrets: the `release` environment exists and gates the jobs, but the secrets haven't moved there yet.
- Published 2026-11-10 21:09 UTC on the owner's go-ahead, marked latest; `latest.json` (0.8.0, 13 platforms), the DMG, both AppImages, the arm64 deb and rpm and the Windows installer resolve publicly. Homebrew workflow run [42902291210](https://github.com/Abulkosim/wick/actions/runs/42902291210) pushed `wick 0.8.0` with the DMG checksum.
- After publishing, 2026-11-12, on an x86_64 Ubuntu 24.04 server (Xvfb, no tray): the public `Wick_0.8.0_amd64.AppImage` and its deb and rpm matched `SHA256SUMS.txt`; the smoke test passed on the AppImage (logind inhibitor while awake, none after stop or `kill -9`); the deb installed with its dependencies on a clean `ubuntu:24.04` container and the rpm on `fedora:latest` (45), and `wick status` ran from both.
- 2026-11-12: the owner moved the signing secrets into the `release` environment and deleted the repo-level copies; the next tag build is the first to sign with them.
- Pending 👤: `updater_install` on a Mac against `Wick.app.tar.gz`; update the installed app in-app; Linux on real hardware (#22) and on a real ARM board; Windows VM.

Checksums:

    0a1affdba5d3b110f26fa1362e97450605b75ffb1970e796d9a87909caf08efe  latest.json
    a0cfbeabdbdd61bf71f1b54dcaa24c0911348e8f2956ac5758ca594f55e955a7  Wick_0.8.0_aarch64.AppImage
    28a55e2279eb1b16722c77155b9b5c30fd8e80aaa789f9fc397690efdb66a168  Wick_0.8.0_aarch64.AppImage.sig
    9116fe19fc5fd21986217485415082036539205ebff8e630a08998d1313f7105  Wick_0.8.0_amd64.AppImage
    6fd7853af0e1df786be7f8795e6ffa634838a401c3ce66650069932afb658f95  Wick_0.8.0_amd64.AppImage.sig
    d117c0b0ca7936dccf650ce5a3f365f96bb4c2f7f2eb9b108e2cf592d8cb3f78  Wick_0.8.0_amd64.deb
    d6c05dbe52d3c4d5a01694d438a459222aae73df95d774fe29b037a349aa9e1f  Wick_0.8.0_amd64.deb.sig
    b4a3e2b64cf4cad962fd2ca57c4e902598d0ace99f5374e3c07575ead6658305  Wick_0.8.0_arm64.deb
    fa105fa42fa687831520170e5e636b70b287cd9fe85153f0291742532fa015af  Wick_0.8.0_arm64.deb.sig
    4d9bba36561f7fc983524463a5170cc24a0f2d8e497f48ab2116f48ded55341a  Wick_0.8.0_universal.dmg
    3d31da296428c3a4b25285cd06513814d910353a770292efd65603985ba3eaa2  Wick_0.8.0_x64_en-US.msi
    65b41010608e41a67c12b54e8b9eb78f71391fef4fd6d076f523ca16a66bb8ac  Wick_0.8.0_x64_en-US.msi.sig
    0edc02c3442f041b824952dbccda7219123c89e985409f28b9b3c44d02839157  Wick_0.8.0_x64-setup.exe
    9810adc671e4cf1ea8c7db52a0bbeaa82cc2b52d2090407036fd112f9dc3e54b  Wick_0.8.0_x64-setup.exe.sig
    e2de5f0e7881526069f64ea110106040b41c675dd563ab4f6f6da1d573e92235  Wick-0.8.0-1.aarch64.rpm
    cbdd111c02a1f417fbf9654d04afa9670300c804b5f3b08b8a2d640161f9ccc4  Wick-0.8.0-1.aarch64.rpm.sig
    4a6b114012e2121e33feef99f77c1f73c062b675e5bd4271b75ae7d75aadffd0  Wick-0.8.0-1.x86_64.rpm
    c4dc4d09923dfaddd03a285f36aa9973c2dff57784b0a179838e1e46bc15a752  Wick-0.8.0-1.x86_64.rpm.sig
    836291759b8473b14ab54630b21f6a91342fb89038bac4dfb97157cc06012a72  Wick.app.tar.gz
    57366a69282933720e4d83ca6b814e336b7036d08dffe48f95ebc44a48efee7a  Wick.app.tar.gz.sig

## v0.7.1 — 2026-11-05

Fixes from the Codex cross-review of v0.7.0 (PR [#17](https://github.com/Abulkosim/wick/pull/17), release prep [#18](https://github.com/Abulkosim/wick/pull/18)). Tag `v0.7.1`, CI run [41987450083](https://github.com/Abulkosim/wick/actions/runs/41987450083): frontend, macOS universal, Windows x64, Linux x64 and the draft release passed; `main` run [41986297074](https://github.com/Abulkosim/wick/actions/runs/41986297074) passed before tagging.

- New tests: connecting gives up on a socket that never answers; untrusted portals are only asked for idle; `online=2` counts as plugged in; mixed battery units aren't summed.
- Live in the Linux container: the no-tray window stays on Escape, and closing it quits Wick with no inhibitor left; the smoke test passed.
- Tag build's x86_64 runner: `updater_install_linux` on the signed AppImage; the smoke test on the binary and the AppImage.
- Draft: 14/14 checksums OK; provenance for all nine files; SBOM attestations on the DMG and AppImage; `latest.json` 0.7.1 with nine platform entries. `updater_install` against the draft archive passed; DMG universal, codesign valid, 0.7.1.
- Published 2026-11-05 07:38 UTC, marked latest; `latest.json`, AppImage, deb and rpm resolve publicly. Homebrew workflow run [41988626047](https://github.com/Abulkosim/wick/actions/runs/41988626047) pushed `wick 0.7.1` with the DMG checksum.
- Owner's Mac updated 0.7.0 → 0.7.1 from the live endpoint; history kept, the open-ended session restarted.
- Pending 👤: Linux on real hardware with GNOME and KDE, Wayland, Xfce with a real screensaver; an in-app Linux update (first possible from 0.7.1 to the next release); Windows VM.

Checksums:

    d708d7456763dc7ec4f4f30640b5e1d3220e35bf6f5744909f7d6b2295098327  latest.json
    2b9797b7d340a1b74a682153b80a4751df8ce6b12f48f18ad08436e71d281bba  Wick_0.7.1_amd64.AppImage
    58ca3bdeddd0d0c0cd8b520bc0715394ef44300afb2653fb2fc38097c2077e9e  Wick_0.7.1_amd64.AppImage.sig
    4a526a26b1501eed83ce611cc417b8747cc8d289a6a58a232fe0b10a7a5c21c6  Wick_0.7.1_amd64.deb
    8e417e71f97af02249579136a695778433e09d0283b287b999cea0db43ad7920  Wick_0.7.1_amd64.deb.sig
    2026b64717aebf863587b4c8ed5af0c742f413e4f2b5c26d6225e70586e60412  Wick_0.7.1_universal.dmg
    920684c8d101dd587e6352d7003d856d0eb87328c2499ecad1c11a42092176a5  Wick_0.7.1_x64_en-US.msi
    ce6c79d95c04c35d6021d7988715a753120ecd828abb561ffdf4d39b3645370e  Wick_0.7.1_x64_en-US.msi.sig
    1e3599aa372cead5ed74c0b7666b2c52d4bbd18163d8ef957b8f3e52abe5b699  Wick_0.7.1_x64-setup.exe
    8164e6332b427ce1b6b2f7caa3957e94dd806fc6822c0f94f6e81d9c06027fcf  Wick_0.7.1_x64-setup.exe.sig
    492daaecb66c47995bf04102d5a90c2f80fa4d731f95276a020bce81cd596070  Wick-0.7.1-1.x86_64.rpm
    070f52e6067e6daf96fa1702ad89e402dac6ed4d1c62611bab88bcd26975ac13  Wick-0.7.1-1.x86_64.rpm.sig
    1e5601cee925ee72163818cbefd57523bdb75bf7c31a84603bc79e061943cfa4  Wick.app.tar.gz
    e55b5c95e3597d6263ff2f02a3f312b3c735b2717b5003e4fae96e1e4a5f52d0  Wick.app.tar.gz.sig

## v0.7.0 — 2026-11-05

The Linux release. Tag `v0.7.0`, CI run [41979379718](https://github.com/Abulkosim/wick/actions/runs/41979379718): frontend, macOS universal, Windows x64, Linux x64 and the draft release passed. Built through PRs [#10](https://github.com/Abulkosim/wick/pull/10) (core), [#11](https://github.com/Abulkosim/wick/pull/11) (shell), [#14](https://github.com/Abulkosim/wick/pull/14) (packages and pipeline) and [#15](https://github.com/Abulkosim/wick/pull/15) (release prep), each green before merging; `main` run [41978694191](https://github.com/Abulkosim/wick/actions/runs/41978694191) passed before tagging.

- Local suite (`scripts/check-all.sh`) on macOS; Linux clippy, tests and builds in an Ubuntu 22.04 container with real systemd, logind, polkit and D-Bus (`scripts/linux/Dockerfile`).
- Linux, in the container: a real click on the tray icon under an Xfce panel opens the popover at the icon; Off, Awake and Settings screenshotted; no panel → titled window that survives focus loss; xfce4-power-manager reports Wick's inhibitor and drops it on exit; `systemd-inhibit --list` shows `Wick` while awake and nothing after stop or `kill -9`; installing the deb makes `xdg-open wick://start?minutes=1` and `wick://stop` work; a signed bundle build (throwaway key) passed the AppImage updater test.
- Linux, on the x86_64 runner of the tag build: `updater_install_linux` on the signed AppImage (tampered bytes and mismatched signed version rejected, then installed and executable); the smoke test on the binary and on the AppImage (the runner user gets logind's idle lock; polkit refuses the sleep lock there, as designed).
- Draft: 14/14 checksums OK; provenance for the DMG, archive, both Windows installers, AppImage, deb, rpm, `latest.json`, `SHA256SUMS.txt`; SBOM attestations on the DMG and AppImage; `latest.json` 0.7.0 with nine platform entries including `linux-x86_64`, `-appimage`, `-deb`, `-rpm`. Released deb: amd64, `Exec=wick %u`, `MimeType=x-scheme-handler/wick;`; rpm x86_64.
- Mac: `updater_install` against the draft archive passed; DMG universal, codesign valid, 0.7.0.
- Published 2026-11-05 06:51 UTC, marked latest; `latest.json` and the AppImage resolve publicly. Homebrew workflow run [41983222960](https://github.com/Abulkosim/wick/actions/runs/41983222960) pushed `wick 0.7.0` with the DMG checksum.
- Owner's Mac updated 0.6.0 → 0.7.0 from the live endpoint; history kept, the open-ended session restarted.
- Pending 👤: Linux on real hardware with GNOME (AppIndicator extension) and KDE, Wayland, and an in-app update from 0.7.0 to the next release; Windows VM. Codex cross-review next (plan 8.5).

Checksums:

    ddc03ac3b674197a3a7074bdd09ccd842001023673fbd9d10f95f468b1197327  latest.json
    601d36264c766aac2c5f675eaf0ea1fee31f3eb337d7064904e207ae1fa0e5d3  Wick_0.7.0_amd64.AppImage
    caccbca0541ef221f0062adaf211ff5e9cf806527771a51cde0956df7a0cfe8d  Wick_0.7.0_amd64.AppImage.sig
    869569db78d0549a7a629e6cd4acb4444f8599356d29e292e9e991c911352e7b  Wick_0.7.0_amd64.deb
    be2a47856dc605434b6b486c428cea4bb693e8a04aa95818073f752f073f1821  Wick_0.7.0_amd64.deb.sig
    10ffc976fad43a570276179987a43b6c771b7a2cbf5861ce14876c8780a20b2e  Wick_0.7.0_universal.dmg
    029be63533f04a4c47ec8e4ff7c29d3cab4b3519901b97b2ecfb2e47b644427d  Wick_0.7.0_x64_en-US.msi
    ea383538b3f0a82794a9e7e6d6bf5cf78a83f0f488cfd7d2396ab08e769066bd  Wick_0.7.0_x64_en-US.msi.sig
    55a22507deb37a6067d21419917ce11401e181da069dec77ec7c0a13073fb780  Wick_0.7.0_x64-setup.exe
    1aebe812213a1abed371dec9985d4e693684484d15dbed56da62f1b2ce4580dd  Wick_0.7.0_x64-setup.exe.sig
    18e3ac30c3538ba0599084e82077d50b110dfeab9dc270ac095a65579cac4726  Wick-0.7.0-1.x86_64.rpm
    5b59f0fc1e19c7359d6eeca078b7f1cc4d27b76e5facfaa4f5c9b268873d04ef  Wick-0.7.0-1.x86_64.rpm.sig
    dc971c30a4632cb8c781a3b3f290f2cb3006e01b9523d1d44f38157232e46a21  Wick.app.tar.gz
    1eb19e762184ba15f8a85e8c99111ab686045c99ed030b540f348278776d8073  Wick.app.tar.gz.sig

## v0.6.0 — 2026-10-31

Tag `v0.6.0`, CI run [41223052164](https://github.com/Abulkosim/wick/actions/runs/41223052164): frontend, macOS universal, Windows x64 and the draft release passed. PR [#9](https://github.com/Abulkosim/wick/pull/9) checks passed before the merge; `main` run [41222421535](https://github.com/Abulkosim/wick/actions/runs/41222421535) passed before tagging.

- Local suite (`scripts/check-all.sh`): all passed, including a runtime test that a trigger ending notifies only when nothing else holds the Mac (not after Stop, during a session, or when the battery guard lets go) and an e2e test of the new setting.
- Draft: 8/8 checksums OK; provenance for DMG, archive, both Windows installers, `latest.json`, `SHA256SUMS.txt`; SBOM attestation on the DMG; `latest.json` 0.6.0 with all five platform entries. DMG app universal, codesign valid, 0.6.0.
- `updater_install` against the draft archive: tampered and wrong-version rejected; 0.6.0 universal installed, codesign valid.
- Published 2026-10-31 17:15 UTC, marked latest. Homebrew workflow run [41223655991](https://github.com/Abulkosim/wick/actions/runs/41223655991) pushed `wick 0.6.0` with the DMG checksum.
- Owner's Mac updated 0.5.0 → 0.6.0 from the live endpoint (`tests/public_update.rs`), codesign valid. Wick was not running, so it was left closed.
- Pending 👤: see a trigger notification banner on screen (turn on Settings → Notifications → When a trigger ends); Raycast store submission; Windows VM.

Checksums:

    a085ea7a7632deb268564cc65a0c5aebfe1b1fae50064139615b10bb574d474f  latest.json
    a2224d9791db60b218025df2a75930966d0929e53d44e8e89733581bcd1a725b  Wick_0.6.0_universal.dmg
    f55ba7b37a6870ef339b4e96bb688072f289b90ecd3e3e8b9e846382daaf635f  Wick_0.6.0_x64_en-US.msi
    11e6cac05c68b79d336f8b16274f8ab3873562de2dbad3ab921b3c6160e85024  Wick_0.6.0_x64_en-US.msi.sig
    1056069f7d1912d881c71f8f3beaa1a99df8a669f132c3da979622e044bc99ec  Wick_0.6.0_x64-setup.exe
    436bab186868dccf15fff8e7619eb43750f97a7592325e7a2852390344187a88  Wick_0.6.0_x64-setup.exe.sig
    b75672b7a8a6d7ff690d3d9c8bf32ff07981c6a91ccf3ddf3f052c1f632bb91e  Wick.app.tar.gz
    e4cbff5dddebbc0f08af79c7154cb0ea1b4679d43e5cef5a0854f963bf0ea065  Wick.app.tar.gz.sig

## v0.5.0 — 2026-10-27

Tag `v0.5.0`, CI run [40483032343](https://github.com/Abulkosim/wick/actions/runs/40483032343): macOS universal, Windows x64 and the draft release passed; the browser job's first attempt stalled on Ubuntu's apt mirror (dpkg lock left behind) and passed on re-run. PR [#6](https://github.com/Abulkosim/wick/pull/6) checks passed before the merge; `main` run [](https://github.com/Abulkosim/wick/actions/runs/) passed before tagging.

- Local suite (`scripts/check-all.sh`): all passed, now including the Raycast extension's tests and `ray build`.
- Activity triggers: pure tests for rates, 32-bit wraps, resets, the 2-minute quiet period and validation. Live on this Mac: a 500 KB/s download made a 100 KB/s network trigger busy; spinning every core made a 30% CPU trigger busy (now a test). On the Windows runner, the `MIB_IF_ROW2` layout test and live `GetIfTable2`/`GetSystemTimes` reads passed.
- Found while testing: macOS hands apps 32-bit, KB-rounded interface byte counters (`netstat` sees 64-bit), so drops after a 32-bit value count as wraps.
- Draft: 8/8 checksums OK; provenance for DMG, archive, both Windows installers, `latest.json`, `SHA256SUMS.txt`; SBOM attestation on the DMG; `latest.json` 0.5.0 with all five platform entries. DMG app universal, codesign valid, 0.5.0.
- `updater_install` against the draft archive: tampered and wrong-version rejected; 0.5.0 universal installed, codesign valid.
- Published 2026-10-27 10:01 UTC, marked latest. Homebrew workflow run [40485227070](https://github.com/Abulkosim/wick/actions/runs/40485227070) pushed `wick 0.5.0` with the DMG checksum.
- Owner's Mac updated 0.4.0 → 0.5.0 from the live endpoint (`tests/public_update.rs`); history kept (491 minutes today), indefinite session restarted with display on.
- Raycast: `ray build`, ESLint and Prettier pass; `ray lint`'s author check got HTTP 500 from Raycast's user API, even for known users.
- Pending 👤: confirm the Raycast `author`, try the extension (`npm run dev` in `raycast/`) and publish it; Windows VM.

Checksums:

    a0680cb7b3eca636a147499f739238f6e5228c8eff27d811a6626ebc92e5c12c  latest.json
    33d2a2af35f9de1b7c69a721a2858b631ec5b7f0420916a8e8cf9ab2503c5724  Wick_0.5.0_universal.dmg
    fa44161145025995d5f73b2c6839dfd96a30b0d33adacdd212455d1e8a187718  Wick_0.5.0_x64_en-US.msi
    cafcce07b76e2d1f7bb4e21888134a03d82cc8079aee31719c942df7aa69bdc7  Wick_0.5.0_x64_en-US.msi.sig
    8ba5b45ce0646c4774ceabe7920b2f5ab82d93bdc5406b14fa1f3bb168a2ca59  Wick_0.5.0_x64-setup.exe
    48a3ab481277773dab53684244b01f1529d5752041898df5760de740f0f53e4c  Wick_0.5.0_x64-setup.exe.sig
    b40174e0e5d008d8bb78ca20454b887c2f1499a9f5635e5382820ec96f2e59ed  Wick.app.tar.gz
    cbb90091d695c7de8242dd2abbbcecab2c714d4f601beb3c0ddc97a827a0adf6  Wick.app.tar.gz.sig

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
