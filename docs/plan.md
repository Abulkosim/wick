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
- **Tray icon:** Margin's ring, template image. Hollow = off; ring drains as a timed session runs down; full ring = indefinite; ring + center dot = held by a trigger. Keeps the family look and reads time left at 16px without text. Optional countdown text beside it (v0.2).
- **Click:** left-click opens the popover (first-run users need to see options); Settings → General → "Clicking the icon" can switch to start/stop. Global shortcut toggles with the default duration.
- **Defaults:** start = indefinitely, system only (display can sleep), stop below 20% battery on battery power, end-of-session notification on, launch at login off until asked once on first run (Margin pattern: setting in General).
- **Poll budget:** process trigger 5 s scan (≈1 ms); app trigger event-driven; who-else 5 s while popover open, 60 s otherwise; battery event-driven.

## Rules

- Before every commit: `pnpm typecheck && pnpm test && pnpm test:e2e`, then in `src-tauri`: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
- UI change → Playwright screenshots light + dark into the test output dir, look at them, refresh `docs/images` if visible copy changed.
- Rust owns all system calls; the frontend sees snapshots (Zod-validated) and sends intents.
- Push to `main` after each verified task. Release only at ⛳.

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
  Done 2026-10-23 with 0.1. Tray redraws at most once per 1% of a session (5 s tick, deduped glyph).
- [x] **1.3 Popover + toggle.** Status line, primary Start/Stop, duration chips (15m, 1h, 2h, Until…, Indefinitely, Custom), "Keep display on" switch. `fit_popover` + ResizeObserver from Margin. Countdown ticks.
  Verify: e2e for off/timed/indefinite; screenshots light + dark; real app: `pmset -g assertions` shows `Wick:` while on, gone after Stop and after Quit. Screenshot. ⛳ no release, M1 done.
  Done 2026-10-23: presets start in one click, Until…/Custom… inline editors, display switch swaps the assertion type without a gap. Real app via `wick start 30` / `wick stop` (forwarded through single-instance): `PreventUserIdleSystemSleep named "Wick: awake until 18:07"` while on; 0 after stop, after quit, and after `kill -9`. Browser screenshots reviewed; real menu-bar screenshot needs Screen Recording permission here 👤.

## Phase 2 — Real (M2) → v0.1.0 ⛳

- [x] **2.1 Settings store.** Versioned JSON, tolerant per-field load, recovery notes, back up before reset (Margin `store.rs` pattern). Settings panel: General, Battery, Appearance, Notifications, Updates, "Saves automatically".
  Verify: unit tests with corrupt fields keep the rest; e2e of each tab + recovery.
  Done 2026-10-23: `settings.json` v1 (theme, keepDisplayOn, startMinutes, iconClick), 0600, atomic replace; damaged or newer files are never overwritten and saving waits for "Back up and reset"; a recovered file is backed up before the first save. Settings is one scrolling page (General, Appearance) for now; Battery, Notifications and Updates sections arrive with their tasks.
- [ ] **2.2 Launch at login + global shortcut.** autostart plugin (`--background`), global-shortcut plugin; no default shortcut (avoids clashes), recorded in Settings → General.
  Verify: toggle reflects `is_enabled`; shortcut starts/stops in real app.
- [ ] **2.3 Battery guard.** IOPS notifications; stop (not pause) below threshold on battery; optional stop in Low Power Mode. Status "Stopped: battery below 20%".
  Verify: pure tests on guard; manual unplug check 👤 noted if not testable here.
- [ ] **2.4 Notifications.** "Session ended. Your Mac can sleep again." and battery stop. Off switch in Settings → Notifications.
  Verify: unit test on when to send; one real notification seen.
- [ ] **2.5 App/process trigger.** Sentence builder "Stay awake while **Xcode** is running": pick a running app (NSWorkspace) or type a process name (`claude`, `codex`, `ffmpeg`, `xcodebuild`). Held state shows "Awake while Xcode is running". Manual session wins over triggers; Stop while held offers "Pause triggers for 1h".
  Verify: pure evaluation tests; real run with `sleep 30` as the watched process: assertion appears, then goes.
- [ ] **2.6 Who else.** `IOPMCopyAssertionsByProcess`, filtered, friendly names: "Zoom is also keeping your Mac awake" with the reason on hover.
  Verify: unit tests on filtering with a fixture; real app shows `caffeinate` started from a terminal.
- [ ] **2.7 Lid hint.** Read `AppleClamshellCausesSleep`; show "Closing the lid will sleep your Mac" on portables while awake.
  Verify: fixture tests; real read on this Mac.
- [ ] **2.8 Every state in the demo bridge.** off, timed, until, indefinite, held by trigger, stopped by battery, others awake, error, recovery, update available.
  Verify: e2e per state, Chromium + WebKit, light + dark screenshots reviewed.
- [ ] **2.9 Updater.** Port `updates.rs`, `UpdateSettings.vue`, daily check, signer key (stored outside repo, secret set), `scripts/`.
  Verify: script tests; updater install test against a local build.

## Phase 3 — Ship (M3)

- [ ] **3.1 README.** Hero screenshots, 3-bullet value, Install in the first screen, comparison vs Amphetamine / KeepingYouAwake / `caffeinate`.
- [ ] **3.2 `docs/install.md`.** Open Anyway on macOS 15/26, `xattr` fallback, uninstall.
- [ ] **3.3 Pages site** from Margin's template; 👤 Settings → Pages → GitHub Actions, homepage.
- [ ] **3.4 Repo hygiene.** SECURITY.md, PRIVACY.md (no network except GitHub update check), CONTRIBUTING, issue forms.
- [ ] **3.5 Release v0.1.0.** Changelog, tag, draft, checksums, updater test, `docs/verification.md`. 👤 owner says "publish".

## Phase 4 — v0.2

- [ ] Windows build (`PowerCreateRequest`, tray, CI job, NSIS/MSI). 👤 VM check.
- [ ] Triggers: on power, external display, CPU busy, network activity, schedule.
- [ ] Pause while the screen is locked (KeepingYouAwake #98, top request).
- [ ] Menu-bar countdown text beside the icon.
- [ ] Presets.
- [ ] URL scheme `wick://start?for=1h` + `wick --json status` CLI for Shortcuts/Raycast.

## Later / maybe

- History ("Awake 3h 12m today"), per-trigger notifications, Homebrew tap (gated, Margin `packaging/`).

## Not doing

- Closed-lid without an external display, privileged helper, `pmset` writes.
- Mouse-jiggle or presence features.
- Wi-Fi network trigger until Location permission is worth it.
- Telemetry, any network besides the GitHub update check.
