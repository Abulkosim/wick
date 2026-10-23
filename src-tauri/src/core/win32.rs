//! The few Win32 calls Wick needs, declared by hand to keep the dependency surface small.
#![allow(non_snake_case, clippy::upper_case_acronyms)]

pub type HANDLE = isize;
pub const INVALID_HANDLE_VALUE: HANDLE = -1;

pub const POWER_REQUEST_CONTEXT_VERSION: u32 = 0;
pub const POWER_REQUEST_CONTEXT_SIMPLE_STRING: u32 = 0x1;
pub const POWER_REQUEST_DISPLAY_REQUIRED: i32 = 0;
pub const POWER_REQUEST_SYSTEM_REQUIRED: i32 = 1;

pub const TH32CS_SNAPPROCESS: u32 = 0x2;
pub const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
pub const STILL_ACTIVE: u32 = 259;
pub const SM_CMONITORS: i32 = 80;

/// `REASON_CONTEXT`: the union is sized for its detailed form; Wick uses the simple string.
#[repr(C)]
pub struct ReasonContext {
    pub version: u32,
    pub flags: u32,
    pub reason: [*const u16; 3],
}

#[repr(C)]
#[derive(Default)]
pub struct SystemPowerStatus {
    pub ac_line_status: u8,
    pub battery_flag: u8,
    pub battery_life_percent: u8,
    pub system_status_flag: u8,
    pub battery_life_time: u32,
    pub battery_full_life_time: u32,
}

#[repr(C)]
pub struct ProcessEntry32W {
    pub size: u32,
    pub usage: u32,
    pub process_id: u32,
    pub default_heap_id: usize,
    pub module_id: u32,
    pub threads: u32,
    pub parent_process_id: u32,
    pub priority_class_base: i32,
    pub flags: u32,
    pub exe_file: [u16; 260],
}

#[link(name = "kernel32")]
extern "system" {
    pub fn PowerCreateRequest(context: *const ReasonContext) -> HANDLE;
    pub fn PowerSetRequest(request: HANDLE, kind: i32) -> i32;
    pub fn PowerClearRequest(request: HANDLE, kind: i32) -> i32;
    pub fn CloseHandle(handle: HANDLE) -> i32;
    pub fn GetSystemPowerStatus(status: *mut SystemPowerStatus) -> i32;
    pub fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> HANDLE;
    pub fn Process32FirstW(snapshot: HANDLE, entry: *mut ProcessEntry32W) -> i32;
    pub fn Process32NextW(snapshot: HANDLE, entry: *mut ProcessEntry32W) -> i32;
    pub fn OpenProcess(access: u32, inherit: i32, process_id: u32) -> HANDLE;
    pub fn QueryFullProcessImageNameW(
        process: HANDLE,
        flags: u32,
        name: *mut u16,
        size: *mut u32,
    ) -> i32;
    pub fn GetExitCodeProcess(process: HANDLE, code: *mut u32) -> i32;
}

#[link(name = "user32")]
extern "system" {
    pub fn GetSystemMetrics(index: i32) -> i32;
}

pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn from_wide(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

/// Full path of a running process, when Windows lets us read it.
pub fn process_path(pid: u32) -> Option<String> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process == 0 {
            return None;
        }
        let mut buffer = [0_u16; 1024];
        let mut size = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size);
        CloseHandle(process);
        (ok != 0).then(|| String::from_utf16_lossy(&buffer[..size as usize]))
    }
}

pub fn process_alive(pid: u32) -> bool {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process == 0 {
            return false;
        }
        let mut code = 0;
        let ok = GetExitCodeProcess(process, &mut code);
        CloseHandle(process);
        ok != 0 && code == STILL_ACTIVE
    }
}
