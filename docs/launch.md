# Launch kit

Drafts for v0.9.0, the first release with agents. 👤 The owner posts; nothing here is posted for you.

## Before posting

- [ ] 👤 v0.9.0 is published and marked latest (plan 11.8). Agents ship in v0.9.0; until then the README's Install table points at v0.8.0, which has no Settings → Agents.
- [ ] 👤 Download links in the README read v0.9.0.
- [ ] If the popover changed since, run `pnpm demo:video` and commit the new `docs/images/demo.mp4` and `demo.gif`.
- [ ] 👤 Attach `docs/images/demo.mp4` (20 s, 1520×1360, H.264) where the post takes video: Reddit, X, Bluesky. HN takes no media; the README GIF covers it.
- [ ] 👤 Read each subreddit's rules on the day. r/linux and r/macapps both limit self-promotion; say you made it.
- [ ] 👤 Be around for a few hours after posting to answer questions.

## Facts the drafts use

Quoted from the repo so the posts and the README say the same thing. Change them there first.

- README: "**While agents work.** Connect Claude Code or Codex in Settings → Agents. Wick keeps the Mac awake while one works and lets it sleep when it finishes or waits for you."
- README: "Wick holds a standard power assertion, `PreventUserIdleSystemSleep`, or `PreventUserIdleDisplaySleep` with **Keep display on**. You can see it in `pmset -g assertions` as `Wick: awake until 16:30`. macOS ties the assertion to Wick's process, so quitting or a crash releases it. Wick never changes system settings."
- README: "On Windows it's a power request (`powercfg /requests`). On Linux it's your desktop's inhibitor plus a systemd-logind lock (`systemd-inhibit --list`), each tied to Wick's process the same way."
- README: "No account, no telemetry, no analytics. The only network request is the daily update check against GitHub Releases, which sends nothing about you."
- README: "Wick has no closed-lid mode on purpose. Keeping a closed Mac awake without an external display needs root and a persistent `pmset` change, and a forgotten session overheats a Mac in a bag."
- README platforms: "macOS 12+ · Apple Silicon and Intel", "Windows 10/11 x64", "Linux x86_64 · Ubuntu 22.04, Debian 12, Fedora 36 and newer", "Linux arm64 · same distributions".
- agents.md: "A "while `claude` is running" trigger keeps the computer up all night while the agent sits at a prompt."
- agents.md, measured: "`wick hook claude < payload`: 30–49 ms over 20 runs (median 34), 0 bytes of stdout, exit 0."
- agents.md: "Known gaps, accepted: approving a permission request fires nothing in either agent, so an approved long command shows "waiting" until its `PostToolUse`."
- Comparison, rows copied from the README's "Why Wick" table:

|  | Wick | Amphetamine | KeepingYouAwake | `caffeinate` |
| --- | :-: | :-: | :-: | :-: |
| macOS | ✓ | ✓ | ✓ | ✓ |
| Windows | ✓ | – | – | – |
| Linux | ✓ | – | – | – |
| While an app runs | ✓ | ✓ | – | – |
| While a command-line tool runs (`claude`, `ffmpeg`) | ✓ | – | – | `-w pid` |
| While coding agents work, not while they wait for you | ✓ | – | – | – |
| Shows what else keeps the Mac awake | ✓ | – | – | – |
| Closed lid without an external display | – | ✓ (admin script) | – | – |
| Open source, free | ✓ | – | ✓ | ✓ |

## Show HN

**Title** (66 characters):

> Show HN: Wick – keeps your computer awake while coding agents work

Alternative (73):

> Show HN: Wick – a keep-awake app that lets the Mac sleep when agents wait

**URL:** https://github.com/Abulkosim/wick

**First comment:**

> Wick is a small menu bar app (tray on Windows and Linux) that keeps the computer awake: for 15 minutes, until 16:30, until you stop it, or while an app or command runs.
>
> The new part is agents. A "while `claude` is running" rule keeps the computer up all night while the agent sits at a prompt waiting for you. So Wick listens to the agents' own hooks instead. Settings → Agents → Connect adds hooks to `~/.claude/settings.json`, or to `~/.codex/hooks.json` and marks them trusted in `~/.codex/config.toml` so Codex runs them without its review screen. Both files are backed up first, and Disconnect removes only Wick's entries. Each hook runs `wick hook claude`, which writes a small file saying "working" or "waiting" and exits: 30–49 ms over 20 runs, no output, always exit 0, so it can't slow down or break the agent. The app reads those files every 5 seconds. While an agent works, the computer stays awake; when it finishes or waits for you, it can sleep again, and a notification says "Claude Code finished in wick. Your Mac can sleep again." If the agent is killed, its pid is gone and the hold goes with it.
>
> For anything else: `wick run -- make test` keeps it awake while a command runs and exits with its code, and `wick hold render --pid 4242` holds until a process ends.
>
> Under the hood it's the OS's normal mechanism: `PreventUserIdleSystemSleep` on macOS (you'll see `Wick: awake until 16:30` in `pmset -g assertions`), a power request on Windows, and the desktop's inhibitor plus a systemd-logind lock on Linux. All are tied to Wick's process, so a crash releases them. Wick never changes system settings.
>
> No closed-lid mode, on purpose: keeping a closed Mac awake without an external display needs root and a persistent `pmset` change, and a forgotten session overheats a Mac in a bag.
>
> Known gap: approving a permission prompt fires no hook in either agent, so after you approve a long command Wick shows "waiting" until the command finishes.
>
> Free, MIT, Tauri 2 + Rust + Vue. No account, no telemetry; the only network request is the daily update check against GitHub Releases. Builds aren't signed with an Apple Developer ID or a Windows certificate, so the first launch shows a warning; the README has the steps, and there's a Homebrew cask (`brew install --cask abulkosim/tap/wick`).

## r/macapps

**Title:**

> [Free, open source] Wick – keeps your Mac awake while Claude Code or Codex works, sleeps when they wait

**Body** (attach `demo.mp4`):

> I made Wick, a menu bar app that keeps the Mac awake. Free, MIT, no account, no telemetry.
>
> - **One click.** Until you stop it, 15 minutes, an hour, two hours, until a time, or any length. The menu bar candle burns down as the time runs out.
> - **While agents work.** Connect Claude Code or Codex in Settings → Agents. Wick keeps the Mac awake while one works and lets it sleep when it finishes or waits for you.
> - **While something runs.** "Stay awake while **Xcode** is running", or a command like `ffmpeg`, or while the network or CPU is busy. Also on power, with an external display, or on a schedule.
> - **Says what else is awake.** "caffeinate · Started by claude", "Google Chrome · Playing audio".
> - Battery guard (stops below 20% by default), global shortcut, launch at login, `wick` CLI, `wick://` links for Shortcuts, and a Raycast extension.
>
> How it compares (from the README):
>
> |  | Wick | Amphetamine | KeepingYouAwake | `caffeinate` |
> | --- | :-: | :-: | :-: | :-: |
> | While an app runs | ✓ | ✓ | – | – |
> | While a command-line tool runs (`claude`, `ffmpeg`) | ✓ | – | – | `-w pid` |
> | While coding agents work, not while they wait for you | ✓ | – | – | – |
> | Shows what else keeps the Mac awake | ✓ | – | – | – |
> | Closed lid without an external display | – | ✓ (admin script) | – | – |
> | Open source, free | ✓ | – | ✓ | ✓ |
>
> No closed-lid mode, on purpose: it needs root and a persistent `pmset` change. Wick says "Closing the lid will sleep your Mac" when that's true instead.
>
> macOS 12+, Apple Silicon and Intel. It isn't signed with an Apple Developer ID, so on first launch click **Done**, then **System Settings → Privacy & Security → Open Anyway**. Or `brew install --cask abulkosim/tap/wick`.
>
> https://github.com/Abulkosim/wick

## r/linux

**Title:**

> Wick: a tray app that inhibits sleep while a coding agent works, then lets the machine sleep (AppImage, deb, rpm; x86_64 and arm64)

**Body:**

> I made Wick, a keep-awake tray app. It started on macOS and now runs on Linux too. Free, MIT, Tauri 2 + Rust.
>
> - It holds your desktop's inhibitor plus a systemd-logind lock, so you can see it in `systemd-inhibit --list` ("Wick: awake while Claude Code works"). Both are tied to Wick's process; a crash releases them. It never changes system settings.
> - Timed sessions, until a time, or until you stop it, from the tray or the command line: `wick start 45`, `wick start 16:30`, `wick stop`, `wick status` (JSON).
> - `wick run -- make test` keeps the machine awake while the command runs and exits with its code, like `caffeinate -i <command>` on macOS.
> - Agents: connect Claude Code or Codex in Settings → Agents and it stays awake while one works, and lets the machine sleep when it finishes or waits for you. Hooks write a small state file; they exit 0 with no output in 30–49 ms, so they can't break the agent.
> - Triggers: while an app or process runs, network or CPU busy, on power, external display, schedule. Optional pause while the screen is locked.
>
> Ubuntu 22.04, Debian 12, Fedora 36 and newer, on x86_64 and arm64 (a Raspberry Pi 4 or 5 with a desktop works). The candle sits in the panel's tray; on GNOME without the AppIndicator extension it opens as a window. No telemetry; the only network request is the daily update check against GitHub Releases.
>
> https://github.com/Abulkosim/wick

## X / Bluesky

One post, under 280 characters with the link. Attach `demo.mp4`.

> Wick keeps your computer awake while Claude Code or Codex works, and lets it sleep when the agent finishes or waits for you.
>
> Also timers, until a time, or while an app runs. Free and MIT, for macOS, Windows and Linux.
>
> https://github.com/Abulkosim/wick
