use crate::{
    core::{
        activity,
        assertion::{self, Assertion, Kind},
        display,
        history::{self, Day, History, Tracker},
        lid, lock,
        others::{self, Other},
        power::{self, Power},
        processes,
        session::{Request, Session},
        settings::{Settings, Theme},
        store::{self, StoredState},
        triggers::{self, Trigger, TriggerKind},
    },
    tray,
};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

const TICK: Duration = Duration::from_secs(5);
const IDLE_TICK: Duration = Duration::from_secs(60);
const SAVE_HISTORY_EVERY_MS: i64 = 60_000;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub revision: u64,
    pub session: Session,
    pub settings: Settings,
    pub error: Option<String>,
    /// Why Wick last stopped on its own, until the next start.
    pub notice: Option<String>,
    pub power: Power,
    /// The screen is locked and Wick has let go until it unlocks.
    pub paused_by_lock: bool,
    /// Whether closing the lid would sleep the Mac; `None` without a lid.
    pub lid_sleeps: Option<bool>,
    pub triggers: Vec<Trigger>,
    /// Ids of enabled triggers whose app or process is running now.
    pub running: Vec<String>,
    /// The running app or process keeping the Mac awake while no session is on.
    pub held_by: Option<String>,
    /// A running trigger the owner stopped; it counts again once it quits.
    pub paused_by: Option<String>,
    /// Other processes keeping the Mac awake right now.
    pub others: Vec<Other>,
    /// Set when the settings file can't be read. Saving stays off until the owner resets it.
    pub recovery: Option<String>,
    pub recovered: Vec<String>,
    /// Minutes Wick kept the Mac awake on each of the last seven days, today last.
    pub history: Vec<Day>,
}

#[derive(Default)]
struct Inner {
    view: AppState,
    assertion: Option<Assertion>,
    held_for: Option<(Kind, Session, Option<String>)>,
    store_path: Option<PathBuf>,
    /// Ids of triggers ignored until they stop matching.
    paused: Vec<String>,
    /// The trigger holding the Mac awake right now.
    held_id: Option<String>,
    last_update_check_at: Option<String>,
    history: History,
    history_path: Option<PathBuf>,
    tracker: Tracker,
    history_saved_at: i64,
    history_dirty: bool,
}

#[derive(Default)]
pub struct Runtime {
    inner: Mutex<Inner>,
    wake: Notify,
    pub update_wake: Notify,
    keep_on_exit: std::sync::atomic::AtomicBool,
    activity: Mutex<activity::Monitor>,
}

/// Local weekday (0 = Monday) and minutes past midnight, for schedules.
fn local_clock() -> (u8, u32) {
    use chrono::{Datelike, Timelike};
    let now = chrono::Local::now();
    (
        now.weekday().num_days_from_monday() as u8,
        now.hour() * 60 + now.minute(),
    )
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn assertion_name(session: &Session, held_by: Option<&str>) -> String {
    if let (false, Some(name)) = (session.is_on(), held_by) {
        return format!("Wick: awake while {name}");
    }
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
        let held_by = (!session.is_on())
            .then(|| self.view.held_by.clone())
            .flatten();
        if self.view.paused_by_lock || (!session.is_on() && held_by.is_none()) {
            self.assertion = None;
            self.held_for = None;
            return Ok(());
        }
        let kind = if self.view.settings.keep_display_on {
            Kind::Display
        } else {
            Kind::System
        };
        let key = (kind, session, held_by);
        if self.assertion.is_some() && self.held_for.as_ref() == Some(&key) {
            return Ok(());
        }
        let timeout = session
            .remaining_ms(now)
            .map(|ms| Duration::from_millis(ms as u64) + Duration::from_secs(2));
        let next = assertion::create(kind, &assertion_name(&session, key.2.as_deref()), timeout)?;
        self.assertion = Some(next);
        self.held_for = Some(key);
        Ok(())
    }

    /// Decides which trigger holds the Mac, from the current list and pause state, so a
    /// Resume or Stop that lands mid-tick isn't overwritten.
    fn evaluate_triggers(&mut self, context: &triggers::Context, power: &Power) {
        let active = triggers::active(&self.view.triggers, context);
        self.paused
            .retain(|id| active.iter().any(|trigger| &trigger.id == id));
        let allowed = power::stop_reason(power, &self.view.settings.guard()).is_none();
        let holder = active
            .iter()
            .find(|trigger| allowed && !self.paused.contains(&trigger.id));
        self.held_id = holder.map(|trigger| trigger.id.clone());
        self.view.held_by = holder.map(|trigger| trigger.phrase());
        self.view.paused_by = active
            .iter()
            .find(|trigger| self.paused.contains(&trigger.id))
            .map(|trigger| trigger.until());
        self.view.running = active.iter().map(|trigger| trigger.id.clone()).collect();
    }

    /// Counts the time an assertion is held, saving at most once a minute while it is.
    /// `closing` ends the count as if it were released.
    fn track(&mut self, now: i64, closing: bool) {
        let holding = !closing && self.assertion.is_some() && self.view.settings.keep_history;
        if let Some((from, to)) = self.tracker.advance(holding, now, history::monotonic_ms()) {
            self.history.add(from, to, &chrono::Local);
            self.history_dirty = true;
        }
        let today = history::local_day(now);
        if self.history_dirty && (!holding || now - self.history_saved_at >= SAVE_HISTORY_EVERY_MS)
        {
            self.history.prune(today);
            let saved = self
                .history_path
                .as_ref()
                .is_none_or(|path| history::save(path, &self.history).is_ok());
            if saved {
                self.history_dirty = false;
                self.history_saved_at = now;
            }
        }
        self.view.history = self.history.recent(today, history::SHOWN_DAYS);
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
            let mut stored = StoredState::new(self.view.settings.clone());
            stored.triggers = self.view.triggers.clone();
            stored.last_update_check_at = self.last_update_check_at.clone();
            store::save(path, &stored)?;
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
        let history_path = store_path.with_file_name(history::FILE);
        let history = history::load(&history_path);
        let today = history::local_day(now_ms());
        Self {
            inner: Mutex::new(Inner {
                view: AppState {
                    power: power::read(),
                    lid_sleeps: lid::closing_sleeps(),
                    settings: stored.settings,
                    triggers: stored.triggers,
                    recovery,
                    recovered: stored.recovered,
                    history: history.recent(today, history::SHOWN_DAYS),
                    ..AppState::default()
                },
                store_path: Some(store_path),
                history,
                history_path: Some(history_path),
                last_update_check_at: stored.last_update_check_at,
                ..Inner::default()
            }),
            wake: Notify::new(),
            update_wake: Notify::new(),
            keep_on_exit: Default::default(),
            activity: Default::default(),
        }
    }

    pub fn state(&self) -> AppState {
        self.inner
            .lock()
            .map(|inner| inner.view.clone())
            .unwrap_or_default()
    }

    /// Applies a change, then swaps the assertion, then saves. A failed swap keeps the old
    /// assertion and state; a failed save rolls the change back. Returns the view and whether it
    /// changed.
    fn commit<T>(
        &self,
        change: impl FnOnce(&mut Inner, i64) -> Result<T, String>,
    ) -> Result<(AppState, bool), String> {
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
            inner.view = previous;
            return Err(error);
        }
        if inner.view.settings != previous.settings || inner.view.triggers != previous.triggers {
            if let Err(error) = inner.persist() {
                inner.view = previous;
                let _ = inner.reconcile(now);
                return Err(error);
            }
        }
        inner.view.error = None;
        inner.track(now, false);
        let changed = inner.view != previous;
        if changed {
            inner.changed();
        }
        Ok((inner.view.clone(), changed))
    }

    /// For intents: publishes and wakes the ticker so triggers and timers react at once.
    fn update<T>(
        &self,
        app: &AppHandle,
        change: impl FnOnce(&mut Inner, i64) -> Result<T, String>,
    ) -> Result<AppState, String> {
        let (view, _) = self.commit(change)?;
        publish(app, &view);
        self.wake.notify_one();
        Ok(view)
    }

    /// For the ticker: publishes only when something changed, and doesn't wake itself.
    fn update_quiet<T>(
        &self,
        app: &AppHandle,
        change: impl FnOnce(&mut Inner, i64) -> Result<T, String>,
    ) -> Result<AppState, String> {
        let (view, changed) = self.commit(change)?;
        if changed {
            publish(app, &view);
        }
        Ok(view)
    }

    /// Picks up a session the previous run left behind in this boot (crash or update restart).
    pub fn resume(&self, app: &AppHandle) {
        let Ok(directory) = app.path().app_data_dir() else {
            return;
        };
        if let Some(session) = crate::status::resumable(&directory, now_ms()) {
            let _ = self.update(app, |inner, _| {
                inner.view.session = session;
                Ok(())
            });
        }
    }

    /// Lets the next launch resume the session, for a restart Wick starts itself.
    pub fn keep_session_on_exit(&self, app: &AppHandle) {
        self.keep_on_exit
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if let (Ok(directory), Ok(mut inner)) = (app.path().app_data_dir(), self.inner.lock()) {
            inner.changed();
            crate::status::write(&directory, &inner.view, true);
        }
    }

    pub fn keeps_session_on_exit(&self) -> bool {
        self.keep_on_exit.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Saves the time counted so far, for a quit or restart that keeps the session.
    pub fn flush_history(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.track(now_ms(), true);
        }
    }

    pub fn clear_history(&self, app: &AppHandle) -> Result<AppState, String> {
        self.update(app, |inner, now| {
            if let Some(path) = &inner.history_path {
                history::remove(path)?;
            }
            let holding = inner.assertion.is_some();
            inner.tracker.advance(holding, now, history::monotonic_ms());
            inner.history = History::default();
            inner.history_dirty = false;
            Ok(())
        })
    }

    pub fn extend(&self, app: &AppHandle, minutes: u32) -> Result<AppState, String> {
        self.update(app, |inner, now| {
            inner.view.session = inner.view.session.extend(minutes, now)?;
            Ok(())
        })
    }

    pub fn start(&self, app: &AppHandle, request: Request) -> Result<AppState, String> {
        let power = power::read();
        self.update(app, |inner, _| {
            inner.view.power = power;
            if let Some(reason) = power::stop_reason(&power, &inner.view.settings.guard()) {
                let reason = reason.trim_start_matches("Stopped: ");
                return Err(format!(
                    "Not started: {reason}. Plug in, or change it in Settings → Battery."
                ));
            }
            inner.view.session = Session::start(request, &chrono::Local::now())?;
            inner.view.notice = None;
            Ok(())
        })
    }

    /// Ends the session. With no session, it pauses the trigger holding the Mac awake.
    pub fn stop(&self, app: &AppHandle) -> Result<AppState, String> {
        self.update(app, |inner, _| {
            if !inner.view.session.is_on() && inner.view.held_by.take().is_some() {
                inner.paused = inner.view.running.clone();
                let held = inner.held_id.take();
                inner.view.paused_by = inner
                    .view
                    .triggers
                    .iter()
                    .find(|trigger| Some(&trigger.id) == held.as_ref())
                    .map(|trigger| trigger.until());
            }
            inner.view.session = Session::Off;
            inner.view.notice = None;
            Ok(())
        })
    }

    pub fn resume_triggers(&self, app: &AppHandle) -> Result<AppState, String> {
        self.update(app, |inner, _| {
            inner.paused.clear();
            inner.view.paused_by = None;
            inner.view.held_by = None;
            Ok(())
        })
    }

    pub fn save_triggers(&self, app: &AppHandle, list: Vec<Trigger>) -> Result<AppState, String> {
        triggers::validate_all(&list)?;
        self.update(app, |inner, _| {
            inner
                .paused
                .retain(|id| list.iter().any(|trigger| &trigger.id == id));
            inner.view.triggers = list;
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

    /// Stops whatever keeps the Mac awake (pausing a trigger), or starts a session.
    pub fn toggle(&self, app: &AppHandle) -> Result<AppState, String> {
        let state = self.state();
        if state.session.is_on() || state.held_by.is_some() {
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
            let settings = Settings {
                launch_at_login: inner.view.settings.launch_at_login,
                ..Settings::default()
            };
            if let Some(path) = &inner.store_path {
                store::reset(path, &StoredState::new(settings.clone()))?;
            }
            inner.view.settings = settings;
            inner.view.recovery = None;
            inner.view.recovered.clear();
            Ok(())
        })
    }

    /// Reads what the current state needs, then applies it in one update: triggers, screen lock,
    /// what the popover shows, and ending a session that ran out or tripped the battery guard.
    /// Returns when to look again.
    pub fn tick(&self, app: &AppHandle) -> Duration {
        let now = now_ms();
        let snapshot = self.state();
        let visible = app
            .get_webview_window("main")
            .and_then(|window| window.is_visible().ok())
            .unwrap_or(false);
        let power = power::read();
        let enabled = |kind| {
            snapshot
                .triggers
                .iter()
                .any(|trigger| trigger.enabled && trigger.kind == kind)
        };
        let processes = if triggers::needs_processes(&snapshot.triggers) {
            processes::scan(enabled(TriggerKind::App))
        } else {
            Vec::new()
        };
        let counters = activity::read(&snapshot.triggers, now);
        let busy = self
            .activity
            .lock()
            .map(|mut monitor| monitor.update(&snapshot.triggers, counters))
            .unwrap_or_default();
        let context = triggers::Context {
            busy: &busy,
            processes: &processes,
            on_power: !power.on_battery,
            external_display: enabled(TriggerKind::Display) && display::external_connected(),
            clock: local_clock(),
        };
        let holding =
            snapshot.session.is_on() || snapshot.held_by.is_some() || snapshot.paused_by_lock;
        let locked = snapshot.settings.pause_when_locked && holding && lock::screen_locked();
        let others = visible
            .then(|| assertion::list().ok().map(|held| others::current(&held)))
            .flatten();
        let lid_sleeps = visible.then(lid::closing_sleeps);
        let mut ended: Option<(String, bool)> = None;
        let result = self.update_quiet(app, |inner, now| {
            inner.view.power = power;
            inner.view.paused_by_lock = locked;
            if let Some(others) = others {
                inner.view.others = others;
            }
            if let Some(lid_sleeps) = lid_sleeps {
                inner.view.lid_sleeps = lid_sleeps;
            }
            inner.evaluate_triggers(&context, &power);
            if let Due::End { notice, message } = due(&inner.view, power, now) {
                inner.view.session = Session::Off;
                inner.view.notice = notice;
                let message = match &inner.view.held_by {
                    Some(phrase) if message.starts_with("Session ended") => {
                        format!("Session ended. Still awake while {phrase}.")
                    }
                    _ => message,
                };
                ended = Some((message, inner.view.settings.notify_on_end));
            }
            Ok(())
        });
        if let Some((message, true)) = ended.filter(|_| result.is_ok()) {
            notify(app, &message);
        }
        let state = result.unwrap_or(snapshot);
        tray::update(app, &state, now);
        let watching = state.triggers.iter().any(|trigger| trigger.enabled)
            || (state.settings.pause_when_locked && holding)
            || visible;
        match state.session.remaining_ms(now) {
            Some(ms) => TICK.min(Duration::from_millis(ms.max(0) as u64)),
            None if state.session.is_on() || watching => TICK,
            None => IDLE_TICK,
        }
    }

    pub fn last_update_check_at(&self) -> Option<String> {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| inner.last_update_check_at.clone())
    }

    /// Remembers the check across launches. A damaged settings file stays untouched.
    pub fn record_update_check(&self, at: String) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_update_check_at = Some(at);
            if inner.view.recovery.is_none() {
                let _ = inner.persist();
            }
        }
    }

    /// Wakes the ticker, e.g. so it polls faster while the popover is open.
    pub fn poke(&self) {
        self.wake.notify_one();
    }
}

#[derive(Debug, PartialEq)]
enum Due {
    Nothing,
    Power,
    End {
        notice: Option<String>,
        message: String,
    },
}

fn due(state: &AppState, power: Power, now: i64) -> Due {
    if state.session.expired(now) {
        return Due::End {
            notice: None,
            message: "Session ended. Your Mac can sleep again.".into(),
        };
    }
    if state.session.is_on() {
        if let Some(reason) = power::stop_reason(&power, &state.settings.guard()) {
            return Due::End {
                message: format!("{reason}. Your Mac can sleep again."),
                notice: Some(reason),
            };
        }
    }
    if state.power != power {
        Due::Power
    } else {
        Due::Nothing
    }
}

fn notify(app: &AppHandle, title: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).show();
}

pub fn publish(app: &AppHandle, view: &AppState) {
    tray::update(app, view, now_ms());
    if let Ok(directory) = app.path().app_data_dir() {
        let resume = app
            .try_state::<Arc<Runtime>>()
            .is_some_and(|runtime| runtime.keeps_session_on_exit());
        crate::status::write(&directory, view, resume);
    }
    crate::popover::set_theme(
        app,
        match view.settings.theme {
            Theme::System => None,
            Theme::Light => Some(tauri::Theme::Light),
            Theme::Dark => Some(tauri::Theme::Dark),
        },
    );
    let _ = app.emit("state_changed", view);
}

pub fn start_ticker(app: AppHandle, runtime: Arc<Runtime>) {
    tauri::async_runtime::spawn(async move {
        loop {
            let wait = runtime.tick(&app);
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
            assertion_name(&Session::Indefinite { started_at: 0 }, None),
            "Wick: awake until stopped"
        );
        assert_eq!(
            assertion_name(&Session::Off, Some("Xcode is running")),
            "Wick: awake while Xcode is running"
        );
        assert!(assertion_name(
            &Session::For {
                started_at: 0,
                ends_at: 3_600_000
            },
            Some("Xcode")
        )
        .starts_with("Wick: awake until "));
    }

    #[test]
    fn due_ends_on_time_or_low_battery() {
        let battery = Power {
            on_battery: true,
            percent: Some(10),
            low_power_mode: false,
        };
        let timed = AppState {
            session: Session::For {
                started_at: 0,
                ends_at: 1000,
            },
            ..AppState::default()
        };
        assert_eq!(due(&timed, Power::default(), 999), Due::Nothing);
        assert_eq!(
            due(&timed, Power::default(), 1000),
            Due::End {
                notice: None,
                message: "Session ended. Your Mac can sleep again.".into()
            }
        );
        assert_eq!(
            due(&timed, battery, 10),
            Due::End {
                notice: Some("Stopped: battery below 20%".into()),
                message: "Stopped: battery below 20%. Your Mac can sleep again.".into()
            }
        );
        let off = AppState::default();
        assert_eq!(due(&off, battery, 10), Due::Power);
    }

    fn trigger(id: &str, kind: TriggerKind) -> Trigger {
        Trigger {
            id: id.into(),
            kind,
            name: "x".into(),
            path: None,
            schedule: None,
            threshold: None,
            enabled: true,
        }
    }

    #[test]
    fn a_failed_save_rolls_back_and_an_unchanged_commit_stays_quiet() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        std::fs::write(&path, "{ damaged").unwrap();
        let runtime = Runtime::new(path);
        let before = runtime.state();
        let result = runtime.commit(|inner, _| {
            inner.view.settings.start_minutes = Some(30);
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(runtime.state(), before);
        let (view, changed) = runtime.commit(|_, _| Ok(())).unwrap();
        assert!(!changed);
        assert_eq!(view.revision, before.revision);
    }

    #[test]
    fn evaluation_keeps_a_pause_until_its_trigger_ends() {
        let mut inner = Inner::default();
        inner.view.triggers = vec![trigger("p", TriggerKind::Power)];
        let plugged = triggers::Context {
            on_power: true,
            ..triggers::Context::default()
        };
        inner.evaluate_triggers(&plugged, &Power::default());
        assert_eq!(inner.view.held_by.as_deref(), Some("on power"));
        inner.paused = vec!["p".into()];
        inner.evaluate_triggers(&plugged, &Power::default());
        assert_eq!(inner.view.held_by, None);
        assert_eq!(inner.view.paused_by.as_deref(), Some("you unplug"));
        inner.evaluate_triggers(&triggers::Context::default(), &Power::default());
        assert!(inner.paused.is_empty());
        assert_eq!(inner.view.paused_by, None);
        inner.evaluate_triggers(&plugged, &Power::default());
        assert_eq!(inner.view.held_by.as_deref(), Some("on power"));
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
    fn history_saves_when_counting_stops_and_loads_on_the_next_launch() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        let noon = chrono::Local::now()
            .date_naive()
            .and_hms_opt(12, 0, 0)
            .and_then(|time| time.and_local_timezone(chrono::Local).earliest())
            .unwrap()
            .timestamp_millis();
        {
            let runtime = Runtime::new(path.clone());
            let mut inner = runtime.inner.lock().unwrap();
            inner.history.add(noon - 120_000, noon, &chrono::Local);
            inner.history_dirty = true;
            inner.track(noon, true);
            assert!(!inner.history_dirty);
            assert_eq!(inner.view.history.len(), 7);
            assert_eq!(inner.view.history[6].minutes, 2);
        }
        let today = Runtime::new(path).state().history;
        assert_eq!(today.last().unwrap().minutes, 2);
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
        inner.view.session = Session::Indefinite { started_at: now };
        inner.view.paused_by_lock = true;
        inner.reconcile(now).unwrap();
        assert!(ours().is_empty());
        inner.view.paused_by_lock = false;
        inner.reconcile(now).unwrap();
        assert_eq!(ours(), ["PreventUserIdleDisplaySleep"]);
    }
}
