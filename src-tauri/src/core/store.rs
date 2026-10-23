use super::{
    settings::Settings,
    triggers::{self, Trigger},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const VERSION: u32 = 1;
const MAX_STORE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoredState {
    pub version: u32,
    pub settings: Settings,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    #[serde(skip)]
    pub recovered: Vec<String>,
}

impl StoredState {
    pub fn new(settings: Settings) -> Self {
        Self {
            version: VERSION,
            settings,
            triggers: Vec::new(),
            recovered: Vec::new(),
        }
    }
}

fn invalid_store() -> String {
    "Wick's settings file is damaged or unsupported.".into()
}

/// A missing file is a first run. A damaged or newer file is an error, so it is never overwritten.
pub fn load(path: &Path) -> Result<StoredState, String> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StoredState::new(Settings::default()));
        }
        Err(_) => return Err("Wick could not read its settings file.".into()),
    };
    let metadata = file
        .metadata()
        .map_err(|_| "Wick could not read its settings file.")?;
    if !metadata.is_file() || metadata.len() > MAX_STORE_BYTES {
        return Err(invalid_store());
    }
    let mut bytes = Vec::new();
    file.take(MAX_STORE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Wick could not read its settings file.")?;
    let Value::Object(mut map) = serde_json::from_slice(&bytes).map_err(|_| invalid_store())?
    else {
        return Err(invalid_store());
    };
    match map.get("version").and_then(Value::as_u64) {
        Some(version) if version == u64::from(VERSION) => {}
        Some(version) if version > u64::from(VERSION) => {
            return Err("This settings file needs a newer version of Wick.".into())
        }
        _ => return Err(invalid_store()),
    }
    let mut recovered = Vec::new();
    let settings = Settings::recover(map.remove("settings"), &mut recovered);
    let triggers = triggers::recover(map.remove("triggers"), &mut recovered);
    Ok(StoredState {
        version: VERSION,
        settings,
        triggers,
        recovered,
    })
}

pub fn save(path: &Path, state: &StoredState) -> Result<(), String> {
    state.settings.validate()?;
    triggers::validate_all(&state.triggers)?;
    let parent = path.parent().ok_or("Invalid settings location.")?;
    fs::create_dir_all(parent).map_err(|_| "Wick could not create its settings folder.")?;
    let mut file =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| "Wick could not save settings.")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| "Wick could not protect its settings file.")?;
    }
    serde_json::to_writer_pretty(&mut file, state).map_err(|_| "Could not encode settings.")?;
    file.write_all(b"\n")
        .map_err(|_| "Could not write settings.")?;
    file.as_file()
        .sync_all()
        .map_err(|_| "Could not flush settings.")?;
    file.persist(path)
        .map_err(|_| "Could not replace settings atomically.")?;
    Ok(())
}

pub fn backup(path: &Path) -> Result<PathBuf, String> {
    let target = backup_path(path)?;
    fs::copy(path, &target).map_err(|_| "Wick could not back up its settings file.")?;
    Ok(target)
}

/// Moves the current file aside (kept as a backup) and writes `state` in its place.
pub fn reset(path: &Path, state: &StoredState) -> Result<Option<PathBuf>, String> {
    let target = match fs::symlink_metadata(path) {
        Ok(_) => {
            let target = backup_path(path)?;
            fs::rename(path, &target).map_err(|_| "Wick could not back up its settings file.")?;
            Some(target)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err("Wick could not read its settings file.".into()),
    };
    save(path, state)?;
    Ok(target)
}

fn backup_path(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Invalid settings location.")?;
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    (0..100)
        .map(|attempt| match attempt {
            0 => path.with_file_name(format!("{name}.bak-{stamp}")),
            _ => path.with_file_name(format!("{name}.bak-{stamp}-{attempt}")),
        })
        .find(|candidate| fs::symlink_metadata(candidate).is_err())
        .ok_or_else(|| "Wick could not choose a backup file name.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::settings::Theme;

    fn temp() -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        (directory, path)
    }

    fn backups(path: &Path) -> Vec<PathBuf> {
        fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|entry| entry.to_string_lossy().contains(".bak-"))
            .collect()
    }

    #[test]
    fn missing_file_is_a_first_run() {
        let (_directory, path) = temp();
        assert_eq!(load(&path).unwrap(), StoredState::new(Settings::default()));
    }

    #[test]
    fn saves_and_loads_round_trip() {
        let (_directory, path) = temp();
        let state = StoredState::new(Settings {
            theme: Theme::Dark,
            keep_display_on: true,
            start_minutes: Some(90),
            ..Settings::default()
        });
        save(&path, &state).unwrap();
        assert_eq!(load(&path).unwrap(), state);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn damaged_and_newer_files_are_errors_and_stay_untouched() {
        let (_directory, path) = temp();
        for (content, message) in [
            ("{ not json", "damaged"),
            ("[]", "damaged"),
            (r#"{"settings":{}}"#, "damaged"),
            (r#"{"version":2,"settings":{}}"#, "newer version"),
        ] {
            fs::write(&path, content).unwrap();
            assert!(load(&path).unwrap_err().contains(message), "{content}");
            assert_eq!(fs::read_to_string(&path).unwrap(), content);
        }
    }

    #[test]
    fn bad_fields_recover_with_notes() {
        let (_directory, path) = temp();
        fs::write(
            &path,
            r#"{"version":1,"settings":{"theme":"dark","iconClick":42}}"#,
        )
        .unwrap();
        let state = load(&path).unwrap();
        assert_eq!(state.settings.theme, Theme::Dark);
        assert_eq!(
            state.recovered,
            ["Clicking the icon was reset to its default."]
        );
    }

    #[test]
    fn triggers_round_trip_and_bad_ones_drop() {
        let (_directory, path) = temp();
        let mut state = StoredState::new(Settings::default());
        state.triggers = vec![Trigger {
            id: "t1".into(),
            kind: triggers::TriggerKind::Process,
            name: "claude".into(),
            path: None,
            enabled: true,
        }];
        save(&path, &state).unwrap();
        assert_eq!(load(&path).unwrap(), state);
        fs::write(
            &path,
            r#"{"version":1,"settings":{},"triggers":[{"id":"x","kind":"app","name":"Nope","enabled":true}]}"#,
        )
        .unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.triggers.is_empty());
        assert_eq!(
            loaded.recovered,
            ["1 trigger could not be read and was removed."]
        );
    }

    #[test]
    fn oversized_file_is_rejected() {
        let (_directory, path) = temp();
        fs::write(&path, vec![b' '; MAX_STORE_BYTES as usize + 1]).unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn reset_keeps_the_old_file_as_a_backup() {
        let (_directory, path) = temp();
        fs::write(&path, "{ damaged").unwrap();
        let backup = reset(&path, &StoredState::new(Settings::default()))
            .unwrap()
            .unwrap();
        assert_eq!(fs::read_to_string(backup).unwrap(), "{ damaged");
        assert_eq!(load(&path).unwrap(), StoredState::new(Settings::default()));
        assert_eq!(backups(&path).len(), 1);
    }

    #[test]
    fn backup_copies_without_touching_the_original() {
        let (_directory, path) = temp();
        save(&path, &StoredState::new(Settings::default())).unwrap();
        let before = fs::read(&path).unwrap();
        let copy = backup(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read(copy).unwrap(), before);
    }
}
