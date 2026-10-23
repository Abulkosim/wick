/// Whether closing the lid would sleep this Mac now. `None` on Macs without a lid.
/// It reads `No` in clamshell mode (power, external display and keyboard attached).
#[cfg(target_os = "macos")]
pub fn closing_sleeps() -> Option<bool> {
    use core_foundation::{
        base::{CFType, CFTypeRef, TCFType},
        boolean::CFBoolean,
        dictionary::CFMutableDictionaryRef,
        string::{CFString, CFStringRef},
    };
    use std::ffi::{c_char, c_void};

    #[link(name = "IOKit", kind = "framework")]
    extern "C" {
        fn IOServiceMatching(name: *const c_char) -> CFMutableDictionaryRef;
        fn IOServiceGetMatchingService(main_port: u32, matching: CFMutableDictionaryRef) -> u32;
        fn IORegistryEntryCreateCFProperty(
            entry: u32,
            key: CFStringRef,
            allocator: *const c_void,
            options: u32,
        ) -> CFTypeRef;
        fn IOObjectRelease(object: u32) -> i32;
    }

    unsafe {
        let matching = IOServiceMatching(c"IOPMrootDomain".as_ptr());
        if matching.is_null() {
            return None;
        }
        let entry = IOServiceGetMatchingService(0, matching);
        if entry == 0 {
            return None;
        }
        let read = |key: &str| {
            let key = CFString::new(key);
            let value = IORegistryEntryCreateCFProperty(
                entry,
                key.as_concrete_TypeRef(),
                std::ptr::null(),
                0,
            );
            (!value.is_null()).then(|| CFType::wrap_under_create_rule(value))
        };
        let has_lid = read("AppleClamshellState").is_some();
        let causes = read("AppleClamshellCausesSleep")
            .and_then(|value| value.downcast::<CFBoolean>())
            .map(bool::from);
        IOObjectRelease(entry);
        if has_lid {
            causes
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn closing_sleeps() -> Option<bool> {
    None
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn matches_ioreg() {
        let output = std::process::Command::new("/usr/sbin/ioreg")
            .args(["-r", "-c", "IOPMrootDomain", "-d", "1"])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&output.stdout);
        let expected = text
            .lines()
            .find(|line| line.contains("\"AppleClamshellCausesSleep\""))
            .map(|line| line.trim_end().ends_with("Yes"));
        let has_lid = text.contains("\"AppleClamshellState\"");
        assert_eq!(super::closing_sleeps(), expected.filter(|_| has_lid));
    }
}
