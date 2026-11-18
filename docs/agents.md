# Agents

Design for v0.9.0. Wick keeps the computer awake while coding agents work, and lets it sleep when they finish or wait for you.

A "while `claude` is running" trigger keeps the computer up all night while the agent sits at a prompt. Hooks tell Wick when an agent starts working and when it stops, so the hold matches the work.

## What was proven first (2026-11-17, x86_64 Linux)

- Claude Code 2.1.326, `claude -p` with `--settings` hooks: `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `Stop`, `SessionEnd` fired in order, each with `session_id`, `transcript_path`, `cwd`, `hook_event_name` as JSON on stdin.
- Codex 0.166: `notify = [program, ...]` in `config.toml` fired once at turn end, payload as the last argv argument. `hooks.json` did not fire under `codex exec` at first. The Codex spike (11.1, below) found why: Codex runs a hook only once it is trusted.

## Holds

A hold is a JSON file in `status::holds_directory()`: `<status dir>/agents/` (the directory `status.json` lives in) on macOS and Windows, and always `~/.local/share/com.abulkosim.wick/agents/` on Linux, whatever `XDG_DATA_HOME` says, since a desktop launcher and a terminal often set it differently. One file per agent session, named `<agent>-<session>.json`, where both parts match `[A-Za-z0-9_-]{1,64}` (anything else is hashed to hex). Written atomically (temp file + rename). Files survive Wick not running and are read when it starts.

```json
{
  "v": 1,
  "agent": "claude",
  "session": "d442c2f9-7e86-4aa5-a5ba-c14469f84e18",
  "label": "wick",
  "cwd": "/Users/me/work/wick",
  "pid": 4242,
  "pidStart": 1794940845,
  "state": "working",
  "since": 1794940846,
  "updated": 1794940852
}
```

- `agent`: `claude`, `codex` or `other`. Display names: Claude Code, Codex, and for `other` the label.
- `label`: basename of `cwd`, at most 64 chars; for `wick hold <name>`, the name.
- `pid`: the agent's own process. The CLI walks up from its parent, skipping shells (`sh`, `bash`, `zsh`, `dash`, `fish`, `cmd.exe`, `powershell.exe`, `pwsh`, `node` when its parent matches the agent) until it reaches the agent; `null` if not found. `pidStart` (process start time, seconds) guards against pid reuse.
- `state`: `working` holds the computer awake; `waiting` doesn't.
- `since`: when the current state began; `updated`: last event.

Wick drops a hold, deleting its file, when its pid is gone or has a different start time, or when it has had no event for 24 h. A hold with `pid: null` expires 6 h after `updated`. A working hold whose process is suspended (Ctrl+Z; `T` in `/proc/<pid>/stat`, `SSTOP` on macOS) counts as waiting until it resumes; the file stays as it is.

## Events

| Agent | Event | Effect |
| --- | --- | --- |
| Claude Code | `UserPromptSubmit`, `PreToolUse`, `PostToolUse` | working |
| Claude Code | `Stop`, `Notification` (permission or idle prompt), `PermissionRequest` | waiting |
| Claude Code | `SessionEnd` | remove |
| Codex | `UserPromptSubmit`, `PreToolUse`, `PostToolUse` | working |
| Codex | `PermissionRequest`, `Stop`, `Interrupt` | waiting |
| Codex | `SessionEnd` | remove (can fire twice; removing is idempotent) |

`SubagentStop` and `SessionStart` change nothing.

Known gaps, accepted: approving a permission request fires nothing in either agent, so an approved long command shows "waiting" until its `PostToolUse`. Treating `PermissionRequest` as working instead would keep the computer up all night when nobody answers. Esc in Claude Code fires no `Stop`; the hold stays working until the next event, an idle-prompt `Notification`, or the process exiting.

## Codex mechanism (11.1 spike, codex-cli 0.166, daemon 0.169)

Hooks, not `notify`. `notify` fires only at turn end, never on Esc, an approval wait or session start or end; in the TUI it also fires for a hidden title-generation thread; and it is a single slot, so taking it replaces the user's own notifier.

Hooks are on by default (`codex features list`: `hooks stable true`), but Codex runs a hook only once it is trusted. Trust lives in `config.toml`:

```toml
[hooks.state."/Users/me/.codex/hooks.json:stop:<group>:0"]
trusted_hash = "sha256:<hex>"
```

The hash is SHA-256 over compact JSON with sorted keys and UTF-8 text as is: `{"event_name":"<snake_event>","hooks":[{"async":false,"command":"<cmd>","timeout":<t>,"type":"command"}]}`, plus `"matcher"` only when set. `<t>` is the effective timeout (default 600; `SessionEnd` and `Interrupt` are clamped to 1). The formula was checked against `currentHash` from `codex app-server` `hooks/list` on 0.166 and 0.169. An untrusted hook is skipped silently under `codex exec`; the TUI shows "Hooks need review". With Wick's computed trust, `codex exec` fired every hook with no bypass flag and the TUI opened with no review screen. The Connect dialog says so: "Adds hooks to ~/.codex/hooks.json and marks them trusted in ~/.codex/config.toml."

Where hooks run: under `codex exec`, the exec process (the hold's pid). In the TUI, the shared `codex app-server` daemon runs them; it is detached, outlives the TUI and serves every TUI session, so a TUI hold follows the daemon's pid. `/quit` fires no `SessionEnd`; the daemon fires it later (seen 1–2 min after). `kill -9` of `codex exec` fires nothing; the pid check drops the hold.

## CLI

The hook path must not start Tauri, print to stdout (Claude Code adds `UserPromptSubmit` stdout to the prompt), or fail the agent: it always exits 0 within 100 ms, and logs problems to stderr only with `WICK_DEBUG=1`.

- `wick hook claude` reads hook JSON from stdin (at most 1 MB).
- `wick hook codex [json]` reads hook JSON from stdin. A `notify` payload as the last argument (`agent-turn-complete` → waiting) is still understood, but Connect doesn't use it.
- `wick hold <name> [--pid N]`, `wick release <name>`: holds for scripts and other agents (`agent: other`).
- `wick run -- <command> [args]`: holds while the command runs, releases after, exits with its code. Like `caffeinate -i <command>` on every platform. Ctrl+C reaches the command through the terminal and Wick waits for it; Wick passes on SIGTERM but not SIGINT, which the command already gets (a second one makes many tools quit at once). A command killed by HUP, INT, KILL, PIPE, ALRM or TERM ends Wick with the same signal; any other (ABRT, SEGV) makes Wick exit with 128 + the signal instead of crashing too. On Windows a bare name that isn't an `.exe` (`npm`, `pnpm`) is looked up with `PATHEXT`, so `.cmd` shims run.
- Windows: release builds are GUI-subsystem programs, so `cmd` and PowerShell don't wait for `wick run`, `wick hold` or `wick status` and show none of their output. Agents run hooks through pipes and wait, so hooks work. Until a console `wick` ships (plan 11.9), use `Start-Process -Wait -NoNewWindow` or Git Bash for `wick run`.

## App

- Read `agents/` on every trigger tick (5 s) and when the popover opens. Working holds keep the computer awake like a trigger, after the triggers themselves, and not while the battery guard applies. The hold phrase (`heldBy`): "Claude Code works", or "2 agents work" for more than one; the popover shows "While Claude Code works" and the project in the Agents card, and a screen reader hears "Awake while Claude Code works in wick".
- The ticker stays at 5 s while holds exist, or while agents are on and the `agents/` folder exists. Connect makes the folder, so the first hook is seen within 5 s. Without it (`wick hold` before any connect) the first hold can wait for the idle tick, up to a minute.
- Stop with no session pauses working holds until they stop working: "Paused until Claude Code finishes", or "the agents finish". Resume clears the pause. Saving triggers keeps agent pauses (keys `agent:<id>`, which no trigger id can match).
- Snapshot field `agents`: `[{ "id": "claude-<session>", "agent": "claude", "name": "Claude Code", "label": "wick", "state": "working", "since": 1794940846, "paused": false }]`, at most 16, working first then newest. `paused`: a Stop holds it off. `heldByAgents`: `heldBy` names agents, not a trigger; the frontend never rebuilds the phrase. Zod-validated like every payload.
- Setting `agents: { enabled: bool }` (default true). Off: holds are read and shown but don't keep the computer awake.
- Notification when the last working hold ends and nothing else keeps the computer awake, under the trigger-ended notification setting, labeled "When a trigger or agent ends": "Claude Code finished in wick. Your Mac can sleep again." or "Claude Code is waiting for you in wick." ("PC" on Windows and "computer" on Linux, like every other notification.)

## Connecting

Settings → Agents lists Claude Code and Codex: Connected / Not connected / Not installed, and Connect / Disconnect.

- Connect shows what will change ("Adds hooks to ~/.claude/settings.json. The file is backed up first.") and asks first.
- Writes the absolute path of the running Wick binary (`"<exe>" hook claude`) for each event above. Keeps every other key and hook, key order and 2-space indent. Backs the file up once to `settings.json.wick-backup` before the first change. Running it twice changes nothing.
- Disconnect removes only Wick's entries (command contains ` hook claude` and ends in a Wick binary path).
- Shows "Reconnect" when the recorded binary path no longer exists (app moved).
- A file that isn't valid JSON is left alone, with an error naming the file.

As built (11.4, `src-tauri/src/core/connect.rs`):

- `CLAUDE_CONFIG_DIR` and `CODEX_HOME` come from Wick's own environment. An app opened from Finder, the Dock or a desktop launcher doesn't see variables set in `~/.zshrc`; the Connect confirmation says so ("Wick reads CLAUDE_CONFIG_DIR from its own environment, not from your shell profile."). On macOS, `launchctl setenv` and restarting Wick makes it see them.
- Claude Code: `$CLAUDE_CONFIG_DIR/settings.json`, else `~/.claude/settings.json`. One group per event, `{ "hooks": [{ "type": "command", "command": "\"<exe>\" hook claude" }] }`, no matcher: a real `claude -p --model haiku` (2.1.326) fired `UserPromptSubmit`, `PreToolUse`, `PermissionRequest`, `PostToolUse`, `Stop` and `SessionEnd` into it, and the user's own `Stop` hook still ran.
- Codex: `$CODEX_HOME/hooks.json` (or `config.toml` when it already holds `[[hooks.<Event>]]`), one group per event with `timeout` 5 (1 for `Interrupt` and `SessionEnd`), plus `[hooks.state."<hooks file>:<event>:<group>:0"] trusted_hash` in `config.toml`, so Codex runs them without its review screen. Codex resolves a symlinked `CODEX_HOME` before naming the file in the key, but not `~/.codex`; Wick does the same. A real `codex exec` (0.166) fired every hook with no bypass flag. Codex shows "Connected" only when every hook is trusted with the current hash; otherwise "Codex doesn’t trust Wick’s hooks yet. Connect again." `[features] hooks = false` shows "Hooks are turned off in ~/.codex/config.toml.": as an error with no Wick hooks (Connect then refuses: "… Turn them on, then connect."), or as a note on a connected row while Wick's hooks are there, so Disconnect still removes them.
- Group-level: a group is Wick's when its only handler runs `"<wick binary>" hook <agent>`. Connect keeps Wick's first group in place (rewritten if the path changed), drops other Wick groups, appends where none exists. Disconnect removes Wick's groups, then the event keys and `hooks` object it emptied, and moves the trust keys of later Codex groups down so they stay trusted.
- Files: written atomically through a symlink to its target, keeping key order (duplicates included), 2-space indent and a trailing newline; TOML through `toml_edit`, keeping comments. Backed up once to `<file>.wick-backup` before the first change. A file Wick created (no backup) is removed when Disconnect empties it. A file changed while Wick was saving it is left alone: "… changed while Wick was saving it. Try again."
- `<exe>`: the running binary, or `$APPIMAGE` for an AppImage. A translocated macOS app (run from Downloads) gets "Move Wick to Applications, open it from there, then connect." On Windows the path uses forward slashes inside double quotes, which `cmd` and Git Bash both run; a path with `$`, `` ` `` or `%`, which they would expand, gets "Wick’s location has characters agents can’t run. Move Wick and try again."

## Out of scope for v0.9

Agents without hooks (gemini, aider, opencode, cursor-agent) are detected only through `wick hold`/`wick run` or the existing process trigger.

## Commands (frontend ↔ Rust)

- `agent_connections()` → `[{ "agent": "claude" | "codex", "name": "Claude Code", "status": "connected" | "disconnected" | "stale" | "missing" | "error", "file": "~/.claude/settings.json", "error": null | "…" }]`. `error` is set with `error`, and on `connected` it is a note shown in place of "Connected" (Codex hooks turned off). `missing`: the agent isn't installed (its config directory doesn't exist). `stale`: hooks point at a Wick binary that no longer exists.
- `connect_agent(agent)` and `disconnect_agent(agent)` return the same list.
- `agents.enabled` is saved through the existing settings command like every other setting.
- One Zod schema per payload in `src/lib/model.ts`. `tests/fixtures/state.json` and `connections.json` are written by a Rust test (`WICK_UPDATE_FIXTURES=1`) and parsed by `tests/unit/fixtures.test.ts`, so a field added on one side fails the other's tests.

## Measured (2026-11-17, x86_64 Linux, debug build)

The app ran under `xvfb-run -a dbus-run-session` with a temporary HOME. Claude Code and Codex were connected through `connect::connect` (the code the Connect button runs). State was polled every 0.5 s from `systemd-inhibit --list`, `wick status` and the hold files.

- `wick hook claude < payload`: 30–49 ms over 20 runs (median 34), 0 bytes of stdout, exit 0.
- `claude -p --model haiku` (2.1.326) running `sleep 20`: hold working 4 s after launch, inhibitor "Wick: awake while Claude Code works" 2.7 s later, held through the command; hold removed at `SessionEnd` and the inhibitor gone 1.4 s after.
- `kill -9` of a working `claude`: hold and inhibitor gone 1.7 s later.
- `Stop` alone (the session stays open, as in an interactive `claude`), sent with `wick hook claude`: hold waiting, inhibitor gone within 5.1 s (one tick); `wick status` `awake: false`.
- `codex exec` (0.166) running `sleep 20`: hold and inhibitor together (5 s tick); `Stop` → waiting, `SessionEnd` → removed 0.6 s later; inhibitor gone 2.2 s after.
- `kill -9` of a working `codex exec`: hold and inhibitor gone 1.9 s later.
- `scripts/linux-smoke.sh` with the hold checks (`wick hold`, `wick release`, `wick run`): passed in 49 s.
