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

#[cfg(not(target_os = "macos"))]
pub fn screen_locked() -> bool {
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_unlocked_test_run_reads_unlocked() {
        assert!(!super::screen_locked());
    }
}
