# Research

Done 2026-10-23 on macOS 26.6.2 (Mac14,10, Apple Silicon). Sources numbered per section. Local probes changed no system settings.

## Decisions it led to

- No closed-lid mode without an external display. It needs root (`pmset disablesleep`), a privileged helper can't be installed with ad-hoc signing, and a forgotten session overheats a Mac in a bag. Show whether closing the lid will sleep the Mac instead.
- Use `PreventUserIdleSystemSleep` for "system only" and `PreventUserIdleDisplaySleep` for "keep display on". Set `TimeoutSeconds` on timed sessions as a backstop to our own timer.
- Assertions die with the process (verified with `kill -9`), so a crash can't leave the Mac awake.
- v0.1 adds the two things nobody does well: "while an app or process is running" (including CLI tools like `claude`, `codex`, `ffmpeg`, `xcodebuild`) and "who else is keeping your Mac awake".
- Wi-Fi network trigger waits: it needs Location Services permission and still returns nil in reported cases.
- No mouse-jiggle. Everything in v0.1 is doable from Rust with `objc2-*` crates; no Swift.

## Competitors

Checked 2026-10-23. "n/v" = not verified.

| App | Price / distribution | Durations | Auto triggers | Closed lid | Jiggle | Display-only | Battery guard | Automation | Open source | Last release |
|---|---|---|---|---|---|---|---|---|---|---|
| **Amphetamine** [1] | Free, Mac App Store | Indefinite, for, until, extendable | App/frontmost app, download, displays, USB/BT, power, battery, IP, Wi-Fi, VPN, DNS, audio, drive, CPU, idle | Yes: script + admin; Apple Silicon also needs Power Protect [1][2] | Yes | Yes | End on low battery | Hotkeys, AppleScript | No [3] | 5.3.2, 2023-11-10 |
| **KeepingYouAwake** [4][5] | Free, GitHub + brew cask | Presets, indefinite | External display (since 1.6.3) | No | No | Yes | Threshold + Low Power Mode | URL scheme; no global shortcut (#190) | MIT, 7.0k★, `caffeinate` wrapper | 1.6.8, 2025-09-12 |
| **Caffeine** (IntelliScape) [6] | Free | Toggle + one timeout | None | No | No | n/v | No | n/v | MIT, 764★ | 1.6.4, 2026-04-20 |
| **Caffeine** (iannuttall rewrite) [7] | Free, GitHub | Timers | Coding-agent hooks, process watch | Best-effort | No | Yes | Yes | n/v | MIT, 33★ | 0.1.1, 2026-08-13 |
| **Lungo** [8] | $4, App Store / Setapp | Presets 10m–indefinite; no until/custom | None (Shortcuts DIY); pauses on lock | Only on charger + external display | No | Yes | Stops on battery | Shortcut, URL scheme, Shortcuts, CLI, Raycast | No | n/v |
| **Theine** [9] | $8.99, App Store | Timer, indefinite | None | No | No | n/v | n/v | Hotkey | No | 4.3 |
| **Raycast Coffee** [10] | Free, Raycast Store | For, until, indefinite | App running, schedule | No | No | Display/system/disk | No | Raycast commands | MIT | 2026-10-15 |
| **Owly** [11] | Free, App Store | Timer, menu-bar countdown | Schedule, power, apps | No | No | Yes | Off below % | Shortcut | No | 2.5 |
| **`caffeinate`** [12] | Built in | `-t`, `-w pid`, wraps a command | None | No | No | `-d` vs `-i`/`-s` | No | It is the CLI | Apple | — |

New in 2025–26: agent keep-awake tools. Adrafinil (lid-closed while agents run, `pmset disablesleep` + hooks, 521★, Show HN 124 points) [13][14], Lidless (188★) [15], plus many small Show HNs (StayUp, Vigil, Agents Sleep Preventer, LidAwake) [17]. Codex reportedly added its own "Prevent sleep while running" toggle [14].

**Praised:** one click and simplicity [6][11]; Amphetamine's trigger depth [1][14]; no subscription or upsell [9]; `caffeinate -dimsu <cmd>` needs nothing installed [14].

**Complained about:**
- Amphetamine closed-lid needs a separate script, admin and Power Protect [1][2]; black screen on wake on Tahoe (#16) [3]; "no effect on 15.3.x" (#11) [3]; broke on Tahoe until reinstalled [18]; no update since 2023-11 [1].
- Amphetamine's app trigger misses background processes; users ask for a by-PID trigger (#15) [3]. Menus are dense [19].
- KeepingYouAwake turns itself off (#211), fails on 15.x (#220), remaining time goes negative after sleep (#157), external-display trigger broken (#235), display cycling on Tahoe (#244); the cup icon is unclear and renders black/white inconsistently (#100, #57, #119) [20].
- Theine 4 rewrite: more clicks, hidden presets, 300 MB memory, rating prompts [9]. Lungo: no until-time, no custom duration [8].
- Forgotten sessions: laptop "discharged in my bag" [13].

**Missing everywhere:**
- KeepingYouAwake's top requests [20]: pause while screen is locked (#98, 16 👍), start on power (#104, 11), until a time (#161, 10), schedule (#189, 8), global shortcut (#190), menu-bar countdown (#154), quiet notifications (#172).
- CLI process triggers: Amphetamine can't see them [1][3]; only tiny agent-only apps cover them [13][17].
- "Who else is keeping my Mac awake": guides still say `pmset -g assertions` [18][22].
- Power and a clean UI together, free and open source.
- Honest closed-lid safety: `disablesleep` tools risk leaving it on [13][14].

Reddit was unreachable from the research environment; complaints come from App Store, GitHub issues, HN, Apple Discussions and blogs. [16] and [19] are vendor/listing pages, weaker evidence.

1. https://apps.apple.com/us/app/amphetamine/id937984704?mt=12
2. https://mac.softpedia.com/progChangelog/Amphetamine-Changelog-140109.html
3. https://github.com/x74353/Amphetamine/issues · https://github.com/x74353/Amphetamine-Enhancer
4. https://github.com/newmarcel/KeepingYouAwake
5. https://github.com/newmarcel/KeepingYouAwake/blob/master/CHANGELOG.md
6. https://github.com/domzilla/Caffeine
7. https://github.com/iannuttall/caffeine
8. https://sindresorhus.com/lungo
9. https://apps.apple.com/app/theine/id955848755
10. https://github.com/raycast/extensions/tree/main/extensions/coffee
11. https://apps.apple.com/app/id882812218
12. https://ss64.com/mac/caffeinate.html
13. https://github.com/kageroumado/adrafinil
14. https://news.ycombinator.com/item?id=48701512
15. https://github.com/nghialuong/Lidless
16. https://alternativeto.net/software/amphetamine/?platform=mac
17. https://news.ycombinator.com/item?id=48607172 · https://news.ycombinator.com/item?id=49803251 · https://news.ycombinator.com/item?id=49954491 · https://news.ycombinator.com/item?id=49263800
18. https://discussions.apple.com/thread/256149052
19. https://notchy.dev/alternatives/amphetamine/
20. https://github.com/newmarcel/KeepingYouAwake/issues?q=sort%3Areactions-%2B1-desc
21. https://twit.tv/shows/hands-on-mac/episodes/137
22. https://www.macworld.com/article/218839/find-out-whats-keeping-your-mac-awake.html

## macOS mechanics

### Assertion types

| Type ([IOPMLib.h]) | Effect | Privilege |
|---|---|---|
| `PreventUserIdleSystemSleep` | Stops idle system sleep; display may still sleep. Mac may still sleep on lid close, Apple menu, low battery. | none |
| `PreventUserIdleDisplaySleep` | Stops display idle sleep, and with it idle system sleep. Doesn't wake a display already off. | none |
| `PreventSystemSleep` | Deprecated since 10.9 in the header, still created by `caffeinate -s`; AC only per `man caffeinate`. | none (created unprivileged, ret 0) |
| `UserIsActive` / `IOPMAssertionDeclareUserActivity` | Lights the display and resets the idle timer up to the user's display-sleep setting. | none |

`caffeinate` ([caffeinate.c]): `-d` → `PreventUserIdleDisplaySleep`; `-i` → `PreventUserIdleSystemSleep`; `-s` → `PreventSystemSleep`; `-u` → `UserIsActive` (5 s); `-m` → `PreventDiskIdle`; `-w pid` uses a `DISPATCH_PROC_EXIT` source.

**Release on exit:** `kill -9` on `caffeinate -di` removed both assertions within 0.5 s; a C probe that exited without `IOPMAssertionRelease` left nothing. powerd tracks the owner process. `pmset` setting changes persist; assertions don't.

**Timeouts:** `kIOPMAssertionTimeoutKey` + `TimeoutAction` (`TurnOff`/`Release`/`Log`). Timed sessions set it so even a hung app releases at the deadline.

### Lid close

- No assertion stops lid-close sleep for third-party code (header; [KeepingYouAwake README][kya]).
- Built-in clamshell mode needs AC, an external display and an external keyboard/mouse ([OWC]). `IOPMrootDomain` exposes `AppleClamshellState` and `AppleClamshellCausesSleep`; locally on AC + external display `AppleClamshellCausesSleep = Yes`, so "external display connected" alone doesn't predict clamshell. Use the flag for the hint.
- No external display: needs `pmset -a disablesleep 1` (root, persistent, system-wide). A crash leaves the Mac never sleeping ([Awake], [Sleepless]). Amphetamine uses a downloaded Power Protect script; Claffeinate a privileged helper ([Claffeinate]).
- `SMAppService.daemon` requires the same Apple signing identity as the app; ad-hoc signing fails with `Operation not permitted` (Apple DTS: [799910], [745720], [757463]). Free distribution rules out a helper; only an admin prompt per use or a sudoers file remain. Not doing it.

### Battery

- Assertions don't override low-battery sleep (header). Our guard is comfort, not safety.
- `IOPSCopyPowerSourcesInfo` + `IOPSCopyPowerSourcesList` + `IOPSGetPowerSourceDescription`: `Current Capacity`, `Max Capacity`, `Power Source State`, `Is Charging`, `Time to Empty`, `LPM Active`. `IOPSGetProvidingPowerSourceType`, `IOPSGetBatteryWarningLevel`. No privilege.
- Push: `IOPSNotificationCreateRunLoopSource`. `IOPSGetPercentRemaining` isn't public; compute capacity / max.

### Who else is keeping it awake

- `IOPMCopyAssertionsByProcess` → pid → array of dicts, no privilege (ret 0). Keys: `AssertType`, `AssertName`, `Process Name`, `AssertPID`, `AssertStartWhen`, `TimeoutSeconds`, `BundlePath`, `AssertionId`, …
- Friendly name/icon via `NSRunningApplication(processIdentifier:)`.
- Filter our pid and system noise: `powerd` "Prevent sleep while display is on", `bluetoothd`, `WindowServer` `UserIsActive`, `Internal*` types.
- No public change notification; poll ~5 s while the popover is open, ~60 s otherwise.

### Trigger costs

| Trigger | Mechanism | Cost |
|---|---|---|
| App running | `NSWorkspace.runningApplications` + launch/terminate notifications | event-driven |
| Process by name | `proc_listallpids` + `proc_name` | 612 pids in 0.98 ms; poll 5 s. Root-owned pids don't resolve; user CLIs do |
| Power source / battery | IOPS run-loop notification | event-driven |
| External display | `CGGetActiveDisplayList` + `CGDisplayIsBuiltin`, `CGDisplayRegisterReconfigurationCallback` | event-driven |
| Network throughput | `sysctl NET_RT_IFLIST2` `if_data64` deltas (`getifaddrs` counters are 32-bit and wrap) | poll 5 s |
| CPU load | `host_statistics(HOST_CPU_LOAD_INFO)` deltas | cheap poll |
| Screen locked | `CGSessionCopyCurrentDictionary["CGSSessionScreenIsLocked"]`, `com.apple.screenIsLocked` distributed notification | event-driven |
| Schedule | Rust timers, re-evaluate on `NSWorkspace.didWake` | trivial |
| Mounted volume | `NSWorkspace.didMount`/`didUnmount` | event-driven |
| Wi-Fi SSID | `CWInterface.ssid()` returned nil locally; needs Location Services authorization ([732431], [769950], [124189]) | permission prompt; deferred |

### Rust fit

`objc2-io-kit`, `objc2-app-kit`, `objc2-foundation`, `objc2-core-graphics` (0.3.x) cover assertions, IOPS, workspace and display APIs; libproc via `extern "C"`. CoreLocation (only for SSID) is the one place Swift would be easier. Nothing in v0.1 needs Swift.

### Windows (v0.2)

`PowerCreateRequest` + `PowerSetRequest(SystemRequired/DisplayRequired)` with a reason string shown in `powercfg /requests`, as Chromium does ([chromium], [ms]). Process-scoped. `powercfg /requests` needs admin, so "who else" may be mac-only.

### Verified locally

- `pmset -g`: `sleep 1 (sleep prevented by bluetoothd, caffeinate, caffeinate, powerd)`, `displaysleep 10`.
- `caffeinate -{d,i,s,u,m} -t 30` created the five types above with `TimeoutActionRelease`.
- `caffeinate -di &` then `kill -9`: 2 assertions before, 0 after.
- C probe: `IOPMCopyAssertionsByProcess ret=0x0`; unprivileged `PreventSystemSleep` ret 0, released on exit; IOPS `ProvidingType=AC Power`, `LPM Active=false`.
- `ioreg IOPMrootDomain`: `SleepDisabled = No`, `AppleClamshellCausesSleep = Yes`, `AppleClamshellState = No`.
- Display probe: built-in reports builtin 1, external 0. Lock key absent while unlocked. CoreWLAN `ssid: nil`.
- libproc scan: 612 pids, 422 named, 0.98 ms.

[IOPMLib.h]: https://developer.apple.com/documentation/iokit/iopmlib_h
[caffeinate.c]: https://github.com/apple-oss-distributions/PowerManagement/blob/main/caffeinate/caffeinate.c
[kya]: https://github.com/newmarcel/KeepingYouAwake
[OWC]: https://eshop.macsales.com/blog/64640-how-to-use-your-mac-laptop-in-closed-display-clamshell-mode/
[Awake]: https://github.com/yatindma/Awake
[Sleepless]: https://github.com/Aboudjem/Sleepless
[Claffeinate]: https://github.com/matwu/Claffeinate/releases
[799910]: https://developer.apple.com/forums/thread/799910
[745720]: https://developer.apple.com/forums/thread/745720
[757463]: https://developer.apple.com/forums/thread/757463
[732431]: https://developer.apple.com/forums/thread/732431
[769950]: https://developer.apple.com/forums/thread/769950
[124189]: https://developer.apple.com/forums/thread/124189
[chromium]: https://chromium.googlesource.com/git/chromium/+/trunk/content/browser/power_save_blocker_win.cc
[ms]: https://learn.microsoft.com/en-us/windows/win32/power/power-management-functions

## Name

Checked brew casks, Mac App Store (iTunes Search API), GitHub. `Abulkosim/<name>` free for all.

| Name | Verdict |
|---|---|
| Vigil | Taken by two keep-awake apps ([PH](https://www.producthunt.com/products/vigil-companion), [hunted](https://hunted.space/product/vigil-stay-awake)) |
| Linger | Taken by a keep-awake App Store app ([link](https://apps.apple.com/us/app/linger/id6776560374)) |
| Lantern, Stay | Existing brew casks |
| Porchlight | Mac menu-bar app with its own cask |
| Tend, Pilot, Waking, Ember | Crowded or adjacent |
| **Wick** | **Chosen.** No cask, no keep-awake app. Others: an iOS deadline tracker ([link](https://apps.apple.com/us/app/wick-deadline-tracker/id6801678088)), a Mac reading-focus dimmer ([PH](https://www.producthunt.com/products/wick)) |
| Bide, Hold | Runners-up |
