use crate::{
    core::settings::IconClick,
    core::{
        candle::Glyph,
        session::{short_duration, Request, Session},
    },
    popover::{self, TRAY_ID},
    runtime::{self, AppState, Runtime},
};
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

/// 18 pt at 2x on macOS; 32 px suits Windows at 100–200% scaling and Linux panels, which scale
/// the pixmap to their own size.
const ICON_SIZE: u32 = if cfg!(target_os = "macos") { 36 } else { 32 };

#[derive(Default)]
struct Shown {
    revision: u64,
    glyph: Option<Glyph>,
    /// The panel colour the icon was drawn for (Linux).
    dark: Option<bool>,
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
    if state.paused_by_lock && (state.session.is_on() || state.held_by.is_some()) {
        return "Wick · Paused while the screen is locked".into();
    }
    match state.session {
        Session::Off => match &state.held_by {
            Some(phrase) => format!("Wick · Awake while {phrase}{screen}"),
            None => "Wick · Off".into(),
        },
        Session::Indefinite { .. } => format!("Wick · Awake{screen}"),
        session => format!(
            "Wick · Awake · {} left{screen}",
            short_duration(session.remaining_ms(now).unwrap_or(0))
        ),
    }
}

/// Quantized so the tray redraws about once per 1% of a session, not every tick.
fn glyph(state: &AppState, now: i64) -> Glyph {
    if state.paused_by_lock {
        return Glyph::Off;
    }
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
        let dark = panel_dark();
        if (shown.glyph != Some(glyph) || shown.dark != Some(dark))
            && tray
                .set_icon_with_as_template(Some(icon(glyph)), cfg!(target_os = "macos"))
                .is_ok()
        {
            shown.glyph = Some(glyph);
            shown.dark = Some(dark);
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

#[cfg(target_os = "linux")]
fn panel_dark() -> bool {
    panel::dark()
}

/// Only Linux redraws for the panel colour.
#[cfg(not(target_os = "linux"))]
fn panel_dark() -> bool {
    false
}

#[cfg(target_os = "macos")]
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
    Image::new_owned(
        crate::core::candle::tinted_rgba(ICON_SIZE, glyph, ink(!light)),
        ICON_SIZE,
        ICON_SIZE,
    )
}

/// Panels don't tint pixmaps either. GNOME's top bar is dark in every theme; elsewhere the
/// panel usually follows the desktop's colour scheme. Many desktops report none, so the candle
/// also gets a halo in the opposite colour and reads on either.
#[cfg(target_os = "linux")]
pub fn icon(glyph: Glyph) -> Image<'static> {
    let dark = panel::dark();
    Image::new_owned(
        crate::core::candle::outlined_rgba(ICON_SIZE, glyph, ink(dark), ink(!dark)),
        ICON_SIZE,
        ICON_SIZE,
    )
}

/// The panel colour, read over D-Bus off the main thread and cached: icons are drawn on the main
/// thread, where a stalled portal would freeze the menu and the popover.
#[cfg(target_os = "linux")]
pub mod panel {
    use std::sync::{
        atomic::{AtomicU8, Ordering},
        Mutex,
    };
    use std::time::{Duration, Instant};

    /// 0 until read, then 1 for dark, 2 for light.
    static DARK: AtomicU8 = AtomicU8::new(0);
    static READ_AT: Mutex<Option<Instant>> = Mutex::new(None);
    const REFRESH: Duration = Duration::from_secs(60);

    /// The last reading; dark until the first one, like most default panels.
    pub fn dark() -> bool {
        DARK.load(Ordering::Relaxed) != 2
    }

    /// Re-reads the colour scheme at most once a minute. Returns whether it changed.
    pub fn refresh() -> bool {
        {
            let Ok(mut read_at) = READ_AT.lock() else {
                return false;
            };
            if read_at.is_some_and(|at| at.elapsed() < REFRESH) {
                return false;
            }
            *read_at = Some(Instant::now());
        }
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase();
        let dark = if ["gnome", "ubuntu", "unity", "pantheon"]
            .iter()
            .any(|name| desktop.contains(name))
        {
            true
        } else {
            let scheme = crate::core::linux::bus::color_scheme();
            if desktop.contains("kde") {
                scheme == Some(1)
            } else {
                scheme != Some(2)
            }
        };
        let value = if dark { 1 } else { 2 };
        DARK.swap(value, Ordering::Relaxed) != value
    }
}

/// Ink for a tray that doesn't tint its icons.
#[cfg(not(target_os = "macos"))]
fn ink(dark_panel: bool) -> [u8; 3] {
    if dark_panel {
        [0xf2, 0xf5, 0xf0]
    } else {
        [0x1f, 0x2a, 0x24]
    }
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
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => popover::show(app, false),
            "settings" => popover::show(app, true),
            "quit" => app.exit(0),
            "stop" => runtime::act(app, |app, runtime| runtime.stop(app)),
            "indefinite" => {
                runtime::act(app, |app, runtime| runtime.start(app, Request::Indefinite))
            }
            id => {
                if let Some(minutes) = id
                    .strip_prefix("for-")
                    .and_then(|minutes| minutes.parse().ok())
                {
                    runtime::act(app, move |app, runtime| {
                        runtime.start(app, Request::For { minutes })
                    });
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            let app = tray.app_handle();
            tauri_plugin_positioner::on_tray_event(app, &event);
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                position,
                ..
            } = event
            {
                popover::remember_click(app, position.x, position.y);
                let runtime = app.state::<Arc<Runtime>>();
                if runtime.state().settings.icon_click == IconClick::Toggle {
                    runtime::act(app, |app, runtime| runtime.toggle(app));
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
    fn tooltip_and_glyph_follow_held_and_locked_states() {
        let mut state = AppState {
            held_by: Some("Xcode is running".into()),
            ..AppState::default()
        };
        assert_eq!(tooltip(&state, 0), "Wick · Awake while Xcode is running");
        assert_eq!(glyph(&state, 0), Glyph::Held);
        state.paused_by_lock = true;
        assert_eq!(
            tooltip(&state, 0),
            "Wick · Paused while the screen is locked"
        );
        assert_eq!(glyph(&state, 0), Glyph::Off);
        assert_eq!(tooltip(&AppState::default(), 0), "Wick · Off");
    }

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
