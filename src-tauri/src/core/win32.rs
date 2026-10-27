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

/// `FILETIME` as one 100 ns count.
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct FileTime {
    pub low: u32,
    pub high: u32,
}

impl FileTime {
    fn ticks(self) -> u64 {
        (u64::from(self.high) << 32) | u64::from(self.low)
    }
}

/// `MIB_IF_ROW2` (1352 bytes), with only the fields Wick reads named.
#[repr(C)]
pub struct MibIfRow2 {
    _identity: [u8; 8],
    pub interface_index: u32,
    _names: [u8; 1116],
    pub if_type: u32,
    _media: [u8; 20],
    /// Bit 0: hardware interface. Bit 1: filter interface.
    pub flags: u8,
    _status: [u8; 55],
    pub in_octets: u64,
    _in_counts: [u8; 64],
    pub out_octets: u64,
    _out_counts: [u8; 64],
}

#[repr(C)]
pub struct MibIfTable2 {
    pub entries: u32,
    pub table: [MibIfRow2; 1],
}

const IF_TYPE_SOFTWARE_LOOPBACK: u32 = 24;

#[link(name = "kernel32")]
extern "system" {
    pub fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
}

#[link(name = "iphlpapi")]
extern "system" {
    pub fn GetIfTable2(table: *mut *mut MibIfTable2) -> u32;
    pub fn FreeMibTable(memory: *const std::ffi::c_void);
}

/// Busy and total CPU time across all cores. Kernel time includes idle time.
pub fn cpu_times() -> Option<(u64, u64)> {
    let (mut idle, mut kernel, mut user) = Default::default();
    if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
        return None;
    }
    let total = FileTime::ticks(kernel) + FileTime::ticks(user);
    Some((total.saturating_sub(FileTime::ticks(idle)), total))
}

/// Bytes in and bytes out for each physical adapter, keyed like `activity::Counters`; filter
/// and loopback rows would count the same traffic again.
pub fn network_bytes() -> Vec<(u32, u64)> {
    let mut table: *mut MibIfTable2 = std::ptr::null_mut();
    if unsafe { GetIfTable2(&mut table) } != 0 || table.is_null() {
        return Vec::new();
    }
    let rows = unsafe {
        std::slice::from_raw_parts(
            std::ptr::addr_of!((*table).table).cast::<MibIfRow2>(),
            (*table).entries as usize,
        )
    };
    let interfaces = rows
        .iter()
        .filter(|row| {
            row.flags & 0b01 != 0
                && row.flags & 0b10 == 0
                && row.if_type != IF_TYPE_SOFTWARE_LOOPBACK
        })
        .flat_map(|row| {
            let key = row.interface_index.wrapping_mul(2);
            [(key, row.in_octets), (key.wrapping_add(1), row.out_octets)]
        })
        .collect();
    unsafe { FreeMibTable(table.cast()) };
    interfaces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_rows_match_the_windows_layout() {
        assert_eq!(std::mem::size_of::<MibIfRow2>(), 1352);
        assert_eq!(std::mem::offset_of!(MibIfRow2, interface_index), 8);
        assert_eq!(std::mem::offset_of!(MibIfRow2, if_type), 1128);
        assert_eq!(std::mem::offset_of!(MibIfRow2, flags), 1152);
        assert_eq!(std::mem::offset_of!(MibIfRow2, in_octets), 1208);
        assert_eq!(std::mem::offset_of!(MibIfRow2, out_octets), 1280);
        assert_eq!(std::mem::offset_of!(MibIfTable2, table), 8);
    }

    #[test]
    fn reads_live_counters() {
        let (busy, total) = cpu_times().unwrap();
        assert!(total >= busy);
        assert!(!network_bytes().is_empty() || std::env::var_os("CI").is_some());
    }
}
