use serde::Serialize;
use std::{sync::Mutex, time::Instant};
use tauri::{AppHandle, Emitter, Manager};
#[cfg(not(target_os = "macos"))]
use tauri_plugin_positioner::{Position, WindowExt};

pub const TRAY_ID: &str = "wick";
const POPOVER_WIDTH: f64 = 360.0;
const POPOVER_HEIGHT: f64 = 720.0;
const POPOVER_MIN_HEIGHT: f64 = 160.0;

#[derive(Default)]
pub struct Visibility {
    pub last_blur: Mutex<Option<Instant>>,
    theme: Mutex<Option<Option<tauri::Theme>>>,
    pub placement: Mutex<PopoverPlacement>,
    content_height: Mutex<Option<f64>>,
    /// Where the tray icon was last clicked, on panels that report it (Linux).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    click: Mutex<Option<(f64, f64)>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PopoverPlacement {
    pub revision: u64,
    pub arrow_x: Option<f64>,
    /// No tray to anchor to: Wick is an ordinary window with a title bar.
    pub framed: bool,
    /// Whether a global shortcut can be registered (not on Wayland).
    pub shortcuts: bool,
}

impl Default for PopoverPlacement {
    fn default() -> Self {
        Self {
            revision: 0,
            arrow_x: None,
            framed: false,
            shortcuts: !wayland(),
        }
    }
}

/// Wayland lets neither apps place their own windows nor grab keys globally.
pub fn wayland() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE").is_ok_and(|kind| kind == "wayland"))
}

/// Notes where the tray icon was clicked, to open the popover beside it.
pub fn remember_click(app: &AppHandle, x: f64, y: f64) {
    if let Ok(mut click) = app.state::<Visibility>().click.lock() {
        *click = (x > 0.0 || y > 0.0).then_some((x, y));
    }
}

/// Without a tray (GNOME with no AppIndicator extension), Wick lives in a normal window:
/// a title bar to move and close it, a taskbar entry, and no hiding when it loses focus.
pub fn use_window(app: &AppHandle) {
    if let Ok(mut placement) = app.state::<Visibility>().placement.lock() {
        placement.framed = true;
        placement.revision = placement.revision.saturating_add(1);
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_decorations(true);
        let _ = window.set_skip_taskbar(false);
        let _ = window.set_always_on_top(false);
        let _ = window.set_shadow(true);
    }
}

pub fn framed(app: &AppHandle) -> bool {
    app.state::<Visibility>()
        .placement
        .lock()
        .is_ok_and(|placement| placement.framed)
}

fn popover_height(requested: Option<f64>) -> f64 {
    requested
        .filter(|height| height.is_finite())
        .map_or(POPOVER_HEIGHT, |height| {
            height.ceil().clamp(POPOVER_MIN_HEIGHT, POPOVER_HEIGHT)
        })
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Bounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

struct Layout {
    frame: Bounds,
    arrow_x: Option<f64>,
}

fn layout(work: Bounds, anchor: Option<Bounds>, scale: f64, macos: bool, height: f64) -> Layout {
    let anchor = anchor.filter(|rect| rect.width > 0.0 && rect.height > 0.0);
    let width = (POPOVER_WIDTH * scale).round().min(work.width);
    let height = (height * scale).round().min(work.height);
    let (x, y) = match anchor {
        Some(tray) => {
            let above = tray.y - height;
            let y = if macos || above < work.y {
                tray.y + tray.height
            } else {
                above
            };
            (tray.x + tray.width / 2.0 - width / 2.0, y)
        }
        None => (
            work.x + (work.width - width) / 2.0,
            work.y + (work.height - height) / 2.0,
        ),
    };
    let x = x.clamp(work.x, work.x + work.width - width).round();
    let y = y.clamp(work.y, work.y + work.height - height).round();
    let arrow_x = anchor.filter(|_| macos).and_then(|tray| {
        let offset = (tray.x + tray.width / 2.0 - x) / scale;
        (offset >= 26.0 && offset <= width / scale - 26.0).then_some(offset)
    });
    Layout {
        frame: Bounds {
            x,
            y,
            width,
            height,
        },
        arrow_x,
    }
}

/// Linux panels don't say where their icons are: open at the end of the panel, top right
/// when the panel is at the top (GNOME) or the screen has none, bottom right otherwise.
#[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
fn panel_corner(work: Bounds, screen: Bounds, scale: f64) -> Bounds {
    let x = work.x + work.width - 24.0 * scale;
    let bottom_panel = work.y + work.height < screen.y + screen.height && work.y <= screen.y;
    let y = if bottom_panel {
        work.y + work.height
    } else {
        work.y - 1.0
    };
    Bounds {
        x,
        y,
        width: 1.0,
        height: 1.0,
    }
}

#[cfg(any(target_os = "macos", test))]
fn cocoa_layout(work: Bounds, anchor: Option<Bounds>, height: f64) -> Layout {
    let flip = |rect: Bounds| Bounds {
        y: -rect.y - rect.height,
        ..rect
    };
    let placed = layout(flip(work), anchor.map(flip), 1.0, true, height);
    Layout {
        frame: flip(placed.frame),
        arrow_x: placed.arrow_x,
    }
}

#[cfg(target_os = "macos")]
fn place_macos_window(app: &AppHandle, window: &tauri::WebviewWindow, height: f64) -> Option<f64> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSScreen, NSWindow};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    let mtm = MainThreadMarker::new()?;
    let bounds = |rect: NSRect| Bounds {
        x: rect.origin.x,
        y: rect.origin.y,
        width: rect.size.width,
        height: rect.size.height,
    };
    let pointer = window.ns_window().ok()?;
    // Keep AppKit geometry in points on the main thread, including mixed-DPI moves.
    let native = unsafe { &*pointer.cast::<NSWindow>() };
    let tray_geometry = app.tray_by_id(TRAY_ID).and_then(|tray| {
        tray.with_inner_tray_icon(move |tray| {
            let mtm = MainThreadMarker::new()?;
            let tray_window = tray.ns_status_item()?.button(mtm)?.window()?;
            let screen = tray_window.screen()?;
            Some((bounds(screen.visibleFrame()), bounds(tray_window.frame())))
        })
        .ok()
        .flatten()
    });
    let (work, anchor) = match tray_geometry {
        Some((work, anchor)) => (work, Some(anchor)),
        None => {
            let screen = native.screen().or_else(|| NSScreen::mainScreen(mtm))?;
            (bounds(screen.visibleFrame()), None)
        }
    };
    let placed = cocoa_layout(work, anchor, height);
    let content_rect = NSRect::new(
        NSPoint::new(placed.frame.x, placed.frame.y),
        NSSize::new(placed.frame.width, placed.frame.height),
    );
    native.setFrame_display(native.frameRectForContentRect(content_rect), true);
    placed.arrow_x
}

#[cfg(target_os = "macos")]
fn fit_backdrop(window: &tauri::WebviewWindow) {
    use objc2::ClassType;
    use objc2_app_kit::{NSAutoresizingMaskOptions, NSVisualEffectView, NSWindow};
    use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize};

    let window = window.clone();
    let handle = window.clone();
    let _ = window.run_on_main_thread(move || {
        let Ok(pointer) = handle.ns_window() else {
            return;
        };
        // Tauri owns this NSWindow; access stays on its main thread while the handle lives.
        let native = unsafe { &*pointer.cast::<NSWindow>() };
        let Some(content) = native.contentView() else {
            return;
        };
        let bounds = content.bounds();
        let frame = NSRect::new(
            NSPoint::new(6.0, if content.isFlipped() { 14.0 } else { 6.0 }),
            NSSize::new(
                (bounds.size.width - 12.0).max(0.0),
                (bounds.size.height - 20.0).max(0.0),
            ),
        );
        for view in content.subviews() {
            if view.isKindOfClass(NSVisualEffectView::class()) {
                view.setAutoresizingMask(
                    NSAutoresizingMaskOptions::ViewWidthSizable
                        | NSAutoresizingMaskOptions::ViewHeightSizable,
                );
                view.setFrame(frame);
            }
        }
        native.invalidateShadow();
    });
}

/// Makes the native window, and with it the vibrancy behind the popover, follow Wick's theme
/// instead of the system appearance. `None` follows the system.
pub fn set_theme(app: &AppHandle, theme: Option<tauri::Theme>) {
    let visibility = app.state::<Visibility>();
    let Ok(mut current) = visibility.theme.lock() else {
        return;
    };
    if *current == Some(theme) {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        if window.set_theme(theme).is_ok() {
            *current = Some(theme);
        }
    }
}

pub fn fit(app: &AppHandle, height: f64) -> Result<(), String> {
    let height = popover_height(Some(height));
    {
        let visibility = app.state::<Visibility>();
        let mut current = visibility
            .content_height
            .lock()
            .map_err(|_| "Popover size is unavailable.")?;
        if *current == Some(height) {
            return Ok(());
        }
        *current = Some(height);
    }
    let handle = app.clone();
    app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window("main") {
            if window.is_visible().unwrap_or(false) {
                place(&handle, &window);
            }
        }
    })
    .map_err(|_| "Popover size is unavailable.".into())
}

fn place(app: &AppHandle, window: &tauri::WebviewWindow) {
    let height = popover_height(
        app.state::<Visibility>()
            .content_height
            .lock()
            .ok()
            .and_then(|value| *value),
    );
    #[cfg(target_os = "macos")]
    let arrow_x = place_macos_window(app, window, height);
    #[cfg(not(target_os = "macos"))]
    let arrow_x = {
        let position = Position::TrayCenter;
        if window.move_window_constrained(position).is_err() {
            let _ = window.center();
        }
        let clicked = app
            .state::<Visibility>()
            .click
            .lock()
            .ok()
            .and_then(|click| *click)
            .filter(|_| cfg!(target_os = "linux") && !framed(app))
            .map(|(x, y)| {
                (
                    tauri::PhysicalPosition::new(x, y),
                    tauri::PhysicalSize::new(1.0, 1.0),
                )
            });
        let anchor = app
            .tray_by_id(TRAY_ID)
            .and_then(|tray| tray.rect().ok().flatten())
            .map(|rect| {
                (
                    rect.position.to_physical::<f64>(1.0),
                    rect.size.to_physical::<f64>(1.0),
                )
            })
            .filter(|(_, size)| size.width > 0.0 && size.height > 0.0)
            .or(clicked);
        let monitor = anchor
            .and_then(|(position, _)| {
                window
                    .monitor_from_point(position.x, position.y)
                    .ok()
                    .flatten()
            })
            .or_else(|| window.current_monitor().ok().flatten());
        let mut arrow_x = None;
        if let Some(monitor) = monitor {
            let work = monitor.work_area();
            let work = Bounds {
                x: f64::from(work.position.x),
                y: f64::from(work.position.y),
                width: f64::from(work.size.width),
                height: f64::from(work.size.height),
            };
            let screen = Bounds {
                x: f64::from(monitor.position().x),
                y: f64::from(monitor.position().y),
                width: f64::from(monitor.size().width),
                height: f64::from(monitor.size().height),
            };
            let anchor = anchor
                .map(|(position, size)| Bounds {
                    x: position.x,
                    y: position.y,
                    width: size.width,
                    height: size.height,
                })
                .or_else(|| {
                    (cfg!(target_os = "linux") && !framed(app))
                        .then(|| panel_corner(work, screen, monitor.scale_factor()))
                });
            let placed = layout(work, anchor, monitor.scale_factor(), false, height);
            let resized = window.set_size(tauri::PhysicalSize::new(
                placed.frame.width as u32,
                placed.frame.height as u32,
            ));
            let positioned = window.set_position(tauri::PhysicalPosition::new(
                placed.frame.x as i32,
                placed.frame.y as i32,
            ));
            if resized.is_ok() && positioned.is_ok() {
                arrow_x = placed.arrow_x;
            }
        }
        arrow_x
    };
    if let Ok(mut placement) = app.state::<Visibility>().placement.lock() {
        placement.revision = placement.revision.saturating_add(1);
        placement.arrow_x = arrow_x;
        let _ = window.emit("popover_placement_changed", placement.clone());
    }
    #[cfg(target_os = "macos")]
    fit_backdrop(window);
}

pub fn show(app: &AppHandle, settings: bool) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window("main") {
            place(&handle, &window);
            let _ = window.show();
            let _ = window.set_focus();
            if settings {
                let _ = handle.emit("open_settings", ());
            }
            let _ = handle.emit("popover_shown", ());
            if let Some(runtime) = handle.try_state::<std::sync::Arc<crate::runtime::Runtime>>() {
                runtime.poke();
            }
        }
    });
}

pub fn toggle(app: &AppHandle) {
    let blurred = app
        .state::<Visibility>()
        .last_blur
        .lock()
        .ok()
        .and_then(|value| *value)
        .is_some_and(|time| time.elapsed().as_millis() < 250);
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) || blurred {
            let _ = window.hide();
        } else {
            show(app, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_opens_at_the_panel_end() {
        let screen = Bounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        };
        let top = Bounds {
            y: 32.0,
            height: 1048.0,
            ..screen
        };
        let placed = layout(top, Some(panel_corner(top, screen, 1.0)), 1.0, false, 400.0);
        assert_eq!((placed.frame.x, placed.frame.y), (1560.0, 32.0));
        let bottom = Bounds {
            height: 1032.0,
            ..screen
        };
        let placed = layout(
            bottom,
            Some(panel_corner(bottom, screen, 2.0)),
            2.0,
            false,
            400.0,
        );
        assert_eq!(placed.frame.x + placed.frame.width, 1920.0);
        assert_eq!(placed.frame.y + placed.frame.height, 1032.0);
        assert_eq!(placed.arrow_x, None);
    }

    fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
        Bounds {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn cocoa_popover_hangs_from_the_tray_on_every_screen() {
        for (work, tray) in [
            (
                bounds(0.0, 0.0, 1440.0, 875.0),
                bounds(1368.0, 875.0, 24.0, 25.0),
            ),
            (
                bounds(1440.0, -180.0, 1920.0, 1055.0),
                bounds(3288.0, 875.0, 24.0, 25.0),
            ),
            (
                bounds(-1920.0, -180.0, 1920.0, 1055.0),
                bounds(-1872.0, 875.0, 24.0, 25.0),
            ),
            (
                bounds(0.0, 900.0, 1920.0, 1055.0),
                bounds(1848.0, 1955.0, 24.0, 25.0),
            ),
        ] {
            let placed = cocoa_layout(work, Some(tray), POPOVER_HEIGHT);
            assert_eq!(placed.frame.width, POPOVER_WIDTH);
            assert_eq!(placed.frame.height, POPOVER_HEIGHT);
            assert!(placed.frame.x >= work.x);
            assert!(placed.frame.x + placed.frame.width <= work.x + work.width);
            assert_eq!(placed.frame.y + placed.frame.height, tray.y);
            assert_eq!(
                placed.frame.x + placed.arrow_x.unwrap(),
                tray.x + tray.width / 2.0
            );
        }
    }

    #[test]
    fn small_work_area_and_missing_tray_stay_on_screen() {
        let work = bounds(-300.0, 900.0, 300.0, 400.0);
        let placed = cocoa_layout(work, None, POPOVER_HEIGHT);
        assert_eq!(placed.frame, work);
        assert_eq!(placed.arrow_x, None);
    }

    #[test]
    fn retina_arrow_tracks_tray_after_right_edge_clamping() {
        let work = bounds(0.0, 50.0, 2880.0, 1750.0);
        let tray = bounds(2736.0, 0.0, 48.0, 50.0);
        let placed = layout(work, Some(tray), 2.0, true, POPOVER_HEIGHT);
        assert_eq!(placed.frame.x, 2880.0 - POPOVER_WIDTH * 2.0);
        assert_eq!(placed.frame.y, 50.0);
        assert_eq!(placed.frame.x + placed.arrow_x.unwrap() * 2.0, 2760.0);
    }

    #[test]
    fn hidden_or_edge_anchor_has_no_misleading_arrow() {
        let work = bounds(0.0, 25.0, 1280.0, 775.0);
        let centered = layout(work, None, 1.0, true, POPOVER_HEIGHT);
        assert_eq!(
            centered.frame.x,
            ((1280.0 - POPOVER_WIDTH) / 2.0_f64).round()
        );
        assert_eq!(centered.arrow_x, None);
        let hidden = bounds(0.0, 800.0, 34.0, 0.0);
        let fallback = layout(work, Some(hidden), 1.0, true, POPOVER_HEIGHT);
        assert_eq!(fallback.frame, centered.frame);
        let edge = bounds(1260.0, 0.0, 20.0, 25.0);
        assert_eq!(
            layout(work, Some(edge), 1.0, true, POPOVER_HEIGHT).arrow_x,
            None
        );
    }

    #[test]
    fn height_follows_content_within_bounds() {
        assert_eq!(popover_height(None), POPOVER_HEIGHT);
        assert_eq!(popover_height(Some(f64::NAN)), POPOVER_HEIGHT);
        assert_eq!(popover_height(Some(311.2)), 312.0);
        assert_eq!(popover_height(Some(40.0)), POPOVER_MIN_HEIGHT);
        assert_eq!(popover_height(Some(2000.0)), POPOVER_HEIGHT);
    }

    #[test]
    fn short_popover_stays_attached_to_the_tray() {
        let work = bounds(0.0, 0.0, 1440.0, 875.0);
        let tray = bounds(1368.0, 875.0, 24.0, 25.0);
        let placed = cocoa_layout(work, Some(tray), 300.0);
        assert_eq!(placed.frame.height, 300.0);
        assert_eq!(placed.frame.y + placed.frame.height, tray.y);
    }

    #[test]
    fn windows_taskbar_placement_respects_each_scale() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let work = bounds(-1920.0 * scale, 0.0, 1920.0 * scale, 1040.0 * scale);
            let tray = bounds(-80.0 * scale, 1040.0 * scale, 24.0 * scale, 40.0 * scale);
            let placed = layout(work, Some(tray), scale, false, POPOVER_HEIGHT);
            assert_eq!(placed.frame.width, (POPOVER_WIDTH * scale).round());
            assert_eq!(placed.frame.x + placed.frame.width, 0.0);
            assert_eq!(placed.frame.y + placed.frame.height, tray.y);
            assert_eq!(placed.arrow_x, None);
        }
    }
}
