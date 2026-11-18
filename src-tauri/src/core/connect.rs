mod json;

use json::{Json, Object};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use toml_edit::{ArrayOfTables, DocumentMut, InlineTable, Item, Table, TableLike};

const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// Claude Code events Wick listens to (docs/agents.md, Events).
const CLAUDE_EVENTS: [(&str, Option<i64>); 7] = [
    ("UserPromptSubmit", None),
    ("PreToolUse", None),
    ("PostToolUse", None),
    ("Stop", None),
    ("Notification", None),
    ("PermissionRequest", None),
    ("SessionEnd", None),
];

/// Codex hook events with their timeouts in seconds. Codex caps `Interrupt` and `SessionEnd`
/// at 1 s.
const CODEX_EVENTS: [(&str, Option<i64>); 8] = [
    ("SessionStart", Some(5)),
    ("UserPromptSubmit", Some(5)),
    ("PreToolUse", Some(5)),
    ("PermissionRequest", Some(5)),
    ("PostToolUse", Some(5)),
    ("Stop", Some(5)),
    ("Interrupt", Some(1)),
    ("SessionEnd", Some(1)),
];

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Agent {
    Claude,
    Codex,
}

impl Agent {
    pub const ALL: [Agent; 2] = [Agent::Claude, Agent::Codex];

    pub fn name(self) -> &'static str {
        match self {
            Agent::Claude => "Claude Code",
            Agent::Codex => "Codex",
        }
    }

    fn word(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
        }
    }

    fn events(self) -> &'static [(&'static str, Option<i64>)] {
        match self {
            Agent::Claude => &CLAUDE_EVENTS,
            Agent::Codex => &CODEX_EVENTS,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Connected,
    Disconnected,
    Stale,
    Missing,
    Error,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Connection {
    pub agent: Agent,
    pub name: &'static str,
    pub status: Status,
    pub file: String,
    pub error: Option<String>,
}

/// Where the agents keep their settings, and the Wick binary their hooks run.
#[derive(Clone, Debug)]
pub struct Places {
    pub home: Option<PathBuf>,
    pub claude: Option<PathBuf>,
    pub codex: Option<PathBuf>,
    pub exe: Result<String, String>,
    pub windows: bool,
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

impl Places {
    pub fn current() -> Self {
        let home = std::env::home_dir().filter(|path| path.is_absolute());
        Self {
            claude: env_path("CLAUDE_CONFIG_DIR")
                .or_else(|| home.as_ref().map(|h| h.join(".claude"))),
            // Codex resolves CODEX_HOME, but not ~/.codex, before naming hooks in trust keys.
            codex: env_path("CODEX_HOME")
                .map(|path| fs::canonicalize(&path).unwrap_or(path))
                .or_else(|| home.as_ref().map(|h| h.join(".codex"))),
            home,
            exe: current_exe().and_then(|exe| runnable(exe, cfg!(windows))),
            windows: cfg!(windows),
        }
    }

    fn claude_file(&self) -> Option<PathBuf> {
        self.claude.as_ref().map(|dir| dir.join("settings.json"))
    }

    fn shown(&self, path: &Path) -> String {
        shown(path, self.home.as_deref())
    }
}

/// The binary agents should run: the AppImage itself rather than its temporary mount.
fn current_exe() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    if let Some(image) = env_path("APPIMAGE") {
        return image
            .into_os_string()
            .into_string()
            .map_err(|_| unusable_location());
    }
    let exe = std::env::current_exe().map_err(|_| "Wick couldn’t find its own location.")?;
    let text = exe
        .into_os_string()
        .into_string()
        .map_err(|_| unusable_location())?;
    let text = text.strip_suffix(" (deleted)").unwrap_or(&text);
    let text = text.strip_prefix(r"\\?\").unwrap_or(text);
    if cfg!(target_os = "macos") && text.contains("/AppTranslocation/") {
        return Err("Move Wick to Applications, open it from there, then connect.".into());
    }
    Ok(text.to_string())
}

/// On Windows the hook command keeps the path as is inside double quotes, where Git Bash still
/// expands `$` and `` ` `` and `cmd` expands `%`. A path with them is refused.
pub fn runnable(exe: String, windows: bool) -> Result<String, String> {
    if windows && exe.contains(['$', '`', '%']) {
        Err(unusable_location())
    } else {
        Ok(exe)
    }
}

fn unusable_location() -> String {
    "Wick’s location has characters agents can’t run. Move Wick and try again.".into()
}

/// `~/.claude/settings.json` for a file under the home folder, on every platform, else the
/// full path.
pub fn shown(path: &Path, home: Option<&Path>) -> String {
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) => std::iter::once("~".into())
            .chain(rest.iter().map(|part| part.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("/"),
        None => path.display().to_string(),
    }
}

/// `"<exe>" hook claude`. POSIX shells read the quoted path literally once `\`, `"`, `$` and
/// `` ` `` are escaped. On Windows the path uses forward slashes, which `cmd` and Git Bash
/// both run as is; it can't hold a quote, and `runnable` refuses `$`, `` ` `` and `%`.
pub fn hook_command(exe: &str, agent: Agent, windows: bool) -> String {
    let path = if windows {
        exe.replace('\\', "/")
    } else {
        let mut escaped = String::with_capacity(exe.len());
        for character in exe.chars() {
            if matches!(character, '\\' | '"' | '$' | '`') {
                escaped.push('\\');
            }
            escaped.push(character);
        }
        escaped
    };
    format!("\"{path}\" hook {}", agent.word())
}

/// The Wick binary a hook command runs, when the command is Wick's hook for `agent`.
pub fn wick_binary(command: &str, agent: Agent, windows: bool) -> Option<String> {
    let rest = command
        .trim()
        .strip_suffix(&format!(" hook {}", agent.word()))?
        .trim_end();
    let path = if let Some(inner) = rest.strip_prefix('"') {
        let inner = inner.strip_suffix('"')?;
        if windows {
            inner.to_string()
        } else {
            unescape(inner)
        }
    } else if let Some(inner) = rest.strip_prefix('\'') {
        inner.strip_suffix('\'')?.to_string()
    } else if windows {
        rest.to_string()
    } else {
        unescape(rest)
    };
    is_wick(&path).then_some(path)
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            if let Some(next) = characters.next() {
                out.push(next);
            }
        } else {
            out.push(character);
        }
    }
    out
}

/// `wick`, `Wick`, `wick.exe`, `Wick_0.9.0_amd64.AppImage`.
fn is_wick(path: &str) -> bool {
    let name = path.rsplit(['/', '\\']).next().unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    lower
        .strip_prefix("wick")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '-', '_', ' ']))
}

/// `PreToolUse` → `pre_tool_use`, as Codex names events in trust keys.
pub fn snake_case(event: &str) -> String {
    let mut out = String::with_capacity(event.len() + 4);
    for (index, character) in event.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                out.push('_');
            }
            out.push(character.to_ascii_lowercase());
        } else {
            out.push(character);
        }
    }
    out
}

/// Codex trusts a hook group by this hash: SHA-256 over its compact JSON with sorted keys.
/// Without a timeout Codex uses 600 s, and it caps `session_end` and `interrupt` at 1 s.
pub fn trust_hash(
    event: &str,
    command: &str,
    timeout: Option<i64>,
    matcher: Option<&str>,
) -> String {
    let quoted = |text: &str| serde_json::Value::String(text.into()).to_string();
    let timeout = timeout.unwrap_or(600);
    let timeout = if matches!(event, "session_end" | "interrupt") {
        timeout.min(1)
    } else {
        timeout
    };
    let mut canonical = format!(
        "{{\"event_name\":{},\"hooks\":[{{\"async\":false,\"command\":{},\"timeout\":{timeout},\"type\":\"command\"}}]",
        quoted(event),
        quoted(command),
    );
    if let Some(matcher) = matcher {
        canonical.push_str(&format!(",\"matcher\":{}", quoted(matcher)));
    }
    canonical.push('}');
    format!("sha256:{:x}", Sha256::digest(canonical.as_bytes()))
}

/// A hook group Wick installed: a single command handler that runs Wick.
#[derive(Clone, Debug, PartialEq)]
struct Hook {
    binary: String,
    command: String,
    timeout: Option<i64>,
    matcher: Option<String>,
}

/// A handler's `type`, `command` and `timeout`.
type Handler<'a> = (Option<&'a str>, Option<&'a str>, Option<i64>);

/// Each event's groups, in file order: Wick's, or `None` for anyone else's.
type Layout = Vec<(String, Vec<Option<Hook>>)>;

/// What a change does to one event's groups.
#[derive(Debug, PartialEq)]
struct Plan {
    event: String,
    /// The new index of each existing group, `None` when it is removed.
    moves: Vec<Option<usize>>,
    /// Wick's group after the change, and whether it must be written.
    wick: Option<(usize, bool)>,
}

impl Plan {
    fn moves_groups(&self) -> bool {
        self.moves
            .iter()
            .enumerate()
            .any(|(old, new)| *new != Some(old))
    }

    /// Wick's group is new: no existing group was kept for it.
    fn appends(&self) -> bool {
        self.wick
            .is_some_and(|(index, _)| index == self.moves.iter().flatten().count())
    }

    fn changes(&self) -> bool {
        self.moves_groups() || self.appends() || self.wick.is_some_and(|(_, write)| write)
    }
}

/// Connecting keeps Wick's first group in each of its events, rewritten when it differs, drops
/// any other Wick group, and appends one where none exists. Disconnecting drops them all.
fn plan(
    layout: &Layout,
    targets: &[(&str, Option<i64>)],
    desired: impl Fn(&str, Option<i64>) -> Hook,
) -> Vec<Plan> {
    let mut events: Vec<&str> = layout.iter().map(|(event, _)| event.as_str()).collect();
    for (event, _) in targets {
        if !events.contains(event) {
            events.push(event);
        }
    }
    let empty = Vec::new();
    events
        .into_iter()
        .filter_map(|event| {
            let groups = layout
                .iter()
                .find(|(name, _)| name == event)
                .map_or(&empty, |(_, groups)| groups);
            let target = targets.iter().find(|(name, _)| *name == event);
            let keep = target.and(groups.iter().position(Option::is_some));
            let mut next = 0;
            let moves: Vec<Option<usize>> = groups
                .iter()
                .enumerate()
                .map(|(index, group)| {
                    (group.is_none() || Some(index) == keep).then(|| {
                        next += 1;
                        next - 1
                    })
                })
                .collect();
            let wick = target.map(|(name, timeout)| {
                let want = desired(name, *timeout);
                match keep {
                    Some(index) => (
                        moves[index].unwrap_or(index),
                        groups[index].as_ref() != Some(&want),
                    ),
                    None => (next, true),
                }
            });
            let plan = Plan {
                event: event.to_string(),
                moves,
                wick,
            };
            (plan.wick.is_some() || plan.changes()).then_some(plan)
        })
        .collect()
}

fn desired(exe: &str, agent: Agent, windows: bool) -> impl Fn(&str, Option<i64>) -> Hook + '_ {
    move |_, timeout| Hook {
        binary: exe.to_string(),
        command: hook_command(exe, agent, windows),
        timeout,
        matcher: None,
    }
}

fn read_hook(
    handlers: Option<Vec<Handler>>,
    matcher: Option<&str>,
    agent: Agent,
    windows: bool,
) -> Option<Hook> {
    let [(kind, command, timeout)] = handlers?.try_into().ok()?;
    if kind != Some("command") {
        return None;
    }
    let command = command?;
    Some(Hook {
        binary: wick_binary(command, agent, windows)?,
        command: command.to_string(),
        timeout,
        matcher: matcher.map(String::from),
    })
}

fn json_hook(group: &Json, agent: Agent, windows: bool) -> Option<Hook> {
    let handlers = group.get("hooks")?.as_array().map(|handlers| {
        handlers
            .iter()
            .map(|handler| {
                (
                    handler.get("type").and_then(Json::as_str),
                    handler.get("command").and_then(Json::as_str),
                    handler.get("timeout").and_then(Json::as_i64),
                )
            })
            .collect()
    });
    read_hook(
        handlers,
        group.get("matcher").and_then(Json::as_str),
        agent,
        windows,
    )
}

fn json_layout(root: &Object, agent: Agent, windows: bool) -> Result<Layout, ()> {
    let Some(hooks) = root.get("hooks") else {
        return Ok(Vec::new());
    };
    hooks
        .as_object()
        .ok_or(())?
        .iter()
        .map(|(event, groups)| {
            let groups = groups.as_array().ok_or(())?;
            Ok((
                event.to_string(),
                groups
                    .iter()
                    .map(|group| json_hook(group, agent, windows))
                    .collect(),
            ))
        })
        .collect()
}

fn json_group(hook: &Hook) -> Json {
    let mut handler = vec![
        ("type", "command".into()),
        ("command", hook.command.as_str().into()),
    ];
    if let Some(timeout) = hook.timeout {
        handler.push(("timeout", timeout.into()));
    }
    json::object(vec![("hooks", Json::Array(vec![json::object(handler)]))])
}

/// Applies the plans to a settings or hooks.json root. Returns whether anything changed.
fn apply_json(root: &mut Object, plans: &[Plan], hook: &impl Fn(&str) -> Hook) -> bool {
    if !plans.iter().any(Plan::changes) {
        return false;
    }
    if root.get("hooks").is_none() {
        root.insert("hooks", Json::Object(Object::default()));
    }
    let Some(events) = root.get_mut("hooks").and_then(Json::as_object_mut) else {
        return false;
    };
    let mut emptied = false;
    for plan in plans.iter().filter(|plan| plan.changes()) {
        let old = match events.get_mut(&plan.event) {
            Some(Json::Array(groups)) => std::mem::take(groups),
            _ => Vec::new(),
        };
        let had = !old.is_empty();
        let mut groups: Vec<Json> = old
            .into_iter()
            .zip(&plan.moves)
            .filter_map(|(group, new)| new.map(|_| group))
            .collect();
        match plan.wick {
            Some((index, _)) if index == groups.len() => {
                groups.push(json_group(&hook(&plan.event)))
            }
            Some((index, true)) => groups[index] = json_group(&hook(&plan.event)),
            _ => {}
        }
        if groups.is_empty() && had {
            events.remove(&plan.event);
            emptied = true;
        } else {
            events.insert(&plan.event, Json::Array(groups));
        }
    }
    if emptied && events.is_empty() {
        root.remove("hooks");
    }
    true
}

fn toml_hook(group: &dyn TableLike, agent: Agent, windows: bool) -> Option<Hook> {
    fn handler(table: &dyn TableLike) -> Handler<'_> {
        (
            table.get("type").and_then(Item::as_str),
            table.get("command").and_then(Item::as_str),
            table.get("timeout").and_then(Item::as_integer),
        )
    }
    let item = group.get("hooks")?;
    let handlers = if let Some(tables) = item.as_array_of_tables() {
        Some(tables.iter().map(|table| handler(table)).collect())
    } else {
        item.as_array().map(|values| {
            values
                .iter()
                .map(|value| match value.as_inline_table() {
                    Some(table) => handler(table),
                    None => (None, None, None),
                })
                .collect()
        })
    };
    read_hook(
        handlers,
        group.get("matcher").and_then(Item::as_str),
        agent,
        windows,
    )
}

/// Hooks in `config.toml`: `[[hooks.<Event>]]` tables, or arrays of inline tables.
fn toml_layout(document: &DocumentMut, agent: Agent, windows: bool) -> Result<Layout, ()> {
    let Some(hooks) = document.get("hooks") else {
        return Ok(Vec::new());
    };
    let hooks = hooks.as_table_like().ok_or(())?;
    let mut layout = Vec::new();
    for (event, item) in hooks.iter() {
        let groups = if let Some(tables) = item.as_array_of_tables() {
            tables
                .iter()
                .map(|group| toml_hook(group, agent, windows))
                .collect()
        } else if let Some(values) = item.as_array() {
            values
                .iter()
                .map(|value| {
                    value
                        .as_inline_table()
                        .and_then(|group| toml_hook(group, agent, windows))
                })
                .collect()
        } else {
            continue;
        };
        layout.push((event.to_string(), groups));
    }
    Ok(layout)
}

fn toml_group(hook: &Hook) -> Table {
    let mut handler = Table::new();
    handler.insert("type", toml_edit::value("command"));
    handler.insert("command", toml_edit::value(hook.command.as_str()));
    if let Some(timeout) = hook.timeout {
        handler.insert("timeout", toml_edit::value(timeout));
    }
    let mut handlers = ArrayOfTables::new();
    handlers.push(handler);
    let mut group = Table::new();
    group.insert("hooks", Item::ArrayOfTables(handlers));
    group
}

fn implicit_table() -> Item {
    let mut table = Table::new();
    table.set_implicit(true);
    Item::Table(table)
}

/// Applies the plans to `[[hooks.<Event>]]` tables in config.toml. Returns whether anything
/// changed, or `Err` for hooks in a shape Wick doesn't edit.
fn apply_toml(
    document: &mut DocumentMut,
    plans: &[Plan],
    hook: &impl Fn(&str) -> Hook,
) -> Result<bool, ()> {
    if !plans.iter().any(Plan::changes) {
        return Ok(false);
    }
    let hooks = document
        .entry("hooks")
        .or_insert_with(implicit_table)
        .as_table_mut()
        .ok_or(())?;
    let mut emptied = false;
    for plan in plans.iter().filter(|plan| plan.changes()) {
        let groups = hooks
            .entry(&plan.event)
            .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()))
            .as_array_of_tables_mut()
            .ok_or(())?;
        let had = !groups.is_empty();
        for (index, new) in plan.moves.iter().enumerate().rev() {
            if new.is_none() {
                groups.remove(index);
            }
        }
        match plan.wick {
            Some((index, _)) if index == groups.len() => {
                groups.push(toml_group(&hook(&plan.event)))
            }
            Some((index, true)) => {
                *groups.get_mut(index).ok_or(())? = toml_group(&hook(&plan.event))
            }
            _ => {}
        }
        if groups.is_empty() && had {
            hooks.remove(&plan.event);
            emptied = true;
        }
    }
    if emptied && hooks.is_empty() {
        document.remove("hooks");
    }
    Ok(true)
}

fn trusted_hash<'a>(document: &'a DocumentMut, key: &str) -> Option<&'a str> {
    document
        .get("hooks")?
        .as_table_like()?
        .get("state")?
        .as_table_like()?
        .get(key)?
        .as_table_like()?
        .get("trusted_hash")?
        .as_str()
}

fn trust_key(file: &str, event: &str, group: usize) -> String {
    format!("{file}:{}:{group}:0", snake_case(event))
}

/// Moves `[hooks.state."<file>:<event>:<group>:<handler>"]` entries with their groups, drops
/// the removed groups' entries, and trusts Wick's group, so other hooks keep their trust.
fn apply_trust(
    document: &mut DocumentMut,
    file: &str,
    plans: &[Plan],
    hook: &impl Fn(&str) -> Hook,
) -> Result<bool, ()> {
    let mut changed = false;
    let mut removed = false;
    for plan in plans {
        let snake = snake_case(&plan.event);
        let prefix = format!("{file}:{snake}:");
        let wanted = plan.wick.map(|(index, _)| {
            let want = hook(&plan.event);
            (
                trust_key(file, &plan.event, index),
                trust_hash(&snake, &want.command, want.timeout, want.matcher.as_deref()),
            )
        });
        let moved = plan.moves_groups();
        if !moved
            && wanted
                .as_ref()
                .is_none_or(|(key, hash)| trusted_hash(document, key) == Some(hash))
        {
            continue;
        }
        if document.get("hooks").is_none() {
            document.insert("hooks", implicit_table());
        }
        let hooks = document["hooks"].as_table_like_mut().ok_or(())?;
        if hooks.get("state").is_none() {
            hooks.insert("state", implicit_table());
        }
        let state = hooks.get_mut("state").ok_or(())?;
        let inline = state.is_inline_table();
        let state = state.as_table_like_mut().ok_or(())?;
        if moved {
            let entries: Vec<(String, Option<String>)> = state
                .iter()
                .filter_map(|(key, _)| {
                    let (group, handler) = key.strip_prefix(&prefix)?.split_once(':')?;
                    let group: usize = group.parse().ok()?;
                    let new = plan.moves.get(group)?;
                    (*new != Some(group)).then(|| {
                        (
                            key.to_string(),
                            new.map(|new| format!("{prefix}{new}:{handler}")),
                        )
                    })
                })
                .collect();
            let mut renamed = Vec::new();
            for (key, new) in entries {
                if let Some(item) = state.remove(&key) {
                    changed = true;
                    match new {
                        Some(new) => renamed.push((new, item)),
                        None => removed = true,
                    }
                }
            }
            for (key, item) in renamed {
                state.insert(&key, item);
            }
        }
        if let Some((key, hash)) = wanted {
            let existing = state
                .get_mut(&key)
                .and_then(Item::as_table_like_mut)
                .filter(|_| !plan.appends());
            match existing {
                Some(entry) if entry.get("trusted_hash").and_then(Item::as_str) == Some(&hash) => {}
                Some(entry) => {
                    entry.insert("trusted_hash", toml_edit::value(hash));
                    changed = true;
                }
                None => {
                    let item = if inline {
                        let mut entry = InlineTable::new();
                        entry.insert("trusted_hash", hash.into());
                        Item::Value(entry.into())
                    } else {
                        let mut entry = Table::new();
                        entry.insert("trusted_hash", toml_edit::value(hash));
                        Item::Table(entry)
                    };
                    state.insert(&key, item);
                    changed = true;
                }
            }
        }
    }
    if removed {
        if let Some(hooks) = document.get_mut("hooks").and_then(Item::as_table_like_mut) {
            if hooks
                .get("state")
                .and_then(Item::as_table_like)
                .is_some_and(|state| state.is_empty())
            {
                hooks.remove("state");
            }
            if hooks.is_empty() {
                document.remove("hooks");
            }
        }
    }
    Ok(changed)
}

/// A file as read: its text, or `None` when it doesn't exist yet.
struct Source {
    path: PathBuf,
    shown: String,
    text: Option<String>,
}

fn read_text(path: &Path, shown: &str, format: &str) -> Result<Option<String>, String> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(format!("Wick couldn’t read {shown}.")),
    };
    let unreadable = || format!("Wick couldn’t read {shown}.");
    let metadata = file.metadata().map_err(|_| unreadable())?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Err(unreadable());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unreadable())?;
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| invalid(shown, format))
}

fn invalid(shown: &str, format: &str) -> String {
    format!("{shown} isn’t valid {format}. Wick left it unchanged.")
}

fn unknown_shape(shown: &str) -> String {
    format!("{shown} has hooks Wick can’t read. Wick left it unchanged.")
}

impl Source {
    fn read(path: PathBuf, places: &Places, format: &str) -> Result<Self, String> {
        let shown = places.shown(&path);
        let text = read_text(&path, &shown, format)?;
        Ok(Self { path, shown, text })
    }

    fn json(&self) -> Result<Object, String> {
        let text = self.text.as_deref().unwrap_or_default();
        if text.trim().is_empty() {
            return Ok(Object::default());
        }
        match serde_json::from_str(text) {
            Ok(Json::Object(root)) => Ok(root),
            _ => Err(invalid(&self.shown, "JSON")),
        }
    }

    fn toml(&self) -> Result<DocumentMut, String> {
        self.text
            .as_deref()
            .unwrap_or_default()
            .parse()
            .map_err(|_| invalid(&self.shown, "TOML"))
    }

    /// Replaces the file atomically, following a symlink to its target. Backs the original up
    /// once to `<name>.wick-backup` and refuses when the file changed since it was read. A file
    /// Wick created (it has no backup) is removed once `empty`.
    fn write(&self, text: &str, empty: bool) -> Result<(), String> {
        let failed = || format!("Wick couldn’t save {}.", self.shown);
        let target = fs::canonicalize(&self.path).unwrap_or_else(|_| self.path.clone());
        if read_text(&target, &self.shown, "text")? != self.text {
            return Err(format!(
                "{} changed while Wick was saving it. Try again.",
                self.shown
            ));
        }
        let existing = fs::metadata(&target).ok();
        if existing.is_some() {
            let mut name = self.path.file_name().ok_or_else(failed)?.to_os_string();
            name.push(".wick-backup");
            let backup = self.path.with_file_name(name);
            if fs::symlink_metadata(&backup).is_err() {
                let link = fs::symlink_metadata(&self.path)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink());
                if empty && !link {
                    return fs::remove_file(&self.path).map_err(|_| failed());
                }
                fs::copy(&target, &backup).map_err(|_| failed())?;
            }
        }
        let parent = target.parent().ok_or_else(failed)?;
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| failed())?;
        match &existing {
            Some(metadata) => file
                .as_file()
                .set_permissions(metadata.permissions())
                .map_err(|_| failed())?,
            #[cfg(unix)]
            None => {
                use std::os::unix::fs::PermissionsExt;
                file.as_file()
                    .set_permissions(fs::Permissions::from_mode(0o600))
                    .map_err(|_| failed())?
            }
            #[cfg(not(unix))]
            None => {}
        }
        file.write_all(text.as_bytes()).map_err(|_| failed())?;
        file.as_file().sync_all().map_err(|_| failed())?;
        file.persist(&target).map_err(|_| failed())?;
        Ok(())
    }
}

fn pretty(root: Object) -> String {
    json::pretty(&Json::Object(root))
}

fn write_json(source: &Source, root: Object) -> Result<(), String> {
    let empty = root.is_empty();
    source.write(&pretty(root), empty)
}

fn write_toml(source: &Source, document: &DocumentMut) -> Result<(), String> {
    let text = document.to_string();
    source.write(&text, text.trim().is_empty())
}

/// Every agent Wick can connect, and whether it is.
pub fn connections(places: &Places) -> Vec<Connection> {
    Agent::ALL
        .into_iter()
        .map(|agent| connection(places, agent))
        .collect()
}

pub fn connection(places: &Places, agent: Agent) -> Connection {
    let (file, outcome, note) = match agent {
        Agent::Claude => {
            let (file, outcome) = claude_status(places);
            (file, outcome, None)
        }
        Agent::Codex => codex_status(places),
    };
    let (status, error) = match outcome {
        Ok(status) => (status, note),
        Err(error) => (Status::Error, Some(error)),
    };
    Connection {
        agent,
        name: agent.name(),
        status,
        file,
        error,
    }
}

fn installed(directory: Option<&PathBuf>) -> Option<&PathBuf> {
    directory.filter(|directory| directory.is_dir())
}

fn judge(
    layout: &Layout,
    agent: Agent,
    trusted: impl Fn(&str, usize, &Hook) -> bool,
) -> Result<Status, String> {
    let hooks: Vec<_> = layout
        .iter()
        .flat_map(|(event, groups)| {
            groups.iter().enumerate().filter_map(move |(index, group)| {
                group.as_ref().map(|hook| (event.as_str(), index, hook))
            })
        })
        .collect();
    if hooks.is_empty() {
        return Ok(Status::Disconnected);
    }
    if hooks
        .iter()
        .any(|(_, _, hook)| !Path::new(&hook.binary).is_file())
    {
        return Ok(Status::Stale);
    }
    for (event, _) in agent.events() {
        match hooks.iter().find(|(name, _, _)| name == event) {
            None => return Err("Some of Wick’s hooks are missing. Connect again.".into()),
            Some((event, index, hook)) if !trusted(event, *index, hook) => {
                return Err("Codex doesn’t trust Wick’s hooks yet. Connect again.".into())
            }
            _ => {}
        }
    }
    Ok(Status::Connected)
}

fn claude_status(places: &Places) -> (String, Result<Status, String>) {
    let path = places
        .claude_file()
        .unwrap_or_else(|| PathBuf::from("~/.claude/settings.json"));
    let shown = places.shown(&path);
    if installed(places.claude.as_ref()).is_none() {
        return (shown, Ok(Status::Missing));
    }
    let status = Source::read(path, places, "JSON")
        .and_then(|source| source.json().map(|root| (source, root)))
        .and_then(|(source, root)| {
            json_layout(&root, Agent::Claude, places.windows)
                .map_err(|_| unknown_shape(&source.shown))
        })
        .and_then(|layout| judge(&layout, Agent::Claude, |_, _, _| true));
    (shown, status)
}

/// Codex's hook file: config.toml when it already holds `[[hooks.<Event>]]`, else hooks.json.
struct CodexFiles {
    config: Source,
    document: DocumentMut,
    hooks: Option<Source>,
}

impl CodexFiles {
    fn read(places: &Places, directory: &Path) -> Result<Self, String> {
        let config = Source::read(directory.join("config.toml"), places, "TOML")?;
        let document = config.toml()?;
        let in_config = document
            .get("hooks")
            .and_then(Item::as_table_like)
            .is_some_and(|hooks| {
                hooks.iter().any(|(key, item)| {
                    key != "state" && (item.is_array_of_tables() || item.is_array())
                })
            });
        let hooks = if in_config {
            None
        } else {
            Some(Source::read(directory.join("hooks.json"), places, "JSON")?)
        };
        Ok(Self {
            config,
            document,
            hooks,
        })
    }

    fn hook_file(&self) -> &Source {
        self.hooks.as_ref().unwrap_or(&self.config)
    }

    fn key(&self) -> String {
        self.hook_file().path.display().to_string()
    }

    fn layout(&self, windows: bool) -> Result<(Layout, Option<Object>), String> {
        match &self.hooks {
            Some(source) => {
                let root = source.json()?;
                let layout = json_layout(&root, Agent::Codex, windows)
                    .map_err(|_| unknown_shape(&source.shown))?;
                Ok((layout, Some(root)))
            }
            None => toml_layout(&self.document, Agent::Codex, windows)
                .map(|layout| (layout, None))
                .map_err(|_| unknown_shape(&self.config.shown)),
        }
    }
}

fn hooks_off(document: &DocumentMut) -> bool {
    document
        .get("features")
        .and_then(Item::as_table_like)
        .and_then(|features| features.get("hooks"))
        .and_then(Item::as_bool)
        == Some(false)
}

fn hooks_off_note(files: &CodexFiles) -> Option<String> {
    hooks_off(&files.document).then(|| format!("Hooks are turned off in {}.", files.config.shown))
}

/// The hooks file, the connection, and a note. With hooks turned off, Wick's hooks never run:
/// without them the row says so in place of Connect; with them it stays connected, so they can
/// be removed, and carries the note.
fn codex_status(places: &Places) -> (String, Result<Status, String>, Option<String>) {
    let Some(directory) = installed(places.codex.as_ref()) else {
        let path = places
            .codex
            .as_ref()
            .map(|directory| directory.join("hooks.json"))
            .unwrap_or_else(|| PathBuf::from("~/.codex/hooks.json"));
        return (places.shown(&path), Ok(Status::Missing), None);
    };
    let files = match CodexFiles::read(places, directory) {
        Ok(files) => files,
        Err(error) => {
            return (
                places.shown(&directory.join("hooks.json")),
                Err(error),
                None,
            )
        }
    };
    let shown = files.hook_file().shown.clone();
    let status = codex_hooks_status(places, &files);
    match hooks_off_note(&files) {
        Some(note) if matches!(status, Ok(Status::Disconnected)) => (shown, Err(note), None),
        Some(note) if status.is_ok() || has_wick_hooks(places, &files) => {
            (shown, Ok(Status::Connected), Some(note))
        }
        _ => (shown, status, None),
    }
}

fn has_wick_hooks(places: &Places, files: &CodexFiles) -> bool {
    files.layout(places.windows).is_ok_and(|(layout, _)| {
        layout
            .iter()
            .any(|(_, groups)| groups.iter().any(Option::is_some))
    })
}

fn codex_hooks_status(places: &Places, files: &CodexFiles) -> Result<Status, String> {
    let key = files.key();
    files.layout(places.windows).and_then(|(layout, _)| {
        judge(&layout, Agent::Codex, |event, index, hook| {
            let snake = snake_case(event);
            trusted_hash(&files.document, &trust_key(&key, event, index)).is_some_and(|hash| {
                hash == trust_hash(&snake, &hook.command, hook.timeout, hook.matcher.as_deref())
            })
        })
    })
}

/// Adds Wick's hooks for `agent`, or repairs them. Running it twice changes nothing.
pub fn connect(places: &Places, agent: Agent) -> Result<(), String> {
    change(places, agent, true)
}

/// Removes exactly the hooks Wick added for `agent`.
pub fn disconnect(places: &Places, agent: Agent) -> Result<(), String> {
    change(places, agent, false)
}

fn change(places: &Places, agent: Agent, on: bool) -> Result<(), String> {
    let directory = match agent {
        Agent::Claude => installed(places.claude.as_ref()),
        Agent::Codex => installed(places.codex.as_ref()),
    };
    let Some(directory) = directory else {
        return if on {
            Err(format!("{} isn’t installed.", agent.name()))
        } else {
            Ok(())
        };
    };
    let exe = if on {
        places.exe.clone()?
    } else {
        String::new()
    };
    let want = desired(&exe, agent, places.windows);
    let targets: &[(&str, Option<i64>)] = if on { agent.events() } else { &[] };
    let timeout = |event: &str| {
        agent
            .events()
            .iter()
            .find(|(name, _)| *name == event)
            .and_then(|(_, timeout)| *timeout)
    };
    let hook = |event: &str| want(event, timeout(event));
    match agent {
        Agent::Claude => {
            let source = Source::read(directory.join("settings.json"), places, "JSON")?;
            let mut root = source.json()?;
            let layout = json_layout(&root, agent, places.windows)
                .map_err(|_| unknown_shape(&source.shown))?;
            let plans = plan(&layout, targets, &want);
            if apply_json(&mut root, &plans, &hook) {
                write_json(&source, root)?;
            }
            Ok(())
        }
        Agent::Codex => {
            let mut files = CodexFiles::read(places, directory)?;
            if let Some(note) = hooks_off_note(&files).filter(|_| on) {
                return Err(format!("{note} Turn them on, then connect."));
            }
            let (layout, root) = files.layout(places.windows)?;
            let plans = plan(&layout, targets, &want);
            let key = files.key();
            let shape = |source: &Source| unknown_shape(&source.shown);
            match (root, &files.hooks) {
                (Some(mut root), Some(source)) => {
                    let hooks_changed = apply_json(&mut root, &plans, &hook);
                    let trust_changed = apply_trust(&mut files.document, &key, &plans, &hook)
                        .map_err(|_| shape(&files.config))?;
                    let write_trust = || {
                        if trust_changed {
                            write_toml(&files.config, &files.document)
                        } else {
                            Ok(())
                        }
                    };
                    let write_hooks = || {
                        if hooks_changed {
                            write_json(source, root.clone())
                        } else {
                            Ok(())
                        }
                    };
                    if on {
                        write_trust()?;
                        write_hooks()
                    } else {
                        write_hooks()?;
                        write_trust()
                    }
                }
                _ => {
                    let hooks_changed = apply_toml(&mut files.document, &plans, &hook)
                        .map_err(|_| shape(&files.config))?;
                    let trust_changed = apply_trust(&mut files.document, &key, &plans, &hook)
                        .map_err(|_| shape(&files.config))?;
                    if hooks_changed || trust_changed {
                        write_toml(&files.config, &files.document)?;
                    }
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
