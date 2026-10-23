use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Process {
    pub pid: i32,
    pub name: String,
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningApp {
    pub name: String,
    pub path: String,
}

/// The `.app` bundle a process runs from, if its executable sits in `Contents/MacOS`.
pub fn bundle_of(executable: &str) -> Option<&str> {
    let end = executable.find(".app/Contents/MacOS/")? + ".app".len();
    let bundle = &executable[..end];
    (!bundle[..bundle.len() - 4].contains(".app/")).then_some(bundle)
}

/// Apps the user can pick, one per bundle, sorted by name. System and Wick's own are left out.
pub fn running_apps(processes: &[Process]) -> Vec<RunningApp> {
    let mut apps: Vec<RunningApp> = processes
        .iter()
        .filter_map(|process| bundle_of(process.path.as_deref()?))
        .filter(|bundle| !bundle.starts_with("/System/") && !bundle.ends_with("/Wick.app"))
        .map(|bundle| RunningApp {
            name: bundle
                .rsplit('/')
                .next()
                .unwrap_or(bundle)
                .trim_end_matches(".app")
                .to_string(),
            path: bundle.to_string(),
        })
        .collect();
    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.path.cmp(&b.path))
    });
    apps.dedup_by(|a, b| a.path == b.path);
    apps
}

#[cfg(target_os = "macos")]
pub fn scan() -> Vec<Process> {
    use std::ffi::{c_char, c_int, c_void, CStr};

    extern "C" {
        fn proc_listallpids(buffer: *mut c_void, size: c_int) -> c_int;
        fn proc_name(pid: c_int, buffer: *mut c_void, size: u32) -> c_int;
        fn proc_pidpath(pid: c_int, buffer: *mut c_void, size: u32) -> c_int;
    }

    let own = std::process::id() as i32;
    let mut pids = vec![0_i32; 4096];
    let count = unsafe {
        proc_listallpids(
            pids.as_mut_ptr().cast(),
            (pids.len() * std::mem::size_of::<i32>()) as c_int,
        )
    };
    if count <= 0 {
        return Vec::new();
    }
    pids.truncate(count as usize);
    let read =
        |pid: i32, call: unsafe extern "C" fn(c_int, *mut c_void, u32) -> c_int, size: usize| {
            let mut buffer = vec![0_u8; size];
            let length = unsafe { call(pid, buffer.as_mut_ptr().cast(), size as u32) };
            (length > 0).then(|| {
                unsafe { CStr::from_ptr(buffer.as_ptr().cast::<c_char>()) }
                    .to_string_lossy()
                    .into_owned()
            })
        };
    pids.into_iter()
        .filter(|&pid| pid > 0 && pid != own)
        .filter_map(|pid| {
            Some(Process {
                pid,
                name: read(pid, proc_name, 64)?,
                path: read(pid, proc_pidpath, 4096),
            })
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
pub fn scan() -> Vec<Process> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(name: &str, path: &str) -> Process {
        Process {
            pid: 1,
            name: name.into(),
            path: Some(path.into()),
        }
    }

    #[test]
    fn finds_the_outer_bundle_only() {
        assert_eq!(
            bundle_of("/Applications/Xcode.app/Contents/MacOS/Xcode"),
            Some("/Applications/Xcode.app")
        );
        assert_eq!(
            bundle_of("/Applications/Claude.app/Contents/Helpers/Claude Helper.app/Contents/MacOS/Claude Helper"),
            None
        );
        assert_eq!(bundle_of("/usr/local/bin/ffmpeg"), None);
    }

    #[test]
    fn lists_apps_once_by_name_without_system_or_wick() {
        let apps = running_apps(&[
            process(
                "zoom.us",
                "/Applications/zoom.us.app/Contents/MacOS/zoom.us",
            ),
            process("Xcode", "/Applications/Xcode.app/Contents/MacOS/Xcode"),
            process("Xcode", "/Applications/Xcode.app/Contents/MacOS/Xcode"),
            process(
                "Finder",
                "/System/Library/CoreServices/Finder.app/Contents/MacOS/Finder",
            ),
            process("wick", "/Applications/Wick.app/Contents/MacOS/wick"),
            process("claude", "/Users/me/.local/share/claude/versions/2.1.309"),
        ]);
        assert_eq!(
            apps.iter().map(|app| app.name.as_str()).collect::<Vec<_>>(),
            ["Xcode", "zoom.us"]
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn scans_this_mac_quickly() {
        let started = std::time::Instant::now();
        let processes = scan();
        assert!(processes.iter().any(|process| process.name == "launchd"
            || process.name == "Finder"
            || process.name == "zsh"
            || process.name == "cargo"));
        assert!(started.elapsed().as_millis() < 500);
    }
}
