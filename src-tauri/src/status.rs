use crate::runtime::AppState;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const FILE: &str = "status.json";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status<'a> {
    pid: u32,
    awake: bool,
    session: &'a crate::core::session::Session,
    ends_at: Option<i64>,
    held_by: Option<&'a str>,
    paused_by: Option<&'a str>,
    paused_while_locked: bool,
    keep_display_on: bool,
    boot: Option<i64>,
    /// Set only around a restart Wick starts itself, so only that restart resumes the session.
    resume: bool,
}

pub fn snapshot_with(state: &AppState, pid: u32, resume: bool) -> Value {
    let held = state.held_by.as_deref().filter(|_| !state.session.is_on());
    serde_json::to_value(Status {
        pid,
        awake: (state.session.is_on() || held.is_some()) && !state.paused_by_lock,
        session: &state.session,
        ends_at: state.session.ends_at(),
        held_by: held,
        paused_by: state.paused_by.as_deref(),
        paused_while_locked: state.paused_by_lock,
        keep_display_on: state.settings.keep_display_on,
        boot: crate::core::boot::boot_time(),
        resume,
    })
    .unwrap_or_default()
}

/// The last snapshot written, so writes from different threads stay in revision order and an
/// unchanged state isn't rewritten.
static LAST: std::sync::Mutex<Option<(PathBuf, u64, Value)>> = std::sync::Mutex::new(None);

/// Written on every change so `wick status` can answer without the running app.
pub fn write(directory: &Path, state: &AppState, resume: bool) {
    let value = snapshot_with(state, std::process::id(), resume);
    let Ok(mut last) = LAST.lock() else {
        return;
    };
    if let Some((path, revision, previous)) = last.as_ref() {
        if path == directory && (state.revision < *revision || *previous == value) {
            return;
        }
    }
    let _ = std::fs::create_dir_all(directory);
    let Ok(mut file) = tempfile::NamedTempFile::new_in(directory) else {
        return;
    };
    if serde_json::to_writer(&mut file, &value).is_ok()
        && file.persist(directory.join(FILE)).is_ok()
    {
        *last = Some((directory.to_path_buf(), state.revision, value));
    }
}

#[cfg(windows)]
pub fn default_directory() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|data| PathBuf::from(data).join("com.abulkosim.wick"))
}

#[cfg(not(windows))]
pub fn default_directory() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support/com.abulkosim.wick"))
}

#[cfg(windows)]
fn alive(pid: u64) -> bool {
    u32::try_from(pid).is_ok_and(|pid| pid > 0 && crate::core::win32::process_alive(pid))
}

#[cfg(not(windows))]
fn alive(pid: u64) -> bool {
    extern "C" {
        fn kill(pid: i32, signal: i32) -> i32;
    }
    i32::try_from(pid).is_ok_and(|pid| pid > 0 && unsafe { kill(pid, 0) } == 0)
}

/// A session saved just before Wick restarted itself (an update) in this boot that hasn't run
/// out. A quit, crash or force-quit never resumes.
pub fn resumable(directory: &Path, now: i64) -> Option<crate::core::session::Session> {
    let saved: Value = serde_json::from_slice(&std::fs::read(directory.join(FILE)).ok()?).ok()?;
    let boot = saved["boot"].as_i64();
    if saved["resume"] != true
        || !crate::core::boot::same_boot(boot, crate::core::boot::boot_time())
    {
        return None;
    }
    let session: crate::core::session::Session =
        serde_json::from_value(saved["session"].clone()).ok()?;
    (session.is_on() && !session.expired(now)).then_some(session)
}

/// What `wick status` prints: the saved snapshot if its app is still running.
pub fn read(directory: &Path) -> Value {
    let saved: Option<Value> = std::fs::read(directory.join(FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    match saved {
        Some(mut value) if value["pid"].as_u64().is_some_and(alive) => {
            value["running"] = json!(true);
            value
        }
        _ => json!({ "running": false, "awake": false }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::Session;

    #[test]
    fn status_reports_the_live_app_and_a_stale_file_as_stopped() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(read(directory.path())["running"], false);
        let state = AppState {
            session: Session::For {
                started_at: 0,
                ends_at: 60_000,
            },
            ..AppState::default()
        };
        write(directory.path(), &state, false);
        let live = read(directory.path());
        assert_eq!(live["running"], true);
        assert_eq!(live["awake"], true);
        assert_eq!(live["endsAt"], 60_000);
        assert_eq!(live["session"]["mode"], "for");
        let mut stale = snapshot_with(&state, 0, false);
        stale["pid"] = json!(999_999_999u64);
        std::fs::write(directory.path().join(FILE), stale.to_string()).unwrap();
        assert_eq!(
            read(directory.path()),
            json!({ "running": false, "awake": false })
        );
    }

    #[test]
    fn resumes_only_a_live_session_from_this_boot() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(resumable(directory.path(), 0), None);
        let running = AppState {
            session: Session::For {
                started_at: 0,
                ends_at: 60_000,
            },
            ..AppState::default()
        };
        write(directory.path(), &running, false);
        assert_eq!(resumable(directory.path(), 30_000), None);
        write(directory.path(), &running, true);
        assert_eq!(resumable(directory.path(), 30_000), Some(running.session));
        assert_eq!(resumable(directory.path(), 60_000), None);
        let stopped = AppState {
            revision: 1,
            ..AppState::default()
        };
        write(directory.path(), &stopped, false);
        assert_eq!(resumable(directory.path(), 30_000), None);
        let mut other_boot = snapshot_with(&running, 1, true);
        other_boot["boot"] = json!(1);
        std::fs::write(directory.path().join(FILE), other_boot.to_string()).unwrap();
        assert_eq!(resumable(directory.path(), 30_000), None);
    }

    #[test]
    fn held_and_locked_states() {
        let mut state = AppState {
            held_by: Some("Xcode is running".into()),
            ..AppState::default()
        };
        assert_eq!(snapshot_with(&state, 1, false)["awake"], true);
        assert_eq!(
            snapshot_with(&state, 1, false)["heldBy"],
            "Xcode is running"
        );
        state.paused_by_lock = true;
        assert_eq!(snapshot_with(&state, 1, false)["awake"], false);
    }
}
