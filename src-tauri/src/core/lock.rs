/// Whether the login session's screen is locked. False without a GUI session.
#[cfg(target_os = "macos")]
pub fn screen_locked() -> bool {
    use core_foundation::{
        base::{CFType, TCFType},
        boolean::CFBoolean,
        dictionary::{CFDictionary, CFDictionaryRef},
        number::CFNumber,
        string::CFString,
    };

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGSessionCopyCurrentDictionary() -> CFDictionaryRef;
    }

    let raw = unsafe { CGSessionCopyCurrentDictionary() };
    if raw.is_null() {
        return false;
    }
    let session: CFDictionary<CFString, CFType> =
        unsafe { CFDictionary::wrap_under_create_rule(raw) };
    session
        .find(CFString::new("CGSSessionScreenIsLocked"))
        .map(|value| {
            value
                .downcast::<CFBoolean>()
                .map(bool::from)
                .or_else(|| {
                    value
                        .downcast::<CFNumber>()
                        .and_then(|number| number.to_i32())
                        .map(|n| n != 0)
                })
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// logind's `LockedHint`, which GNOME, KDE and most lockers set.
#[cfg(target_os = "linux")]
pub fn screen_locked() -> bool {
    super::linux::bus::screen_locked()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn screen_locked() -> bool {
    false
}

#[cfg(test)]
mod tests {
    /// CI machines are never locked; a developer's may be while tests run.
    #[test]
    fn an_unlocked_ci_run_reads_unlocked() {
        let locked = super::screen_locked();
        if std::env::var_os("CI").is_some() {
            assert!(!locked);
        }
    }
}
