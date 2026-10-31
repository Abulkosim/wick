use super::{
    processes::{bundle_of, Process},
    schedule::Schedule,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_TRIGGERS: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriggerKind {
    App,
    Process,
    /// On the power adapter.
    Power,
    /// An external display is connected.
    Display,
    /// Inside a weekly time window.
    Schedule,
    /// Network traffic above a threshold, in KB/s.
    Network,
    /// CPU use above a threshold, in percent.
    Cpu,
}

pub const NETWORK_THRESHOLDS: std::ops::RangeInclusive<u32> = 10..=1_000_000;
pub const CPU_THRESHOLDS: std::ops::RangeInclusive<u32> = 1..=100;

/// What a trigger can look at on each tick.
#[derive(Default)]
pub struct Context<'a> {
    pub processes: &'a [Process],
    pub on_power: bool,
    pub external_display: bool,
    /// Local weekday (0 = Monday) and minutes past midnight.
    pub clock: (u8, u32),
    /// Ids of activity triggers that saw activity within the last two minutes.
    pub busy: &'a [String],
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<Schedule>,
    /// KB/s for network, percent for CPU.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<u32>,
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
        if self.kind == TriggerKind::Schedule {
            return match (&self.schedule, &self.path) {
                (Some(schedule), None) => schedule.validate(),
                _ => Err("A schedule needs days and times.".into()),
            };
        }
        if self.schedule.is_some() {
            return Err("Only schedules have days and times.".into());
        }
        let range = match self.kind {
            TriggerKind::Network => Some(NETWORK_THRESHOLDS),
            TriggerKind::Cpu => Some(CPU_THRESHOLDS),
            _ => None,
        };
        match (range, self.threshold) {
            (Some(range), Some(threshold)) if range.contains(&threshold) => {}
            (Some(_), _) => return Err("Choose how busy counts as busy.".into()),
            (None, Some(_)) => return Err("Only activity has a threshold.".into()),
            (None, None) => {}
        }
        match (self.kind, self.path.as_deref()) {
            (TriggerKind::Schedule, _) => unreachable!(),
            (
                TriggerKind::Power | TriggerKind::Display | TriggerKind::Network | TriggerKind::Cpu,
                None,
            ) => Ok(()),
            (
                TriggerKind::Power | TriggerKind::Display | TriggerKind::Network | TriggerKind::Cpu,
                Some(_),
            ) => Err("A condition has no app path.".into()),
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

    /// "Xcode is running", "on power": completes "Awake while …".
    pub fn phrase(&self) -> String {
        match self.kind {
            TriggerKind::App | TriggerKind::Process => format!("{} is running", self.name.trim()),
            TriggerKind::Power => "on power".into(),
            TriggerKind::Display => "an external display is connected".into(),
            TriggerKind::Schedule => match &self.schedule {
                Some(schedule) => format!("on schedule until {}", schedule.end),
                None => "on schedule".into(),
            },
            TriggerKind::Network => "the network is busy".into(),
            TriggerKind::Cpu => "the CPU is busy".into(),
        }
    }

    /// "Xcode quits", "you unplug": completes "Paused until …".
    pub fn until(&self) -> String {
        match self.kind {
            TriggerKind::App | TriggerKind::Process => format!("{} quits", self.name.trim()),
            TriggerKind::Power => "you unplug".into(),
            TriggerKind::Display => "the display is disconnected".into(),
            TriggerKind::Schedule => "the schedule ends".into(),
            TriggerKind::Network => "the network is quiet".into(),
            TriggerKind::Cpu => "the CPU is quiet".into(),
        }
    }

    /// "Xcode quit", "Unplugged": what happened when this trigger stopped holding.
    pub fn ended(&self) -> String {
        match self.kind {
            TriggerKind::App | TriggerKind::Process => format!("{} quit", self.name.trim()),
            TriggerKind::Power => "Unplugged".into(),
            TriggerKind::Display => "External display disconnected".into(),
            TriggerKind::Schedule => "Schedule ended".into(),
            TriggerKind::Network => "The network is quiet".into(),
            TriggerKind::Cpu => "The CPU is quiet".into(),
        }
    }

    pub fn holds(&self, context: &Context) -> bool {
        self.enabled
            && match self.kind {
                TriggerKind::Power => context.on_power,
                TriggerKind::Display => context.external_display,
                TriggerKind::Schedule => self
                    .schedule
                    .as_ref()
                    .is_some_and(|schedule| schedule.contains(context.clock.0, context.clock.1)),
                TriggerKind::Network | TriggerKind::Cpu => context.busy.contains(&self.id),
                TriggerKind::App | TriggerKind::Process => context
                    .processes
                    .iter()
                    .any(|process| self.matches(process)),
            }
    }

    pub fn matches(&self, process: &Process) -> bool {
        match self.kind {
            TriggerKind::Power
            | TriggerKind::Display
            | TriggerKind::Schedule
            | TriggerKind::Network
            | TriggerKind::Cpu => false,
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
                        .and_then(|path| path.rsplit(['/', '\\']).next())
                        .map(|file| file.strip_suffix(".exe").unwrap_or(file))
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
        let condition = matches!(
            trigger.kind,
            TriggerKind::Power | TriggerKind::Display | TriggerKind::Network | TriggerKind::Cpu
        );
        if condition
            && triggers[..index]
                .iter()
                .any(|other| other.kind == trigger.kind)
        {
            return Err("That condition is already in the list.".into());
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

/// Enabled triggers that hold right now, in list order.
pub fn active<'a>(triggers: &'a [Trigger], context: &Context) -> Vec<&'a Trigger> {
    triggers
        .iter()
        .filter(|trigger| trigger.holds(context))
        .collect()
}

pub fn needs_processes(triggers: &[Trigger]) -> bool {
    triggers.iter().any(|trigger| {
        trigger.enabled && matches!(trigger.kind, TriggerKind::App | TriggerKind::Process)
    })
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
            schedule: None,
            threshold: None,
            enabled: true,
        }
    }

    fn named(id: &str, name: &str) -> Trigger {
        Trigger {
            id: id.into(),
            kind: TriggerKind::Process,
            name: name.into(),
            path: None,
            schedule: None,
            threshold: None,
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
        let context = Context {
            processes: &processes,
            ..Context::default()
        };
        let active: Vec<&str> = active(&triggers, &context)
            .iter()
            .map(|trigger| trigger.id.as_str())
            .collect();
        assert_eq!(active, ["a"]);
    }

    fn condition(id: &str, kind: TriggerKind) -> Trigger {
        Trigger {
            id: id.into(),
            kind,
            name: "condition".into(),
            path: None,
            schedule: None,
            threshold: None,
            enabled: true,
        }
    }

    #[test]
    fn conditions_follow_power_and_displays() {
        let power = condition("p", TriggerKind::Power);
        let display = condition("d", TriggerKind::Display);
        let plugged = Context {
            on_power: true,
            ..Context::default()
        };
        assert!(power.holds(&plugged));
        assert!(!display.holds(&plugged));
        let docked = Context {
            external_display: true,
            ..Context::default()
        };
        assert!(display.holds(&docked));
        assert!(!power.holds(&docked));
        assert_eq!(power.phrase(), "on power");
        assert_eq!(power.until(), "you unplug");
        assert_eq!(display.until(), "the display is disconnected");
        assert_eq!(named("x", "claude").phrase(), "claude is running");
        assert_eq!(named("x", "claude").until(), "claude quits");
        assert!(validate_all(&[power.clone(), condition("q", TriggerKind::Power)]).is_err());
        let mut with_path = power;
        with_path.path = Some("/Applications/X.app".into());
        assert!(with_path.validate().is_err());
    }

    #[test]
    fn activity_holds_while_busy_and_needs_a_threshold() {
        let mut network = condition("n", TriggerKind::Network);
        assert!(network.validate().is_err());
        network.threshold = Some(1_000);
        assert!(network.validate().is_ok());
        network.threshold = Some(5);
        assert!(network.validate().is_err());
        network.threshold = Some(1_000);
        let mut cpu = condition("c", TriggerKind::Cpu);
        cpu.threshold = Some(101);
        assert!(cpu.validate().is_err());
        cpu.threshold = Some(25);
        let mut power = condition("p", TriggerKind::Power);
        power.threshold = Some(25);
        assert!(power.validate().is_err());
        let busy = ["n".to_string()];
        let context = Context {
            busy: &busy,
            ..Context::default()
        };
        assert!(network.holds(&context));
        assert!(!cpu.holds(&context));
        assert_eq!(network.phrase(), "the network is busy");
        assert_eq!(cpu.until(), "the CPU is quiet");
        assert!(validate_all(&[network.clone(), cpu.clone()]).is_ok());
        let mut second = network.clone();
        second.id = "m".into();
        assert!(validate_all(&[network, second]).is_err());
    }

    #[test]
    fn schedules_hold_inside_their_window() {
        let mut work = condition("s", TriggerKind::Schedule);
        assert!(work.validate().is_err());
        work.schedule = Some(Schedule {
            days: vec![0, 1, 2, 3, 4],
            start: "09:00".into(),
            end: "18:00".into(),
        });
        assert!(work.validate().is_ok());
        let at = |day, minute| Context {
            clock: (day, minute),
            ..Context::default()
        };
        assert!(work.holds(&at(2, 600)));
        assert!(!work.holds(&at(5, 600)));
        assert_eq!(work.phrase(), "on schedule until 18:00");
        assert_eq!(work.until(), "the schedule ends");
        let mut process = named("p", "claude");
        process.schedule = work.schedule.clone();
        assert!(process.validate().is_err());
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
