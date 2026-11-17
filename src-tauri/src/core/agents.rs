//! Agent holds: one JSON file per agent session in `<status dir>/agents/`, written by
//! `wick hook`, `wick hold` and `wick run`, and read by the app. See docs/agents.md.

use super::processes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

pub const DIRECTORY: &str = "agents";
/// The most a hook reads from stdin or one argument.
pub const MAX_PAYLOAD: usize = 1024 * 1024;
/// The most holds the snapshot carries.
pub const SHOWN: usize = 16;
const VERSION: u32 = 1;
const MAX_FILE: u64 = 64 * 1024;
const MAX_LABEL: usize = 64;
/// A hold with no event for this long is dropped, whatever its process.
const STALE_SECONDS: i64 = 24 * 3600;
/// A hold without a process to watch is dropped after this long without an event.
const UNWATCHED_SECONDS: i64 = 6 * 3600;
/// Start times read at different moments can differ by a second of rounding.
const START_SLACK_SECONDS: i64 = 2;
/// The ticker's id for "agents hold the computer awake".
pub const HOLDER: &str = "agents";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Agent {
    Claude,
    Codex,
    Other,
}

impl Agent {
    pub fn key(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    /// The agent's own process, by its normalized command name.
    fn runs_as(self, name: &str) -> bool {
        match self {
            Self::Claude => name == "claude",
            Self::Codex => name == "codex" || name.starts_with("codex-"),
            Self::Other => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Working,
    Waiting,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hold {
    pub v: u32,
    pub agent: Agent,
    pub session: String,
    pub label: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub pid_start: Option<i64>,
    pub state: State,
    pub since: i64,
    pub updated: i64,
}

impl Hold {
    pub fn id(&self) -> String {
        id(self.agent, &self.session)
    }

    /// "Claude Code", "Codex", or the label for scripts and other agents.
    pub fn name(&self) -> &str {
        match self.agent {
            Agent::Claude => "Claude Code",
            Agent::Codex => "Codex",
            Agent::Other => &self.label,
        }
    }

    /// "Claude Code is working in wick"; "build is working" for `wick hold build`.
    fn working(&self) -> String {
        match self.agent {
            Agent::Other => format!("{} is working", self.label),
            _ if self.label.is_empty() => format!("{} is working", self.name()),
            _ => format!("{} is working in {}", self.name(), self.label),
        }
    }

    pub fn is_working(&self) -> bool {
        self.state == State::Working
    }
}

/// What an event does to its session's hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Working,
    Waiting,
    Remove,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub session: String,
    pub cwd: Option<String>,
    pub effect: Effect,
}

fn text(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| payload.get(*key)?.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(String::from)
}

/// A notification that asks for the owner: a permission prompt or the idle prompt. Older
/// Claude Code versions send only the message.
fn asks_for_owner(payload: &Value) -> bool {
    match payload.get("notification_type").and_then(Value::as_str) {
        Some(kind) => matches!(kind, "permission_prompt" | "idle_prompt"),
        None => text(payload, &["message"]).is_some_and(|message| {
            let message = message.to_lowercase();
            message.contains("permission") || message.contains("waiting for your input")
        }),
    }
}

/// Hook events, named the same by Claude Code and Codex. `SessionStart` and `SubagentStop`
/// change nothing.
fn hook_effect(event: &str, payload: &Value) -> Option<Effect> {
    match event {
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" => Some(Effect::Working),
        "Stop" | "PermissionRequest" => Some(Effect::Waiting),
        "Notification" => asks_for_owner(payload).then_some(Effect::Waiting),
        "SessionEnd" => Some(Effect::Remove),
        _ => None,
    }
}

/// A Claude Code hook payload from stdin.
pub fn claude_event(payload: &Value) -> Option<Event> {
    let effect = hook_effect(&text(payload, &["hook_event_name"])?, payload)?;
    Some(Event {
        session: text(payload, &["session_id"])?,
        cwd: text(payload, &["cwd"]),
        effect,
    })
}

/// A Codex hook payload (stdin, named like Claude Code's) or a `notify` payload (the last
/// argument), whose `agent-turn-complete` means the turn ended and Codex waits for the owner.
pub fn codex_event(payload: &Value) -> Option<Event> {
    let effect = match text(payload, &["hook_event_name"]) {
        Some(event) => hook_effect(&event, payload)?,
        None => match text(payload, &["type"])?.as_str() {
            "agent-turn-complete" => Effect::Waiting,
            _ => return None,
        },
    };
    Some(Event {
        session: text(
            payload,
            &["session_id", "thread-id", "thread_id", "conversation_id"],
        )?,
        cwd: text(payload, &["cwd"]),
        effect,
    })
}

/// FNV-1a, so an unsafe session id always maps to the same file.
fn hash(value: &str) -> String {
    let hash = value.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")
}

/// The session as it appears in file names and ids: as is when it is 1–64 of
/// `[A-Za-z0-9_-]`, else hashed to hex.
fn safe(value: &str) -> String {
    let plain = (1..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if plain {
        value.to_string()
    } else {
        hash(value)
    }
}

pub fn id(agent: Agent, session: &str) -> String {
    format!("{}-{}", agent.key(), safe(session))
}

pub fn path(directory: &Path, agent: Agent, session: &str) -> PathBuf {
    directory.join(format!("{}.json", id(agent, session)))
}

pub fn directory(status: &Path) -> PathBuf {
    status.join(DIRECTORY)
}

fn clip(value: &str) -> String {
    value.chars().take(MAX_LABEL).collect()
}

/// The last part of a path, on any platform's separators.
pub fn basename(path: &str) -> String {
    let name = path
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default();
    clip(name)
}

/// What one event changes, before the hold is written.
pub struct Update {
    pub agent: Agent,
    pub session: String,
    pub label: String,
    pub cwd: Option<String>,
    pub state: State,
    pub pid: Option<u32>,
    pub pid_start: Option<i64>,
}

/// The hold after an event. `since` moves only when the state changes.
pub fn next(existing: Option<&Hold>, update: Update, now: i64) -> Hold {
    let since = existing
        .filter(|hold| hold.state == update.state)
        .map_or(now, |hold| hold.since);
    Hold {
        v: VERSION,
        agent: update.agent,
        session: update.session,
        label: clip(&update.label),
        cwd: update.cwd,
        pid: update.pid,
        pid_start: update.pid.and(update.pid_start),
        state: update.state,
        since,
        updated: now,
    }
}

enum Loaded {
    Hold(Box<Hold>),
    /// Written by a newer Wick; left alone.
    Newer,
    Invalid,
}

fn load(path: &Path) -> Loaded {
    let Ok(file) = fs::File::open(path) else {
        return Loaded::Invalid;
    };
    let mut bytes = Vec::new();
    if file.take(MAX_FILE + 1).read_to_end(&mut bytes).is_err() || bytes.len() as u64 > MAX_FILE {
        return Loaded::Invalid;
    }
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return Loaded::Invalid;
    };
    match value.get("v").and_then(Value::as_u64) {
        Some(version) if version > u64::from(VERSION) => return Loaded::Newer,
        Some(version) if version == u64::from(VERSION) => {}
        _ => return Loaded::Invalid,
    }
    serde_json::from_value(value).map_or(Loaded::Invalid, |hold| Loaded::Hold(Box::new(hold)))
}

/// The hold in `path`, if it is one this version reads.
pub fn read(path: &Path) -> Option<Hold> {
    match load(path) {
        Loaded::Hold(hold) => Some(*hold),
        _ => None,
    }
}

/// Writes the whole file under a temporary name and renames it into place, so a reader sees
/// the old hold or the new one, never part of one.
pub fn write(directory: &Path, hold: &Hold) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer(&mut file, hold)?;
    file.write_all(b"\n")?;
    file.persist(path(directory, hold.agent, &hold.session))
        .map_err(|error| error.error)?;
    Ok(())
}

/// Removes a hold; one that is already gone is fine.
pub fn remove(directory: &Path, agent: Agent, session: &str) -> io::Result<()> {
    match fs::remove_file(path(directory, agent, session)) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Applies a hook event. `owner` finds the agent's process, given the hold so far.
pub fn record(
    directory: &Path,
    agent: Agent,
    event: Event,
    now: i64,
    owner: impl FnOnce(Option<&Hold>) -> (Option<u32>, Option<i64>),
) -> io::Result<()> {
    let state = match event.effect {
        Effect::Remove => return remove(directory, agent, &event.session),
        Effect::Working => State::Working,
        Effect::Waiting => State::Waiting,
    };
    let existing = read(&path(directory, agent, &event.session));
    let (pid, pid_start) = owner(existing.as_ref());
    let cwd = event
        .cwd
        .or_else(|| existing.as_ref().and_then(|hold| hold.cwd.clone()));
    let label = cwd.as_deref().map(basename).unwrap_or_default();
    let hold = next(
        existing.as_ref(),
        Update {
            agent,
            session: event.session,
            label,
            cwd,
            state,
            pid,
            pid_start,
        },
        now,
    );
    write(directory, &hold)
}

/// Whether Wick should drop a hold: its process is gone or was replaced, it has had no event
/// for 24 h, or, with no process to watch, for 6 h.
pub fn expired(hold: &Hold, now: i64, live: impl Fn(u32, Option<i64>) -> bool) -> bool {
    let quiet = now - hold.updated;
    match hold.pid {
        _ if quiet > STALE_SECONDS => true,
        None => quiet > UNWATCHED_SECONDS,
        Some(pid) => !live(pid, hold.pid_start),
    }
}

/// Whether `pid` runs and is the same process that started at `start`.
pub fn alive(pid: u32, start: Option<i64>) -> bool {
    processes::alive(pid)
        && match (start, processes::start_time(pid)) {
            (Some(expected), Some(actual)) => (expected - actual).abs() <= START_SLACK_SECONDS,
            _ => true,
        }
}

/// Every current hold. Expired and unreadable files are deleted; temporary files from a write
/// in progress and files from a newer Wick are skipped.
pub fn read_all(directory: &Path, now: i64, live: impl Fn(u32, Option<i64>) -> bool) -> Vec<Hold> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut holds = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(stem) = name
            .to_str()
            .filter(|name| !name.starts_with('.'))
            .and_then(|name| name.strip_suffix(".json"))
        else {
            continue;
        };
        let path = entry.path();
        match load(&path) {
            Loaded::Hold(hold) if hold.id() == stem && !expired(&hold, now, &live) => {
                holds.push(*hold)
            }
            Loaded::Newer => {}
            _ => {
                let _ = fs::remove_file(&path);
            }
        }
    }
    holds
}

/// Lower-case command name without directory or `.exe`.
fn normalized(name: &str) -> String {
    let name = basename(name).to_lowercase();
    match name.strip_suffix(".exe") {
        Some(stem) => stem.to_string(),
        None => name,
    }
}

const SHELLS: [&str; 8] = [
    "sh",
    "bash",
    "zsh",
    "dash",
    "fish",
    "cmd",
    "powershell",
    "pwsh",
];

/// Walks up from `from` (the hook's parent) to the agent's own process, skipping the shells
/// that run hook commands and a `node` whose parent is the agent. `lookup` gives a process's
/// parent and command name.
pub fn find_agent(
    from: u32,
    agent: Agent,
    lookup: impl Fn(u32) -> Option<(u32, String)>,
) -> Option<u32> {
    let mut pid = from;
    for _ in 0..16 {
        if pid == 0 {
            return None;
        }
        let (parent, name) = lookup(pid)?;
        let name = normalized(&name);
        if agent.runs_as(&name) {
            return Some(pid);
        }
        let skip = SHELLS.contains(&name.as_str())
            || (name == "node"
                && lookup(parent).is_some_and(|(_, name)| agent.runs_as(&normalized(&name))));
        if !skip || parent == pid {
            return None;
        }
        pid = parent;
    }
    None
}

/// This process's parent and the agent above it, with its start time.
#[cfg(unix)]
pub fn agent_process(agent: Agent) -> Option<(u32, Option<i64>)> {
    let lookup = |pid: u32| {
        let (parent, name) = processes::info(i32::try_from(pid).ok()?)?;
        Some((u32::try_from(parent).ok()?, name))
    };
    let pid = find_agent(std::os::unix::process::parent_id(), agent, lookup)?;
    Some((pid, processes::start_time(pid)))
}

#[cfg(windows)]
pub fn agent_process(agent: Agent) -> Option<(u32, Option<i64>)> {
    let table = processes::parents();
    let lookup = |pid: u32| table.get(&pid).cloned();
    let (parent, _) = lookup(std::process::id())?;
    let pid = find_agent(parent, agent, lookup)?;
    Some((pid, processes::start_time(pid)))
}

#[cfg(not(any(unix, windows)))]
pub fn agent_process(_agent: Agent) -> Option<(u32, Option<i64>)> {
    None
}

/// The hold's place in the popover.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub id: String,
    pub agent: Agent,
    pub name: String,
    pub label: String,
    pub state: State,
    pub since: i64,
}

/// Working holds first, then the newest; at most `SHOWN`.
pub fn views(holds: &[Hold]) -> Vec<View> {
    let mut sorted: Vec<&Hold> = holds.iter().collect();
    sorted.sort_by(|a, b| {
        b.is_working()
            .cmp(&a.is_working())
            .then(b.since.cmp(&a.since))
            .then_with(|| a.id().cmp(&b.id()))
    });
    sorted
        .into_iter()
        .take(SHOWN)
        .map(|hold| View {
            id: hold.id(),
            agent: hold.agent,
            name: hold.name().to_string(),
            label: hold.label.clone(),
            state: hold.state,
            since: hold.since,
        })
        .collect()
}

/// What keeps the computer awake: "Claude Code is working in wick", or "2 agents are working".
pub fn phrase(working: &[&Hold]) -> Option<String> {
    match working {
        [] => None,
        [hold] => Some(hold.working()),
        many => Some(format!("{} agents are working", many.len())),
    }
}

/// What a pause waits for: "Claude Code stops working", or "the agents stop working".
pub fn until(paused: &[&Hold]) -> Option<String> {
    match paused {
        [] => None,
        [hold] => Some(format!("{} stops working", hold.name())),
        _ => Some("the agents stop working".into()),
    }
}

/// The notification when the last working hold ends: finished (its hold is gone) or waiting.
pub fn ended(hold: &Hold, gone: bool, can_sleep: &str) -> String {
    let place = match hold.agent {
        Agent::Other => String::new(),
        _ if hold.label.is_empty() => String::new(),
        _ => format!(" in {}", hold.label),
    };
    if gone {
        format!("{} finished{place}. {can_sleep}", hold.name())
    } else {
        format!("{} is waiting for you{place}.", hold.name())
    }
}

/// The key a Stop pauses a working hold under, apart from trigger ids.
pub fn pause_key(hold: &Hold) -> String {
    format!("agent:{}", hold.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn claude(event: &str) -> Value {
        json!({
            "session_id": "d442c2f9-7e86-4aa5-a5ba-c14469f84e18",
            "transcript_path": "/home/me/.claude/projects/x.jsonl",
            "cwd": "/home/me/work/wick",
            "hook_event_name": event,
        })
    }

    fn hold(agent: Agent, session: &str, state: State, since: i64) -> Hold {
        Hold {
            v: VERSION,
            agent,
            session: session.into(),
            label: "wick".into(),
            cwd: Some("/home/me/work/wick".into()),
            pid: Some(4242),
            pid_start: Some(1_000),
            state,
            since,
            updated: since,
        }
    }

    #[test]
    fn claude_events_map_to_effects() {
        let effect = |event: &str| claude_event(&claude(event)).map(|event| event.effect);
        for event in ["UserPromptSubmit", "PreToolUse", "PostToolUse"] {
            assert_eq!(effect(event), Some(Effect::Working), "{event}");
        }
        for event in ["Stop", "PermissionRequest"] {
            assert_eq!(effect(event), Some(Effect::Waiting), "{event}");
        }
        assert_eq!(effect("SessionEnd"), Some(Effect::Remove));
        for event in [
            "SessionStart",
            "SubagentStop",
            "PreCompact",
            "Notification",
            "",
        ] {
            assert_eq!(effect(event), None, "{event}");
        }
        let event = claude_event(&claude("Stop")).unwrap();
        assert_eq!(event.session, "d442c2f9-7e86-4aa5-a5ba-c14469f84e18");
        assert_eq!(event.cwd.as_deref(), Some("/home/me/work/wick"));
        assert_eq!(claude_event(&json!({ "hook_event_name": "Stop" })), None);
        assert_eq!(claude_event(&json!([1, 2])), None);
    }

    #[test]
    fn only_notifications_that_ask_for_the_owner_wait() {
        let notification = |extra: Value| {
            let mut payload = claude("Notification");
            payload
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            claude_event(&payload).map(|event| event.effect)
        };
        let waiting = Some(Effect::Waiting);
        assert_eq!(
            notification(json!({ "notification_type": "permission_prompt" })),
            waiting
        );
        assert_eq!(
            notification(json!({ "notification_type": "idle_prompt" })),
            waiting
        );
        assert_eq!(
            notification(json!({ "notification_type": "auth_success", "message": "permission" })),
            None
        );
        assert_eq!(
            notification(json!({ "message": "Claude needs your permission to use Bash" })),
            waiting
        );
        assert_eq!(
            notification(json!({ "message": "Claude is waiting for your input" })),
            waiting
        );
        assert_eq!(notification(json!({ "message": "Done" })), None);
    }

    #[test]
    fn codex_hooks_and_notify_map_to_effects() {
        let hook = |event: &str| {
            codex_event(&json!({
                "session_id": "019a-thread",
                "cwd": "/home/me/work/wick",
                "hook_event_name": event,
            }))
            .map(|event| event.effect)
        };
        assert_eq!(hook("UserPromptSubmit"), Some(Effect::Working));
        assert_eq!(hook("PreToolUse"), Some(Effect::Working));
        assert_eq!(hook("PostToolUse"), Some(Effect::Working));
        assert_eq!(hook("PermissionRequest"), Some(Effect::Waiting));
        assert_eq!(hook("Stop"), Some(Effect::Waiting));
        assert_eq!(hook("SessionStart"), None);
        let notify = codex_event(&json!({
            "type": "agent-turn-complete",
            "thread-id": "019a7c3e-1d2b-7c40-9e21-5a6b7c8d9e0f",
            "turn-id": "12345",
            "cwd": "/home/me/work/wick",
            "input-messages": ["Rename foo to bar"],
            "last-assistant-message": "Done.",
        }))
        .unwrap();
        assert_eq!(notify.effect, Effect::Waiting);
        assert_eq!(notify.session, "019a7c3e-1d2b-7c40-9e21-5a6b7c8d9e0f");
        assert_eq!(
            codex_event(&json!({ "type": "agent-turn-started", "thread-id": "t" })),
            None
        );
        assert_eq!(codex_event(&json!({ "type": "agent-turn-complete" })), None);
    }

    #[test]
    fn unsafe_session_ids_are_hashed_in_file_names() {
        let directory = Path::new("/x");
        assert_eq!(
            path(directory, Agent::Claude, "d442c2f9-7e86_4aa5"),
            Path::new("/x/claude-d442c2f9-7e86_4aa5.json")
        );
        for unsafe_id in ["../../etc/passwd", "a b", "", &"a".repeat(65), "é"] {
            let id = id(Agent::Codex, unsafe_id);
            let part = id.strip_prefix("codex-").unwrap();
            assert_eq!(part.len(), 16, "{unsafe_id}");
            assert!(part.bytes().all(|byte| byte.is_ascii_hexdigit()));
        }
        assert_eq!(id(Agent::Other, "a/b"), id(Agent::Other, "a/b"));
        assert_ne!(id(Agent::Other, "a/b"), id(Agent::Other, "a/c"));
        assert_eq!(id(Agent::Other, &"a".repeat(64)).len(), 70);
    }

    #[test]
    fn labels_are_the_folder_name() {
        assert_eq!(basename("/Users/me/work/wick"), "wick");
        assert_eq!(basename("/Users/me/work/wick/"), "wick");
        assert_eq!(basename(r"C:\Users\me\wick"), "wick");
        assert_eq!(basename("wick"), "wick");
        assert_eq!(basename(&"é".repeat(80)).chars().count(), 64);
    }

    #[test]
    fn since_moves_only_when_the_state_changes() {
        let update = |state| Update {
            agent: Agent::Claude,
            session: "s".into(),
            label: "wick".into(),
            cwd: None,
            state,
            pid: None,
            pid_start: Some(5),
        };
        let first = next(None, update(State::Working), 100);
        assert_eq!(
            (first.since, first.updated, first.pid_start),
            (100, 100, None)
        );
        let again = next(Some(&first), update(State::Working), 160);
        assert_eq!((again.since, again.updated), (100, 160));
        let waiting = next(Some(&again), update(State::Waiting), 200);
        assert_eq!((waiting.since, waiting.updated), (200, 200));
    }

    #[test]
    fn records_events_and_writes_atomically() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join(DIRECTORY);
        let event = |name: &str| claude_event(&claude(name)).unwrap();
        let mut walks = 0;
        record(
            &directory,
            Agent::Claude,
            event("UserPromptSubmit"),
            100,
            |existing| {
                assert!(existing.is_none());
                walks += 1;
                (Some(4242), Some(90))
            },
        )
        .unwrap();
        let file = path(
            &directory,
            Agent::Claude,
            "d442c2f9-7e86-4aa5-a5ba-c14469f84e18",
        );
        let saved = read(&file).unwrap();
        assert_eq!(saved.state, State::Working);
        assert_eq!((saved.pid, saved.pid_start), (Some(4242), Some(90)));
        assert_eq!(saved.label, "wick");
        let raw: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        assert_eq!(raw["v"], 1);
        assert_eq!(raw["pidStart"], 90);
        assert_eq!(raw["agent"], "claude");
        record(&directory, Agent::Claude, event("Stop"), 130, |existing| {
            (existing.unwrap().pid, existing.unwrap().pid_start)
        })
        .unwrap();
        let saved = read(&file).unwrap();
        assert_eq!(
            (saved.state, saved.since, saved.updated),
            (State::Waiting, 130, 130)
        );
        let names: Vec<_> = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, ["claude-d442c2f9-7e86-4aa5-a5ba-c14469f84e18.json"]);
        record(
            &directory,
            Agent::Claude,
            event("SessionEnd"),
            140,
            |_| unreachable!(),
        )
        .unwrap();
        assert!(!file.exists());
        remove(&directory, Agent::Claude, "gone").unwrap();
        assert_eq!(walks, 1);
    }

    #[test]
    fn expiry_follows_the_process_and_the_clock() {
        let live = |pid: u32, start: Option<i64>| pid == 4242 && start == Some(1_000);
        let mut held = hold(Agent::Claude, "s", State::Working, 0);
        assert!(!expired(&held, 23 * 3600, live));
        assert!(expired(&held, 24 * 3600 + 1, live));
        held.pid = Some(4243);
        assert!(expired(&held, 60, live), "the process is gone");
        held.pid = Some(4242);
        held.pid_start = Some(2_000);
        assert!(expired(&held, 60, live), "the pid was reused");
        held.pid = None;
        assert!(!expired(&held, 6 * 3600, |_, _| false));
        assert!(expired(&held, 6 * 3600 + 1, |_, _| false));
    }

    #[test]
    fn reading_drops_dead_and_broken_files_and_skips_temporary_ones() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path();
        let live = |pid: u32, _: Option<i64>| pid == 4242;
        write(directory, &hold(Agent::Claude, "alive", State::Working, 10)).unwrap();
        let mut dead = hold(Agent::Codex, "dead", State::Working, 10);
        dead.pid = Some(1);
        write(directory, &dead).unwrap();
        fs::write(directory.join("claude-broken.json"), "{ not json").unwrap();
        fs::write(directory.join(".tmpAbC123"), "{").unwrap();
        let mut newer =
            serde_json::to_value(hold(Agent::Claude, "newer", State::Working, 10)).unwrap();
        newer["v"] = json!(2);
        fs::write(directory.join("claude-newer.json"), newer.to_string()).unwrap();
        let misnamed = hold(Agent::Claude, "other-name", State::Working, 10);
        fs::write(
            directory.join("claude-misnamed.json"),
            serde_json::to_string(&misnamed).unwrap(),
        )
        .unwrap();
        let holds = read_all(directory, 20, live);
        assert_eq!(holds.len(), 1);
        assert_eq!(holds[0].session, "alive");
        let mut left: Vec<_> = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [".tmpAbC123", "claude-alive.json", "claude-newer.json"]
        );
        assert!(read_all(&directory.join("missing"), 0, live).is_empty());
    }

    #[test]
    fn this_process_is_alive_and_a_wrong_start_is_not() {
        let own = std::process::id();
        assert!(alive(own, None));
        if let Some(start) = processes::start_time(own) {
            assert!(alive(own, Some(start)));
            assert!(!alive(own, Some(start - 3600)));
        }
        assert!(!alive(0, None));
    }

    #[cfg(unix)]
    #[test]
    fn a_killed_agent_drops_its_hold() {
        let temp = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let pid = child.id();
        let mut held = hold(Agent::Claude, "killed", State::Working, 0);
        held.pid = Some(pid);
        held.pid_start = processes::start_time(pid);
        write(temp.path(), &held).unwrap();
        assert_eq!(read_all(temp.path(), 10, alive).len(), 1);
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(read_all(temp.path(), 10, alive).is_empty());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    }

    fn table<'a>(rows: &'a [(u32, u32, &'a str)]) -> impl Fn(u32) -> Option<(u32, String)> + 'a {
        move |pid| {
            rows.iter()
                .find(|row| row.0 == pid)
                .map(|row| (row.1, row.2.to_string()))
        }
    }

    #[test]
    fn the_walk_skips_shells_to_the_agent() {
        let rows = [
            (1, 0, "systemd"),
            (100, 1, "-zsh"),
            (200, 100, "claude"),
            (300, 200, "/bin/sh"),
            (400, 300, "bash"),
            (500, 1, "node"),
            (600, 200, "node"),
            (700, 600, "sh"),
            (800, 100, "vim"),
            (900, 800, "sh"),
            (1000, 1100, "codex-x86_64-unknown-linux-musl"),
            (1100, 1, "node"),
            (1200, 0, r"C:\Program Files\Claude\claude.EXE"),
            (1300, 1200, "cmd.exe"),
            (1400, 1300, "pwsh.exe"),
            (1500, 1500, "sh"),
        ];
        let lookup = table(&rows);
        assert_eq!(find_agent(400, Agent::Claude, &lookup), Some(200));
        assert_eq!(find_agent(200, Agent::Claude, &lookup), Some(200));
        assert_eq!(find_agent(700, Agent::Claude, &lookup), Some(200));
        assert_eq!(find_agent(900, Agent::Claude, &lookup), None);
        assert_eq!(find_agent(500, Agent::Claude, &lookup), None);
        assert_eq!(find_agent(400, Agent::Codex, &lookup), None);
        assert_eq!(find_agent(1000, Agent::Codex, &lookup), Some(1000));
        assert_eq!(find_agent(1400, Agent::Claude, &lookup), Some(1200));
        assert_eq!(find_agent(1500, Agent::Claude, &lookup), None);
        assert_eq!(find_agent(99, Agent::Claude, &lookup), None);
        assert_eq!(find_agent(0, Agent::Claude, &lookup), None);
        assert_eq!(find_agent(400, Agent::Other, &lookup), None);
    }

    #[test]
    fn phrases_name_the_agent_and_folder() {
        let claude = hold(Agent::Claude, "a", State::Working, 0);
        let codex = hold(Agent::Codex, "b", State::Working, 0);
        let mut build = hold(Agent::Other, "build", State::Working, 0);
        build.label = "build".into();
        assert_eq!(phrase(&[]), None);
        assert_eq!(
            phrase(&[&claude]).as_deref(),
            Some("Claude Code is working in wick")
        );
        assert_eq!(phrase(&[&build]).as_deref(), Some("build is working"));
        assert_eq!(
            phrase(&[&claude, &codex]).as_deref(),
            Some("2 agents are working")
        );
        let mut nowhere = codex.clone();
        nowhere.label.clear();
        assert_eq!(phrase(&[&nowhere]).as_deref(), Some("Codex is working"));
        assert_eq!(
            until(&[&claude]).as_deref(),
            Some("Claude Code stops working")
        );
        assert_eq!(
            until(&[&claude, &codex]).as_deref(),
            Some("the agents stop working")
        );
        assert_eq!(
            ended(&claude, true, "Your Mac can sleep again."),
            "Claude Code finished in wick. Your Mac can sleep again."
        );
        assert_eq!(
            ended(&claude, false, "Your Mac can sleep again."),
            "Claude Code is waiting for you in wick."
        );
        assert_eq!(
            ended(&build, true, "Your computer can sleep again."),
            "build finished. Your computer can sleep again."
        );
    }

    #[test]
    fn views_put_working_first_then_newest_and_cap_the_list() {
        let mut holds: Vec<Hold> = (0..20)
            .map(|index| hold(Agent::Claude, &format!("s{index}"), State::Waiting, index))
            .collect();
        holds.push(hold(Agent::Codex, "old-work", State::Working, -50));
        let views = views(&holds);
        assert_eq!(views.len(), SHOWN);
        assert_eq!(views[0].id, "codex-old-work");
        assert_eq!(views[0].name, "Codex");
        assert_eq!(views[1].id, "claude-s19");
        assert_eq!(views[2].id, "claude-s18");
        let value = serde_json::to_value(&views[1]).unwrap();
        assert_eq!(
            value,
            json!({ "id": "claude-s19", "agent": "claude", "name": "Claude Code", "label": "wick", "state": "waiting", "since": 19 })
        );
    }
}
