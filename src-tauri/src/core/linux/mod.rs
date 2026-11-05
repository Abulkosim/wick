//! Linux: systemd-logind, the XDG desktop portal, `/proc` and `/sys`.

#[cfg(target_os = "linux")]
pub mod bus;
pub mod parse;
#[cfg(target_os = "linux")]
pub mod system;
