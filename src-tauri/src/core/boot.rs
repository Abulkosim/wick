/// When the computer last booted, in Unix seconds. Used to resume a session only within the
/// same boot, never after a restart of the machine.
#[cfg(target_os = "macos")]
pub fn boot_time() -> Option<i64> {
    use std::ffi::{c_int, c_void};
    extern "C" {
        fn sysctlbyname(
            name: *const std::ffi::c_char,
            old: *mut c_void,
            old_length: *mut usize,
            new: *mut c_void,
            new_length: usize,
        ) -> c_int;
    }
    #[repr(C)]
    struct Timeval {
        seconds: i64,
        micros: i32,
    }
    let mut value = Timeval {
        seconds: 0,
        micros: 0,
    };
    let mut size = std::mem::size_of::<Timeval>();
    let status = unsafe {
        sysctlbyname(
            c"kern.boottime".as_ptr(),
            (&mut value as *mut Timeval).cast(),
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };
    (status == 0 && value.seconds > 0).then_some(value.seconds)
}

#[cfg(windows)]
pub fn boot_time() -> Option<i64> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetTickCount64() -> u64;
    }
    let uptime = unsafe { GetTickCount64() } as i64 / 1000;
    Some(chrono::Utc::now().timestamp() - uptime)
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn boot_time() -> Option<i64> {
    None
}

/// Boot times read at different moments can differ by a second of rounding.
pub fn same_boot(a: Option<i64>, b: Option<i64>) -> bool {
    matches!((a, b), (Some(a), Some(b)) if (a - b).abs() <= 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_time_is_in_the_past_and_stable() {
        let now = chrono::Utc::now().timestamp();
        if let Some(boot) = boot_time() {
            assert!(boot < now && boot > now - 400 * 24 * 3600);
            assert!(same_boot(Some(boot), boot_time()));
        }
        assert!(!same_boot(None, Some(1)));
        assert!(!same_boot(Some(1), Some(100)));
    }
}
