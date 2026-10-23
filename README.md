<p><img src="docs/images/icon.png" alt="" width="72" /></p>

# Wick

**Keeps your Mac awake.** On demand, for a set time, or while an app or command is running. A quiet menu bar app for macOS.

[![CI](https://github.com/Abulkosim/wick/actions/workflows/ci.yml/badge.svg)](https://github.com/Abulkosim/wick/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Abulkosim/wick)](https://github.com/Abulkosim/wick/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Abulkosim/wick/total)](https://github.com/Abulkosim/wick/releases)
[![Platform](https://img.shields.io/badge/platform-macOS%2012%2B-747a87)](docs/install.md)
[![MIT license](https://img.shields.io/badge/license-MIT-747a87)](LICENSE)

[Download](https://github.com/Abulkosim/wick/releases/latest) · [Install](#install) · [What it does](#what-it-does) · [Privacy](#privacy) · [Changelog](CHANGELOG.md) · [Contributing](CONTRIBUTING.md)

<p>
  <img src="docs/images/popover-light.png" alt="Wick awake for two hours, with Xcode, claude and ffmpeg as triggers" width="300" />
  <img src="docs/images/others-dark.png" alt="Wick in dark mode listing caffeinate, Google Chrome and zoom.us as also keeping the Mac awake" width="300" />
</p>

- **One click.** Start until you stop it, or for 15 minutes, an hour, two hours, until a time, or any length. The menu bar candle burns down as the time runs out.
- **While something runs.** "Stay awake while **Xcode** is running", or **claude**, **codex**, **ffmpeg**, **xcodebuild**. Wick watches apps and command-line tools alike.
- **Says what else is awake.** "caffeinate · Started by claude", "Google Chrome · Playing audio". No more `pmset -g assertions`.

## Install

Download **v0.1.0** from [GitHub Releases](https://github.com/Abulkosim/wick/releases/latest): [Universal DMG](https://github.com/Abulkosim/wick/releases/download/v0.1.0/Wick_0.1.0_universal.dmg) for macOS 12+, Apple Silicon and Intel.

Wick is not signed with an Apple Developer ID, so the first launch shows a warning. Drag Wick to Applications and open it. When **“Wick” Not Opened** appears, click **Done**, then **System Settings → Privacy & Security → Open Anyway**. Wick lives in the menu bar, with no Dock icon. Step by step, updates and uninstall: [docs/install.md](docs/install.md).

Updates are signed and verified in the app. Wick checks once a day and installs only when you choose.

## What it does

| | |
| --- | --- |
| **Sessions** | Until stopped, 15 min, 1 hour, 2 hours, until a time (rolls to tomorrow), or a custom length up to 7 days. Time left stays right across sleep. |
| **Display** | Keeps only the system awake by default. **Keep display on** also stops the screen from dimming and locking. |
| **Triggers** | Stay awake while an app or command runs. Stop pauses it until that app quits. |
| **Also awake** | Lists other apps holding the Mac awake, with their reason, including audio playing through macOS and `caffeinate` started by a tool. |
| **Battery** | Stops below a level you choose (20% by default), and optionally in Low Power Mode. |
| **Lid** | Warns "Closing the lid will sleep your Mac" when that is true, and stays quiet in clamshell mode. |
| **Shortcut** | A global shortcut you record in Settings starts or stops from any app. |
| **Notifications** | "Session ended. Your Mac can sleep again." Only when Wick stops on its own. |
| **Command line** | `wick start`, `wick start 45` (minutes), `wick stop`, sent to the running app. |

<p>
  <img src="docs/images/settings-light.png" alt="Wick settings: start duration, icon click, shortcut, launch at login and battery" width="300" />
</p>

## Why Wick

|  | Wick | [Amphetamine](https://apps.apple.com/app/amphetamine/id937984704) | [KeepingYouAwake](https://github.com/newmarcel/KeepingYouAwake) | `caffeinate` |
| --- | :-: | :-: | :-: | :-: |
| One-click start and timed sessions | ✓ | ✓ | ✓ | – |
| Until a set time | ✓ | ✓ | – | – |
| While an app runs | ✓ | ✓ | – | – |
| While a command-line tool runs (`claude`, `ffmpeg`) | ✓ | – | – | `-w pid` |
| Shows what else keeps the Mac awake | ✓ | – | – | – |
| Battery guard | ✓ | ✓ | ✓ | – |
| Global shortcut | ✓ | ✓ | – | – |
| Closed lid without an external display | – | ✓ (admin script) | – | – |
| Open source, free | ✓ | – | ✓ | ✓ |

Wick has no closed-lid mode on purpose. Keeping a closed Mac awake without an external display needs root and a persistent `pmset` change, and a forgotten session overheats a Mac in a bag. Wick tells you when closing the lid will sleep the Mac instead.

## How it works

Wick holds a standard power assertion, `PreventUserIdleSystemSleep`, or `PreventUserIdleDisplaySleep` with **Keep display on**. You can see it in `pmset -g assertions` as `Wick: awake until 16:30`. macOS ties the assertion to Wick's process, so quitting or a crash releases it. Wick never changes system settings.

## Privacy

No account, no telemetry, no analytics. The only network request is the daily update check against GitHub Releases, which sends nothing about you. Settings stay in `~/Library/Application Support/com.abulkosim.wick/`. Details: [PRIVACY.md](PRIVACY.md). Vulnerabilities: [SECURITY.md](SECURITY.md).

## Develop

Run from source, preview every state in a browser, and run the tests: [CONTRIBUTING.md](CONTRIBUTING.md). Wick's design follows its sibling, [Margin](https://github.com/Abulkosim/margin).

## License

[MIT](LICENSE)
