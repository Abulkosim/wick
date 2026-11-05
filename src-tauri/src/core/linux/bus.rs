//! D-Bus calls to systemd-logind, the XDG desktop portal and the desktop's session manager.

use super::parse::{Inhibitor, LidPolicy, SessionInhibitor};
use std::{collections::HashMap, sync::Mutex, time::Duration};
use zbus::{
    blocking::{connection::Builder, Connection},
    zvariant::{OwnedFd, OwnedObjectPath, OwnedValue, Value},
};

const LOGIN1: &str = "org.freedesktop.login1";
const MANAGER_PATH: &str = "/org/freedesktop/login1";
const MANAGER: &str = "org.freedesktop.login1.Manager";
const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
/// A slow or hung service must not stall the ticker or an intent.
const TIMEOUT: Duration = Duration::from_secs(3);

/// One shared connection per bus, reopened after an error.
static SYSTEM: Mutex<Option<Connection>> = Mutex::new(None);
static SESSION: Mutex<Option<Connection>> = Mutex::new(None);

fn connect(session: bool) -> zbus::Result<Connection> {
    let builder = if session {
        Builder::session()?
    } else {
        Builder::system()?
    };
    builder.method_timeout(TIMEOUT).build()
}

/// Runs `call` on the shared connection, dropping it on failure so the next call reconnects.
fn with<T>(session: bool, call: impl FnOnce(&Connection) -> zbus::Result<T>) -> Option<T> {
    let slot = if session { &SESSION } else { &SYSTEM };
    let mut slot = slot.lock().ok()?;
    if slot.is_none() {
        *slot = connect(session).ok();
    }
    let result = call(slot.as_ref()?);
    if result.is_err() {
        *slot = None;
    }
    result.ok()
}

fn property<T>(
    connection: &Connection,
    destination: &str,
    path: &str,
    interface: &str,
    name: &str,
) -> zbus::Result<T>
where
    T: TryFrom<OwnedValue>,
    T::Error: Into<zbus::Error>,
{
    let reply = connection.call_method(
        Some(destination),
        path,
        Some("org.freedesktop.DBus.Properties"),
        "Get",
        &(interface, name),
    )?;
    let value: OwnedValue = reply.body().deserialize()?;
    T::try_from(value).map_err(Into::into)
}

/// What keeps the computer awake while it is held: a portal request on its own session
/// connection, a ScreenSaver cookie on the same connection, and a logind file descriptor.
/// Closing the connection or the descriptor releases each, so a crash releases all of them.
pub struct Inhibition {
    session: Option<Connection>,
    portal: Option<OwnedObjectPath>,
    screensaver: Option<u32>,
    _logind: Option<OwnedFd>,
}

impl std::fmt::Debug for Inhibition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Inhibition")
            .field("portal", &self.portal.is_some())
            .field("screensaver", &self.screensaver.is_some())
            .field("logind", &self._logind.is_some())
            .finish()
    }
}

/// `display` also keeps the screen from blanking and locking.
pub fn inhibit(reason: &str, display: bool) -> Result<Inhibition, String> {
    // Its own connection: the portal ties the request to it, so it must outlive the request.
    let session = connect(true).ok();
    let portal = session.as_ref().and_then(|connection| {
        // Suspend = 4, Idle = 8.
        let flags: u32 = if display { 4 | 8 } else { 4 };
        let mut options: HashMap<&str, Value> = HashMap::new();
        options.insert("reason", Value::from(reason));
        connection
            .call_method(
                Some(PORTAL),
                PORTAL_PATH,
                Some("org.freedesktop.portal.Inhibit"),
                "Inhibit",
                &("", flags, options),
            )
            .and_then(|reply| reply.body().deserialize::<OwnedObjectPath>())
            .ok()
    });
    let screensaver = match (&session, portal.is_none() && display) {
        (Some(connection), true) => connection
            .call_method(
                Some("org.freedesktop.ScreenSaver"),
                "/org/freedesktop/ScreenSaver",
                Some("org.freedesktop.ScreenSaver"),
                "Inhibit",
                &("Wick", reason),
            )
            .and_then(|reply| reply.body().deserialize::<u32>())
            .ok(),
        _ => None,
    };
    // Without a desktop portal nothing else stops a window manager's idle suspend, so take
    // logind's sleep lock too; `systemctl suspend -i` still works.
    let what = if portal.is_some() || screensaver.is_some() {
        "idle"
    } else {
        "idle:sleep"
    };
    let logind = connect(false).ok().and_then(|connection| {
        connection
            .call_method(
                Some(LOGIN1),
                MANAGER_PATH,
                Some(MANAGER),
                "Inhibit",
                &(what, "Wick", reason, "block"),
            )
            .and_then(|reply| reply.body().deserialize::<OwnedFd>())
            .ok()
    });
    if portal.is_none() && screensaver.is_none() && logind.is_none() {
        return Err(
            "Neither the desktop portal nor systemd-logind accepted Wick's request. Is a desktop session running?"
                .into(),
        );
    }
    Ok(Inhibition {
        session: session.filter(|_| portal.is_some() || screensaver.is_some()),
        portal,
        screensaver,
        _logind: logind,
    })
}

impl Inhibition {
    pub fn uses_portal(&self) -> bool {
        self.portal.is_some()
    }

    pub fn uses_logind(&self) -> bool {
        self._logind.is_some()
    }
}

impl Drop for Inhibition {
    fn drop(&mut self) {
        let Some(connection) = &self.session else {
            return;
        };
        if let Some(handle) = &self.portal {
            let _ = connection.call_method(
                Some(PORTAL),
                handle.as_str(),
                Some("org.freedesktop.portal.Request"),
                "Close",
                &(),
            );
        }
        if let Some(cookie) = self.screensaver {
            let _ = connection.call_method(
                Some("org.freedesktop.ScreenSaver"),
                "/org/freedesktop/ScreenSaver",
                Some("org.freedesktop.ScreenSaver"),
                "UnInhibit",
                &(cookie,),
            );
        }
    }
}

/// logind's inhibitors: (what, who, why, mode, uid, pid).
pub fn logind_inhibitors() -> Vec<Inhibitor> {
    with(false, |connection| {
        let reply = connection.call_method(
            Some(LOGIN1),
            MANAGER_PATH,
            Some(MANAGER),
            "ListInhibitors",
            &(),
        )?;
        let rows: Vec<(String, String, String, String, u32, u32)> = reply.body().deserialize()?;
        Ok(rows
            .into_iter()
            .map(|(what, who, why, mode, _uid, pid)| Inhibitor {
                what,
                who,
                why,
                mode,
                pid,
            })
            .collect())
    })
    .unwrap_or_default()
}

/// Inhibitors GNOME or KDE hold for apps (Firefox playing a video, a download).
pub fn session_inhibitors() -> Vec<SessionInhibitor> {
    let gnome = with(true, |connection| {
        let reply = connection.call_method(
            Some("org.gnome.SessionManager"),
            "/org/gnome/SessionManager",
            Some("org.gnome.SessionManager"),
            "GetInhibitors",
            &(),
        )?;
        let paths: Vec<OwnedObjectPath> = reply.body().deserialize()?;
        let mut list = Vec::new();
        for path in paths {
            let call = |method: &str| {
                connection.call_method(
                    Some("org.gnome.SessionManager"),
                    path.as_str(),
                    Some("org.gnome.SessionManager.Inhibitor"),
                    method,
                    &(),
                )
            };
            let flags: u32 = call("GetFlags")?.body().deserialize()?;
            // Suspend = 4, Idle = 8.
            if flags & (4 | 8) == 0 {
                continue;
            }
            list.push(SessionInhibitor {
                app: call("GetAppId")?.body().deserialize()?,
                reason: call("GetReason")?.body().deserialize()?,
                display: flags & 8 != 0,
            });
        }
        Ok(list)
    });
    let kde = || {
        with(true, |connection| {
            let reply = connection.call_method(
                Some("org.kde.Solid.PowerManagement"),
                "/org/kde/Solid/PowerManagement/PolicyAgent",
                Some("org.kde.Solid.PowerManagement.PolicyAgent"),
                "ListInhibitions",
                &(),
            )?;
            let rows: Vec<(String, String)> = reply.body().deserialize()?;
            Ok(rows
                .into_iter()
                .map(|(app, reason)| SessionInhibitor {
                    app,
                    reason,
                    display: false,
                })
                .collect())
        })
    };
    gnome.or_else(kde).unwrap_or_default()
}

/// Whether this user's graphical session reports its screen as locked.
pub fn screen_locked() -> bool {
    with(false, |connection| {
        let mut paths = Vec::new();
        if let Some(id) = std::env::var_os("XDG_SESSION_ID") {
            paths.push(format!(
                "/org/freedesktop/login1/session/{}",
                escape(&id.to_string_lossy())
            ));
        }
        paths.push("/org/freedesktop/login1/session/auto".into());
        let mut last = Err(zbus::Error::Failure("No login session.".into()));
        for path in &paths {
            last = property::<bool>(
                connection,
                LOGIN1,
                path,
                "org.freedesktop.login1.Session",
                "LockedHint",
            );
            if last.is_ok() {
                break;
            }
        }
        last
    })
    .unwrap_or(false)
}

/// logind escapes object path elements: every byte outside `[A-Za-z0-9]` becomes `_xx`.
fn escape(element: &str) -> String {
    element
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() {
                (byte as char).to_string()
            } else {
                format!("_{byte:02x}")
            }
        })
        .collect()
}

/// logind's lid settings and dock state, plus whether a desktop has taken over the lid.
pub fn lid_policy() -> Option<LidPolicy> {
    let mut policy = with(false, |connection| {
        let text = |name: &str| property::<String>(connection, LOGIN1, MANAGER_PATH, MANAGER, name);
        let flag = |name: &str| property::<bool>(connection, LOGIN1, MANAGER_PATH, MANAGER, name);
        Ok(LidPolicy {
            handle: text("HandleLidSwitch")?,
            external_power: text("HandleLidSwitchExternalPower").unwrap_or_default(),
            docked: text("HandleLidSwitchDocked").unwrap_or_default(),
            on_external_power: flag("OnExternalPower").unwrap_or(false),
            is_docked: flag("Docked").unwrap_or(false),
            taken_over: false,
        })
    })?;
    let own = std::process::id();
    policy.taken_over = logind_inhibitors().iter().any(|inhibitor| {
        inhibitor.mode == "block"
            && inhibitor.pid != own
            && inhibitor
                .what
                .split(':')
                .any(|what| what == "handle-lid-switch")
    });
    Some(policy)
}

/// power-profiles-daemon's "power-saver" profile is Linux's Low Power Mode.
pub fn power_saver() -> bool {
    with(false, |connection| {
        property::<String>(
            connection,
            "org.freedesktop.UPower.PowerProfiles",
            "/org/freedesktop/UPower/PowerProfiles",
            "org.freedesktop.UPower.PowerProfiles",
            "ActiveProfile",
        )
        .or_else(|_| {
            property::<String>(
                connection,
                "net.hadess.PowerProfiles",
                "/net/hadess/PowerProfiles",
                "net.hadess.PowerProfiles",
                "ActiveProfile",
            )
        })
    })
    .is_some_and(|profile| profile == "power-saver")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ids_are_escaped_like_logind() {
        assert_eq!(escape("2"), "2");
        assert_eq!(escape("c1"), "c1");
        assert_eq!(escape("a-b"), "a_2db");
    }

    /// Needs a system bus with logind, as on a desktop or the CI runner. Skips without one.
    #[test]
    fn logind_inhibitor_appears_and_goes_on_drop() {
        if connect(false).is_err() {
            eprintln!("no system bus; skipping");
            return;
        }
        let reason = format!("Wick: test {}", std::process::id());
        let ours = || {
            logind_inhibitors()
                .into_iter()
                .filter(|inhibitor| inhibitor.why == reason)
                .count()
        };
        let Ok(inhibition) = inhibit(&reason, false) else {
            eprintln!("logind refused the inhibitor here; skipping");
            return;
        };
        eprintln!(
            "portal: {}, logind: {}",
            inhibition.uses_portal(),
            inhibition.uses_logind()
        );
        if inhibition.uses_logind() {
            assert_eq!(ours(), 1);
        }
        drop(inhibition);
        assert_eq!(ours(), 0);
    }
}
