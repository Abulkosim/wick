use crate::{
    core::session::Request,
    popover::{self, PopoverPlacement, Visibility},
    runtime::{AppState, Runtime},
};
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

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
