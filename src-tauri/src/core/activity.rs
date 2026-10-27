use super::triggers::{Trigger, TriggerKind};
use std::collections::HashMap;

/// How long activity must stay below its threshold before a trigger lets go, so a pause
/// between files or build steps doesn't drop the hold.
pub const QUIET_MS: i64 = 120_000;
/// Samples further apart than this span a sleep; their average means nothing.
const MAX_SAMPLE_GAP_MS: i64 = 60_000;

/// Running totals since boot. Counters may wrap at 32 bits; see `grew`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Counters {
    pub at_ms: i64,
    /// Bytes received and bytes sent, per physical interface: keyed by interface index * 2,
    /// plus 1 for sent.
    pub network: Vec<(u32, u64)>,
    /// (busy, total) CPU time in ticks, if readable.
    pub cpu: Option<(u64, u64)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rates {
    pub network_bytes_per_second: u64,
    pub cpu_percent: u32,
}

/// How much a counter grew. macOS hands apps 32-bit byte counters and its CPU ticks are 32-bit,
/// so a smaller value after a 32-bit one is a wrap; any other drop is a reset and counts nothing.
fn grew(before: u64, after: u64) -> u64 {
    match after.checked_sub(before) {
        Some(delta) => delta,
        None if before <= u64::from(u32::MAX) => after + (1 << 32) - before,
        None => 0,
    }
}

/// The average rates between two samples, or `None` if they are too far apart.
pub fn rates(previous: &Counters, next: &Counters) -> Option<Rates> {
    let elapsed = next.at_ms - previous.at_ms;
    if elapsed <= 0 || elapsed > MAX_SAMPLE_GAP_MS {
        return None;
    }
    let bytes: u64 = next
        .network
        .iter()
        .filter_map(|(index, total)| {
            previous
                .network
                .iter()
                .find(|(other, _)| other == index)
                .map(|(_, before)| grew(*before, *total))
        })
        .sum();
    let cpu_percent = match (previous.cpu, next.cpu) {
        (Some((busy_before, total_before)), Some((busy, total))) => {
            let total = grew(total_before, total);
            let busy = grew(busy_before, busy).min(total);
            (busy * 100).checked_div(total).unwrap_or(0) as u32
        }
        _ => 0,
    };
    Some(Rates {
        network_bytes_per_second: bytes.saturating_mul(1000) / elapsed as u64,
        cpu_percent,
    })
}

/// Whether `trigger` sees activity above its threshold in `rates`.
fn above(trigger: &Trigger, rates: &Rates) -> bool {
    let threshold = u64::from(trigger.threshold.unwrap_or(u32::MAX));
    match trigger.kind {
        TriggerKind::Network => rates.network_bytes_per_second >= threshold * 1000,
        TriggerKind::Cpu => u64::from(rates.cpu_percent) >= threshold,
        _ => false,
    }
}

pub fn watches(trigger: &Trigger) -> bool {
    trigger.enabled && matches!(trigger.kind, TriggerKind::Network | TriggerKind::Cpu)
}

/// Remembers the last sample and when each activity trigger last saw activity.
#[derive(Debug, Default)]
pub struct Monitor {
    last: Option<Counters>,
    last_above: HashMap<String, i64>,
}

impl Monitor {
    /// Takes a new sample and returns the ids of activity triggers that are busy: above their
    /// threshold now or within `QUIET_MS` of it.
    pub fn update(&mut self, triggers: &[Trigger], counters: Option<Counters>) -> Vec<String> {
        let watched: Vec<&Trigger> = triggers.iter().filter(|trigger| watches(trigger)).collect();
        self.last_above
            .retain(|id, _| watched.iter().any(|trigger| &trigger.id == id));
        let Some(counters) = counters.filter(|_| !watched.is_empty()) else {
            self.last = None;
            return Vec::new();
        };
        let now = counters.at_ms;
        if let Some(rates) = self.last.as_ref().and_then(|last| rates(last, &counters)) {
            for trigger in &watched {
                if above(trigger, &rates) {
                    self.last_above.insert(trigger.id.clone(), now);
                }
            }
        }
        self.last = Some(counters);
        watched
            .iter()
            .filter(|trigger| {
                self.last_above
                    .get(&trigger.id)
                    .is_some_and(|at| now - at <= QUIET_MS)
            })
            .map(|trigger| trigger.id.clone())
            .collect()
    }
}

/// Reads the counters the enabled activity triggers need.
pub fn read(triggers: &[Trigger], now: i64) -> Option<Counters> {
    let wants = |kind| {
        triggers
            .iter()
            .any(|trigger| watches(trigger) && trigger.kind == kind)
    };
    let (network, cpu) = (wants(TriggerKind::Network), wants(TriggerKind::Cpu));
    if !network && !cpu {
        return None;
    }
    Some(Counters {
        at_ms: now,
        network: if network {
            platform::network()
        } else {
            Vec::new()
        },
        cpu: cpu.then(platform::cpu).flatten(),
    })
}

#[cfg(target_os = "macos")]
mod platform {
    use std::ffi::c_void;

    const CTL_NET: i32 = 4;
    const PF_ROUTE: i32 = 17;
    const NET_RT_IFLIST2: i32 = 6;
    const RTM_IFINFO2: u8 = 0x12;
    const IFT_ETHER: u8 = 0x06;
    /// `if_msghdr2` is 32 bytes, then `if_data64` (packed to 4): type at 0, bytes in at 64,
    /// bytes out at 72.
    const DATA: usize = 32;

    extern "C" {
        fn sysctl(
            name: *mut i32,
            length: u32,
            old: *mut c_void,
            old_length: *mut usize,
            new: *mut c_void,
            new_length: usize,
        ) -> i32;
        fn mach_host_self() -> u32;
        fn mach_task_self() -> u32;
        fn mach_port_deallocate(task: u32, name: u32) -> i32;
        fn host_statistics(host: u32, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
    }

    fn u64_at(bytes: &[u8], offset: usize) -> Option<u64> {
        bytes
            .get(offset..offset + 8)
            .map(|slice| u64::from_ne_bytes(slice.try_into().unwrap_or_default()))
    }

    /// Ethernet-type interfaces (Wi-Fi, Ethernet, USB adapters). Tunnels and loopback are left
    /// out so VPN traffic isn't counted twice.
    pub fn network() -> Vec<(u32, u64)> {
        let mut mib = [CTL_NET, PF_ROUTE, 0, 0, NET_RT_IFLIST2, 0];
        let mut length = 0usize;
        let mut call = |buffer: *mut c_void, length: &mut usize| unsafe {
            sysctl(
                mib.as_mut_ptr(),
                mib.len() as u32,
                buffer,
                length,
                std::ptr::null_mut(),
                0,
            )
        };
        if call(std::ptr::null_mut(), &mut length) != 0 || length == 0 {
            return Vec::new();
        }
        let mut buffer = vec![0u8; length + 4096];
        length = buffer.len();
        if call(buffer.as_mut_ptr().cast(), &mut length) != 0 {
            return Vec::new();
        }
        buffer.truncate(length);
        let mut interfaces = Vec::new();
        let mut offset = 0;
        while offset + 4 <= buffer.len() {
            let message_length = u16::from_ne_bytes([buffer[offset], buffer[offset + 1]]) as usize;
            if message_length == 0 {
                break;
            }
            let message = &buffer[offset..(offset + message_length).min(buffer.len())];
            if message.len() >= DATA + 80 && message[3] == RTM_IFINFO2 && message[DATA] == IFT_ETHER
            {
                let index = u16::from_ne_bytes([message[12], message[13]]);
                if let (Some(received), Some(sent)) =
                    (u64_at(message, DATA + 64), u64_at(message, DATA + 72))
                {
                    let key = u32::from(index) * 2;
                    interfaces.extend([(key, received), (key + 1, sent)]);
                }
            }
            offset += message_length;
        }
        interfaces
    }

    /// Busy and total ticks across all cores: user, system, idle, nice.
    pub fn cpu() -> Option<(u64, u64)> {
        const HOST_CPU_LOAD_INFO: i32 = 3;
        let mut ticks = [0i32; 4];
        let mut count = ticks.len() as u32;
        let ok = unsafe {
            let host = mach_host_self();
            let result = host_statistics(host, HOST_CPU_LOAD_INFO, ticks.as_mut_ptr(), &mut count);
            mach_port_deallocate(mach_task_self(), host);
            result == 0
        };
        if !ok {
            return None;
        }
        let [user, system, idle, nice] = ticks.map(|value| u64::from(value as u32));
        let busy = user + system + nice;
        Some((busy, busy + idle))
    }
}

#[cfg(windows)]
mod platform {
    pub fn network() -> Vec<(u32, u64)> {
        super::super::win32::network_bytes()
    }

    pub fn cpu() -> Option<(u64, u64)> {
        super::super::win32::cpu_times()
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod platform {
    pub fn network() -> Vec<(u32, u64)> {
        Vec::new()
    }

    pub fn cpu() -> Option<(u64, u64)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trigger(id: &str, kind: TriggerKind, threshold: u32) -> Trigger {
        Trigger {
            id: id.into(),
            kind,
            name: "activity".into(),
            path: None,
            schedule: None,
            threshold: Some(threshold),
            enabled: true,
        }
    }

    fn sample(at_ms: i64, bytes: u64, cpu: (u64, u64)) -> Option<Counters> {
        Some(Counters {
            at_ms,
            network: vec![(4, bytes)],
            cpu: Some(cpu),
        })
    }

    #[test]
    fn rates_average_between_samples() {
        let a = sample(0, 1_000, (100, 1_000)).unwrap();
        let b = sample(5_000, 5_001_000, (400, 2_000)).unwrap();
        assert_eq!(
            rates(&a, &b),
            Some(Rates {
                network_bytes_per_second: 1_000_000,
                cpu_percent: 30
            })
        );
        let wrapped = sample(0, u64::from(u32::MAX) - 99, (0, 0)).unwrap();
        let after = sample(1_000, 100, (0, 0)).unwrap();
        assert_eq!(
            rates(&wrapped, &after).unwrap().network_bytes_per_second,
            200
        );
        let reset = sample(1_000, 100, (0, 0)).unwrap();
        assert_eq!(
            rates(&sample(0, 1 << 40, (0, 0)).unwrap(), &reset)
                .unwrap()
                .network_bytes_per_second,
            0
        );
        assert_eq!(rates(&a, &sample(120_000, 0, (0, 0)).unwrap()), None);
        let mut new_interface = b.clone();
        new_interface.network.push((9, 1 << 40));
        assert_eq!(
            rates(&a, &new_interface).unwrap().network_bytes_per_second,
            1_000_000
        );
    }

    #[test]
    fn busy_holds_until_quiet_for_two_minutes() {
        let network = trigger("n", TriggerKind::Network, 1_000);
        let cpu = trigger("c", TriggerKind::Cpu, 50);
        let list = [network, cpu];
        let mut monitor = Monitor::default();
        assert!(monitor.update(&list, sample(0, 0, (0, 0))).is_empty());
        // 2 MB/s and 20% CPU: only the network is busy.
        assert_eq!(
            monitor.update(&list, sample(5_000, 10_000_000, (20, 100))),
            ["n"]
        );
        // Quiet again, but within two minutes.
        assert_eq!(
            monitor.update(&list, sample(60_000, 10_000_000, (40, 200))),
            ["n"]
        );
        assert!(monitor
            .update(&list, sample(125_001, 10_000_000, (60, 300)))
            .is_empty());
        // CPU at 90%.
        assert_eq!(
            monitor.update(&list, sample(130_000, 10_000_000, (150, 400))),
            ["c"]
        );
    }

    #[test]
    fn disabled_or_removed_triggers_forget_their_activity() {
        let mut network = trigger("n", TriggerKind::Network, 100);
        let mut monitor = Monitor::default();
        monitor.update(std::slice::from_ref(&network), sample(0, 0, (0, 0)));
        assert_eq!(
            monitor.update(
                std::slice::from_ref(&network),
                sample(1_000, 1_000_000, (0, 0))
            ),
            ["n"]
        );
        network.enabled = false;
        assert!(monitor
            .update(std::slice::from_ref(&network), None)
            .is_empty());
        network.enabled = true;
        assert!(monitor
            .update(
                std::slice::from_ref(&network),
                sample(2_000, 1_000_000, (0, 0))
            )
            .is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reads_live_counters() {
        let list = [
            trigger("n", TriggerKind::Network, 1),
            trigger("c", TriggerKind::Cpu, 1),
        ];
        let counters = read(&list, 0).unwrap();
        assert!(!counters.network.is_empty(), "no Ethernet-type interfaces");
        let (busy, total) = counters.cpu.unwrap();
        assert!(total > busy && busy > 0);
    }
}
