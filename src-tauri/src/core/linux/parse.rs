//! Text formats from `/proc`, `/sys` and logind, parsed without touching the system so they are
//! tested on every platform.

use crate::core::{others::Other, power::Power};

/// One entry of `/sys/class/power_supply`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Supply {
    /// `Mains`, `USB`, `Battery`, …
    pub kind: String,
    /// `Device` for a mouse or keyboard battery, `System` or absent for the computer's own.
    pub scope: Option<String>,
    pub online: Option<bool>,
    pub status: Option<String>,
    pub capacity: Option<u8>,
    /// `energy_*` (µWh) or `charge_*` (µAh), whichever the battery reports.
    pub now: Option<u64>,
    pub full: Option<u64>,
}

/// On battery when the computer has a battery and no adapter is online. Several batteries (some
/// ThinkPads) are combined by their charge, not averaged.
pub fn power(supplies: &[Supply], low_power_mode: bool) -> Power {
    let batteries: Vec<&Supply> = supplies
        .iter()
        .filter(|supply| supply.kind == "Battery" && supply.scope.as_deref() != Some("Device"))
        .collect();
    let adapter_online = supplies
        .iter()
        .any(|supply| supply.kind != "Battery" && supply.online == Some(true));
    let discharging = batteries
        .iter()
        .any(|battery| battery.status.as_deref() == Some("Discharging"));
    let (now, full) = batteries
        .iter()
        .try_fold((0u64, 0u64), |(now, full), battery| {
            Some((now + battery.now?, full + battery.full?))
        })
        .unwrap_or((0, 0));
    let percent = if let Some(ratio) = now.saturating_mul(100).checked_div(full) {
        Some(ratio.min(100) as u8)
    } else {
        let readings: Vec<u32> = batteries
            .iter()
            .filter_map(|battery| battery.capacity.map(u32::from))
            .collect();
        (!readings.is_empty())
            .then(|| (readings.iter().sum::<u32>() / readings.len() as u32).min(100) as u8)
    };
    Power {
        on_battery: !batteries.is_empty() && (discharging || !adapter_online),
        percent: percent.filter(|_| !batteries.is_empty()),
        low_power_mode,
    }
}

/// `btime` from `/proc/stat`: boot time in Unix seconds.
pub fn boot_time(stat: &str) -> Option<i64> {
    stat.lines()
        .find_map(|line| line.strip_prefix("btime "))
        .and_then(|value| value.trim().parse().ok())
}

/// Busy and total CPU time, in clock ticks, from the first line of `/proc/stat`.
pub fn cpu_times(stat: &str) -> Option<(u64, u64)> {
    let line = stat.lines().find(|line| line.starts_with("cpu "))?;
    let values: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .map(|value| value.parse().unwrap_or(0))
        .collect();
    let field = |index: usize| values.get(index).copied().unwrap_or(0);
    // user nice system idle iowait irq softirq steal
    let idle = field(3) + field(4);
    let busy = field(0) + field(1) + field(2) + field(5) + field(6) + field(7);
    (values.len() >= 4).then_some((busy, busy + idle))
}

/// Interface name with bytes received and sent, from `/proc/net/dev`.
pub fn net_dev(text: &str) -> Vec<(String, u64, u64)> {
    text.lines()
        .skip(2)
        .filter_map(|line| {
            let (name, counters) = line.split_once(':')?;
            let counters: Vec<u64> = counters
                .split_whitespace()
                .map(|value| value.parse().unwrap_or(0))
                .collect();
            Some((
                name.trim().to_string(),
                *counters.first()?,
                *counters.get(8)?,
            ))
        })
        .collect()
}

/// Parent pid and kernel command name from `/proc/<pid>/stat`. The name sits in parentheses
/// and may itself contain spaces or parentheses, so it ends at the last `)`.
pub fn proc_stat(text: &str) -> Option<(i32, String)> {
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    let name = text.get(open + 1..close)?.to_string();
    let mut rest = text.get(close + 1..)?.split_whitespace();
    let _state = rest.next()?;
    let ppid = rest.next()?.parse().ok()?;
    Some((ppid, name))
}

/// The basename of argv[0] from `/proc/<pid>/cmdline`: what users typed (`claude`), which the
/// 15-character kernel name may cut short.
pub fn argv0(cmdline: &[u8]) -> Option<String> {
    let first = cmdline.split(|&byte| byte == 0).next()?;
    let first = String::from_utf8_lossy(first);
    // Some programs rewrite their title into argv[0] with spaces ("sshd: user@pts/0").
    let command = first.split(' ').next()?;
    let name = command
        .rsplit('/')
        .next()?
        .trim_start_matches('-')
        .trim_end_matches(':');
    (!name.is_empty()).then(|| name.to_string())
}

/// Whether a connector other than the built-in panel is connected. `connectors` are
/// `/sys/class/drm` entry names (`card0-eDP-1`) with their `status`.
pub fn external_display(connectors: &[(String, String)]) -> bool {
    connectors.iter().any(|(name, status)| {
        let kind = name.split_once('-').map_or(name.as_str(), |(_, kind)| kind);
        let internal = ["eDP", "LVDS", "DSI"]
            .iter()
            .any(|prefix| kind.starts_with(prefix));
        status.trim() == "connected" && !internal
    })
}

/// `state:      closed` from `/proc/acpi/button/lid/*/state`: true when open.
pub fn lid_open(text: &str) -> Option<bool> {
    let state = text.split(':').nth(1)?.trim();
    match state {
        "open" => Some(true),
        "closed" => Some(false),
        _ => None,
    }
}

/// logind's lid settings and state, as `org.freedesktop.login1.Manager` reports them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LidPolicy {
    pub handle: String,
    pub external_power: String,
    pub docked: String,
    pub on_external_power: bool,
    pub is_docked: bool,
    /// Another process (GNOME, KDE) handles the lid itself.
    pub taken_over: bool,
}

/// Whether closing the lid would sleep or turn off the computer under logind's rules.
pub fn closing_sleeps(policy: &LidPolicy) -> bool {
    if policy.taken_over {
        return false;
    }
    let action = if policy.is_docked {
        &policy.docked
    } else if policy.on_external_power && !policy.external_power.is_empty() {
        &policy.external_power
    } else {
        &policy.handle
    };
    matches!(
        action.as_str(),
        "suspend"
            | "hibernate"
            | "hybrid-sleep"
            | "suspend-then-hibernate"
            | "sleep"
            | "poweroff"
            | "halt"
    )
}

/// One of logind's `ListInhibitors` rows.
#[derive(Clone, Debug, PartialEq)]
pub struct Inhibitor {
    pub what: String,
    pub who: String,
    pub why: String,
    pub mode: String,
    pub pid: u32,
}

/// One inhibitor a desktop's session manager holds for an app (GNOME, KDE).
#[derive(Clone, Debug, PartialEq)]
pub struct SessionInhibitor {
    pub app: String,
    pub reason: String,
    /// Keeps the display on, not only the system.
    pub display: bool,
}

/// Processes that hold sleep inhibitors for their own housekeeping.
const SYSTEM: [&str; 6] = [
    "ModemManager",
    "NetworkManager",
    "UPower",
    "GNOME Shell",
    "gsd-power",
    "PowerDevil",
];

/// Others keeping the computer awake, from logind and the desktop's session manager, without
/// Wick's own inhibitors and system services that only delay sleep.
pub fn others(
    logind: &[Inhibitor],
    session: &[SessionInhibitor],
    own_pid: u32,
    parent_of: &dyn Fn(u32) -> Option<String>,
) -> Vec<Other> {
    let mut others: Vec<Other> = Vec::new();
    let mut push = |name: String, reason: Option<String>, display: bool, pid: i32, parent| {
        if let Some(existing) = others.iter_mut().find(|other| other.name == name) {
            existing.display |= display;
            if existing.reason.is_none() {
                existing.reason = reason;
            }
            return;
        }
        others.push(Other {
            pid,
            name,
            display,
            reason,
            parent,
        });
    };
    let reason = |text: &str, name: &str| {
        let text = text.trim();
        (!text.is_empty() && !text.eq_ignore_ascii_case(name))
            .then(|| text.chars().take(80).collect::<String>())
    };
    for inhibitor in logind {
        let blocks = inhibitor.mode == "block"
            && inhibitor
                .what
                .split(':')
                .any(|what| what == "idle" || what == "sleep");
        if !blocks
            || inhibitor.pid == own_pid
            || inhibitor.why.starts_with("Wick: ")
            || SYSTEM.contains(&inhibitor.who.as_str())
        {
            continue;
        }
        let name = inhibitor.who.trim().to_string();
        if name.is_empty() {
            continue;
        }
        let helper = name == "systemd-inhibit" || name.starts_with("systemd-inhibit ");
        let parent = helper.then(|| parent_of(inhibitor.pid)).flatten();
        let name = if helper {
            "systemd-inhibit".to_string()
        } else {
            name
        };
        let display = inhibitor.what.split(':').any(|what| what == "idle");
        push(
            name.clone(),
            reason(&inhibitor.why, &name),
            display,
            inhibitor.pid as i32,
            parent,
        );
    }
    for (index, inhibitor) in session.iter().enumerate() {
        if inhibitor.reason.starts_with("Wick: ") {
            continue;
        }
        let name = friendly_app(&inhibitor.app);
        if name.is_empty() || SYSTEM.contains(&name.as_str()) {
            continue;
        }
        // Session inhibitors have no pid; a negative index keeps rows distinct.
        push(
            name.clone(),
            reason(&inhibitor.reason, &name),
            inhibitor.display,
            -(index as i32) - 1,
            None,
        );
    }
    others.sort_by_key(|other| other.name.to_lowercase());
    others
}

/// `org.mozilla.firefox` → `firefox`, `firefox.desktop` → `firefox`, `Firefox` stays.
pub fn friendly_app(app: &str) -> String {
    let app = app.trim().trim_end_matches(".desktop");
    if app.contains('.') && !app.contains(' ') {
        app.rsplit('.').next().unwrap_or(app).to_string()
    } else {
        app.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn battery(status: &str, capacity: u8) -> Supply {
        Supply {
            kind: "Battery".into(),
            status: Some(status.into()),
            capacity: Some(capacity),
            ..Supply::default()
        }
    }

    fn mains(online: bool) -> Supply {
        Supply {
            kind: "Mains".into(),
            online: Some(online),
            ..Supply::default()
        }
    }

    #[test]
    fn power_follows_adapters_and_batteries() {
        let laptop = power(&[mains(false), battery("Discharging", 82)], false);
        assert_eq!(
            laptop,
            Power {
                on_battery: true,
                percent: Some(82),
                low_power_mode: false
            }
        );
        assert!(!power(&[mains(true), battery("Charging", 40)], false).on_battery);
        assert!(!power(&[mains(true), battery("Not charging", 80)], false).on_battery);
        let desktop = power(&[mains(true)], true);
        assert_eq!(desktop.percent, None);
        assert!(!desktop.on_battery && desktop.low_power_mode);
        let mut mouse = battery("Discharging", 5);
        mouse.scope = Some("Device".into());
        assert_eq!(power(&[mains(true), mouse], false).percent, None);
        let thinkpad = power(
            &[
                Supply {
                    now: Some(10),
                    full: Some(50),
                    ..battery("Discharging", 20)
                },
                Supply {
                    now: Some(40),
                    full: Some(50),
                    ..battery("Discharging", 80)
                },
            ],
            false,
        );
        assert_eq!(thinkpad.percent, Some(50));
        let usb_c = Supply {
            kind: "USB".into(),
            online: Some(true),
            ..Supply::default()
        };
        assert!(!power(&[usb_c, battery("Unknown", 60)], false).on_battery);
    }

    #[test]
    fn proc_files() {
        let stat = "cpu  100 5 50 800 20 3 2 1 0 0\ncpu0 1 2 3 4\nbtime 1792982137\n";
        assert_eq!(boot_time(stat), Some(1_792_982_137));
        assert_eq!(cpu_times(stat), Some((161, 981)));
        assert_eq!(cpu_times("intr 1"), None);
        let dev = "Inter-|   Receive\n face |bytes\n    lo: 100 1 0 0 0 0 0 0 100 1 0 0 0 0 0 0\n  wlp2s0: 5000 10 0 0 0 0 0 0 700 9 0 0 0 0 0 0\n";
        assert_eq!(
            net_dev(dev),
            [
                ("lo".to_string(), 100, 100),
                ("wlp2s0".to_string(), 5000, 700)
            ]
        );
        assert_eq!(
            proc_stat("4242 (Web Content (x)) S 4100 4242 4100 0 -1"),
            Some((4100, "Web Content (x)".to_string()))
        );
        assert_eq!(proc_stat("garbage"), None);
        assert_eq!(
            argv0(b"/usr/local/bin/claude\0--resume\0").as_deref(),
            Some("claude")
        );
        assert_eq!(argv0(b"-bash\0").as_deref(), Some("bash"));
        assert_eq!(argv0(b"sshd: me@pts/0\0").as_deref(), Some("sshd"));
        assert_eq!(argv0(b""), None);
    }

    #[test]
    fn displays_and_lid() {
        let connector = |name: &str, status: &str| (name.to_string(), status.to_string());
        assert!(!external_display(&[
            connector("card0-eDP-1", "connected"),
            connector("card0-HDMI-A-1", "disconnected"),
        ]));
        assert!(external_display(&[
            connector("card0-eDP-1", "connected"),
            connector("card0-DP-2", "connected\n"),
        ]));
        assert_eq!(lid_open("state:      open\n"), Some(true));
        assert_eq!(lid_open("state:      closed"), Some(false));
        assert_eq!(lid_open(""), None);
        let mut policy = LidPolicy {
            handle: "suspend".into(),
            external_power: "".into(),
            docked: "ignore".into(),
            ..LidPolicy::default()
        };
        assert!(closing_sleeps(&policy));
        policy.on_external_power = true;
        assert!(closing_sleeps(&policy));
        policy.external_power = "lock".into();
        assert!(!closing_sleeps(&policy));
        policy.is_docked = true;
        assert!(!closing_sleeps(&policy));
        policy.is_docked = false;
        policy.on_external_power = false;
        policy.taken_over = true;
        assert!(!closing_sleeps(&policy));
    }

    #[test]
    fn others_skip_wick_delays_and_services() {
        let inhibitor = |what: &str, who: &str, why: &str, mode: &str, pid: u32| Inhibitor {
            what: what.into(),
            who: who.into(),
            why: why.into(),
            mode: mode.into(),
            pid,
        };
        let logind = [
            inhibitor("sleep", "NetworkManager", "Disconnect", "delay", 800),
            inhibitor(
                "handle-lid-switch",
                "gsd-power",
                "External monitor",
                "block",
                900,
            ),
            inhibitor("idle", "Wick", "Wick: awake until 16:30", "block", 7),
            inhibitor("idle:sleep", "systemd-inhibit", "make", "block", 4100),
            inhibitor("sleep", "Steam", "Downloading", "block", 3000),
            inhibitor("sleep", "Steam", "Downloading", "block", 3001),
        ];
        let session = [
            SessionInhibitor {
                app: "org.mozilla.firefox".into(),
                reason: "Playing video".into(),
                display: true,
            },
            SessionInhibitor {
                app: "".into(),
                reason: "Wick: awake until stopped".into(),
                display: false,
            },
        ];
        let list = others(&logind, &session, 7, &|pid| {
            (pid == 4100).then(|| "cargo".to_string())
        });
        assert_eq!(
            list,
            [
                Other {
                    pid: -1,
                    name: "firefox".into(),
                    display: true,
                    reason: Some("Playing video".into()),
                    parent: None
                },
                Other {
                    pid: 3000,
                    name: "Steam".into(),
                    display: false,
                    reason: Some("Downloading".into()),
                    parent: None
                },
                Other {
                    pid: 4100,
                    name: "systemd-inhibit".into(),
                    display: true,
                    reason: Some("make".into()),
                    parent: Some("cargo".into())
                },
            ]
        );
        assert_eq!(friendly_app("firefox.desktop"), "firefox");
        assert_eq!(friendly_app("Google Chrome"), "Google Chrome");
    }
}
