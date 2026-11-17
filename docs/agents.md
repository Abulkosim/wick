# Agents

Design for v0.9.0. Wick keeps the computer awake while coding agents work, and lets it sleep when they finish or wait for you.

A "while `claude` is running" trigger keeps the computer up all night while the agent sits at a prompt. Hooks tell Wick when an agent starts working and when it stops, so the hold matches the work.

## What was proven first (2026-11-17, x86_64 Linux)

- Claude Code 2.1.326, `claude -p` with `--settings` hooks: `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `Stop`, `SessionEnd` fired in order, each with `session_id`, `transcript_path`, `cwd`, `hook_event_name` as JSON on stdin.
- Codex 0.166: `notify = [program, ...]` in `config.toml` fired once at turn end, payload as the last argv argument. `hooks.json` (events `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PermissionRequest`, `Stop`; feature flag `hooks`) did not fire under `codex exec` in a project or a separate `CODEX_HOME`; open question for the Codex spike.

## Holds

A hold is a JSON file in `<status dir>/agents/` (the directory `status.json` lives in, from `status::default_directory()`). One file per agent session, named `<agent>-<session>.json`, where both parts match `[A-Za-z0-9_-]{1,64}` (anything else is hashed to hex). Written atomically (temp file + rename). Files survive Wick not running and are read when it starts.

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

Wick drops a hold, deleting its file, when its pid is gone or has a different start time, or when it has had no event for 24 h. A hold with `pid: null` expires 6 h after `updated`.

## Events

| Agent | Event | Effect |
| --- | --- | --- |
| Claude Code | `UserPromptSubmit`, `PreToolUse`, `PostToolUse` | working |
| Claude Code | `Stop`, `Notification` (permission or idle prompt), `PermissionRequest` | waiting |
| Claude Code | `SessionEnd` | remove |
| Codex | per the spike: hooks if they can be made reliable, else `notify` (turn end → waiting) plus whatever marks start | |

`SubagentStop` and `SessionStart` change nothing.

## CLI

The hook path must not start Tauri, print to stdout (Claude Code adds `UserPromptSubmit` stdout to the prompt), or fail the agent: it always exits 0 within 100 ms, and logs problems to stderr only with `WICK_DEBUG=1`.

- `wick hook claude` reads hook JSON from stdin (at most 1 MB).
- `wick hook codex [json]` reads JSON from the last argument (notify) or stdin (hooks).
- `wick hold <name> [--pid N]`, `wick release <name>`: holds for scripts and other agents (`agent: other`).
- `wick run -- <command> [args]`: holds while the command runs, releases after, exits with its code. Like `caffeinate -i <command>` on every platform.

## App

- Read `agents/` on every trigger tick (5 s) and when the popover opens. Working holds keep the computer awake like a trigger. The hold phrase: "Claude Code is working in wick", or "2 agents are working" for more than one.
- Snapshot field `agents`: `[{ "id": "claude-<session>", "agent": "claude", "name": "Claude Code", "label": "wick", "state": "working", "since": 1794940846 }]`, at most 16, working first then newest. Zod-validated like every payload.
- Setting `agents: { enabled: bool }` (default true). Off: holds are read and shown but don't keep the computer awake.
- Notification when the last working hold ends and nothing else keeps the computer awake, under the existing trigger-ended notification setting: "Claude Code finished in wick. Your Mac can sleep again." or "Claude Code is waiting for you in wick." ("computer" on Windows and Linux).

## Connecting

Settings → Agents lists Claude Code and Codex: Connected / Not connected / Not installed, and Connect / Disconnect.

- Connect shows what will change ("Adds hooks to ~/.claude/settings.json.") and asks first.
- Writes the absolute path of the running Wick binary (`"<exe>" hook claude`) for each event above. Keeps every other key and hook, key order and 2-space indent. Backs the file up once to `settings.json.wick-backup` before the first change. Running it twice changes nothing.
- Disconnect removes only Wick's entries (command contains ` hook claude` and ends in a Wick binary path).
- Shows "Reconnect" when the recorded binary path no longer exists (app moved).
- A file that isn't valid JSON is left alone, with an error naming the file.

## Out of scope for v0.9

Agents without hooks (gemini, aider, opencode, cursor-agent) are detected only through `wick hold`/`wick run` or the existing process trigger.

## Commands (frontend ↔ Rust)

- `agent_connections()` → `[{ "agent": "claude" | "codex", "name": "Claude Code", "status": "connected" | "disconnected" | "stale" | "missing" | "error", "file": "~/.claude/settings.json", "error": null | "…" }]`. `missing`: the agent isn't installed (its config directory doesn't exist). `stale`: hooks point at a Wick binary that no longer exists.
- `connect_agent(agent)` and `disconnect_agent(agent)` return the same list.
- `agents.enabled` is saved through the existing settings command like every other setting.
