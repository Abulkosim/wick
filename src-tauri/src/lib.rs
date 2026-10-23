mod commands;
pub mod core;
mod popover;
mod tray;

use popover::Visibility;
use std::time::Instant;
use tauri::{Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            popover::show(app, false)
        }))
        .plugin(tauri_plugin_positioner::init())
        .manage(Visibility::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_popover_placement,
            commands::fit_popover,
            commands::hide_window,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            tray::setup(app.handle())?;
            if !std::env::args().any(|argument| argument == "--background") {
                popover::show(app.handle(), false);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::Focused(false) => {
                if let Ok(mut last_blur) =
                    window.app_handle().state::<Visibility>().last_blur.lock()
                {
                    *last_blur = Some(Instant::now());
                }
                let _ = window.hide();
            }
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                let _ = window.hide();
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("Wick could not start")
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                popover::show(_app, false);
            }
        });
}
