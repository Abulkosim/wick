use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Power {
    pub on_battery: bool,
    /// `None` on Macs without a battery.
    pub percent: Option<u8>,
    pub low_power_mode: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guard {
    pub enabled: bool,
    pub threshold: u8,
    pub stop_in_low_power_mode: bool,
}

/// Why a running session should end now, if it should.
pub fn stop_reason(power: &Power, guard: &Guard) -> Option<String> {
    if !power.on_battery {
        return None;
    }
    if guard.enabled
        && power
            .percent
            .is_some_and(|percent| percent < guard.threshold)
    {
        return Some(format!("Stopped: battery below {}%", guard.threshold));
    }
    if guard.stop_in_low_power_mode && power.low_power_mode {
        return Some("Stopped: Low Power Mode is on".into());
    }
    None
}

#[cfg(target_os = "macos")]
pub fn read() -> Power {
    use core_foundation::{
        array::{CFArray, CFArrayRef},
        base::{CFType, CFTypeRef, TCFType},
        dictionary::{CFDictionary, CFDictionaryRef},
        number::CFNumber,
        string::CFString,
    };

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        fn IOPSCopyPowerSourcesInfo() -> CFTypeRef;
        fn IOPSCopyPowerSourcesList(blob: CFTypeRef) -> CFArrayRef;
        fn IOPSGetPowerSourceDescription(blob: CFTypeRef, source: CFTypeRef) -> CFDictionaryRef;
    }

    let low_power_mode = objc2_foundation::NSProcessInfo::processInfo().isLowPowerModeEnabled();
    let mut power = Power {
        low_power_mode,
        ..Power::default()
    };
    unsafe {
        let blob = IOPSCopyPowerSourcesInfo();
        if blob.is_null() {
            return power;
        }
        let blob = CFType::wrap_under_create_rule(blob);
        let list = IOPSCopyPowerSourcesList(blob.as_CFTypeRef());
        if list.is_null() {
            return power;
        }
        let list: CFArray<CFType> = CFArray::wrap_under_create_rule(list);
        for source in list.iter() {
            let description =
                IOPSGetPowerSourceDescription(blob.as_CFTypeRef(), source.as_CFTypeRef());
            if description.is_null() {
                continue;
            }
            let description: CFDictionary<CFString, CFType> =
                CFDictionary::wrap_under_get_rule(description);
            let text = |key: &str| {
                description
                    .find(CFString::new(key))
                    .and_then(|value| value.downcast::<CFString>())
                    .map(|value| value.to_string())
            };
            let number = |key: &str| {
                description
                    .find(CFString::new(key))
                    .and_then(|value| value.downcast::<CFNumber>())
                    .and_then(|value| value.to_i64())
            };
            if text("Type").as_deref() != Some("InternalBattery") {
                continue;
            }
            power.on_battery = text("Power Source State").as_deref() == Some("Battery Power");
            if let (Some(current), Some(max)) = (number("Current Capacity"), number("Max Capacity"))
            {
                if max > 0 {
                    power.percent = Some((current * 100 / max).clamp(0, 100) as u8);
                }
            }
        }
    }
    power
}

#[cfg(not(target_os = "macos"))]
pub fn read() -> Power {
    Power::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUARD: Guard = Guard {
        enabled: true,
        threshold: 20,
        stop_in_low_power_mode: false,
    };

    fn battery(percent: u8) -> Power {
        Power {
            on_battery: true,
            percent: Some(percent),
            low_power_mode: false,
        }
    }

    #[test]
    fn stops_at_or_below_the_threshold_on_battery_only() {
        assert_eq!(stop_reason(&battery(20), &GUARD), None);
        assert_eq!(
            stop_reason(&battery(19), &GUARD).as_deref(),
            Some("Stopped: battery below 20%")
        );
        let charging = Power {
            on_battery: false,
            ..battery(5)
        };
        assert_eq!(stop_reason(&charging, &GUARD), None);
        let off = Guard {
            enabled: false,
            ..GUARD
        };
        assert_eq!(stop_reason(&battery(5), &off), None);
    }

    #[test]
    fn low_power_mode_only_when_asked() {
        let saving = Power {
            low_power_mode: true,
            ..battery(80)
        };
        assert_eq!(stop_reason(&saving, &GUARD), None);
        let asked = Guard {
            stop_in_low_power_mode: true,
            ..GUARD
        };
        assert_eq!(
            stop_reason(&saving, &asked).as_deref(),
            Some("Stopped: Low Power Mode is on")
        );
    }

    #[test]
    fn desktops_never_stop() {
        let desktop = Power::default();
        assert_eq!(stop_reason(&desktop, &GUARD), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reads_this_mac() {
        let power = read();
        assert!(power.percent.is_none_or(|percent| percent <= 100));
    }
}
