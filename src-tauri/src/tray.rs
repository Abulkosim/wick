use crate::{
    core::settings::IconClick,
    core::{
        candle::Glyph,
        session::{short_duration, Request, Session},
    },
    popover::{self, TRAY_ID},
    runtime::{AppState, Runtime},
};
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

/// 18 pt at 2x on macOS; 32 px suits Windows at 100–200% scaling.
const ICON_SIZE: u32 = if cfg!(windows) { 32 } else { 36 };

#[derive(Default)]
struct Shown {
    revision: u64,
    glyph: Option<Glyph>,
    tooltip: String,
    title: Option<String>,
    stoppable: Option<bool>,
}

#[derive(Default)]
pub struct Presentation(Mutex<Shown>);

/// The menu's Stop item, enabled only while there is something to stop.
pub struct StopItem(MenuItem<tauri::Wry>);

pub fn tooltip(state: &AppState, now: i64) -> String {
    let screen = if state.settings.keep_display_on {
        " · display on"
    } else {
        ""
    };
    match state.session {
        Session::Off => "Wick · Off".into(),
        Session::Indefinite { .. } => format!("Wick · Awake{screen}"),
        session => format!(
            "Wick · Awake · {} left{screen}",
            short_duration(session.remaining_ms(now).unwrap_or(0))
        ),
    }
}

/// Quantized so the tray redraws about once per 1% of a session, not every tick.
fn glyph(state: &AppState, now: i64) -> Glyph {
    match state.session.glyph(now, state.held_by.is_some()) {
        Glyph::Remaining(value) => Glyph::Remaining((value * 100.0).ceil() / 100.0),
        glyph => glyph,
    }
}

/// Time left beside the icon, when asked for and a timed session is running.
pub fn title(state: &AppState, now: i64) -> Option<String> {
    state
        .settings
        .show_time_left
        .then(|| state.session.remaining_ms(now))
        .flatten()
        .map(short_duration)
}

#[cfg(target_os = "macos")]
fn set_tabular_title(tray: &tauri::tray::TrayIcon, title: Option<&str>) -> bool {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSFont, NSFontWeightRegular};

    if tray.set_title(title).is_err() {
        return false;
    }
    let _ = tray.with_inner_tray_icon(|tray| {
        let mtm = MainThreadMarker::new()?;
        let button = tray.ns_status_item()?.button(mtm)?;
        let size = NSFont::menuBarFontOfSize(0.0).pointSize();
        let font =
            NSFont::monospacedDigitSystemFontOfSize_weight(size, unsafe { NSFontWeightRegular });
        button.setFont(Some(&font));
        Some(())
    });
    true
}

#[cfg(not(target_os = "macos"))]
fn set_tabular_title(tray: &tauri::tray::TrayIcon, title: Option<&str>) -> bool {
    tray.set_title(title).is_ok()
}

pub fn update(app: &AppHandle, state: &AppState, now: i64) {
    let glyph = glyph(state, now);
    let tooltip = tooltip(state, now);
    let title = title(state, now);
    let stoppable = state.session.is_on() || state.held_by.is_some();
    let revision = state.revision;
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(tray) = handle.tray_by_id(TRAY_ID) else {
            return;
        };
        let presentation = handle.state::<Presentation>();
        let Ok(mut shown) = presentation.0.lock() else {
            return;
        };
        if revision < shown.revision {
            return;
        }
        shown.revision = revision;
        if shown.glyph != Some(glyph)
            && tray
                .set_icon_with_as_template(Some(icon(glyph)), cfg!(target_os = "macos"))
                .is_ok()
        {
            shown.glyph = Some(glyph);
        }
        if shown.tooltip != tooltip && tray.set_tooltip(Some(&tooltip)).is_ok() {
            shown.tooltip = tooltip;
        }
        if shown.title != title && set_tabular_title(&tray, title.as_deref()) {
            shown.title = title;
        }
        if shown.stoppable != Some(stoppable) {
            if let Some(stop) = handle.try_state::<StopItem>() {
                if stop.0.set_enabled(stoppable).is_ok() {
                    shown.stoppable = Some(stoppable);
                }
            }
        }
    });
}

#[cfg(not(windows))]
pub fn icon(glyph: Glyph) -> Image<'static> {
    Image::new_owned(
        crate::core::candle::template_rgba(ICON_SIZE, glyph),
        ICON_SIZE,
        ICON_SIZE,
    )
}

/// Windows doesn't tint tray icons, so draw dark ink on a light taskbar and light on dark.
#[cfg(windows)]
pub fn icon(glyph: Glyph) -> Image<'static> {
    let light = windows_registry::CURRENT_USER
        .open(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_u32("SystemUsesLightTheme"))
        .is_ok_and(|value| value == 1);
    let ink = if light {
        [0x1f, 0x2a, 0x24]
    } else {
        [0xf2, 0xf5, 0xf0]
    };
    Image::new_owned(
        crate::core::candle::tinted_rgba(ICON_SIZE, glyph, ink),
        ICON_SIZE,
        ICON_SIZE,
    )
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Open Wick", true, None::<&str>)?;
    let quick = |id: &str, label: &str| MenuItem::with_id(app, id, label, true, None::<&str>);
    let fifteen = quick("for-15", "Keep awake for 15 minutes")?;
    let hour = quick("for-60", "Keep awake for 1 hour")?;
    let two_hours = quick("for-120", "Keep awake for 2 hours")?;
    let forever = quick("indefinite", "Keep awake until stopped")?;
    let stop_item = MenuItem::with_id(app, "stop", "Stop", false, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Wick", true, Some("CmdOrCtrl+Q"))?;
    let menu = Menu::with_items(
        app,
        &[
            &show_item,
            &PredefinedMenuItem::separator(app)?,
            &fifteen,
            &hour,
            &two_hours,
            &forever,
            &stop_item,
            &PredefinedMenuItem::separator(app)?,
            &settings_item,
            &quit_item,
        ],
    )?;
    app.manage(StopItem(stop_item));
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon(Glyph::Off))
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Wick · Off")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let runtime = app.state::<Arc<Runtime>>();
            let result = match event.id.as_ref() {
                "show" => {
                    popover::show(app, false);
                    Ok(())
                }
                "settings" => {
                    popover::show(app, true);
                    Ok(())
                }
                "quit" => {
                    app.exit(0);
                    Ok(())
                }
                "stop" => runtime.stop(app).map(|_| ()),
                "indefinite" => runtime.start(app, Request::Indefinite).map(|_| ()),
                id => match id
                    .strip_prefix("for-")
                    .and_then(|minutes| minutes.parse().ok())
                {
                    Some(minutes) => runtime.start(app, Request::For { minutes }).map(|_| ()),
                    None => Ok(()),
                },
            };
            if let Err(error) = result {
                use tauri_plugin_notification::NotificationExt;
                let _ = app.notification().builder().title(error).show();
            }
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            tauri_plugin_positioner::on_tray_event(app, &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let runtime = app.state::<Arc<Runtime>>();
                if runtime.state().settings.icon_click == IconClick::Toggle {
                    let _ = runtime.toggle(app);
                } else {
                    popover::toggle(app);
                }
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_shows_time_left_only_when_asked_and_timed() {
        let mut state = AppState {
            session: Session::For {
                started_at: 0,
                ends_at: 42 * 60_000,
            },
            ..AppState::default()
        };
        assert_eq!(title(&state, 0), None);
        state.settings.show_time_left = true;
        assert_eq!(title(&state, 0).as_deref(), Some("42m"));
        assert_eq!(title(&state, 41 * 60_000 + 1).as_deref(), Some("1m"));
        state.session = Session::Indefinite { started_at: 0 };
        assert_eq!(title(&state, 0), None);
        state.session = Session::Off;
        assert_eq!(title(&state, 0), None);
    }
}
