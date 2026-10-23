use crate::{
    core::ring::{template_rgba, Glyph},
    popover::{self, TRAY_ID},
};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

const ICON_SIZE: u32 = 36;

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
                popover::toggle(app);
            }
        })
        .build(app)?;
    Ok(())
}
