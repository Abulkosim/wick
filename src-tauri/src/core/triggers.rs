use super::processes::{bundle_of, Process};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_TRIGGERS: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerKind {
    App,
    Process,
}

/// "Stay awake while <name> is running". Apps match by bundle path, processes by name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trigger {
    pub id: String,
    pub kind: TriggerKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub enabled: bool,
}

impl Trigger {
    pub fn validate(&self) -> Result<(), String> {
        let valid_id = !self.id.is_empty()
            && self.id.len() <= 64
            && self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-');
        if !valid_id {
            return Err("A trigger has an invalid id.".into());
        }
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
            return Err("Name the app or process in 64 characters or fewer.".into());
        }
        match (self.kind, self.path.as_deref()) {
            (TriggerKind::App, Some(path))
                if path.starts_with('/') && path.ends_with(".app") && path.len() <= 1024 =>
            {
                Ok(())
            }
            (TriggerKind::App, _) => Err("Choose an app from the list.".into()),
            (TriggerKind::Process, None) if !name.contains('/') => Ok(()),
            (TriggerKind::Process, _) => Err("Type a process name, not a path.".into()),
        }
    }

    pub fn matches(&self, process: &Process) -> bool {
        match self.kind {
            TriggerKind::App => self
                .path
                .as_deref()
                .is_some_and(|path| process.path.as_deref().and_then(bundle_of) == Some(path)),
            TriggerKind::Process => {
                let name = self.name.trim();
                process.name.eq_ignore_ascii_case(name)
                    || process
                        .path
                        .as_deref()
                        .and_then(|path| path.rsplit('/').next())
                        .is_some_and(|file| file.eq_ignore_ascii_case(name))
            }
        }
    }
}

pub fn validate_all(triggers: &[Trigger]) -> Result<(), String> {
    if triggers.len() > MAX_TRIGGERS {
        return Err(format!("Keep it to {MAX_TRIGGERS} triggers."));
    }
    for (index, trigger) in triggers.iter().enumerate() {
        trigger.validate()?;
        if triggers[..index].iter().any(|other| other.id == trigger.id) {
            return Err("Two triggers share an id.".into());
        }
    }
    Ok(())
}

/// Keeps each readable trigger; notes how many had to be dropped.
pub fn recover(value: Option<Value>, notes: &mut Vec<String>) -> Vec<Trigger> {
    let items = match value {
        None | Some(Value::Null) => return Vec::new(),
        Some(Value::Array(items)) => items,
        Some(_) => {
            notes.push("Triggers could not be read and were cleared.".into());
            return Vec::new();
        }
    };
    let mut triggers: Vec<Trigger> = Vec::new();
    let mut dropped = 0;
    for item in items {
        match serde_json::from_value::<Trigger>(item) {
            Ok(trigger)
                if trigger.validate().is_ok()
                    && triggers.len() < MAX_TRIGGERS
                    && !triggers.iter().any(|other| other.id == trigger.id) =>
            {
                triggers.push(trigger)
            }
            _ => dropped += 1,
        }
    }
    if dropped > 0 {
        notes.push(match dropped {
            1 => "1 trigger could not be read and was removed.".into(),
            count => format!("{count} triggers could not be read and were removed."),
        });
    }
    triggers
}

/// Enabled triggers whose app or process is running, in list order.
pub fn active<'a>(triggers: &'a [Trigger], processes: &[Process]) -> Vec<&'a Trigger> {
    triggers
        .iter()
        .filter(|trigger| {
            trigger.enabled && processes.iter().any(|process| trigger.matches(process))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn process(name: &str, path: &str) -> Process {
        Process {
            pid: 1,
            name: name.into(),
            path: Some(path.into()),
        }
    }

    fn app(id: &str, name: &str, path: &str) -> Trigger {
        Trigger {
            id: id.into(),
            kind: TriggerKind::App,
            name: name.into(),
            path: Some(path.into()),
            enabled: true,
        }
    }

    fn named(id: &str, name: &str) -> Trigger {
        Trigger {
            id: id.into(),
            kind: TriggerKind::Process,
            name: name.into(),
            path: None,
            enabled: true,
        }
    }

    #[test]
    fn apps_match_their_bundle_not_helpers_or_lookalikes() {
        let xcode = app("a", "Xcode", "/Applications/Xcode.app");
        assert!(xcode.matches(&process(
            "Xcode",
            "/Applications/Xcode.app/Contents/MacOS/Xcode"
        )));
        assert!(!xcode.matches(&process(
            "Xcode",
            "/Applications/Xcode-beta.app/Contents/MacOS/Xcode"
        )));
        assert!(!xcode.matches(&process(
            "Simulator",
            "/Applications/Xcode.app/Contents/Developer/Applications/Simulator.app/Contents/MacOS/Simulator"
        )));
    }

    #[test]
    fn processes_match_by_name_or_executable() {
        let claude = named("c", "claude");
        assert!(claude.matches(&process(
            "claude",
            "/Users/me/.local/share/claude/versions/2.1.309"
        )));
        let codex = named("x", "codex");
        assert!(codex.matches(&process(
            "codex",
            "/Users/me/.codex/packages/standalone/releases/0.164.0/bin/codex"
        )));
        assert!(!codex.matches(&process(
            "codex-code-mode-host",
            "/bin/codex-code-mode-host"
        )));
        let long = named("l", "a-very-long-command-name");
        assert!(long.matches(&process(
            "a-very-long-com",
            "/usr/local/bin/a-very-long-command-name"
        )));
        assert!(named("f", "FFmpeg").matches(&process("ffmpeg", "/opt/homebrew/bin/ffmpeg")));
    }

    #[test]
    fn only_enabled_running_triggers_are_active() {
        let mut off = named("b", "ffmpeg");
        off.enabled = false;
        let triggers = [named("a", "claude"), off, named("c", "xcodebuild")];
        let processes = [
            process("claude", "/x/claude"),
            process("ffmpeg", "/x/ffmpeg"),
        ];
        let active: Vec<&str> = active(&triggers, &processes)
            .iter()
            .map(|trigger| trigger.id.as_str())
            .collect();
        assert_eq!(active, ["a"]);
    }

    #[test]
    fn validation_rejects_bad_names_paths_and_duplicates() {
        assert!(named("a", "claude").validate().is_ok());
        assert!(named("a", " ").validate().is_err());
        assert!(named("a", "/usr/bin/claude").validate().is_err());
        assert!(named("a b", "claude").validate().is_err());
        assert!(app("a", "Xcode", "Xcode.app").validate().is_err());
        let mut no_path = app("a", "Xcode", "/Applications/Xcode.app");
        no_path.path = None;
        assert!(no_path.validate().is_err());
        assert!(validate_all(&[named("a", "x"), named("a", "y")]).is_err());
    }

    #[test]
    fn recovery_keeps_good_triggers() {
        let mut notes = Vec::new();
        let triggers = recover(
            Some(json!([
                { "id": "a", "kind": "process", "name": "claude", "enabled": true },
                { "id": "b", "kind": "app", "name": "Xcode", "enabled": true },
                { "id": "c", "kind": "rocket" },
                { "id": "a", "kind": "process", "name": "dupe", "enabled": true }
            ])),
            &mut notes,
        );
        assert_eq!(triggers, [named("a", "claude")]);
        assert_eq!(notes, ["3 triggers could not be read and were removed."]);
        let mut notes = Vec::new();
        assert!(recover(Some(json!("nope")), &mut notes).is_empty());
        assert_eq!(notes, ["Triggers could not be read and were cleared."]);
    }
}
