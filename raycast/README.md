# Wick for Raycast

Start, stop and check [Wick](https://github.com/Abulkosim/wick), the free menu bar app that keeps your Mac awake.

| Command | What it does |
| --- | --- |
| Keep Awake | Shows the current state and today's awake time. Pick 15 minutes, 1 hour, 2 hours or until stopped, or type minutes (`45`) or a time (`16:30`). |
| Toggle Keep Awake | Starts Wick's default session, or stops whatever keeps the Mac awake. |
| Keep Awake for | Takes `45`, `16:30`, or nothing for until stopped. |
| Stop Keeping Awake | Ends the session. A trigger that's holding the Mac pauses until it ends. |

## Requirements

Wick 0.2.0 or later, installed in Applications or with Homebrew (`brew install --cask abulkosim/tap/wick`), and opened once. The extension talks to Wick through its `wick://` links and reads `wick status`; it needs no permissions and makes no network requests.
