# Contributing

Wick keeps a computer awake from the menu bar or tray: macOS, Windows and Linux. Questions go to [Discussions](https://github.com/Abulkosim/wick/discussions); bugs and requests go to [issues](https://github.com/Abulkosim/wick/issues/new/choose); vulnerabilities go to [SECURITY.md](SECURITY.md).

## Scope and rules

- Keep the computer awake with the platform's standard mechanism only: IOKit power assertions, Windows power requests, desktop and logind inhibitors. Never change `pmset` or other system settings, and never ask for an administrator password to keep it awake.
- No mouse movement, fake activity, or "stay online" features.
- Rust owns every system call. The interface sees state snapshots and sends intents, validated with Zod.
- No telemetry, analytics, or crash reporting. The only network host is GitHub Releases for updates; see [PRIVACY.md](PRIVACY.md).
- Calm by default: shipped defaults should be what most people want on first click.
- Copy is short and plain. Name the action or the information: "Awake until 16:30", not slogans.

## Prerequisites

- Node.js 24 and pnpm 11.25.0 (pinned in `package.json`).
- Rust via [rustup](https://rustup.rs) with `clippy` and `rustfmt`. CI pins 1.99.1; any stable since then works.
- macOS with Xcode Command Line Tools, Windows with WebView2, or Linux with the WebKitGTK toolchain: on Ubuntu/Debian `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev patchelf` ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)). The AppIndicator package is only needed to bundle; the tray speaks StatusNotifierItem itself.
- Playwright browsers for e2e tests: `pnpm exec playwright install chromium webkit`.

## Run

```sh
pnpm install --frozen-lockfile
pnpm tauri dev
```

To work on the interface without the app, use the browser preview:

```sh
pnpm dev
```

Open the printed URL with `?demo=1`. Add `&theme=light` or `&theme=dark`, `&page=settings`, `&platform=windows` or `&platform=linux` (with `&wayland=1` or `&framed=1` for the Linux variants), and `&state=` one of the scenarios in [`src/lib/demo.ts`](src/lib/demo.ts): `off`, `timed`, `until`, `indefinite`, `display`, `battery`, `low-power`, `desktop`, `clamshell`, `locked`, `held`, `paused`, `triggers`, `conditions`, `schedule`, `activity`, `others`, `others-off`, `history`, `error`, `toast`, `recovery`, `recovered`, and the update phases `update-checking`, `update-available`, `update-downloading`, `update-installing`, `update-current`.

`scripts/native-preview.mjs` renders the desktop popover at the height the app asks macOS for, over a light, dark or transparent backdrop, which is closer to the real window than the preview stage.

The running app also takes `wick start [minutes]` and `wick stop` from the command line, which is handy for checking `pmset -g assertions` (or `systemd-inhibit --list` on Linux). On Linux without a desktop, `xvfb-run -a dbus-run-session -- bash scripts/linux-smoke.sh path/to/wick` runs the same checks CI does.

### Linux from another computer

`scripts/linux/Dockerfile` is a Linux desktop in a container: systemd with logind, D-Bus, Xvfb and an Xfce panel for the tray.

```sh
docker build -t wick-linux scripts/linux
docker run -d --name wick-linux --privileged --cgroupns=host -v /sys/fs/cgroup:/sys/fs/cgroup:rw \
  --tmpfs /run --tmpfs /run/lock -v "$PWD":/src -v wick-target:/target wick-linux
pnpm build   # the container builds Rust only
docker exec -w /src/src-tauri -e CARGO_TARGET_DIR=/target wick-linux \
  bash -c 'cargo build --features tauri/custom-protocol && dbus-run-session -- cargo test'
docker exec -w /tmp wick-linux xvfb-run -a dbus-run-session -- bash /src/scripts/linux-smoke.sh /target/debug/wick
```

## Test

`scripts/check-all.sh` is the list: it runs every check CI runs and stops at the first failure.

```sh
pnpm build        # type check and frontend assets; native checks need these
pnpm test
pnpm test:e2e
node --test scripts/*.test.mjs packaging/homebrew/*.test.mjs
(cd raycast && npm test && npm run lint && npx ray build -e dist)
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

CI runs the same commands ([`ci.yml`](.github/workflows/ci.yml)). Some Rust tests create real assertions and processes on macOS and check them through IOKit; they clean up after themselves.

## Layout

```
src-tauri/src/
  commands.rs, cli.rs    command surface, `wick start|stop|toggle|status` and wick:// links
  runtime.rs             state, ticker, persistence, reconciling the assertion
  status.rs              status.json for `wick status` and resuming after an update
  tray.rs, popover.rs    tray icon and popover placement
  shortcut.rs            global shortcut
  updates.rs             update check and install
  core/
    assertion.rs         IOKit assertions, Windows power requests, Linux inhibitors
    session.rs           sessions and time left
    schedule.rs          weekly schedules
    candle.rs            tray icon rendering
    triggers.rs          trigger evaluation and phrases
    activity.rs          network and CPU activity
    processes.rs         process scan
    others.rs            other apps keeping the computer awake
    power.rs, lid.rs     battery guard, Low Power Mode, lid state
    display.rs, lock.rs  external displays, screen lock
    history.rs           time awake per day
    boot.rs              boot time, so a session only resumes in the same boot
    settings.rs, store.rs
    win32.rs             Windows API bindings
    linux/               D-Bus (portal, logind, ScreenSaver), /sys and /proc readers
src/
  App.vue, components/   popover and settings
  design/, styles.css    tokens and controls
  lib/                   bridge, models, formatting, demo states
tests/unit/, tests/e2e/
```

## Commits and pull requests

- Conventional Commits with a body that says what changed and why, e.g. `fix(tray): draw the candle larger and bolder`. No trailers.
- Keep pull requests focused. Fill in the template checklist.
- User-facing changes add a short bullet to the next `changelogs/vX.Y.Z.md`.
- UI changes include light and dark screenshots from the preview.

## Releases

The maintainer tags releases. See [docs/releasing.md](docs/releasing.md). Checks run for each release are recorded in [docs/verification.md](docs/verification.md).

## Conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
