/// Whether a display other than the built-in one is connected, even if it is asleep or mirrored.
#[cfg(target_os = "macos")]
pub fn external_connected() -> bool {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGGetOnlineDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
        fn CGDisplayIsBuiltin(display: u32) -> u32;
    }

    let mut displays = [0_u32; 16];
    let mut count = 0_u32;
    let status = unsafe { CGGetOnlineDisplayList(16, displays.as_mut_ptr(), &mut count) };
    status == 0
        && displays[..count.min(16) as usize]
            .iter()
            .any(|&display| unsafe { CGDisplayIsBuiltin(display) } == 0)
}

/// Windows can't tell a laptop panel from a monitor cheaply; more than one screen counts.
#[cfg(windows)]
pub fn external_connected() -> bool {
    unsafe { super::win32::GetSystemMetrics(super::win32::SM_CMONITORS) > 1 }
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn external_connected() -> bool {
    false
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn agrees_with_system_profiler() {
        let output = std::process::Command::new("/usr/sbin/system_profiler")
            .args(["SPDisplaysDataType", "-json"])
            .output()
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
        let screens: Vec<&serde_json::Value> = json["SPDisplaysDataType"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|gpu| gpu["spdisplays_ndrvs"].as_array().into_iter().flatten())
            .collect();
        if screens.is_empty() {
            return;
        }
        let external = screens
            .iter()
            .any(|screen| screen["spdisplays_connection_type"] != "spdisplays_internal");
        assert_eq!(super::external_connected(), external, "{screens:?}");
    }
}
