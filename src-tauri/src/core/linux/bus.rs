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
    handshake(move || {
        if session {
            Builder::session()
        } else {
            Builder::system()
        }
    })
}

/// Connects, giving up after `TIMEOUT`: `method_timeout` bounds replies, not the handshake, and
/// a bus that accepts the socket but never authenticates would otherwise hang the ticker or an
/// intent. A handshake that never finishes leaves only its own thread waiting.
fn handshake(
    builder: impl FnOnce() -> zbus::Result<Builder<'static>> + Send + 'static,
) -> zbus::Result<Connection> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("wick-dbus-connect".into())
        .spawn(move || {
            let _ =
                sender.send(builder().and_then(|builder| builder.method_timeout(TIMEOUT).build()));
        })
        .map_err(|_| zbus::Error::Failure("Could not start a D-Bus connection.".into()))?;
    receiver
        .recv_timeout(TIMEOUT)
        .unwrap_or_else(|_| Err(zbus::Error::Failure("D-Bus didn't answer in time.".into())))
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

/// What keeps the computer awake while it is held: a portal request, PowerManagement and
/// ScreenSaver cookies on Wick's own session connection, and a logind file descriptor.
/// Closing the connection or the descriptor releases each, so a crash releases all of them.
pub struct Inhibition {
    session: Option<Connection>,
    portal: Option<OwnedObjectPath>,
    power_management: Option<u32>,
    screensaver: Option<u32>,
    _logind: Option<OwnedFd>,
    sleep_lock: bool,
}

impl std::fmt::Debug for Inhibition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Inhibition")
            .field("portal", &self.portal.is_some())
            .field("power_management", &self.power_management.is_some())
            .field("screensaver", &self.screensaver.is_some())
            .field("logind", &self._logind.is_some())
            .field("sleep_lock", &self.sleep_lock)
            .finish()
    }
}

/// `Inhibit(app, reason)` on a session service that answers with a cookie.
fn cookie(connection: &Connection, service: &str, path: &str, reason: &str) -> Option<u32> {
    let interface = format!("{service}{}", interface_suffix(path));
    connection
        .call_method(
            Some(service),
            path,
            Some(interface.as_str()),
            "Inhibit",
            &("Wick", reason),
        )
        .and_then(|reply| reply.body().deserialize::<u32>())
        .ok()
}

/// PowerManagement serves `org.freedesktop.PowerManagement.Inhibit` at `…/Inhibit`;
/// ScreenSaver's interface has the service's own name.
fn interface_suffix(path: &str) -> &'static str {
    if path.ends_with("/Inhibit") {
        ".Inhibit"
    } else {
        ""
    }
}

/// `display` also keeps the screen from blanking and locking.
pub fn inhibit(reason: &str, display: bool) -> Result<Inhibition, String> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let trusted = portal_honours_suspend(&desktop);
    // Suspend = 4, Idle = 8. The portal returns a handle before its backend has run, so its
    // answer only counts where the backend is known to work; elsewhere (Xfce's GTK backend
    // refuses any request that includes suspend) ask it for idle alone.
    let flags = portal_flags(trusted, display);
    // Its own connection: the portal and the session services tie Wick's requests to it, so it
    // must live as long as they do.
    let session = connect(true).ok();
    let portal = session.as_ref().zip(flags).and_then(|(connection, flags)| {
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
    // xfce4-power-manager and KDE's PowerDevil take this one; on Xfce the portal's GTK backend
    // can only block idle, not suspend.
    let power_management = session.as_ref().and_then(|connection| {
        cookie(
            connection,
            "org.freedesktop.PowerManagement",
            "/org/freedesktop/PowerManagement/Inhibit",
            reason,
        )
    });
    // Where the portal's answer can't be trusted, ask the screensaver directly as well.
    let screensaver = session
        .as_ref()
        .filter(|_| display && !(trusted && portal.is_some()))
        .and_then(|connection| {
            cookie(
                connection,
                "org.freedesktop.ScreenSaver",
                "/org/freedesktop/ScreenSaver",
                reason,
            )
        });
    let sleep_lock = needs_sleep_lock(&desktop, portal.is_some(), power_management.is_some());
    // polkit may refuse the sleep lock outside an active seat (SSH, a CI runner); the idle
    // lock alone still stops logind's own idle action.
    let logind = connect(false).ok().and_then(|connection| {
        let lock = |what: &str| {
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
                .map(|fd| (fd, what == "idle:sleep"))
        };
        if sleep_lock {
            lock("idle:sleep").or_else(|| lock("idle"))
        } else {
            lock("idle")
        }
    });
    let held_on_session = portal.is_some() || power_management.is_some() || screensaver.is_some();
    if !held_on_session && logind.is_none() {
        return Err(
            "Neither the desktop nor systemd-logind accepted Wick's request. Is a desktop session running?"
                .into(),
        );
    }
    let (logind, sleep_lock) = match logind {
        Some((fd, sleep)) => (Some(fd), sleep),
        None => (None, false),
    };
    Ok(Inhibition {
        session: session.filter(|_| held_on_session),
        portal,
        power_management,
        screensaver,
        _logind: logind,
        sleep_lock,
    })
}

/// Desktops whose portal backend really blocks suspend; elsewhere (Xfce's GTK backend, wlroots)
/// the portal accepts the request and only blocks idle.
const PORTAL_SUSPENDS: [&str; 6] = ["gnome", "kde", "unity", "pantheon", "cinnamon", "budgie"];

/// Whether logind must also hold a sleep lock: only when nothing the desktop honours for
/// suspend answered. A sleep lock also refuses a manual suspend (unless `-i`), so it is the
/// last resort, for window managers and desktops without a working inhibitor.
pub fn needs_sleep_lock(desktop: &str, portal: bool, power_management: bool) -> bool {
    !(portal && portal_honours_suspend(desktop)) && !power_management
}

/// What to ask the portal for: Suspend = 4, Idle = 8. Where its backend can't block suspend,
/// only idle, and nothing at all when the display may sleep anyway.
fn portal_flags(trusted: bool, display: bool) -> Option<u32> {
    match (trusted, display) {
        (true, true) => Some(4 | 8),
        (true, false) => Some(4),
        (false, true) => Some(8),
        (false, false) => None,
    }
}

fn portal_honours_suspend(desktop: &str) -> bool {
    let desktop = desktop.to_lowercase();
    PORTAL_SUSPENDS.iter().any(|name| desktop.contains(name))
}

impl Inhibition {
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
        if let Some(cookie) = self.power_management {
            let _ = connection.call_method(
                Some("org.freedesktop.PowerManagement"),
                "/org/freedesktop/PowerManagement/Inhibit",
                Some("org.freedesktop.PowerManagement.Inhibit"),
                "UnInhibit",
                &(cookie,),
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

/// The desktop's colour preference from the portal: 1 = dark, 2 = light, 0 = none.
pub fn color_scheme() -> Option<u32> {
    with(true, |connection| {
        let reply = connection.call_method(
            Some(PORTAL),
            PORTAL_PATH,
            Some("org.freedesktop.portal.Settings"),
            "Read",
            &("org.freedesktop.appearance", "color-scheme"),
        )?;
        // `Read` wraps the value in one variant more than `ReadOne`.
        let value: OwnedValue = reply.body().deserialize()?;
        let value = match &*value {
            Value::Value(inner) => inner.try_clone()?,
            _ => value.try_clone()?.into(),
        };
        Ok(u32::try_from(&value).ok())
    })
    .flatten()
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
    fn connections_give_up_on_a_bus_that_never_answers() {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("bus");
        // Accepts the connection, then never speaks.
        let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let address = format!("unix:path={}", socket.display());
        let started = std::time::Instant::now();
        assert!(handshake(move || Builder::address(address.as_str())).is_err());
        assert!(started.elapsed() < TIMEOUT + Duration::from_secs(2));
    }

    #[test]
    fn untrusted_portals_are_only_asked_for_idle() {
        assert_eq!(portal_flags(true, true), Some(12));
        assert_eq!(portal_flags(true, false), Some(4));
        assert_eq!(portal_flags(false, true), Some(8));
        assert_eq!(portal_flags(false, false), None);
        assert!(portal_honours_suspend("ubuntu:GNOME"));
        assert!(!portal_honours_suspend("XFCE"));
    }

    #[test]
    fn sleep_lock_only_without_a_working_desktop_inhibitor() {
        assert!(!needs_sleep_lock("ubuntu:GNOME", true, false));
        assert!(!needs_sleep_lock("KDE", true, true));
        assert!(!needs_sleep_lock("XFCE", true, true));
        assert!(needs_sleep_lock("XFCE", true, false));
        assert!(needs_sleep_lock("sway", true, false));
        assert!(needs_sleep_lock("", false, false));
        assert!(needs_sleep_lock("GNOME", false, false));
    }

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
        eprintln!("{inhibition:?}");
        if inhibition.uses_logind() {
            assert_eq!(ours(), 1);
        }
        drop(inhibition);
        // logind drops the inhibitor when it sees the descriptor close, a moment later.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while ours() > 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert_eq!(ours(), 0);
    }
}
