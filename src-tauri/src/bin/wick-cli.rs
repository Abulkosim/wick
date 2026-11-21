//! The console `wick` for Windows, where `wick.exe` is a GUI-subsystem program. Built only with
//! the `console-cli` feature, which tauri.windows.conf.json turns on.

fn main() {
    std::process::exit(wick_lib::console());
}
