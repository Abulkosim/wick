# Privacy

Wick has no account, no telemetry, no analytics, and no crash reporting.

## Network requests

One host, for update checks:

| Host | When | Sends |
| --- | --- | --- |
| `github.com` (and GitHub's release download host it redirects to) | Two minutes after launch, then at most once a day, or when you click **Check for updates**. Turn the daily check off in **Settings → Updates**. | A standard HTTPS request for `latest.json`, and the update archive only when you choose to install. Nothing about you, your apps, or your settings. |

## What Wick reads

All on your Mac, never sent anywhere:

- **Power assertions** from macOS (`IOPMCopyAssertionsByProcess`), the same list `pmset -g assertions` shows, to list what else keeps the Mac awake.
- **Running processes**: names, executable paths and parent process. All processes only while a trigger is on, or once to list running apps when you add one; otherwise only the processes holding a power assertion. Every 5 seconds at most.
- **Power source**: on battery or power, charge level, Low Power Mode.
- **Lid state**: whether closing the lid would sleep the Mac, from `IOPMrootDomain`.

## What Wick writes

- `~/Library/Application Support/com.abulkosim.wick/settings.json` (`%APPDATA%\com.abulkosim.wick\` on Windows): your settings, triggers (app names and paths, command names) and the time of the last update check. Readable by your user only (0600). A damaged file is kept as `settings.json.bak-<time>` before any reset.
- `~/Library/Application Support/com.abulkosim.wick/history.json`: minutes Wick kept the Mac awake on each of the last 30 days, nothing else. Off and clearable in Settings → History. A damaged file is set aside as `history.json.bak-<time>`.
- `~/Library/Application Support/com.abulkosim.wick/status.json`: the current session, for `wick status`. Rewritten on every change.
- `~/Library/LaunchAgents/Wick.plist`, only if you turn on **Launch at login**.
- A sleep assertion named `Wick: …` while it keeps the Mac awake. macOS releases it when Wick quits or crashes.

On Windows, a power request named after the session while Wick keeps the PC awake; Windows clears it when Wick exits.

Wick never changes `pmset` or any other system setting, needs no administrator password, and asks for no permissions.
