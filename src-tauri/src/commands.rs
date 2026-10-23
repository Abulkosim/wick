use crate::{
    core::{
        processes::{self, RunningApp},
        session::Request,
        settings::Settings,
        triggers::Trigger,
    },
    popover::{self, PopoverPlacement, Visibility},
    runtime::{AppState, Runtime},
};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub fn get_state(runtime: State<'_, Arc<Runtime>>) -> AppState {
    runtime.state()
}

#[tauri::command]
pub fn start_session(
    app: AppHandle,
    runtime: State<'_, Arc<Runtime>>,
    request: Request,
) -> Result<AppState, String> {
    runtime.start(&app, request)
}

#[tauri::command]
pub fn stop_session(app: AppHandle, runtime: State<'_, Arc<Runtime>>) -> Result<AppState, String> {
    runtime.stop(&app)
}

#[tauri::command]
pub fn set_display(
    app: AppHandle,
    runtime: State<'_, Arc<Runtime>>,
    display: bool,
) -> Result<AppState, String> {
    runtime.set_display(&app, display)
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    runtime: State<'_, Arc<Runtime>>,
    settings: Settings,
) -> Result<AppState, String> {
    settings.validate()?;
    let previous = runtime.state().settings;
    set_launch_at_login(&app, previous.launch_at_login, settings.launch_at_login)?;
    if let Err(error) = crate::shortcut::replace(
        &app,
        previous.shortcut.as_deref(),
        settings.shortcut.as_deref(),
    ) {
        let _ = set_launch_at_login(&app, settings.launch_at_login, previous.launch_at_login);
        return Err(error);
    }
    runtime
        .save_settings(&app, settings.clone())
        .inspect_err(|_| {
            let _ = set_launch_at_login(&app, settings.launch_at_login, previous.launch_at_login);
            let _ = crate::shortcut::replace(
                &app,
                settings.shortcut.as_deref(),
                previous.shortcut.as_deref(),
            );
        })
}

fn set_launch_at_login(app: &AppHandle, from: bool, to: bool) -> Result<(), String> {
    if from == to {
        return Ok(());
    }
    let result = if to {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    };
    result.map_err(|_| {
        "Could not change launch at login. Move Wick to Applications and try again.".into()
    })
}

#[tauri::command]
pub fn reset_store(app: AppHandle, runtime: State<'_, Arc<Runtime>>) -> Result<AppState, String> {
    runtime.reset_store(&app)
}

#[tauri::command]
pub fn save_triggers(
    app: AppHandle,
    runtime: State<'_, Arc<Runtime>>,
    triggers: Vec<Trigger>,
) -> Result<AppState, String> {
    runtime.save_triggers(&app, triggers)
}

#[tauri::command]
pub fn resume_triggers(
    app: AppHandle,
    runtime: State<'_, Arc<Runtime>>,
) -> Result<AppState, String> {
    runtime.resume_triggers(&app)
}

#[tauri::command]
pub async fn running_apps() -> Vec<RunningApp> {
    tauri::async_runtime::spawn_blocking(|| processes::running_apps(&processes::scan()))
        .await
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_popover_placement(
    visibility: State<'_, Visibility>,
) -> Result<PopoverPlacement, String> {
    visibility
        .placement
        .lock()
        .map(|placement| placement.clone())
        .map_err(|_| "Popover position is unavailable.".into())
}

#[tauri::command]
pub fn fit_popover(app: AppHandle, height: f64) -> Result<(), String> {
    popover::fit(&app, height)
}

#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or("Window is unavailable.")?
        .hide()
        .map_err(|_| "Could not close the popover.".into())
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}
