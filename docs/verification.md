# Verification

What was checked for each release, newest first.

## Native checks (every release)

- `wick start 30` → `pmset -g assertions` shows `Wick: awake until HH:MM`; `wick stop` removes it.
- Quit and `kill -9` while awake leave no `Wick:` assertion.
- A process trigger holds while the process runs, Stop pauses it, and it releases on exit.
- Popover opens at full height in light and dark, with the theme following Wick's setting.
