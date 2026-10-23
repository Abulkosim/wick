# Brief: keep-awake menu-bar app for macOS

You are building this from zero, as its owner-engineer. This file is the whole context. Read it once, then own the project.

The owner already shipped **Margin** (github.com/Abulkosim/margin), a Tauri menu-bar app that shows Claude/Codex usage. They love how it looks, reads and ships. This new app must feel like Margin's sibling: same styling, same tone, same taste, same changelog and release discipline. It is a separate project and a separate repo.

Clone Margin read-only for reference: `git clone https://github.com/Abulkosim/margin ~/Documents/margin-ref`. Read `src/styles.css`, `src/design/controls.css`, `src/components/ui/*`, `docs/releasing.md`, `docs/verification.md`, `docs/plan.md`, `changelogs/`, `.github/workflows/`, `scripts/`, `README.md`, `site/`. Copy patterns and files freely. Never push to Margin.

## What and why

A menu-bar app that keeps the Mac awake: on demand, for a time, or automatically when conditions hold. Versatile and customizable, but calm by default. One click to start, one click to stop.

Reference apps (study, don't clone): Amphetamine (feature-rich, App Store, dense UI), KeepingYouAwake (open source, `caffeinate` wrapper, minimal), Caffeine, Lungo, Theine, Raycast's Coffee extension, the `caffeinate` and `pmset` CLIs. Ours should match Amphetamine's power with KeepingYouAwake's simplicity, and look better than both.

## Step 1: research, then question the owner

Before writing code:

1. **Research (~30 min, cite sources in `docs/research.md`).**
   - Competitors: features, what users praise and complain about (GitHub issues, Reddit, HN, App Store reviews). What is missing everywhere.
   - macOS mechanics: `IOPMAssertionCreateWithName` assertion types (`PreventUserIdleSystemSleep`, `PreventUserIdleDisplaySleep`, `PreventSystemSleep`), how `caffeinate` uses them, what happens on lid close, on battery, on low battery, with an external display. Whether closed-lid without an external display is possible without root (`pmset disablesleep` needs admin); what a privileged helper would cost in complexity and trust.
   - How to read other processes' assertions (`IOPMCopyAssertionsByProcess`, `pmset -g assertions`) so we can show "Zoom is keeping your Mac awake".
   - Trigger detection costs: running app, running process (e.g. `claude`, `codex`, `ffmpeg`, `xcodebuild`), network throughput, CPU load, power source, battery %, external display, Wi-Fi network (needs Location permission on recent macOS; check), schedule, mounted volume.
   - Whether Tauri 2 + Rust can do all of it cleanly (IOKit via `objc2`/`core-foundation` crates). If a must-have feature needs Swift, say so with evidence.
2. **Question the owner** in one short batch (≤10 questions, multiple choice where possible, your recommendation first). Ask only what changes the build. Suggested:
   - Windows too? (`SetThreadExecutionState` makes it cheap with Tauri; Margin ships both.)
   - Must-have triggers for v0.1, and which wait.
   - Closed-lid mode: skip, external-display-only, or privileged helper.
   - Mouse-jiggle / "stay active" mode: include or not. (Keeps chat apps "online"; easy to misread as cheating presence. Never market it that way.)
   - Same sage accent as Margin, or a sibling hue on the same neutrals.
   - Name pick from your 3 proposals.
   - Commit style: Margin uses one-line imperative subjects; the owner's global default is Conventional Commits with a body. Which here?
3. **Write `docs/plan.md`** (format below) from research + answers. Then build.

Decide everything else yourself. Don't ask the owner to pick between things you can decide.

## Decisions already made (don't relitigate)

- **Stack:** Tauri 2 + Rust core + Vue 3 + TypeScript + Vite, pnpm, Node ≥24. Reka UI for headless controls, Zod for validating every IPC payload. Same as Margin, so the design system ports directly. Tauri plugins: tray, autostart, notification, updater, store (or own JSON), single-instance, positioner, global-shortcut, dialog.
- **Rust owns all system calls.** Frontend only sees state snapshots and sends intents.
- **Fail safe.** Assertions are released on stop, quit and crash (they are process-scoped; verify). Never leave a persistent `pmset` change behind. If we ever change a system setting, back it up, confirm first, restore on stop.
- **No telemetry, no network** except the update check against GitHub Releases.
- **Free only.** Nothing that costs money: no paid signing, no Apple notarization, no paid services. GitHub Actions, Releases, Pages only.
- **Distribution:** GitHub Releases, ad-hoc signed (Apple Silicon needs it), unsigned by a verified publisher. Document right-click → Open. Tauri updater signatures (minisign) from day one.
- **License:** MIT. Author `abulkosim <abulkasimkattaev@gmail.com>`. Repo under `Abulkosim/<name>`.
- **Tone:** it keeps the Mac awake. It's not a productivity hack, not "never get logged out", not "look busy".

## Name

Propose 3: short, memorable, one word, not "-ify", not "Awake"/"Caffeine"/"Amphetamine" clones. Check GitHub, Homebrew casks and the App Store for collisions. Pick one unless the owner objects.

## Design: same family as Margin

Margin's look is the spec. Port it, don't reinvent it.

**Tokens (from Margin `src/styles.css`, use verbatim unless the owner picks a new accent):**

```css
:root {
  font-family: Inter,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;
  --surface:#f5f6f1; --surface-translucent:rgba(247,248,243,.94); --card:#fff; --hover:#e9ede4;
  --ink:#29322d; --secondary:#4d5c51; --tertiary:#59665c;
  --accent:#6c886f; --accent-ink:#526d55; --accent-soft:#e5ece1; --track:#e7ece3;
  --line:rgba(42,56,43,.075); --warning:#916128; --warning-soft:#f5edde; --danger:#a84d45;
  --button:#354e3b; --button-ink:#f8fbf5; --focus:#7d9a7e;
  --shadow:0 18px 70px -20px #1c32283d,0 1px 5px #22352a14;
}
:root[data-theme=dark] {
  --surface:#1b201d; --surface-translucent:rgba(27,32,29,.95); --card:#242a26; --hover:#303831;
  --ink:#e6ece5; --secondary:#b9c3bb; --tertiary:#a6b3a9;
  --accent:#b8d1ba; --accent-ink:#b8d1ba; --accent-soft:#303e32; --track:#3a443b;
  --line:rgba(219,236,221,.07); --warning:#d6b581; --warning-soft:#3b3529; --danger:#e0a39a;
  --button:#b9d2ba; --button-ink:#243629; --focus:#b8d1ba;
  --shadow:0 22px 80px -20px #0009,0 1px 5px #0004;
}
```

**Rules:**
- Inter, 14px base, tight hierarchy, `font-variant-numeric: tabular-nums` on every number and countdown.
- One accent. No borders where spacing or shadow does the job. Radii: popover 12px (mac), cards 10px, controls 6–8px.
- Motion 150–250ms, `ease`, never bouncy. Bars and rings ease to value, countdowns tick, refresh is a pulse, not a spinner. Respect `prefers-reduced-motion`.
- Dark and light both first-class. Follow OS, switchable in Settings.
- Popover: translucent/vibrancy, frameless, anchored to the tray icon, hides on blur, **sizes to its content** (Margin has a `fit_popover` command + ResizeObserver; reuse it). Opens straight on the controls: no title block eating the top. Margin learned this in v0.5.1.
- Visible focus ring (`--focus`, 2px, offset 3px). Full keyboard use. Labels for screen readers.
- Empty, active, paused-by-condition, error and permission-needed states each designed.
- Settings get the same care as the popover. Margin's Settings structure (General, Appearance, Notifications, Updates, sections with one-line hints, "Saves automatically") is the model.
- Bar: a 2026 indie Mac app a designer shipped. Raycast, Linear, Arc, CleanShot, Ice. If a popover screenshot wouldn't get upvotes alone, it isn't done.

**Tray icon:** monochrome template on mac (alpha only, adapts to menu bar), legible at 16px, crisp on Retina. Suggested metaphor, to keep the family resemblance: Margin's ring. Hollow = off; ring that drains as a timed session runs down; full ring = indefinite; small dot = held by a trigger. Optional countdown text beside the icon. Decide and justify in one line.

**Build the smallest version, screenshot light + dark, look at it, iterate.**

## Copy

- Short, practical, plain. Name the action or the information. No slogans, no filler, no exclamation marks, no cute.
- Margin examples: "Usage remaining" (not "Your headroom"), "Resets in 2h 18m · 16:30", "Saves automatically", "Which account the icon follows.", "No accounts connected.", "Check for updates daily".
- For this app, think: "Awake until 16:30", "Awake · 42m left", "Awake while Xcode is running", "Paused: battery below 20%", "Display can sleep", "Zoom is also keeping your Mac awake".
- Notifications are terse: what happened, one line. "Session ended. Your Mac can sleep again."

## Feature direction (validate with research; don't build all at once)

Core (v0.1): one-click toggle; durations (15m, 1h, 2h, until time, indefinitely, custom); keep display on vs system only; launch at login; stop on low battery; end-of-session notification; global shortcut.

Likely next: triggers (app/process running, on power, external display, network activity, CPU busy, schedule), each shown as a plain sentence the user builds ("Stay awake while **Xcode** is running"); "who else is keeping my Mac awake" view; presets; menu-bar countdown text; URL scheme + `--json` CLI for scripts, Shortcuts and Raycast.

Maybe: closed-lid mode, mouse-jiggle, Windows, per-trigger notifications, history ("awake 3h 12m today").

Defaults matter: shipped defaults should be what most people want on first click. Settings exist for the rest.

## Repo layout (adjust freely)

```
src-tauri/src/core/{assertion,triggers,power,schedule,state}.rs
src-tauri/src/{tray,commands}.rs
src/{App.vue, styles.css, design/, components/, components/ui/, components/settings/, lib/}
tests/unit, tests/e2e
changelogs/, CHANGELOG.md, docs/{plan,research,verification,releasing,install}.md
scripts/ (release checks, manifest, checksums — port Margin's)
site/ (GitHub Pages landing, same template as Margin's)
```

- Pure logic (schedules, trigger evaluation, battery guard, countdown formatting) in pure functions with unit tests.
- A demo bridge like Margin's (`?demo=1&state=…`) so every UI state renders in a plain browser for Playwright (Chromium + WebKit) and screenshots. Add a demo state for every designed state.
- Settings: versioned JSON, tolerant per-field loading, recovery notes, back up before reset. Never lose user data on a bad file.

## Workflow

- **`docs/plan.md` is the work queue.** Phases end in ⛳ release boundaries. Each task: one line of what, one **Verify:** line. Tick when verified; write findings under it. Owner-only items marked 👤: prepare everything, leave unchecked, never act for the owner (posting, account settings, VM testing).
- **One task per iteration**, smallest shippable slice. If a task turns out wrong or blocked, write why under it and move on.
- **Before every commit, all green:** `pnpm typecheck && pnpm test && pnpm test:e2e`, then `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` in `src-tauri`.
- **UI change:** screenshot light + dark through Playwright, look at it, refresh `docs/images` if visible copy changed. Write screenshots to the test's output path, never a hard-coded temp path (that broke Margin's CI).
- **Commits:** per the owner's answer above. Either way: imperative, lowercase-ok, no AI attribution, no `Co-Authored-By`, no trailers. Commit as `abulkosim <abulkasimkattaev@gmail.com>`. Push to `main` after each verified task.
- **Code:** no comments unless non-obvious. Lockfiles committed; `--frozen-lockfile` and `--locked` in CI.
- **Reporting:** extremely concise. Short lines. Show screenshots. Ask only real decisions. End with what's done and what the owner must do.

## Releases (exactly like Margin)

Port Margin's `docs/releasing.md`, `scripts/`, and `.github/workflows/` and rename.

- **Version** in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` (one source of truth checked by a script).
- **One `changelogs/vX.Y.Z.md` per release**, linked from `CHANGELOG.md`. Format:

  ```md
  # v0.5.0

  2026-10-22 · [Download](https://github.com/Abulkosim/<name>/releases/tag/v0.5.0)

  - The tray icon follows the session and weekly limits by default. Change it in Settings → Tray shows.
  - Each limit shows the reset time next to the countdown, for example "Resets in 2h 18m · 16:30".
  ```

  A few concrete, user-facing bullets, checked against git history. Say where the setting lives. No implementation detail. Date = actual publish date (UTC); fix it if the draft was dated earlier.
- **`CHANGELOG.md`** is a table: `| Version | Date (UTC) | Changes |` with a one-phrase summary per row.
- GitHub release notes and updater notes come from that changelog file. Test logs and evidence go in `docs/verification.md` (newest release on top: what passed, counts, CI run links, checksums, updater test, what's still pending).
- **CI** (`ci.yml`): Ubuntu 24.04 frontend + Playwright (timeout 45 min; the apt mirror stalls), macOS (fmt, clippy, tests, universal DMG + `.app.tar.gz`), Windows if in scope. Actions pinned by SHA. Least-privilege permissions. Concurrency cancels superseded `main` runs, never tags. Dependabot. Plus `security.yml` (cargo audit/deny, pnpm audit) and `scorecard.yml`.
- **Tag release:** signs updater artifacts, validates versions/lockfiles, generates `latest.json`, `SHA256SUMS.txt`, SPDX SBOM, build provenance attestations, creates a **draft** release.
- **Release flow:** build → tests → CI green on `main` → annotated tag → CI builds draft → download draft, verify checksums, run the real updater install test against the exact archive (tampered bytes and wrong signed version must be rejected) → record in `docs/verification.md` → report to owner → **owner says "publish"** → publish, mark latest → owner updates their installed app in-app. Never edit a published release's files or reuse a version; ship a new patch.
- Generate the updater key with `pnpm tauri signer generate`, store outside the repo with a backup, put it in the `TAURI_SIGNING_PRIVATE_KEY` secret. Losing it strands every install.
- "Small patch" without a release = commit + push only.
- Later, gated off by default: Homebrew tap and winget jobs (Margin has both in `packaging/`).

## Done means

- M1 Alive: tray icon, popover, one-click toggle and durations work; `pmset -g assertions` shows our assertion and it disappears on stop and on quit. Screenshot.
- M2 Real: chosen v0.1 features, settings, launch at login, notifications, battery guard, every state designed and in the demo bridge.
- M3 Ship: README (hero screenshots, 3-bullet value, Install in the first screen, comparison table vs Amphetamine / KeepingYouAwake / `caffeinate`), install notes for unsigned builds, Pages site, v0.1.0 published through the full release flow.
- M4+: whatever the plan says next.

Start with research and the question batch. Keep the owner posted in short lines.
