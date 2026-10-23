use super::session::MAX_MINUTES;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IconClick {
    #[default]
    Open,
    Toggle,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: Theme,
    pub keep_display_on: bool,
    /// What Start, the shortcut and an icon click start. `None` keeps awake until stopped.
    pub start_minutes: Option<u32>,
    pub icon_click: IconClick,
    pub launch_at_login: bool,
    /// An accelerator such as `Alt+Super+KeyW` that starts or stops.
    pub shortcut: Option<String>,
    pub battery_guard: bool,
    pub battery_threshold: u8,
    pub stop_in_low_power_mode: bool,
    pub notify_on_end: bool,
    pub auto_check_updates: bool,
    pub show_time_left: bool,
    pub pause_when_locked: bool,
}

pub const BATTERY_THRESHOLDS: std::ops::RangeInclusive<u8> = 5..=80;

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            keep_display_on: false,
            start_minutes: None,
            icon_click: IconClick::Open,
            launch_at_login: false,
            shortcut: None,
            battery_guard: true,
            battery_threshold: 20,
            stop_in_low_power_mode: false,
            notify_on_end: true,
            auto_check_updates: true,
            show_time_left: false,
            pause_when_locked: false,
        }
    }
}

/// A shortcut needs Command, Control or Option, so it can't swallow normal typing.
pub fn parse_shortcut(value: &str) -> Result<tauri_plugin_global_shortcut::Shortcut, String> {
    use tauri_plugin_global_shortcut::{Modifiers, Shortcut};
    let shortcut: Shortcut = value
        .parse()
        .map_err(|_| "That key combination can't be used as a shortcut.".to_string())?;
    if !shortcut
        .mods
        .intersects(Modifiers::SUPER | Modifiers::CONTROL | Modifiers::ALT)
    {
        return Err("Include Command, Control or Option in the shortcut.".into());
    }
    Ok(shortcut)
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if self
            .start_minutes
            .is_some_and(|minutes| !(1..=MAX_MINUTES).contains(&minutes))
        {
            return Err("Choose a start duration between 1 minute and 7 days.".into());
        }
        if let Some(shortcut) = &self.shortcut {
            parse_shortcut(shortcut)?;
        }
        if !BATTERY_THRESHOLDS.contains(&self.battery_threshold) {
            return Err("Choose a battery level between 5% and 80%.".into());
        }
        Ok(())
    }

    pub fn guard(&self) -> super::power::Guard {
        super::power::Guard {
            enabled: self.battery_guard,
            threshold: self.battery_threshold,
            stop_in_low_power_mode: self.stop_in_low_power_mode,
        }
    }

    /// Keeps every readable field and notes each one that had to fall back.
    pub fn recover(value: Option<Value>, notes: &mut Vec<String>) -> Self {
        let defaults = Self::default();
        let map = match value {
            None | Some(Value::Null) => return defaults,
            Some(Value::Object(map)) => map,
            Some(_) => {
                notes.push("Settings were reset to defaults.".into());
                return defaults;
            }
        };
        let settings = Self {
            theme: field(&map, "theme", defaults.theme, "Theme", notes),
            keep_display_on: field(
                &map,
                "keepDisplayOn",
                defaults.keep_display_on,
                "Keep display on",
                notes,
            ),
            start_minutes: field(
                &map,
                "startMinutes",
                defaults.start_minutes,
                "Start duration",
                notes,
            ),
            icon_click: field(
                &map,
                "iconClick",
                defaults.icon_click,
                "Clicking the icon",
                notes,
            ),
            launch_at_login: field(
                &map,
                "launchAtLogin",
                defaults.launch_at_login,
                "Launch at login",
                notes,
            ),
            shortcut: field(&map, "shortcut", defaults.shortcut, "Shortcut", notes),
            battery_guard: field(
                &map,
                "batteryGuard",
                defaults.battery_guard,
                "Stop on low battery",
                notes,
            ),
            battery_threshold: field(
                &map,
                "batteryThreshold",
                defaults.battery_threshold,
                "Battery level",
                notes,
            ),
            stop_in_low_power_mode: field(
                &map,
                "stopInLowPowerMode",
                defaults.stop_in_low_power_mode,
                "Stop in Low Power Mode",
                notes,
            ),
            notify_on_end: field(
                &map,
                "notifyOnEnd",
                defaults.notify_on_end,
                "Session notifications",
                notes,
            ),
            auto_check_updates: field(
                &map,
                "autoCheckUpdates",
                defaults.auto_check_updates,
                "Check for updates",
                notes,
            ),
            show_time_left: field(
                &map,
                "showTimeLeft",
                defaults.show_time_left,
                "Time left in the menu bar",
                notes,
            ),
            pause_when_locked: field(
                &map,
                "pauseWhenLocked",
                defaults.pause_when_locked,
                "Pause while locked",
                notes,
            ),
        };
        let mut settings = settings;
        if settings
            .start_minutes
            .is_some_and(|minutes| !(1..=MAX_MINUTES).contains(&minutes))
        {
            reset_note("Start duration", notes);
            settings.start_minutes = defaults.start_minutes;
        }
        if settings
            .shortcut
            .as_deref()
            .is_some_and(|shortcut| parse_shortcut(shortcut).is_err())
        {
            reset_note("Shortcut", notes);
            settings.shortcut = None;
        }
        if !BATTERY_THRESHOLDS.contains(&settings.battery_threshold) {
            reset_note("Battery level", notes);
            settings.battery_threshold = defaults.battery_threshold;
        }
        settings
    }
}

fn field<T: DeserializeOwned>(
    map: &Map<String, Value>,
    key: &str,
    fallback: T,
    label: &str,
    notes: &mut Vec<String>,
) -> T {
    match map.get(key) {
        None => fallback,
        Some(value) => serde_json::from_value(value.clone()).unwrap_or_else(|_| {
            reset_note(label, notes);
            fallback
        }),
    }
}

fn reset_note(label: &str, notes: &mut Vec<String>) {
    let note = format!("{label} was reset to its default.");
    if !notes.contains(&note) {
        notes.push(note);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_fields_take_defaults_quietly() {
        let mut notes = Vec::new();
        let settings = Settings::recover(Some(json!({ "theme": "dark" })), &mut notes);
        assert_eq!(settings.theme, Theme::Dark);
        assert_eq!(settings.start_minutes, None);
        assert!(notes.is_empty());
    }

    #[test]
    fn one_bad_field_keeps_the_others() {
        let mut notes = Vec::new();
        let settings = Settings::recover(
            Some(
                json!({ "theme": "neon", "keepDisplayOn": true, "startMinutes": 60, "iconClick": "toggle" }),
            ),
            &mut notes,
        );
        assert_eq!(settings.theme, Theme::System);
        assert!(settings.keep_display_on);
        assert_eq!(settings.start_minutes, Some(60));
        assert_eq!(settings.icon_click, IconClick::Toggle);
        assert_eq!(notes, ["Theme was reset to its default."]);
    }

    #[test]
    fn out_of_range_duration_falls_back() {
        let mut notes = Vec::new();
        let settings = Settings::recover(Some(json!({ "startMinutes": 0 })), &mut notes);
        assert_eq!(settings.start_minutes, None);
        assert_eq!(notes, ["Start duration was reset to its default."]);
        assert!(Settings {
            start_minutes: Some(MAX_MINUTES + 1),
            ..Settings::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn shortcuts_need_a_real_modifier() {
        assert!(parse_shortcut("Alt+Super+KeyW").is_ok());
        assert!(parse_shortcut("Control+Shift+Digit1").is_ok());
        assert!(parse_shortcut("Shift+KeyA").is_err());
        assert!(parse_shortcut("KeyA").is_err());
        assert!(parse_shortcut("Super+Nope").is_err());
        let mut notes = Vec::new();
        let settings = Settings::recover(Some(json!({ "shortcut": "Shift+KeyA" })), &mut notes);
        assert_eq!(settings.shortcut, None);
        assert_eq!(notes, ["Shortcut was reset to its default."]);
    }

    #[test]
    fn battery_level_out_of_range_falls_back() {
        let mut notes = Vec::new();
        let settings = Settings::recover(Some(json!({ "batteryThreshold": 99 })), &mut notes);
        assert_eq!(settings.battery_threshold, 20);
        assert_eq!(notes, ["Battery level was reset to its default."]);
    }

    #[test]
    fn non_object_resets_everything_with_a_note() {
        let mut notes = Vec::new();
        assert_eq!(
            Settings::recover(Some(json!([1, 2])), &mut notes),
            Settings::default()
        );
        assert_eq!(notes, ["Settings were reset to defaults."]);
    }
}
