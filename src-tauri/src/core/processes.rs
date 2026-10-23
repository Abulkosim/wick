use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Process {
    pub pid: i32,
    /// The command name `ps` shows (`claude`), even when the binary file is named otherwise.
    pub name: String,
    pub path: Option<String>,
}

/// `ps`'s command name: `pbi_comm`, unless `pbi_name` is the same name untruncated.
pub fn command_name(comm: &str, name: &str) -> String {
    if comm.is_empty() || (name.starts_with(comm) && name.len() > comm.len()) {
        name.to_string()
    } else {
        comm.to_string()
    }
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
mod native {
    use std::ffi::{c_char, c_int, c_void, CStr};

    extern "C" {
        fn proc_listallpids(buffer: *mut c_void, size: c_int) -> c_int;
        fn proc_pidinfo(
            pid: c_int,
            flavor: c_int,
            arg: u64,
            buffer: *mut c_void,
            size: c_int,
        ) -> c_int;
        fn proc_pidpath(pid: c_int, buffer: *mut c_void, size: u32) -> c_int;
        fn sysctl(
            name: *mut c_int,
            length: u32,
            old: *mut c_void,
            old_length: *mut usize,
            new: *mut c_void,
            new_length: usize,
        ) -> c_int;
    }

    /// The basename of argv[0], which is what users typed (`claude`, not `2.1.309`).
    fn argv0(pid: i32) -> Option<String> {
        const CTL_KERN: c_int = 1;
        const KERN_PROCARGS2: c_int = 49;
        let mut mib = [CTL_KERN, KERN_PROCARGS2, pid];
        let mut size: usize = 8192;
        let mut buffer = vec![0_u8; size];
        let status = unsafe {
            sysctl(
                mib.as_mut_ptr(),
                3,
                buffer.as_mut_ptr().cast(),
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        if status != 0 || size <= 4 {
            return None;
        }
        let rest = &buffer[4..size.min(buffer.len())];
        let after_exec = rest.iter().position(|&byte| byte == 0)?;
        let start = after_exec + rest[after_exec..].iter().position(|&byte| byte != 0)?;
        let length = rest[start..].iter().position(|&byte| byte == 0)?;
        let first = String::from_utf8_lossy(&rest[start..start + length]).into_owned();
        let name = first
            .rsplit('/')
            .next()?
            .trim_start_matches('-')
            .to_string();
        (!name.is_empty()).then_some(name)
    }

    const PROC_PIDTBSDINFO: c_int = 3;
    const BSDINFO_SIZE: usize = 136;
    const PPID: usize = 16;
    const COMM: std::ops::Range<usize> = 48..64;
    const NAME: std::ops::Range<usize> = 64..96;

    fn text(bytes: &[u8]) -> String {
        let end = bytes
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end]).into_owned()
    }

    /// Parent pid and command name, from `proc_bsdinfo`.
    pub fn info(pid: i32) -> Option<(i32, String)> {
        let mut buffer = [0_u8; BSDINFO_SIZE];
        let size = unsafe {
            proc_pidinfo(
                pid,
                PROC_PIDTBSDINFO,
                0,
                buffer.as_mut_ptr().cast(),
                BSDINFO_SIZE as c_int,
            )
        };
        if size as usize != BSDINFO_SIZE {
            return None;
        }
        let ppid = u32::from_ne_bytes(buffer[PPID..PPID + 4].try_into().ok()?) as i32;
        let name = argv0(pid)
            .unwrap_or_else(|| super::command_name(&text(&buffer[COMM]), &text(&buffer[NAME])));
        (!name.is_empty()).then_some((ppid, name))
    }

    pub fn path(pid: i32) -> Option<String> {
        let mut buffer = vec![0_u8; 4096];
        let length = unsafe { proc_pidpath(pid, buffer.as_mut_ptr().cast(), buffer.len() as u32) };
        (length > 0).then(|| {
            unsafe { CStr::from_ptr(buffer.as_ptr().cast::<c_char>()) }
                .to_string_lossy()
                .into_owned()
        })
    }

    pub fn pids() -> Vec<i32> {
        let mut pids = vec![0_i32; 4096];
        let count = unsafe {
            proc_listallpids(
                pids.as_mut_ptr().cast(),
                (pids.len() * std::mem::size_of::<i32>()) as c_int,
            )
        };
        pids.truncate(count.max(0) as usize);
        pids
    }
}

#[cfg(target_os = "macos")]
pub use native::{info, path};

#[cfg(not(target_os = "macos"))]
pub fn info(_pid: i32) -> Option<(i32, String)> {
    None
}

#[cfg(windows)]
pub fn path(pid: i32) -> Option<String> {
    super::win32::process_path(pid as u32)
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn path(_pid: i32) -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
pub fn scan() -> Vec<Process> {
    let own = std::process::id() as i32;
    native::pids()
        .into_iter()
        .filter(|&pid| pid > 0 && pid != own)
        .filter_map(|pid| {
            Some(Process {
                pid,
                name: info(pid)?.1,
                path: path(pid),
            })
        })
        .collect()
}

/// Process names lose `.exe`, so a `claude` trigger matches `claude.exe`.
#[cfg(windows)]
pub fn scan() -> Vec<Process> {
    use super::win32::*;
    let own = std::process::id();
    let mut processes = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return processes;
        }
        let mut entry: ProcessEntry32W = std::mem::zeroed();
        entry.size = std::mem::size_of::<ProcessEntry32W>() as u32;
        let mut more = Process32FirstW(snapshot, &mut entry) != 0;
        while more {
            let pid = entry.process_id;
            if pid != 0 && pid != own {
                let file = from_wide(&entry.exe_file);
                let name = file
                    .strip_suffix(".exe")
                    .or_else(|| file.strip_suffix(".EXE"))
                    .unwrap_or(&file)
                    .to_string();
                processes.push(Process {
                    pid: pid as i32,
                    name,
                    path: None,
                });
            }
            more = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
    }
    processes
}

#[cfg(not(any(target_os = "macos", windows)))]
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
    fn command_names_prefer_what_ps_shows() {
        assert_eq!(command_name("claude", "2.1.309"), "claude");
        assert_eq!(
            command_name("a-very-long-com", "a-very-long-command"),
            "a-very-long-command"
        );
        assert_eq!(command_name("", "zsh"), "zsh");
        assert_eq!(command_name("ffmpeg", "ffmpeg"), "ffmpeg");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn scan_reports_the_name_it_was_started_as() {
        let directory = tempfile::tempdir().unwrap();
        let link = directory.path().join("napper");
        std::os::unix::fs::symlink("/bin/sleep", &link).unwrap();
        let mut child = std::process::Command::new(&link).arg("30").spawn().unwrap();
        let found = (0..100).find_map(|_| {
            std::thread::sleep(std::time::Duration::from_millis(50));
            scan()
                .into_iter()
                .find(|process| process.pid == child.id() as i32)
        });
        child.kill().unwrap();
        let _ = child.wait();
        assert_eq!(found.unwrap().name, "napper");
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
