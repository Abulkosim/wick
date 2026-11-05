/// What the copy calls this computer.
pub const DEVICE: &str = if cfg!(target_os = "macos") {
    "Mac"
} else if cfg!(windows) {
    "PC"
} else {
    "computer"
};

pub mod activity;
pub mod assertion;
pub mod boot;
pub mod candle;
pub mod display;
pub mod history;
pub mod lid;
pub mod linux;
pub mod lock;
pub mod others;
pub mod power;
pub mod processes;
pub mod schedule;
pub mod session;
pub mod settings;
pub mod store;
pub mod triggers;
#[cfg(windows)]
pub mod win32;
