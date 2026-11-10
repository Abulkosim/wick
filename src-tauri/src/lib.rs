mod cli;
mod commands;
pub mod core;
mod popover;
mod runtime;
mod shortcut;
mod status;
mod tray;
mod updates;

use popover::Visibility;
use runtime::Runtime;
use std::{sync::Arc, time::Instant};
use tauri::{Manager, WindowEvent};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_notification::NotificationExt;

/// Runs a command-line or link intent. Returns false when the arguments hold none.
fn perform(app: &tauri::AppHandle, args: &[String]) -> bool {
    match cli::parse(args) {
        Ok(None) => return false,
        Ok(Some(cli::Intent::Start(request))) => {
            runtime::act(app, move |app, runtime| runtime.start(app, request))
        }
        Ok(Some(cli::Intent::Stop)) => runtime::act(app, |app, runtime| runtime.stop(app)),
        Ok(Some(cli::Intent::Toggle)) => runtime::act(app, |app, runtime| runtime.toggle(app)),
        Ok(Some(cli::Intent::Status)) => {}
        Err(error) => eprintln!("wick: {error}"),
    }
    true
}

/// On Windows and Linux a second `wick wick://…` launch is a link: single-instance hands it to
/// the deep-link plugin, whose `on_open_url` already runs it, before calling us with the same
/// arguments.
fn is_forwarded_link(args: &[String], forwards_links: bool) -> bool {
    forwards_links
        && matches!(args, [_, url] if url.parse::<tauri::Url>().is_ok_and(|url| url.scheme() == "wick"))
}

/// Tells the page which platform it runs on before any script loads. WebKitGTK's user agent
/// can claim to be Safari on macOS, so the page can't work it out reliably itself.
fn platform_script() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let platform = if cfg!(target_os = "macos") {
        "mac"
    } else if cfg!(windows) {
        "windows"
    } else {
        "linux"
    };
    tauri::plugin::Builder::new("wick-platform")
        .js_init_script(format!("window.__WICK_PLATFORM__ = {platform:?};"))
        .build()
}

pub fn run() {
    // WebKitGTK's DMA-BUF renderer draws blank windows on some GPU drivers (NVIDIA, older
    // Mesa); the popover is small enough that the shared-memory path costs nothing.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    let args: Vec<String> = std::env::args().collect();
    match cli::parse(&args) {
        Ok(Some(cli::Intent::Status)) => {
            let value = status::default_directory()
                .map(|directory| status::read(&directory))
                .unwrap_or_else(|| serde_json::json!({ "running": false, "awake": false }));
            println!("{value}");
            return;
        }
        Err(error) => {
            eprintln!("wick: {error}");
            std::process::exit(2);
        }
        _ => {}
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if is_forwarded_link(&args, cfg!(any(windows, target_os = "linux"))) {
                return;
            }
            if !perform(app, &args) {
                popover::show(app, false)
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(platform_script())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(shortcut::plugin())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updates::Updates::default())
        .manage(Visibility::default())
        .manage(tray::Presentation::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::start_session,
            commands::stop_session,
            commands::extend_session,
            commands::set_display,
            commands::save_settings,
            commands::reset_store,
            commands::save_triggers,
            commands::resume_triggers,
            commands::clear_history,
            commands::running_apps,
            updates::get_update_status,
            updates::check_for_update,
            updates::install_update,
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
            let mut tray_missing = false;
            if let Err(error) = tray::setup(app.handle()) {
                // Linux desktops without a StatusNotifier host (GNOME without the AppIndicator
                // extension) have no tray; Wick still works as a window.
                if !cfg!(target_os = "linux") {
                    return Err(error.into());
                }
                eprintln!("wick: no tray available ({error}); opening as a window");
                popover::use_window(app.handle());
                tray_missing = true;
            }
            #[cfg(target_os = "linux")]
            if app.env().appimage.is_some() {
                // Packages register wick:// in their .desktop file; an AppImage does it itself.
                if let Err(error) = app.deep_link().register_all() {
                    eprintln!("wick: could not register wick:// links: {error}");
                }
            }
            let linker = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    perform(&linker, &["wick".into(), url.to_string()]);
                }
            });
            let opener = app.handle().clone();
            app.notification()
                .on_action(move |_| popover::show(&opener, false))?;
            runtime.sync_launch_at_login(app.handle());
            runtime.resume(app.handle());
            runtime::publish(app.handle(), &runtime.state());
            let saved = runtime.state().settings.shortcut;
            if let Err(error) = shortcut::replace(app.handle(), None, saved.as_deref()) {
                eprintln!("wick: {error}");
            }
            updates::start_background_checks(app.handle().clone(), runtime.clone());
            runtime::start_ticker(app.handle().clone(), runtime);
            let args: Vec<String> = std::env::args().collect();
            let background = args.iter().any(|arg| arg == "--background") && !tray_missing;
            if !perform(app.handle(), &args) && !background {
                popover::show(app.handle(), false);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::Focused(false) => {
                if popover::framed(window.app_handle()) {
                    return;
                }
                if let Ok(mut last_blur) =
                    window.app_handle().state::<Visibility>().last_blur.lock()
                {
                    *last_blur = Some(Instant::now());
                }
                let _ = window.hide();
            }
            WindowEvent::CloseRequested { api, .. } => {
                // Without a tray the window is Wick's only presence: closing it quits, like any
                // windowed app, instead of leaving a session running out of sight.
                if popover::framed(window.app_handle()) {
                    window.app_handle().exit(0);
                    return;
                }
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
                let runtime = app.state::<Arc<Runtime>>();
                runtime.flush_history();
                if !runtime.keeps_session_on_exit() {
                    let _ = runtime.stop(app);
                }
            }
            _ => {}
        });
}

#[cfg(test)]
mod tests {
    use super::is_forwarded_link;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn a_link_from_a_second_launch_runs_once() {
        assert!(is_forwarded_link(&args(&["wick", "wick://toggle"]), true));
        assert!(is_forwarded_link(&args(&["wick", "WICK://stop"]), true));
        assert!(!is_forwarded_link(&args(&["wick", "wick://toggle"]), false));
        assert!(!is_forwarded_link(&args(&["wick", "toggle"]), true));
        assert!(!is_forwarded_link(
            &args(&["wick", "start", "wick://toggle"]),
            true
        ));
        assert!(!is_forwarded_link(&args(&["wick"]), true));
        assert!(!is_forwarded_link(
            &args(&["wick", "https://example.com"]),
            true
        ));
    }
}
