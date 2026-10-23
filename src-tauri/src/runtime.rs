use crate::{
    core::{
        assertion::{self, Assertion, Kind},
        session::{Request, Session},
        settings::Settings,
        store::{self, StoredState},
    },
    tray,
};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;

const TICK: Duration = Duration::from_secs(5);

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub revision: u64,
    pub session: Session,
    pub settings: Settings,
    pub error: Option<String>,
    /// Set when the settings file can't be read. Saving stays off until the owner resets it.
    pub recovery: Option<String>,
    pub recovered: Vec<String>,
}

#[derive(Default)]
struct Inner {
    view: AppState,
    assertion: Option<Assertion>,
    held_for: Option<(Kind, Session)>,
    store_path: Option<PathBuf>,
}

#[derive(Default)]
pub struct Runtime {
    inner: Mutex<Inner>,
    wake: Notify,
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn assertion_name(session: &Session) -> String {
    match session.ends_at() {
        Some(ends_at) => {
            let time = chrono::DateTime::from_timestamp_millis(ends_at)
                .map(|time| {
                    time.with_timezone(&chrono::Local)
                        .format("%H:%M")
                        .to_string()
                })
                .unwrap_or_default();
            format!("Wick: awake until {time}")
        }
        None => "Wick: awake until stopped".into(),
    }
}

impl Inner {
    /// Hold exactly the assertion the session needs. A replacement is created before the
    /// old one drops, so the Mac is never briefly allowed to sleep.
    fn reconcile(&mut self, now: i64) -> Result<(), String> {
        let session = self.view.session;
        if !session.is_on() {
            self.assertion = None;
            self.held_for = None;
            return Ok(());
        }
        let kind = if self.view.settings.keep_display_on {
            Kind::Display
        } else {
            Kind::System
        };
        if self.assertion.is_some() && self.held_for == Some((kind, session)) {
            return Ok(());
        }
        let timeout = session
            .remaining_ms(now)
            .map(|ms| Duration::from_millis(ms as u64) + Duration::from_secs(2));
        let next = assertion::create(kind, &assertion_name(&session), timeout)?;
        self.assertion = Some(next);
        self.held_for = Some((kind, session));
        Ok(())
    }

    fn changed(&mut self) {
        self.view.revision = self.view.revision.saturating_add(1);
    }

    fn persist(&mut self) -> Result<(), String> {
        self.view.settings.validate()?;
        if let Some(recovery) = &self.view.recovery {
            return Err(format!("{recovery} Reset settings to save changes."));
        }
        if let Some(path) = &self.store_path {
            store::save(path, &StoredState::new(self.view.settings.clone()))?;
        }
        self.view.recovered.clear();
        Ok(())
    }
}

impl Runtime {
    pub fn new(store_path: PathBuf) -> Self {
        let (stored, recovery) = match store::load(&store_path) {
            Ok(stored) => (stored, None),
            Err(error) => (
                StoredState::new(Settings::default()),
                Some(format!("{error} Settings file: {}.", store_path.display())),
            ),
        };
        if !stored.recovered.is_empty() {
            let _ = store::backup(&store_path);
        }
        Self {
            inner: Mutex::new(Inner {
                view: AppState {
                    settings: stored.settings,
                    recovery,
                    recovered: stored.recovered,
                    ..AppState::default()
                },
                store_path: Some(store_path),
                ..Inner::default()
            }),
            wake: Notify::new(),
        }
    }

    pub fn state(&self) -> AppState {
        self.inner
            .lock()
            .map(|inner| inner.view.clone())
            .unwrap_or_default()
    }

    fn update<T>(
        &self,
        app: &AppHandle,
        change: impl FnOnce(&mut Inner, i64) -> Result<T, String>,
    ) -> Result<AppState, String> {
        let view = {
            let mut inner = self
                .inner
                .lock()
                .map_err(|_| "Wick's state is unavailable.")?;
            let now = now_ms();
            let previous = inner.view.clone();
            if let Err(error) = change(&mut inner, now) {
                inner.view = previous;
                return Err(error);
            }
            if inner.view.settings != previous.settings {
                if let Err(error) = inner.persist() {
                    inner.view = previous;
                    return Err(error);
                }
            }
            if let Err(error) = inner.reconcile(now) {
                inner.view = AppState {
                    session: Session::Off,
                    error: Some(error.clone()),
                    ..previous
                };
                inner.assertion = None;
                inner.held_for = None;
                inner.changed();
                let view = inner.view.clone();
                drop(inner);
                publish(app, &view);
                return Err(error);
            }
            inner.view.error = None;
            inner.changed();
            inner.view.clone()
        };
        publish(app, &view);
        self.wake.notify_one();
        Ok(view)
    }

    pub fn start(&self, app: &AppHandle, request: Request) -> Result<AppState, String> {
        self.update(app, |inner, _| {
            inner.view.session = Session::start(request, &chrono::Local::now())?;
            Ok(())
        })
    }

    pub fn stop(&self, app: &AppHandle) -> Result<AppState, String> {
        self.update(app, |inner, _| {
            inner.view.session = Session::Off;
            Ok(())
        })
    }

    pub fn start_default(&self, app: &AppHandle) -> Result<AppState, String> {
        let request = match self.state().settings.start_minutes {
            Some(minutes) => Request::For { minutes },
            None => Request::Indefinite,
        };
        self.start(app, request)
    }

    pub fn toggle(&self, app: &AppHandle) -> Result<AppState, String> {
        if self.state().session.is_on() {
            self.stop(app)
        } else {
            self.start_default(app)
        }
    }

    pub fn save_settings(&self, app: &AppHandle, settings: Settings) -> Result<AppState, String> {
        settings.validate()?;
        self.update(app, |inner, _| {
            inner.view.settings = settings;
            Ok(())
        })
    }

    /// The OS owns launch at login; the setting mirrors it on every launch.
    pub fn sync_launch_at_login(&self, app: &AppHandle) {
        use tauri_plugin_autostart::ManagerExt;
        if let Ok(enabled) = app.autolaunch().is_enabled() {
            if let Ok(mut inner) = self.inner.lock() {
                inner.view.settings.launch_at_login = enabled;
            }
        }
    }

    pub fn set_display(&self, app: &AppHandle, on: bool) -> Result<AppState, String> {
        let settings = Settings {
            keep_display_on: on,
            ..self.state().settings
        };
        self.save_settings(app, settings)
    }

    pub fn reset_store(&self, app: &AppHandle) -> Result<AppState, String> {
        self.update(app, |inner, _| {
            let settings = Settings::default();
            if let Some(path) = &inner.store_path {
                store::reset(path, &StoredState::new(settings.clone()))?;
            }
            inner.view.settings = settings;
            inner.view.recovery = None;
            inner.view.recovered.clear();
            Ok(())
        })
    }

    /// Ends a timed session that has run out and redraws the tray. Returns when to look again.
    fn tick(&self, app: &AppHandle) -> Option<Duration> {
        let now = now_ms();
        let session = self.state().session;
        if session.expired(now) {
            let _ = self.stop(app);
            return None;
        }
        tray::update(app, &self.state(), now);
        session
            .remaining_ms(now)
            .map(|ms| TICK.min(Duration::from_millis(ms as u64)))
    }
}

pub fn publish(app: &AppHandle, view: &AppState) {
    tray::update(app, view, now_ms());
    let _ = app.emit("state_changed", view);
}

pub fn start_ticker(app: AppHandle, runtime: Arc<Runtime>) {
    tauri::async_runtime::spawn(async move {
        loop {
            let wait = runtime.tick(&app).unwrap_or(Duration::from_secs(3600));
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = runtime.wake.notified() => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_say_how_long() {
        assert_eq!(
            assertion_name(&Session::Indefinite { started_at: 0 }),
            "Wick: awake until stopped"
        );
        assert!(assertion_name(&Session::For {
            started_at: 0,
            ends_at: 3_600_000
        })
        .starts_with("Wick: awake until "));
    }

    #[test]
    fn damaged_settings_are_never_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        std::fs::write(&path, "{ damaged").unwrap();
        let runtime = Runtime::new(path.clone());
        let mut inner = runtime.inner.lock().unwrap();
        assert!(inner.view.recovery.is_some());
        inner.view.settings.keep_display_on = true;
        assert!(inner.persist().unwrap_err().contains("Reset settings"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ damaged");
    }

    #[test]
    fn saved_settings_load_on_the_next_launch() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        {
            let runtime = Runtime::new(path.clone());
            let mut inner = runtime.inner.lock().unwrap();
            inner.view.settings.start_minutes = Some(45);
            inner.persist().unwrap();
        }
        assert_eq!(Runtime::new(path).state().settings.start_minutes, Some(45));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reconcile_holds_one_assertion_matching_the_session() {
        let ours = || {
            let pid = std::process::id() as i32;
            assertion::list()
                .unwrap()
                .into_iter()
                .filter(|held| held.pid == pid && held.name.starts_with("Wick: "))
                .map(|held| held.assert_type)
                .collect::<Vec<_>>()
        };
        let mut inner = Inner::default();
        let now = now_ms();
        inner.view.session = Session::Indefinite { started_at: now };
        inner.reconcile(now).unwrap();
        assert_eq!(ours(), ["PreventUserIdleSystemSleep"]);
        inner.view.settings.keep_display_on = true;
        inner.reconcile(now).unwrap();
        assert_eq!(ours(), ["PreventUserIdleDisplaySleep"]);
        inner.view.session = Session::Off;
        inner.reconcile(now).unwrap();
        assert!(ours().is_empty());
    }
}
