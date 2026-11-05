//! Readers for `/proc` and `/sys`.

use super::parse::{self, Supply};
use crate::core::power::Power;
use std::{fs, path::Path};

fn text(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_string())
}

fn number<T: std::str::FromStr>(path: impl AsRef<Path>) -> Option<T> {
    text(path)?.parse().ok()
}

fn supply(directory: &Path) -> Supply {
    let (now, full) = match (
        number(directory.join("energy_now")),
        number(directory.join("energy_full")),
    ) {
        (Some(now), Some(full)) => (Some(now), Some(full)),
        _ => (
            number(directory.join("charge_now")),
            number(directory.join("charge_full")),
        ),
    };
    Supply {
        kind: text(directory.join("type")).unwrap_or_default(),
        scope: text(directory.join("scope")),
        online: number::<u8>(directory.join("online")).map(|value| value == 1),
        status: text(directory.join("status")),
        capacity: number(directory.join("capacity")),
        now,
        full,
    }
}

pub fn power() -> Power {
    let supplies: Vec<Supply> = fs::read_dir("/sys/class/power_supply")
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| supply(&entry.path()))
        .collect();
    let has_battery = supplies.iter().any(|supply| supply.kind == "Battery");
    // Only ask power-profiles-daemon when the answer can matter.
    let low_power_mode = has_battery && super::bus::power_saver();
    parse::power(&supplies, low_power_mode)
}

pub fn boot_time() -> Option<i64> {
    parse::boot_time(&fs::read_to_string("/proc/stat").ok()?)
}

pub fn cpu_times() -> Option<(u64, u64)> {
    parse::cpu_times(&fs::read_to_string("/proc/stat").ok()?)
}

/// Bytes in and out of physical interfaces (those backed by a device; not loopback, bridges,
/// tunnels or containers), keyed like `activity::Counters`.
pub fn network_bytes() -> Vec<(u32, u64)> {
    let Ok(dev) = fs::read_to_string("/proc/net/dev") else {
        return Vec::new();
    };
    parse::net_dev(&dev)
        .into_iter()
        .filter_map(|(name, received, sent)| {
            let interface = Path::new("/sys/class/net").join(&name);
            if !interface.join("device").exists() {
                return None;
            }
            let index: u32 = number(interface.join("ifindex"))?;
            let key = index.wrapping_mul(2);
            Some([(key, received), (key.wrapping_add(1), sent)])
        })
        .flatten()
        .collect()
}

pub fn external_display() -> bool {
    let connectors: Vec<(String, String)> = fs::read_dir("/sys/class/drm")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.contains('-')
                .then(|| text(entry.path().join("status")))
                .flatten()
                .map(|status| (name, status))
        })
        .collect();
    parse::external_display(&connectors)
}

/// Whether this computer has a lid at all.
pub fn has_lid() -> bool {
    fs::read_dir("/proc/acpi/button/lid")
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            text(entry.path().join("state")).is_some_and(|state| parse::lid_open(&state).is_some())
        })
}

pub fn info(pid: i32) -> Option<(i32, String)> {
    let (ppid, comm) = parse::proc_stat(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)?;
    // argv[0] is what users typed and isn't cut at 15 characters like the kernel name; kernel
    // threads have none.
    let name = fs::read(format!("/proc/{pid}/cmdline"))
        .ok()
        .and_then(|cmdline| parse::argv0(&cmdline))
        .unwrap_or(comm);
    (!name.is_empty()).then_some((ppid, name))
}

pub fn path(pid: i32) -> Option<String> {
    fs::read_link(format!("/proc/{pid}/exe")).ok().map(|path| {
        path.to_string_lossy()
            .trim_end_matches(" (deleted)")
            .to_string()
    })
}

pub fn pids() -> Vec<i32> {
    fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_this_system() {
        let boot = boot_time().unwrap();
        assert!(boot > 1_000_000_000);
        let (busy, total) = cpu_times().unwrap();
        assert!(total >= busy && total > 0);
        let own = std::process::id() as i32;
        let (ppid, name) = info(own).unwrap();
        assert!(ppid > 0 && !name.is_empty());
        assert!(path(own).is_some());
        assert!(pids().contains(&own));
        let _ = (power(), external_display(), has_lid(), network_bytes());
    }

    #[test]
    fn names_follow_argv0_when_the_kernel_cuts_them() {
        let directory = tempfile::tempdir().unwrap();
        let link = directory.path().join("a-very-long-command-name");
        std::os::unix::fs::symlink("/bin/sleep", &link).unwrap();
        let mut child = std::process::Command::new(&link).arg("30").spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        let name = info(child.id() as i32).map(|(_, name)| name);
        child.kill().unwrap();
        let _ = child.wait();
        assert_eq!(name.as_deref(), Some("a-very-long-command-name"));
    }
}
