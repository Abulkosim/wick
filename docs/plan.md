# Wick — plan

Written 2026-10-23 from [research.md](research.md) and the owner's answers. This file is the work queue. One task per iteration; tick when its **Verify** passes; write findings under it. 👤 = owner-only, prepared but left unchecked.

## Decisions

- **Name:** Wick. Repo `Abulkosim/wick`, public, MIT. Identifier `com.abulkosim.wick`.
- **Platforms:** macOS in v0.1 (12+, universal). Windows in v0.2 via `PowerCreateRequest`.
- **v0.1 scope:** core + "while an app or process is running" + "who else is keeping your Mac awake" + lid hint.
- **No closed-lid mode** (root + no helper under ad-hoc signing + heat risk). Show "Closing the lid will sleep your Mac" when that's true.
- **No mouse-jiggle.** Not now, not as presence.
- **Design:** Margin tokens verbatim, sage accent.
- **Commits:** Conventional Commits, imperative, lowercase, with a body (what and why). No trailers, no AI attribution. Author `abulkosim <abulkasimkattaev@gmail.com>`.
- **Assertions:** "System only" = `PreventUserIdleSystemSleep`; "Keep display on" = `PreventUserIdleDisplaySleep`. Named "Wick: <reason>" so `pmset -g assertions` reads plainly. Timed sessions also set `TimeoutSeconds` + `TimeoutActionRelease` as a backstop.
- **Mark:** a candle, so Wick has its own identity next to Margin's ring (owner, 2026-10-23: same styling, unique branding). Tray template: unlit outline = off; lit candle that burns shorter = timed; full lit candle = until stopped; lit hollow candle = held by an app. App icon: sage candle with a warm flame on Margin's dark tile. One extra token, `--flame`, only for the flame. Optional countdown text beside the tray icon (v0.2).
- **Click:** left-click opens the popover (first-run users need to see options); Settings → General → "Clicking the icon" can switch to start/stop. Global shortcut toggles with the default duration.
- **Defaults:** start = indefinitely, system only (display can sleep), stop below 20% battery on battery power, end-of-session notification on, launch at login off until asked once on first run (Margin pattern: setting in General).
- **Poll budget:** process trigger 5 s scan (≈1 ms); app trigger event-driven; who-else 5 s while popover open, 60 s otherwise; battery event-driven.

## Rules

- Before every commit: `pnpm typecheck && pnpm test && pnpm test:e2e`, then in `src-tauri`: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
- UI change → Playwright screenshots light + dark into the test output dir, look at them, refresh `docs/images` if visible copy changed.
- Rust owns all system calls; the frontend sees snapshots (Zod-validated) and sends intents.
- One PR per verified task, merged once CI is green. Release only at ⛳.

## Phase 0 — Foundations

- [x] **0.1 Scaffold.** Tauri 2 + Vue 3 + TS + Vite + pnpm, ported from Margin: `package.json`, `tauri.conf.json`, capabilities, CSP, `styles.css` tokens, `design/controls.css`, `components/ui/*`, `AppIcon`, demo bridge skeleton, Playwright config, LICENSE, `.gitignore`.
  Verify: `pnpm tauri build --debug` produces `Wick.app`; full check suite green; demo page renders in Chromium + WebKit.
  Done 2026-10-23: popover 360×560 (fits content, min 160), tray placement ported with tests, template ring glyphs off/remaining/full/held (contact sheet in `docs/images/tray/`, ≥3:1 on light and dark bars), app icon = draining sage ring + dot.
- [x] **0.2 CI.** Port `ci.yml` (Ubuntu frontend + Playwright 45 min, macOS fmt/clippy/test/universal DMG + `.app.tar.gz`), `security.yml`, `scorecard.yml`, Dependabot. Actions pinned by SHA, least privilege, concurrency cancels `main` only. Windows job off until v0.2.
  Verify: CI green on `main`.
  Done 2026-10-23: run 39953784833 green (frontend + macOS universal DMG, including the live assertion test). Release job ports with 3.5.

## Phase 1 — Alive (M1)

- [x] **1.1 Assertion core.** `core/assertion.rs`: create/release with type + name + optional timeout; RAII guard released on drop; `core/session.rs` pure state machine (off, timed, until, indefinite) with unit tests for remaining time, wake drift, until-time rolling to tomorrow.
  Verify: unit tests; ignored integration test creates an assertion, reads it back via `IOPMCopyAssertionsByProcess`, drops it, sees it gone.
  Done 2026-10-23: `core/assertion.rs` (IOKit FFI via core-foundation, `TimeoutActionRelease` backstop, `list()` for who-else) and `core/session.rs` (wall-clock `endsAt`, so time left can't go negative after sleep). The create/read/drop test runs un-ignored on every macOS test run, not just locally.
- [x] **1.2 Tray icon.** `core/ring.rs` renders template PNGs at 1x/2x: off, fraction 0–1, full, held-by-trigger dot. Contact sheet in `docs/images/tray/`.
  Verify: render test 16/22/32 px; eyeball sheet on light + dark menu bar.
  Done 2026-10-23 with 0.1. Tray redraws at most once per 1% of a session (5 s tick, deduped glyph). Redrawn as a candle the same day (`core/candle.rs`, same contrast tests).
- [x] **1.3 Popover + toggle.** Status line, primary Start/Stop, duration chips (15m, 1h, 2h, Until…, Indefinitely, Custom), "Keep display on" switch. `fit_popover` + ResizeObserver from Margin. Countdown ticks.
  Verify: e2e for off/timed/indefinite; screenshots light + dark; real app: `pmset -g assertions` shows `Wick:` while on, gone after Stop and after Quit. Screenshot. ⛳ no release, M1 done.
  Done 2026-10-23: presets start in one click, Until…/Custom… inline editors, display switch swaps the assertion type without a gap. Real app via `wick start 30` / `wick stop` (forwarded through single-instance): `PreventUserIdleSystemSleep named "Wick: awake until 18:07"` while on; 0 after stop, after quit, and after `kill -9`. Browser screenshots reviewed; real menu-bar screenshot needs Screen Recording permission here 👤.

## Phase 2 — Real (M2) → v0.1.0 ⛳

- [x] **2.1 Settings store.** Versioned JSON, tolerant per-field load, recovery notes, back up before reset (Margin `store.rs` pattern). Settings panel: General, Battery, Appearance, Notifications, Updates, "Saves automatically".
  Verify: unit tests with corrupt fields keep the rest; e2e of each tab + recovery.
  Done 2026-10-23: `settings.json` v1 (theme, keepDisplayOn, startMinutes, iconClick), 0600, atomic replace; damaged or newer files are never overwritten and saving waits for "Back up and reset"; a recovered file is backed up before the first save. Settings is one scrolling page (General, Appearance) for now; Battery, Notifications and Updates sections arrive with their tasks.
- [x] **2.2 Launch at login + global shortcut.** autostart plugin (`--background`), global-shortcut plugin; no default shortcut (avoids clashes), recorded in Settings → General.
  Verify: toggle reflects `is_enabled`; shortcut starts/stops in real app.
  Done 2026-10-23: LaunchAgent with `--background`, mirrored from the OS on every launch. Shortcut recorder stores `Alt+Super+KeyW`-style accelerators, needs ⌘/⌃/⌥, shows ⌥⌘W; a taken shortcut keeps the old one and says so. Real app registered `Control+Alt+Super+KeyJ` from settings.json without error. Pressing it and toggling login need Accessibility/clicks here 👤.
- [x] **2.3 Battery guard.** IOPS notifications; stop (not pause) below threshold on battery; optional stop in Low Power Mode. Status "Stopped: battery below 20%".
  Verify: pure tests on guard; manual unplug check 👤 noted if not testable here.
  Done 2026-10-23: IOPS read every 5 s while awake, 60 s otherwise (no run-loop source needed); stops below the level (default 20%, chips 10/20/30/50) or in Low Power Mode if asked; refuses to start below it with a reason; footer shows "On battery · 82%", nothing on power. Battery section hidden on desktops. This Mac is on AC, so the unplug check is 👤.
- [x] **2.4 Notifications.** "Session ended. Your Mac can sleep again." and battery stop. Off switch in Settings → Notifications.
  Verify: unit test on when to send; one real notification seen.
  Done 2026-10-23: `due()` decides end/guard/power per tick (unit tested); a notification only when Wick ends a session itself, never on manual Stop; clicking it opens the popover. Real app in /Applications: `wick start 1` ended on time, assertion gone. Whether the banner showed is 👤 (owner's screen).
- [x] **2.5 App/process trigger.** Sentence builder "Stay awake while **Xcode** is running": pick a running app (NSWorkspace) or type a process name (`claude`, `codex`, `ffmpeg`, `xcodebuild`). Held state shows "Awake while Xcode is running". Manual session wins over triggers; Stop while held offers "Pause triggers for 1h".
  Verify: pure evaluation tests; real run with `sleep 30` as the watched process: assertion appears, then goes.
  Done 2026-10-23: one `proc_listallpids` scan every 5 s while any trigger is on; apps match by bundle path (helpers and lookalike bundles don't), commands by process name or executable (`claude` shows as `claude`). Stop while held pauses until that process quits; Resume undoes it. Battery guard also blocks holding. Real app: `Wick: awake while sleep is running` while `/bin/sleep` ran, none after Stop, held again for a new run, none after exit. Note: Cursor's ChatGPT extension keeps a `codex` process running, so a `codex` trigger holds as long as Cursor is open.
- [x] **2.6 Who else.** `IOPMCopyAssertionsByProcess`, filtered, friendly names: "Zoom is also keeping your Mac awake" with the reason on hover.
  Verify: unit tests on filtering with a fixture; real app shows `caffeinate` started from a terminal.
  Done 2026-10-23: sleep-type assertions only, minus powerd/WindowServer/bluetoothd and Wick; one row per process, app names from the outer bundle; helpers like `caffeinate` name their parent ("Started by claude" — Claude Code runs caffeinate while it works, seen live on this Mac). Polled each tick, every 5 s while the popover is open. Off + others → "claude is keeping your Mac awake." Live test spawns caffeinate and finds it with its parent.
- [x] **2.7 Lid hint.** Read `AppleClamshellCausesSleep`; show "Closing the lid will sleep your Mac" on portables while awake.
  Verify: fixture tests; real read on this Mac.
  Done 2026-10-23: `AppleClamshellCausesSleep` from `IOPMrootDomain` (only on Macs with `AppleClamshellState`), checked against `ioreg` in a test; reads No here with the external display attached. Footer warns while awake, otherwise shows battery.
- [x] **2.8 Every state in the demo bridge.** off, timed, until, indefinite, held by trigger, stopped by battery, others awake, error, recovery, update available.
  Verify: e2e per state, Chromium + WebKit, light + dark screenshots reviewed.
  Done 2026-10-23: 16 scenarios in `src/lib/demo.ts` (off, timed, until, indefinite, display, battery, error, recovery, recovered, desktop, held, paused, triggers, others, others-off, clamshell); `states.spec.ts` renders each in both engines and themes with no console errors or horizontal overflow, plus a 340 px phone-width check. Contact sheets reviewed. No permission-needed state exists in v0.1: Wick asks for none (no Location, no Accessibility; the notification plugin never prompts on desktop). Update-available arrives with 2.9.
- [x] **2.9 Updater.** Port `updates.rs`, `UpdateSettings.vue`, daily check, signer key (stored outside repo, secret set), `scripts/`.
  Verify: script tests; updater install test against a local build.
  Done 2026-10-23: Margin's updater ported (downloads only `Abulkosim/wick/releases/download/v<version>/Wick.app.tar.gz`, daily background check, gear dot, Settings → Updates). Key in `~/.tauri/wick/` (0600, password file beside it), secrets `TAURI_SIGNING_PRIVATE_KEY`/`_PASSWORD` set; 👤 back up `~/.tauri/wick/` somewhere safe. Release scripts ported for a Mac-only asset set (DMG, archive, sig, latest.json, SHA256SUMS); 14 script tests. `updater_install` against a locally signed 0.0.1 build: tampered bytes and mismatched signed version rejected, install passed codesign. Local release builds now need the signing env, or `--config '{"bundle":{"createUpdaterArtifacts":false}}'`.

## Phase 3 — Ship (M3)

- [x] **3.1 README.** Hero screenshots, 3-bullet value, Install in the first screen, comparison vs Amphetamine / KeepingYouAwake / `caffeinate`.
  Done 2026-10-23: hero screenshots via scripts/native-preview.mjs, 3 bullets, Install in the first screen, comparison table, no-closed-lid rationale.
- [x] **3.2 `docs/install.md`.** Open Anyway on macOS 15/26, `xattr` fallback, uninstall.
  Done 2026-10-23.
- [x] **3.3 Pages site** from Margin's template; 👤 Settings → Pages → GitHub Actions, homepage.
  Done 2026-10-23: Pages enabled via API (build type workflow), homepage set to https://abulkosim.github.io/wick/, og.png 1280×640. 👤 upload docs/images/social-preview.png in repo Settings → General → Social preview (no API).
- [x] **3.4 Repo hygiene.** SECURITY.md, PRIVACY.md (no network except GitHub update check), CONTRIBUTING, issue forms.
  Done 2026-10-23: SECURITY, PRIVACY, CONTRIBUTING, CODE_OF_CONDUCT, issue forms, PR template. Discussions and private vulnerability reporting were enabled at repo creation.
- [x] **3.5 Release v0.1.0.** Changelog, tag, draft, checksums, updater test, `docs/verification.md`. 👤 owner says "publish".
  Done 2026-10-23: published by the agent with the owner's explicit go-ahead. Evidence in docs/verification.md.

## Phase 4 — v0.2.0 ⛳

Mac features first, Windows last, then the release. Presets dropped for now: the duration grid plus "Start keeps awake" already covers them; revisit on demand.

- [x] **4.1 Countdown beside the tray icon.** Settings → General → "Show time left in the menu bar" (off by default); monospaced digits, minute resolution ("42m", "1h 05m"), nothing when off or indefinite.
  Verify: unit tests on the title text; e2e of the setting; real app shows the title (owner screenshot 👤 if no capture).
  Done 2026-10-23: `tray::title` (unit tested), Margin's monospaced-digit NSStatusItem title, deduped per minute. 👤 look at it in the menu bar.
- [x] **4.2 Pause while the screen is locked.** Optional (off by default): release the assertion while locked, take it back on unlock; status "Paused while the screen is locked".
  Verify: pure tests on the decision; lock state read via `CGSessionCopyCurrentDictionary` checked against a live read; e2e of the paused state.
  Done 2026-10-23: `core/lock.rs` reads `CGSSessionScreenIsLocked`; reconcile drops the assertion while paused and takes it back on unlock (live IOKit test covers both); timed sessions keep counting by wall clock. Settings → General, off by default. Locking the screen here would interrupt the owner, so the live lock/unlock check is 👤.
- [x] **4.3 Condition triggers.** "Stay awake while on power" and "while an external display is connected", in the same card as app and command triggers.
  Verify: pure evaluation tests; external display read checked against `system_profiler` on this Mac; e2e adding both.
  Done 2026-10-23: `power`/`display` trigger kinds (one each), `Context` evaluation, phrases ("on power", "Paused until you unplug") sent from Rust. External display via `CGGetActiveDisplayList`/`CGDisplayIsBuiltin`, test agrees with `system_profiler`. Live: on battery, a power trigger held nothing (correct). Positive live case pending a plugged-in run.
- [x] **4.4 Schedule.** "Keep awake on weekdays 9:00–18:00"-style windows, evaluated each tick with the local clock; shown as held "Until 18:00 (schedule)".
  Verify: unit tests for day/time windows incl. overnight and DST; e2e of the editor.
  Done 2026-10-23: `core/schedule.rs` (days 0=Mon, "HH:MM", overnight windows belong to the start day; wall-clock local time so DST just shifts with the clock). Added as a `schedule` trigger with a day-chip + time editor. Live: a window around now gave `Wick: awake while on schedule until 21:54`; nothing after quit.
- [x] **4.5 URL scheme and status CLI.** `wick://start?minutes=60`, `wick://start`, `wick://stop`, `wick://toggle`; `wick status` prints JSON for Shortcuts and Raycast.
  Verify: parser unit tests; real `open "wick://start?minutes=1"` creates the assertion; `wick status` JSON checked.
  Done 2026-10-23: tauri-plugin-deep-link (single-instance `deep-link` feature), parser for links and `start [minutes|HH:MM] | stop | toggle | status`. `wick status` reads `status.json` (written on every publish) before Tauri starts and checks the pid. Live: `open wick://start?minutes=1` → `Wick: awake until 21:28`; `wick://toggle` → 0; status JSON correct, `running:false` after quit.
- [x] **4.6 Windows.** `PowerCreateRequest`/`PowerSetRequest`, colour tray icons following the taskbar theme, Toolhelp process scan, `GetSystemPowerStatus` battery guard; no lid hint and no "who else" (needs admin). CI Windows job, NSIS + MSI, Windows entries in `latest.json`.
  Verify: CI builds and tests on windows-latest; 👤 VM install and tray check.
  Done 2026-10-23: CI run 39989912830 green on windows-latest (fmt, clippy -D warnings, tests, NSIS `Wick_0.1.0_x64-setup.exe` + MSI). Win32 declared by hand in `core/win32.rs`. 👤 VM: install, candle in the tray (light and dark taskbar), start/stop with `powercfg /requests`, popover placement.
- [x] **4.7 Release v0.2.0.** Changelog, screenshots, draft, checksums, updater test from the installed v0.1.0. 👤 owner says "publish".
  Done 2026-10-23: published on the owner's "publish"; owner's Mac updated from v0.1.0 through the public updater path. Evidence in docs/verification.md.

## Phase 5 — v0.4.0 ⛳

- [x] **5.1 History.** Count the time Wick holds an assertion, per local day; "Awake today" card with seven day bars; Settings → History (off switch, Clear). Local `history.json`, 30 days. `wick status` gains `awakeTodayMinutes`.
  Verify: pure tests for midnight split, pruning, damaged file, sleep excluded; runtime save/load test; e2e card, clear, off; real app writes `history.json` while awake.
  Done 2026-10-26: wall time capped by the monotonic clock (stops during Mac sleep; Windows treats a >2 min tick gap as sleep). Saved at most once a minute while awake, on every stop and on quit. Card hidden until there is something to show. Live: a 2-minute session saved 60 s mid-session and 120 s at the end.
- [x] **5.2 Homebrew tap.** `Abulkosim/homebrew-tap` with `Casks/wick.rb`; `homebrew.yml` bumps it when a release is published (deploy key, no PAT).
  Verify: `brew style`, `brew audit --online --strict`, `brew livecheck`, real install into a scratch appdir with `wick status` via the linked binary, uninstall; workflow bumps the tap on the v0.4.0 publish.
  Done 2026-10-26: tap seeded with 0.3.0; style clean, audit passed, livecheck 0.3.0; install + `wick status` + uninstall passed (quarantined binary is killed until Open Anyway, now in caveats). Workflow bumped the tap to 0.4.0 on publish.
- [x] **5.3 Release v0.4.0.** Changelog, draft, checksums, updater test, publish, tap bump, owner's Mac updated.
  Done 2026-10-26: merged through PR #4, published on the owner's go-ahead; the tap bumped itself on publish. Evidence in docs/verification.md.

## Phase 6 — v0.5.0 ⛳

- [x] **6.1 Compact history card, taller popover.** One-row card; popover cap 640 → 720 px (placement still clamps to the work area).
  Verify: native preview of the three-trigger state fits without scrolling; fit e2e.
  Done 2026-10-27: the triggers + history state renders at 638 px.
- [x] **6.2 Network and CPU triggers.** "Network above 1 MB/s" and "CPU above 25%" conditions; busy until quiet for 2 minutes; Stop pauses until quiet.
  Verify: pure tests for rates, wraps, hysteresis and validation; live CPU load test; live download makes a network trigger busy; e2e add, hold, pause, resume.
  Done 2026-10-27: macOS hands apps 32-bit, KB-rounded byte counters (netstat, an Apple binary, sees 64-bit), so drops after a 32-bit value count as wraps. Live: a 500 KB/s download made a 100 KB/s trigger busy; spinning every core made a 30% CPU trigger busy. Windows uses GetIfTable2 hardware rows (layout test) and GetSystemTimes; real Windows check 👤.
- [x] **6.3 Raycast extension.** `raycast/`: Keep Awake (state, today's time, presets, typed minutes or time), Toggle, Keep Awake for, Stop. Talks through `wick://` links and `wick status`.
  Verify: unit tests for status text and input; `ray build`; `ray lint`; CI builds it.
  Done 2026-10-27: tests, build and ESLint/Prettier pass. `ray lint`'s author check got HTTP 500 from Raycast's user API, even for known users. 👤 confirm the Raycast username in `raycast/package.json` (`author`), try it with `npm run dev`, then `npm run publish` to open the store PR from your account.
- [x] **6.4 Release v0.5.0.**
  Done 2026-10-27: merged through PR #6, published, tap bumped itself, owner's Mac updated. Evidence in docs/verification.md.

## Phase 7 — v0.6.0 ⛳

- [x] **7.1 Trigger notifications.** Settings → Notifications → "When a trigger ends" (off by default): "Xcode quit. Your Mac can sleep again.", "Unplugged…", "The network is quiet…". Only when nothing else keeps the Mac awake; not on Stop, during a session, or when the battery guard lets go.
  Verify: runtime test for each of those cases; e2e of the setting.
  Done 2026-10-31.
- [x] **7.2 Release v0.6.0.**
  Done 2026-10-31: merged through PR #9, published, tap bumped itself, owner's Mac updated. Evidence in docs/verification.md.

## Phase 8 — Linux → v0.7.0 ⛳

Asked for publicly ("Any plans for a Linux version?"), promised for the end of the week. Every step lands as its own PR, merged once CI is green. x86_64 first; arm64 in Phase 9.

Decisions (research in [research.md](research.md#linux)):

- **Keeping awake:** the XDG desktop portal `Inhibit` (Suspend, plus Idle with **Keep display on**) is what GNOME, KDE and other desktops honour; a systemd-logind `idle` block inhibitor covers logind's own `IdleAction` and shows up in `systemd-inhibit --list` as "Wick". Without a portal, `org.freedesktop.ScreenSaver.Inhibit` keeps the display on. All three end when Wick's D-Bus connection or file descriptor closes, so a quit or crash releases them like an IOKit assertion. Never `sleep` inhibitors: a manual suspend must still work, as on macOS.
- **Tray:** StatusNotifierItem (`tray-icon`'s `ksni` backend), so a left click opens the popover and no libappindicator is needed. Icon ink follows the panel: light on GNOME's always-dark bar, otherwise the portal's colour scheme. With no tray host (GNOME without the AppIndicator extension), Wick opens as a small window instead of disappearing.
- **Popover:** frameless, placed at the panel corner on X11; Wayland compositors place it themselves. Square-ish 8 px radius like Windows, no arrow.
- **Readers:** battery from `/sys/class/power_supply`, Low Power Mode from power-profiles-daemon (`power-saver`), lock from logind `LockedHint`, lid from `/proc/acpi/button/lid` plus logind `HandleLidSwitch*`, external displays from `/sys/class/drm`, processes from `/proc`, activity from `/proc/net/dev` and `/proc/stat`, who else from logind `ListInhibitors`.
- **Packages:** AppImage (in-app updates), `.deb` and `.rpm` (updates through pkexec, or the package manager). Built on Ubuntu 22.04 for glibc 2.35+.
- **Copy:** "Mac" on macOS, "PC" on Windows, "computer" on Linux.

- [x] **8.1 Linux core.** Inhibitors, power, lock, lid, displays, boot time, processes, activity, who else, status path; a Linux CI job (fmt, clippy, tests, build).
  Verify: pure parser tests with fixtures; live tests in Docker (Ubuntu 22.04, aarch64) and on the CI runner: the inhibitor shows in `systemd-inhibit --list` and goes on drop and on kill.
  Done 2026-11-05 (PR #10): parsers tested on every platform; the live logind test passes in the container and on the runner. Found on the way: tauri-plugin-deep-link 2.6 doesn't build with rust-ini ≥ 0.21.2 (pinned, Dependabot told), and async-executor 1.14.0 was yanked.
- [x] **8.2 Linux shell.** KSNI tray with click and themed ink, no-host fallback window, popover placement and look, deep links for AppImage, platform-aware copy and settings, Wayland shortcut note.
  Verify: e2e `?platform=linux`; headless smoke test (Xvfb + D-Bus) in CI: `wick start 1` → inhibitor listed, `wick status` JSON, stop → gone; screenshot under a real panel in Docker.
  Done 2026-11-05 (PR #11): under a real Xfce panel a click opens the popover at the icon; no panel → titled window. Found and fixed: Xfce's GTK portal accepts a suspend inhibit it can't honour (now PowerManagement + a sleep lock where needed, checked against xfce4-power-manager); WebKitGTK left stale text in a transparent window (opaque on Linux, resizable so the fit works); WebKitGTK's user agent can claim macOS (Rust now tells the page); panel colour is unknowable (candle with a halo). On the runner polkit refuses the sleep lock to a non-seat user and the idle lock is kept.
- [x] **8.3 Linux packages and release pipeline.** AppImage, deb, rpm; signed updater artifacts; `latest.json` linux entries; checksums, attestations; install docs.
  Verify: script tests; CI release dry run builds all three; AppImage update installs in Docker.
  Done 2026-11-05 (PR #14): signed bundle build with a throwaway key gave all three packages and signatures; `updater_install_linux` passed on that AppImage; the AppImage passed the smoke test; the deb's desktop entry now passes `%u`, and `xdg-open wick://…` works after installing it. Inter ships for Linux (Debian's DejaVu cut the title).
- [x] **8.4 Release v0.7.0.**
  Done 2026-11-05: published on the owner's go-ahead; tap bumped itself; owner's Mac updated. Evidence in docs/verification.md.
- [x] **8.5 Cross-review with Codex** after the release; fixes go out as a patch.
  Done 2026-11-05: `codex exec` (read-only) over v0.6.0..v0.7.0 found one high and five medium issues, all confirmed and fixed for v0.7.1: Xfce's portal accepted a request it then refused, which hid the ScreenSaver fallback; the no-tray window could be hidden with a session running; a portal read on the main thread at every icon redraw; no timeout on the D-Bus handshake; USB PD PPS chargers (`online=2`) read as unplugged; µWh and µAh batteries summed together.

## Phase 9 — Linux arm64

Same app on ARM Linux (Raspberry Pi 4/5 with a desktop, Asahi, ARM laptops and VMs). Native build on GitHub's free `ubuntu-22.04-arm` runner, so glibc 2.35+ like x64; no cross-compiling.

- [x] **9.1 arm64 build and release pipeline.** CI job, `aarch64.AppImage`, `arm64.deb`, `aarch64.rpm`; signed updater artifacts; `linux-aarch64*` in `latest.json`; checksums, attestations; updater allowlist; install docs.
  Verify: script tests for names, manifest and checksums; Rust test for the allowlist; the arm64 CI job passes fmt, clippy, tests, all three bundles and the smoke test on the binary and the AppImage.
  Done 2026-11-10 (PR #24): Tauri named the bundles `Wick_0.7.1_aarch64.AppImage`, `Wick_0.7.1_arm64.deb`, `Wick-0.7.1-1.aarch64.rpm`, as the manifest expects; 96 Rust tests (including the live logind inhibitor) and the smoke test on the binary and the extracted AppImage pass on the arm runner. linuxdeploy ships aarch64 builds, so the AppImage needs nothing extra.
- [x] **9.2 Release v0.7.2.** README row with arm64 links.
  Not released on its own: the owner asked for one final release after a full sweep (Phase 10), so arm64 ships with it. README links stay on v0.7.1 until then.

## Phase 10 — Sweep → v0.8.0 ⛳

A full pass over code, tests, CI, docs and UX for technical debt, defects and polish, then one release that includes Linux arm64. Each slice lands as its own PR, merged once CI is green. Findings come from three read-only audits (Rust, frontend, CI/docs/site), each checked against the code before it becomes a task.

- [x] **10.1 `wick://` links run once.** On Windows and Linux single-instance passes a second launch's link to the deep-link plugin, which runs it, then to our callback, which ran it again: `wick://toggle` did nothing.
  Verify: unit test of the forwarded-link check; CI on all platforms.
- [x] **10.2 One odd value can't blank the state.** Rust caps other apps' names and per-day minutes to what the page accepts; Zod clamps instead of rejecting.
  Verify: Rust tests for the caps; unit test that an over-long name still parses.
- [x] **10.3 A session ends even when the handover fails.** If the new assertion can't be made at the end of a session, the old one is still released, the error is shown, and the ticker never spins.
  Verify: runtime test with a failing assertion; error state in the demo.
- [x] **10.4 Clock changes.** The macOS assertion timeout and activity triggers follow the monotonic clock, so a clock set back doesn't end a session early or keep a trigger busy.
  Verify: pure tests with a moved wall clock.
- [x] **10.5 No freezes on a slow bus.** Intent commands, tray actions, the shortcut and links run on a worker thread instead of the main thread.
  Verify: CI; Linux smoke test.
  Done: building the inhibitor outside the runtime lock was left out; with intents off the main thread a slow bus only delays the ticker, and the lock keeps assertion swaps ordered.
- [x] **10.6 Nothing fails silently.** Windows updates keep the running session and history; shortcut, icon-click and link errors show as notifications like menu errors.
  Verify: Rust tests where pure; review of the updater hook.
  Done: the updater's `on_before_exit` (Windows only) flushes history and marks the session to resume, then does the plugin's own cleanup.
- [x] **10.7 Store edges.** Reset keeps triggers and the update-check time and re-registers the shortcut; one backup per recovery; recovery notes stay until the user saves.
  Verify: store and runtime tests.
- [x] **10.8 Rust tidy.** Digits-only schedule times, dead code removed, comments fixed, one app id, pin notes in Cargo.toml, DST tests.
  Verify: tests; CI.
  Done: DST cases use `chrono-tz` (dev only): Berlin's spring gap and repeated autumn hour, Santiago's skipped midnight. Limiting app commands with an `AppManifest` was left out: it can only be proven in the real app, the window is the only webview, and the CSP allows no outside scripts.
- [x] **10.9 Keyboard and screen readers.** Typing in a picker never fires shortcuts; the live region skips the countdown; radio groups use arrows; focus ring on every focusable, 3 px offset; Escape closes the time editor; one Quit tooltip.
  Verify: e2e for each; screenshots light + dark.
- [x] **10.10 Popover details.** Active conditions styled as running; one 24-hour time format everywhere; fresh "Until" default and add-trigger form on each open; no preset highlighted after +15; loading state; readable error-toast icon; tabular numbers everywhere.
  Verify: unit tests for time formatting and presets; e2e; screenshots.
- [x] **10.11 Frontend tidy and demo coverage.** Dead CSS and code removed; outgoing IPC payloads validated with Zod; demo states for Low Power Mode, updater phases, schedule-held and an error toast.
  Verify: typecheck, unit, e2e over every demo state.
  Done: 160 e2e in Chromium, 28 demo states in both themes. The demo's Stop/Resume wording still mirrors Rust's phrases by pattern; it only affects the browser preview.
- [x] **10.12 CI hardening.** No caches on tag builds; pinned Rust toolchain, runner images and audit tools; tags must be on `main`; dev dependencies audited; `check-release` rejects prerelease tags and has tests; smoke test cleans up; Dockerfile pinned and watched by Dependabot; `ray lint`; SBOM limited to shipped code.
  Verify: CI green; script tests.
  Done: runners pinned to `macos-26`, `windows-2025`, `ubuntu-22.04(-arm)`, `ubuntu-24.04`; Rust 1.99.1 in CI and the dev container; cargo-audit 0.23.0, cargo-deny 0.20.2. The SBOM still covers the whole checkout: sbom-action can't exclude paths without a custom Syft config, and listing the Raycast extension too is harmless.
- [x] **10.13 Repo settings.** Rulesets for `main` (required checks, no force-push) and `v*` tags; CodeQL default setup; Dependabot security updates; SHA pinning required; a `release` environment for the signing secrets.
  Verify: `gh api` shows each; a PR still merges.
  Done 2026-11-10: rulesets `main` (PR, 7 required checks, no force-push or deletion; admins may bypass in a PR) and `release tags` (create, move, delete `v*`: admins only); CodeQL default setup; Dependabot security updates; SHA pinning required; `release` environment for `v*` tags, used by the native jobs on tag builds.
  👤 Move `TAURI_SIGNING_PRIVATE_KEY` and its password into the `release` environment and delete the repo-level copies.
- [ ] **10.14 Docs.** Package updates do ask for a password on Linux; issue template, SECURITY and CONTRIBUTING cover every platform; README screen-lock note; CONTRIBUTING layout and demo states; `check-all.sh` runs the production build; the repo's Homebrew cask matches the tap; changelog date set before tagging.
  Verify: links resolve; `scripts/check-all.sh` passes.
- [ ] **10.15 Site and social image for every platform.** Meta tags and `og.png` say computer, not Mac.
  Verify: screenshot of the new image; site renders.
  👤 Upload `docs/images/social-preview.png` in the repo's settings.
- [ ] **10.16 Release v0.8.0.** Version, changelog (arm64 + sweep), README with arm64 row, full release flow.
  Verify: full release flow; both Linux updater install tests pass on their signed AppImages in the tag build.

## Later / maybe

- winget (Margin `packaging/winget`).

## Not doing

- Closed-lid without an external display, privileged helper, `pmset` writes.
- Mouse-jiggle or presence features.
- Wi-Fi network trigger until Location permission is worth it.
- Telemetry, any network besides the GitHub update check.
