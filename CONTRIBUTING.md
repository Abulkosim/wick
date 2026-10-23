# Contributing

Wick keeps a Mac awake from the menu bar. Questions go to [Discussions](https://github.com/Abulkosim/wick/discussions); bugs and requests go to [issues](https://github.com/Abulkosim/wick/issues/new/choose); vulnerabilities go to [SECURITY.md](SECURITY.md).

## Scope and rules

- Keep the Mac awake with standard power assertions only. Never change `pmset` or other system settings, and never ask for an administrator password.
- No mouse movement, fake activity, or "stay online" features.
- Rust owns every system call. The interface sees state snapshots and sends intents, validated with Zod.
- No telemetry, analytics, or crash reporting. The only network host is GitHub Releases for updates; see [PRIVACY.md](PRIVACY.md).
- Calm by default: shipped defaults should be what most people want on first click.
- Copy is short and plain. Name the action or the information: "Awake until 16:30", not slogans.

## Prerequisites

- Node.js 24 and pnpm 11.25.0 (pinned in `package.json`).
- Stable Rust via [rustup](https://rustup.rs), 1.90 or newer, with `clippy` and `rustfmt`.
- macOS with Xcode Command Line Tools ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)).
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

Open the printed URL with `?demo=1`. Add `&theme=light` or `&theme=dark`, `&page=settings`, and `&state=` one of the scenarios in [`src/lib/demo.ts`](src/lib/demo.ts): `off`, `timed`, `until`, `indefinite`, `display`, `battery`, `desktop`, `clamshell`, `held`, `paused`, `triggers`, `others`, `others-off`, `error`, `recovery`, `recovered`, `update-available`.

`scripts/native-preview.mjs` renders the desktop popover at the height the app asks macOS for, over a light, dark or transparent backdrop, which is closer to the real window than the preview stage.

The running app also takes `wick start [minutes]` and `wick stop` from the command line, which is handy for checking `pmset -g assertions`.

## Test

`scripts/check-all.sh` runs everything below and stops at the first failure.

```sh
pnpm build        # type check and frontend assets; native checks need these
pnpm test
pnpm test:e2e
node --test scripts/*.test.mjs
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

CI runs the same commands ([`ci.yml`](.github/workflows/ci.yml)). Some Rust tests create real assertions and processes on macOS and check them through IOKit; they clean up after themselves.

## Layout

```
src-tauri/src/
  commands.rs, cli.rs    command surface and `wick start|stop`
  runtime.rs             state, ticker, persistence, reconciling the assertion
  tray.rs, popover.rs    menu bar icon and popover placement
  updates.rs             update check and install
  core/
    assertion.rs         IOKit power assertions
    session.rs           sessions and time left
    candle.rs            tray icon rendering
    triggers.rs          app and command triggers
    processes.rs         process scan
    others.rs            other apps keeping the Mac awake
    power.rs, lid.rs     battery guard, lid state
    settings.rs, store.rs
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
