mod cli;
mod commands;
pub mod core;
mod popover;
mod runtime;
mod shortcut;
mod tray;

use popover::Visibility;
use runtime::Runtime;
use std::{sync::Arc, time::Instant};
use tauri::{Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

/// Runs a command-line intent. Returns false when there was none.
fn perform(app: &tauri::AppHandle, args: &[String]) -> bool {
    let runtime = app.state::<Arc<Runtime>>();
    let result = match cli::parse(args) {
        Ok(None) => return false,
        Ok(Some(cli::Intent::Start(request))) => runtime.start(app, request).map(|_| ()),
        Ok(Some(cli::Intent::Stop)) => runtime.stop(app).map(|_| ()),
        Err(error) => Err(error),
    };
    if let Err(error) = result {
        eprintln!("wick: {error}");
    }
    true
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if !perform(app, &args) {
                popover::show(app, false)
            }
        }))
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(shortcut::plugin())
        .plugin(tauri_plugin_notification::init())
        .manage(Visibility::default())
        .manage(tray::Presentation::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::start_session,
            commands::stop_session,
            commands::set_display,
            commands::save_settings,
            commands::reset_store,
            commands::get_popover_placement,
            commands::fit_popover,
            commands::hide_window,
            commands::quit,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let path = app.path().app_data_dir()?.join("settings.json");
            let runtime = Arc::new(Runtime::new(path));
            app.manage(runtime.clone());
            tray::setup(app.handle())?;
            let opener = app.handle().clone();
            app.notification()
                .on_action(move |_| popover::show(&opener, false))?;
            runtime.sync_launch_at_login(app.handle());
            let saved = runtime.state().settings.shortcut;
            if let Err(error) = shortcut::replace(app.handle(), None, saved.as_deref()) {
                eprintln!("wick: {error}");
            }
            runtime::start_ticker(app.handle().clone(), runtime);
            let args: Vec<String> = std::env::args().collect();
            if !perform(app.handle(), &args) && !args.iter().any(|arg| arg == "--background") {
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
        .run(|app, event| match event {
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => popover::show(app, false),
            tauri::RunEvent::Exit => {
                let _ = app.state::<Arc<Runtime>>().stop(app);
            }
            _ => {}
        });
}
