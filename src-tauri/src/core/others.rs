use super::{assertion::Held, processes::bundle_of};
use serde::Serialize;

/// Another app or tool keeping the Mac awake, one entry per app.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Other {
    pub pid: i32,
    pub name: String,
    pub display: bool,
    pub reason: Option<String>,
    /// The process that started it, for helpers like `caffeinate`.
    pub parent: Option<String>,
}

const SLEEP_TYPES: [&str; 5] = [
    "PreventUserIdleSystemSleep",
    "PreventUserIdleDisplaySleep",
    "PreventSystemSleep",
    "NoIdleSleepAssertion",
    "NoDisplaySleepAssertion",
];
const DISPLAY_TYPES: [&str; 2] = ["PreventUserIdleDisplaySleep", "NoDisplaySleepAssertion"];
const SYSTEM_PROCESSES: [&str; 4] = ["powerd", "WindowServer", "bluetoothd", "kernel_task"];
const HELPERS: [&str; 2] = ["caffeinate", "pmset"];

/// The app name for an executable path: the outermost `.app`, so helpers count as their app.
pub fn app_name(path: &str) -> Option<String> {
    let bundle = bundle_of(path).or_else(|| path.find(".app/").map(|end| &path[..end + 4]))?;
    let name = bundle.rsplit('/').next()?.trim_end_matches(".app");
    (!name.is_empty()).then(|| name.to_string())
}

/// A reason worth showing: a phrase, not an identifier like `com.apple.audio…`.
fn reason(held: &Held, name: &str) -> Option<String> {
    let text = held.name.trim();
    let identifier = !text.contains(' ') && text.contains('.');
    let generic = text.is_empty()
        || identifier
        || text.eq_ignore_ascii_case(name)
        || text.eq_ignore_ascii_case(&held.process)
        || text == "caffeinate command-line tool";
    (!generic).then(|| text.chars().take(80).collect())
}

pub struct Lookup<'a> {
    pub own_pid: i32,
    /// Name for a pid: its app, or its process name.
    pub name_of: &'a dyn Fn(i32) -> Option<String>,
    pub parent_of: &'a dyn Fn(i32) -> Option<String>,
    pub is_system: &'a dyn Fn(i32) -> bool,
}

/// Filters powerd's bookkeeping and Wick itself, attributes system services to the app they
/// act for, and groups by app.
pub fn others(held: &[Held], lookup: &Lookup) -> Vec<Other> {
    let mut others: Vec<Other> = Vec::new();
    for entry in held {
        let acting_for = entry
            .on_behalf_of
            .filter(|pid| *pid > 0 && *pid != entry.pid);
        if entry.pid == lookup.own_pid
            || acting_for == Some(lookup.own_pid)
            || !SLEEP_TYPES.contains(&entry.assert_type.as_str())
            || SYSTEM_PROCESSES.contains(&entry.process.as_str())
        {
            continue;
        }
        if acting_for.is_some_and(|pid| (lookup.is_system)(pid)) {
            continue;
        }
        let pid = acting_for.unwrap_or(entry.pid);
        let name = (lookup.name_of)(pid)
            .or_else(|| entry.bundle_path.as_deref().and_then(app_name))
            .unwrap_or_else(|| entry.process.clone());
        let display = DISPLAY_TYPES.contains(&entry.assert_type.as_str());
        let reason = reason(entry, &name)
            .or_else(|| (entry.process == "coreaudiod").then(|| "Using audio".to_string()));
        let parent = HELPERS
            .contains(&entry.process.as_str())
            .then(|| (lookup.parent_of)(entry.pid))
            .flatten();
        if let Some(existing) = others
            .iter_mut()
            .find(|other| other.name == name && other.parent == parent)
        {
            existing.display |= display;
            let generic = existing
                .reason
                .as_deref()
                .is_none_or(|text| text == "Using audio");
            if generic && reason.is_some() && (acting_for.is_none() || existing.reason.is_none()) {
                existing.reason = reason;
            }
            continue;
        }
        others.push(Other {
            pid,
            reason,
            parent,
            name,
            display,
        });
    }
    others.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.pid.cmp(&b.pid))
    });
    others
}

/// Name for a pid: its app, or the command name `ps` shows.
pub fn name_of(pid: i32) -> Option<String> {
    super::processes::path(pid)
        .as_deref()
        .and_then(app_name)
        .or_else(|| super::processes::info(pid).map(|(_, name)| name))
}

/// macOS's own services (under /System or /usr) aren't something to act on.
pub fn is_system(pid: i32) -> bool {
    super::processes::path(pid)
        .is_some_and(|path| path.starts_with("/System/") || path.starts_with("/usr/"))
}

/// The parent's name, unless it is launchd or a shell.
pub fn parent_of(pid: i32) -> Option<String> {
    let (ppid, _) = super::processes::info(pid)?;
    if ppid <= 1 {
        return None;
    }
    let name = name_of(ppid)?;
    let shells = ["zsh", "bash", "sh", "fish", "login", "launchd"];
    (!shells.contains(&name.as_str())).then_some(name)
}

pub fn current(held: &[Held]) -> Vec<Other> {
    others(
        held,
        &Lookup {
            own_pid: std::process::id() as i32,
            name_of: &name_of,
            parent_of: &parent_of,
            is_system: &is_system,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(pid: i32, process: &str, kind: &str, name: &str, on_behalf_of: Option<i32>) -> Held {
        Held {
            pid,
            process: process.into(),
            assert_type: kind.into(),
            name: name.into(),
            bundle_path: None,
            on_behalf_of,
        }
    }

    fn names(pid: i32) -> Option<String> {
        match pid {
            2585 | 2826 => Some("Google Chrome".into()),
            77 => Some("caffeinate".into()),
            418 => Some("coreaudiod".into()),
            _ => None,
        }
    }

    fn lookup<'a>(
        name_of: &'a dyn Fn(i32) -> Option<String>,
        parent_of: &'a dyn Fn(i32) -> Option<String>,
    ) -> Lookup<'a> {
        Lookup {
            own_pid: 9,
            name_of,
            parent_of,
            is_system: &|pid| pid == 960,
        }
    }

    #[test]
    fn filters_system_noise_and_wick() {
        let list = [
            held(
                347,
                "powerd",
                "PreventUserIdleSystemSleep",
                "Powerd - Prevent sleep while display is on",
                None,
            ),
            held(407, "WindowServer", "UserIsActive", "tickle", None),
            held(
                9,
                "wick",
                "PreventUserIdleSystemSleep",
                "Wick: awake until stopped",
                None,
            ),
            held(
                418,
                "coreaudiod",
                "PreventUserIdleSystemSleep",
                "com.apple.audio.x.context.preventuseridlesleep",
                Some(9),
            ),
            held(50, "backupd", "PreventDiskIdle", "Time Machine", None),
            held(
                77,
                "caffeinate",
                "PreventUserIdleSystemSleep",
                "caffeinate command-line tool",
                None,
            ),
        ];
        let parent = |_| Some("claude".to_string());
        assert_eq!(
            others(&list, &lookup(&names, &parent)),
            [Other {
                pid: 77,
                name: "caffeinate".into(),
                display: false,
                reason: None,
                parent: Some("claude".into()),
            }]
        );
    }

    #[test]
    fn audio_through_coreaudiod_counts_as_the_app_and_merges() {
        let list = [
            held(
                2585,
                "Google Chrome",
                "NoIdleSleepAssertion",
                "Playing audio",
                None,
            ),
            held(
                2585,
                "Google Chrome",
                "NoDisplaySleepAssertion",
                "Video Wake Lock",
                None,
            ),
            held(
                418,
                "coreaudiod",
                "PreventUserIdleSystemSleep",
                "com.apple.audio.BuiltInSpeakerDevice.context.preventuseridlesleep",
                Some(2826),
            ),
            held(
                418,
                "coreaudiod",
                "PreventUserIdleSystemSleep",
                "com.apple.audio.AudioTap-EE0E579F.context.preventuseridlesleep",
                Some(960),
            ),
        ];
        let none = |_| None;
        let others = others(&list, &lookup(&names, &none));
        assert_eq!(others.len(), 1);
        assert_eq!(others[0].name, "Google Chrome");
        assert!(others[0].display);
        assert_eq!(others[0].reason.as_deref(), Some("Playing audio"));
        assert!(others.iter().all(|other| other
            .reason
            .as_deref()
            .is_none_or(|text| !text.starts_with("com."))));
    }

    #[test]
    fn app_names_come_from_the_outer_bundle() {
        assert_eq!(
            app_name("/Applications/zoom.us.app/Contents/MacOS/zoom.us").as_deref(),
            Some("zoom.us")
        );
        assert_eq!(
            app_name("/Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Helpers/Google Chrome Helper.app/Contents/MacOS/Google Chrome Helper").as_deref(),
            Some("Google Chrome")
        );
        assert_eq!(app_name("/usr/bin/caffeinate"), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sees_a_live_caffeinate_and_who_started_it() {
        let mut child = std::process::Command::new("/usr/bin/caffeinate")
            .args(["-i", "-t", "20"])
            .spawn()
            .unwrap();
        let pid = child.id() as i32;
        let found = (0..20).find_map(|_| {
            std::thread::sleep(std::time::Duration::from_millis(100));
            current(&crate::core::assertion::list().unwrap())
                .into_iter()
                .find(|other| {
                    other
                        .parent
                        .as_deref()
                        .is_some_and(|name| name.starts_with("wick"))
                })
        });
        child.kill().unwrap();
        let _ = child.wait();
        let found = found.expect("caffeinate should appear");
        assert_eq!(found.pid, pid);
        assert_eq!(found.name, "caffeinate");
        assert!(found.parent.is_some_and(|name| name.starts_with("wick")));
    }
}
