use crate::{
    core::{
        assertion::{self, Assertion, Kind},
        session::{Request, Session},
    },
    tray,
};
use serde::Serialize;
use std::{
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
    pub display: bool,
    pub error: Option<String>,
}

#[derive(Default)]
struct Inner {
    view: AppState,
    assertion: Option<Assertion>,
    held_for: Option<(Kind, Session)>,
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
        let kind = if self.view.display {
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
}

impl Runtime {
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

    pub fn set_display(&self, app: &AppHandle, display: bool) -> Result<AppState, String> {
        self.update(app, |inner, _| {
            inner.view.display = display;
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
        inner.view.display = true;
        inner.reconcile(now).unwrap();
        assert_eq!(ours(), ["PreventUserIdleDisplaySleep"]);
        inner.view.session = Session::Off;
        inner.reconcile(now).unwrap();
        assert!(ours().is_empty());
    }
}
