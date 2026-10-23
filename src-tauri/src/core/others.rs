use super::assertion::Held;
use serde::Serialize;

/// Another process keeping the Mac awake, one entry per process.
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

fn friendly(held: &Held) -> String {
    held.bundle_path
        .as_deref()
        .and_then(|path| path.split('/').find(|part| part.ends_with(".app")))
        .map(|bundle| bundle.trim_end_matches(".app").to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| held.process.clone())
}

/// A reason worth showing: not the process name again, not a generic tool label.
fn reason(held: &Held, name: &str) -> Option<String> {
    let text = held.name.trim();
    let generic = text.is_empty()
        || text.eq_ignore_ascii_case(name)
        || text.eq_ignore_ascii_case(&held.process)
        || text == "caffeinate command-line tool";
    (!generic).then(|| text.chars().take(80).collect())
}

/// Filters powerd's bookkeeping and Wick itself, and groups by process.
pub fn others(
    held: &[Held],
    own_pid: i32,
    parent_of: impl Fn(i32) -> Option<String>,
) -> Vec<Other> {
    let mut others: Vec<Other> = Vec::new();
    for entry in held {
        if entry.pid == own_pid
            || !SLEEP_TYPES.contains(&entry.assert_type.as_str())
            || SYSTEM_PROCESSES.contains(&entry.process.as_str())
        {
            continue;
        }
        let display = DISPLAY_TYPES.contains(&entry.assert_type.as_str());
        if let Some(existing) = others.iter_mut().find(|other| other.pid == entry.pid) {
            existing.display |= display;
            continue;
        }
        let name = friendly(entry);
        others.push(Other {
            pid: entry.pid,
            reason: reason(entry, &name),
            parent: HELPERS
                .contains(&entry.process.as_str())
                .then(|| parent_of(entry.pid))
                .flatten(),
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

/// The name of `pid`'s parent process, unless it is launchd or a shell.
#[cfg(target_os = "macos")]
pub fn parent_name(pid: i32) -> Option<String> {
    use std::ffi::{c_int, c_void, CStr};

    extern "C" {
        fn proc_pidinfo(
            pid: c_int,
            flavor: c_int,
            arg: u64,
            buffer: *mut c_void,
            size: c_int,
        ) -> c_int;
        fn proc_name(pid: c_int, buffer: *mut c_void, size: u32) -> c_int;
    }
    const PROC_PIDTBSDINFO: c_int = 3;
    const BSDINFO_SIZE: usize = 136;
    const PPID_OFFSET: usize = 16;

    let mut info = [0_u8; BSDINFO_SIZE];
    let read = unsafe {
        proc_pidinfo(
            pid,
            PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            BSDINFO_SIZE as c_int,
        )
    };
    if read as usize != BSDINFO_SIZE {
        return None;
    }
    let ppid = u32::from_ne_bytes(info[PPID_OFFSET..PPID_OFFSET + 4].try_into().ok()?) as i32;
    if ppid <= 1 {
        return None;
    }
    let mut name = [0_u8; 64];
    if unsafe { proc_name(ppid, name.as_mut_ptr().cast(), name.len() as u32) } <= 0 {
        return None;
    }
    let name = unsafe { CStr::from_ptr(name.as_ptr().cast()) }
        .to_string_lossy()
        .into_owned();
    let shells = ["zsh", "bash", "sh", "fish", "login", "launchd"];
    (!shells.contains(&name.as_str())).then_some(name)
}

#[cfg(not(target_os = "macos"))]
pub fn parent_name(_pid: i32) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(pid: i32, process: &str, kind: &str, name: &str, bundle: Option<&str>) -> Held {
        Held {
            pid,
            process: process.into(),
            assert_type: kind.into(),
            name: name.into(),
            bundle_path: bundle.map(String::from),
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
            held(50, "backupd", "PreventDiskIdle", "Time Machine", None),
            held(
                77,
                "caffeinate",
                "PreventUserIdleSystemSleep",
                "caffeinate command-line tool",
                None,
            ),
        ];
        let others = others(&list, 9, |_| Some("claude".into()));
        assert_eq!(
            others,
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
    fn groups_by_process_and_names_apps_by_bundle() {
        let list = [
            held(
                5,
                "CptHost",
                "PreventUserIdleSystemSleep",
                "Zoom meeting",
                Some("/Applications/zoom.us.app/Contents/Frameworks/cpthost.app"),
            ),
            held(
                5,
                "CptHost",
                "PreventUserIdleDisplaySleep",
                "Zoom meeting",
                Some("/Applications/zoom.us.app/Contents/Frameworks/cpthost.app"),
            ),
            held(
                6,
                "Music",
                "PreventUserIdleSystemSleep",
                "Music",
                Some("/System/Applications/Music.app"),
            ),
        ];
        let others = others(&list, 1, |_| None);
        assert_eq!(others.len(), 2);
        assert_eq!(others[1].name, "zoom.us");
        assert_eq!(others[0].name, "Music");
        assert!(others[1].display);
        assert_eq!(others[1].reason.as_deref(), Some("Zoom meeting"));
        assert_eq!(others[0].reason, None);
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
            let held = crate::core::assertion::list().unwrap();
            others(&held, std::process::id() as i32, parent_name)
                .into_iter()
                .find(|other| other.pid == pid)
        });
        child.kill().unwrap();
        let _ = child.wait();
        let found = found.expect("caffeinate should appear");
        assert_eq!(found.name, "caffeinate");
        assert!(!found.display);
        assert!(found.parent.is_some_and(|name| name.starts_with("wick")));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn finds_the_parent_of_a_child_process() {
        let mut child = std::process::Command::new("/bin/sleep")
            .arg("5")
            .spawn()
            .unwrap();
        let parent = parent_name(child.id() as i32);
        child.kill().unwrap();
        let _ = child.wait();
        assert!(parent.is_some_and(|name| !name.is_empty()));
    }
}
