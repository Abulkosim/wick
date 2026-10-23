use crate::{
    core::settings::IconClick,
    core::{
        candle::{template_rgba, Glyph},
        session::{short_duration, Session},
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

const ICON_SIZE: u32 = 36;

#[derive(Default)]
struct Shown {
    revision: u64,
    glyph: Option<Glyph>,
    tooltip: String,
    title: Option<String>,
}

#[derive(Default)]
pub struct Presentation(Mutex<Shown>);

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
                .set_icon_with_as_template(Some(icon(glyph)), true)
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
    });
}

pub fn icon(glyph: Glyph) -> Image<'static> {
    Image::new_owned(template_rgba(ICON_SIZE, glyph), ICON_SIZE, ICON_SIZE)
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Open Wick", true, None::<&str>)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Wick", true, Some("CmdOrCtrl+Q"))?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show_item, &settings_item, &separator, &quit_item])?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon(Glyph::Off))
        .icon_as_template(true)
        .tooltip("Wick · Off")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => popover::show(app, false),
            "settings" => popover::show(app, true),
            "quit" => app.exit(0),
            _ => {}
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
