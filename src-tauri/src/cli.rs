use crate::core::{
    agents::{self, Agent, State},
    processes,
    schedule::minutes as clock_minutes,
    session::{Request, MAX_MINUTES},
};
use std::io::Read;

#[derive(Debug, PartialEq, Eq)]
pub enum Intent {
    Start(Request),
    Stop,
    Toggle,
    Status,
}

pub const USAGE: &str =
    "Usage: wick start [minutes | HH:MM] | wick stop | wick toggle | wick status";

/// Commands for agents and scripts. They run in this process and exit, without the app.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// `agent` is `None` for one this version doesn't know, which the hook ignores.
    Hook {
        agent: Option<Agent>,
        payload: Option<String>,
    },
    Hold {
        name: String,
        pid: Option<u32>,
    },
    Release {
        name: String,
    },
    Run {
        command: Vec<String>,
    },
}

pub const COMMAND_USAGE: &str = "Usage: wick hook claude | wick hook codex [json] | wick hold <name> [--pid N] | wick release <name> | wick run -- <command> [args]";

fn hold_pid(options: &[String]) -> Result<Option<u32>, String> {
    let value = match options {
        [] => return Ok(None),
        [flag, pid] if flag == "--pid" => pid.as_str(),
        [flag] if flag.starts_with("--pid=") => &flag["--pid=".len()..],
        _ => return Err(COMMAND_USAGE.into()),
    };
    match value.parse::<u32>() {
        Ok(pid) if pid > 0 => Ok(Some(pid)),
        _ => Err("Expected a process id after --pid.".into()),
    }
}

/// `wick hook …`, `wick hold …`, `wick release …`, `wick run …`; `None` for anything else.
pub fn command(args: &[String]) -> Option<Result<Command, String>> {
    let (first, rest) = args.get(1..)?.split_first()?;
    let usage = || Err(COMMAND_USAGE.to_string());
    Some(match (first.as_str(), rest) {
        ("hook", []) => Ok(Command::Hook {
            agent: None,
            payload: None,
        }),
        ("hook", [agent, extra @ ..]) => Ok(Command::Hook {
            agent: Agent::parse(agent),
            payload: (agent == "codex").then(|| extra.last().cloned()).flatten(),
        }),
        ("hold", [name, options @ ..]) if !name.trim().is_empty() && !name.starts_with('-') => {
            hold_pid(options).map(|pid| Command::Hold {
                name: name.clone(),
                pid,
            })
        }
        ("release", [name]) if !name.trim().is_empty() => {
            Ok(Command::Release { name: name.clone() })
        }
        ("run", rest) => {
            let command = match rest {
                [dashes, command @ ..] if dashes == "--" => command,
                command => command,
            };
            if command.is_empty() {
                usage()
            } else {
                Ok(Command::Run {
                    command: command.to_vec(),
                })
            }
        }
        ("hold" | "release", _) => usage(),
        _ => return None,
    })
}

fn debug(message: &str) {
    if std::env::var_os("WICK_DEBUG").is_some_and(|value| value == "1") {
        eprintln!("wick: {message}");
    }
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn holds_directory() -> Result<std::path::PathBuf, String> {
    crate::status::default_directory()
        .map(|directory| agents::directory(&directory))
        .ok_or_else(|| "Could not find Wick's data folder.".into())
}

/// Runs a command and returns its exit code. A hook always returns 0 and never prints to
/// stdout, which Claude Code would add to the prompt.
pub fn execute(command: Command) -> i32 {
    let result = match command {
        Command::Hook { agent, payload } => {
            if let Err(error) = hook(agent, payload) {
                debug(&error);
            }
            return 0;
        }
        Command::Hold { name, pid } => hold(&name, pid),
        Command::Release { name } => holds_directory().and_then(|directory| {
            agents::remove(&directory, Agent::Other, &name)
                .map_err(|error| format!("Could not release {name}: {error}"))
        }),
        Command::Run { command } => return run(&command),
    };
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("wick: {error}");
            1
        }
    }
}

fn read_payload(payload: Option<String>) -> Result<String, String> {
    let text = match payload {
        Some(text) => text,
        None => {
            let mut bytes = Vec::new();
            std::io::stdin()
                .take(agents::MAX_PAYLOAD as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|error| format!("Could not read the hook payload: {error}"))?;
            String::from_utf8(bytes).map_err(|_| "The hook payload isn't UTF-8.")?
        }
    };
    if text.len() > agents::MAX_PAYLOAD {
        return Err("The hook payload is over 1 MB.".into());
    }
    Ok(text)
}

fn hook(agent: Option<Agent>, payload: Option<String>) -> Result<(), String> {
    let agent = agent.ok_or("Unknown agent; expected claude or codex.")?;
    let text = read_payload(payload)?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| format!("The hook payload isn't JSON: {error}"))?;
    let event = match agent {
        Agent::Claude => agents::claude_event(&value),
        Agent::Codex => agents::codex_event(&value),
        Agent::Other => None,
    }
    .ok_or("Nothing to record for this event.")?;
    let directory = holds_directory()?;
    agents::record(&directory, agent, event, now(), |existing| {
        // The agent found at the last event, while it still runs, saves walking up again.
        match existing.and_then(|hold| Some((hold.pid?, hold.pid_start))) {
            Some((pid, start)) if agents::alive(pid, start) => (Some(pid), start),
            _ => {
                agents::agent_process(agent).map_or((None, None), |(pid, start)| (Some(pid), start))
            }
        }
    })
    .map_err(|error| format!("Could not write the hold: {error}"))
}

fn current_dir() -> Option<String> {
    std::env::current_dir()
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

fn hold(name: &str, pid: Option<u32>) -> Result<(), String> {
    let pid_start = match pid {
        Some(pid) if !processes::alive(pid) => {
            return Err(format!("No running process has pid {pid}."))
        }
        Some(pid) => processes::start_time(pid),
        None => None,
    };
    let directory = holds_directory()?;
    let existing = agents::read(&agents::path(&directory, Agent::Other, name));
    let hold = agents::next(
        existing.as_ref(),
        agents::Update {
            agent: Agent::Other,
            session: name.to_string(),
            label: name.to_string(),
            cwd: current_dir(),
            state: State::Working,
            pid,
            pid_start,
        },
        now(),
    );
    agents::write(&directory, &hold).map_err(|error| format!("Could not hold {name}: {error}"))
}

/// Holds while the command runs, then releases and exits with its code. Ctrl+C reaches the
/// command, which decides whether to stop; Wick waits for it either way.
fn run(command: &[String]) -> i32 {
    let mut child = match std::process::Command::new(&command[0])
        .args(&command[1..])
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            eprintln!("wick: could not run {}: {error}", command[0]);
            return 127;
        }
    };
    let pid = child.id();
    let session = format!("run-{pid}");
    let directory = holds_directory();
    if let Ok(directory) = &directory {
        signals::pass_on(pid, agents::path(directory, Agent::Other, &session));
        let now = now();
        let label = agents::basename(&command[0]);
        let hold = agents::next(
            None,
            agents::Update {
                agent: Agent::Other,
                session: session.clone(),
                label: label.strip_suffix(".exe").unwrap_or(&label).to_string(),
                cwd: current_dir(),
                state: State::Working,
                pid: Some(pid),
                pid_start: processes::start_time(pid),
            },
            now,
        );
        if let Err(error) = agents::write(directory, &hold) {
            eprintln!("wick: could not hold while {} runs: {error}", hold.label);
        }
    }
    let status = child.wait();
    if let Ok(directory) = &directory {
        let _ = agents::remove(directory, Agent::Other, &session);
    }
    match status {
        Ok(status) => exit_code(status),
        Err(error) => {
            eprintln!("wick: {error}");
            1
        }
    }
}

#[cfg(unix)]
fn exit_code(status: std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    match (status.code(), status.signal()) {
        (Some(code), _) => code,
        (None, Some(signal)) => {
            signals::reraise(signal);
            128 + signal
        }
        _ => 1,
    }
}

#[cfg(not(unix))]
fn exit_code(status: std::process::ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

/// Unix: SIGTERM, and a SIGINT that didn't come from the terminal (which already sent it to
/// the command), go on to the command; Wick releases the hold once the command ends.
#[cfg(unix)]
mod signals {
    use std::{
        ffi::c_int,
        path::PathBuf,
        sync::atomic::{AtomicI32, Ordering},
    };

    const SIGINT: c_int = 2;
    const SIGTERM: c_int = 15;
    const SIG_DFL: usize = 0;
    static CHILD: AtomicI32 = AtomicI32::new(0);

    extern "C" {
        fn signal(signal: c_int, handler: usize) -> usize;
        fn kill(pid: i32, signal: c_int) -> c_int;
        fn getpid() -> i32;
        fn getpgrp() -> i32;
        fn tcgetpgrp(fd: c_int) -> i32;
    }

    extern "C" fn forward(received: c_int) {
        let child = CHILD.load(Ordering::SeqCst);
        let from_terminal = received == SIGINT && unsafe { tcgetpgrp(0) == getpgrp() };
        if child > 0 && !from_terminal {
            unsafe { kill(child, received) };
        }
    }

    pub fn pass_on(pid: u32, _hold: PathBuf) {
        let Ok(pid) = i32::try_from(pid) else {
            return;
        };
        CHILD.store(pid, Ordering::SeqCst);
        let handler = forward as extern "C" fn(c_int) as usize;
        for received in [SIGINT, SIGTERM] {
            unsafe { signal(received, handler) };
        }
    }

    /// Ends Wick the way the command ended, so the shell sees the same signal.
    pub fn reraise(received: c_int) {
        unsafe {
            signal(received, SIG_DFL);
            kill(getpid(), received);
        }
    }
}

/// Windows: Ctrl+C and Ctrl+Break reach the command in the same console and Wick waits for
/// it. Closing the console, logging off or shutting down ends Wick, so the hold goes first.
#[cfg(windows)]
mod signals {
    use std::{path::PathBuf, sync::OnceLock};

    static HOLD: OnceLock<PathBuf> = OnceLock::new();

    unsafe extern "system" fn handle(event: u32) -> i32 {
        use crate::core::win32::{CTRL_BREAK_EVENT, CTRL_C_EVENT};
        if event == CTRL_C_EVENT || event == CTRL_BREAK_EVENT {
            return 1;
        }
        if let Some(path) = HOLD.get() {
            let _ = std::fs::remove_file(path);
        }
        0
    }

    pub fn pass_on(_pid: u32, hold: PathBuf) {
        let _ = HOLD.set(hold);
        unsafe { crate::core::win32::SetConsoleCtrlHandler(Some(handle), 1) };
    }
}

#[cfg(not(any(unix, windows)))]
mod signals {
    pub fn pass_on(_pid: u32, _hold: std::path::PathBuf) {}
}

/// "45" is minutes, "16:30" is a time of day; nothing means until stopped.
fn start(value: Option<&str>) -> Result<Intent, String> {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return Ok(Intent::Start(Request::Indefinite));
    };
    if let Some(total) = clock_minutes(value) {
        return Ok(Intent::Start(Request::Until {
            hour: total / 60,
            minute: total % 60,
        }));
    }
    match value.parse::<u32>() {
        Ok(minutes) if (1..=MAX_MINUTES).contains(&minutes) => {
            Ok(Intent::Start(Request::For { minutes }))
        }
        _ => Err(format!(
            "Expected minutes between 1 and {MAX_MINUTES}, or a time like 16:30."
        )),
    }
}

/// `wick start`, `wick start 45`, `wick start 16:30`, `wick stop`, `wick toggle`, `wick status`.
pub fn parse(args: &[String]) -> Result<Option<Intent>, String> {
    let mut words = args
        .iter()
        .skip(1)
        .filter(|arg| !arg.starts_with("--") && !arg.starts_with("-psn"));
    let first = words.next().map(String::as_str);
    let second = words.next().map(String::as_str);
    if words.next().is_some() {
        return Err(USAGE.into());
    }
    match (first, second) {
        (None, _) => Ok(None),
        (Some(url), None) if url.starts_with("wick://") => parse_url(url).map(Some),
        (Some("start"), value) => start(value).map(Some),
        (Some("stop"), None) => Ok(Some(Intent::Stop)),
        (Some("toggle"), None) => Ok(Some(Intent::Toggle)),
        (Some("status"), None) => Ok(Some(Intent::Status)),
        _ => Err(USAGE.into()),
    }
}

/// `wick://start`, `wick://start?minutes=45`, `wick://start?until=16:30`, `wick://stop`,
/// `wick://toggle`.
pub fn parse_url(url: &str) -> Result<Intent, String> {
    let rest = url
        .strip_prefix("wick://")
        .ok_or("Wick links start with wick://")?;
    let (action, query) = rest.split_once('?').unwrap_or((rest, ""));
    let action = action.trim_end_matches('/');
    let mut pairs = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| pair.split_once('=').unwrap_or((pair, "")));
    let pair = pairs.next();
    if pairs.next().is_some() {
        return Err("Use one of minutes= or until=.".into());
    }
    match (action, pair) {
        ("start", None) => start(None),
        ("start", Some(("minutes", value))) if value.parse::<u32>().is_ok() => start(Some(value)),
        ("start", Some(("until", value))) => {
            let value = value.replace("%3A", ":").replace("%3a", ":");
            match clock_minutes(&value) {
                Some(_) => start(Some(&value)),
                None => Err("Use until=16:30.".into()),
            }
        }
        ("stop", None) => Ok(Intent::Stop),
        ("toggle", None) => Ok(Intent::Toggle),
        _ => Err(format!("Unknown Wick link: {url}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        std::iter::once("wick")
            .chain(words.iter().copied())
            .map(String::from)
            .collect()
    }

    #[test]
    fn parses_commands() {
        assert_eq!(parse(&args(&[])), Ok(None));
        assert_eq!(parse(&args(&["--background"])), Ok(None));
        assert_eq!(
            parse(&args(&["start"])),
            Ok(Some(Intent::Start(Request::Indefinite)))
        );
        assert_eq!(
            parse(&args(&["start", "45"])),
            Ok(Some(Intent::Start(Request::For { minutes: 45 })))
        );
        assert_eq!(
            parse(&args(&["start", "16:30"])),
            Ok(Some(Intent::Start(Request::Until {
                hour: 16,
                minute: 30
            })))
        );
        assert_eq!(parse(&args(&["stop"])), Ok(Some(Intent::Stop)));
        assert_eq!(parse(&args(&["toggle"])), Ok(Some(Intent::Toggle)));
        assert_eq!(parse(&args(&["status"])), Ok(Some(Intent::Status)));
        assert!(parse(&args(&["start", "0"])).is_err());
        assert!(parse(&args(&["start", "soon"])).is_err());
        assert!(parse(&args(&["sleep"])).is_err());
        assert!(parse(&args(&["stop", "now", "please"])).is_err());
    }

    #[test]
    fn parses_agent_commands() {
        let parsed = |words: &[&str]| command(&args(words));
        let hook = |agent, payload: Option<&str>| {
            Some(Ok(Command::Hook {
                agent,
                payload: payload.map(String::from),
            }))
        };
        let hold = |pid| {
            Some(Ok(Command::Hold {
                name: "build".into(),
                pid,
            }))
        };
        let run = |words: &[&str]| {
            Some(Ok(Command::Run {
                command: words.iter().map(|word| word.to_string()).collect(),
            }))
        };
        assert_eq!(parsed(&[]), None);
        assert_eq!(parsed(&["start", "45"]), None);
        assert_eq!(parsed(&["hook", "claude"]), hook(Some(Agent::Claude), None));
        assert_eq!(
            parsed(&["hook", "claude", "{}"]),
            hook(Some(Agent::Claude), None)
        );
        assert_eq!(parsed(&["hook", "codex"]), hook(Some(Agent::Codex), None));
        assert_eq!(
            parsed(&["hook", "codex", "-x", r#"{"type":"agent-turn-complete"}"#]),
            hook(
                Some(Agent::Codex),
                Some(r#"{"type":"agent-turn-complete"}"#)
            )
        );
        assert_eq!(parsed(&["hook", "gemini"]), hook(None, None));
        assert_eq!(parsed(&["hook"]), hook(None, None));
        assert_eq!(parsed(&["hold", "build"]), hold(None));
        assert_eq!(parsed(&["hold", "build", "--pid", "42"]), hold(Some(42)));
        assert_eq!(parsed(&["hold", "build", "--pid=42"]), hold(Some(42)));
        for bad in [
            &["hold", "build", "--pid", "x"][..],
            &["hold", "build", "--pid", "0"],
            &["hold", "build", "--pid"],
            &["hold", "build", "extra"],
            &["hold"],
            &["hold", "--pid", "4"],
            &["release"],
            &["release", "a", "b"],
            &["run"],
            &["run", "--"],
        ] {
            assert!(parsed(bad).unwrap().is_err(), "{bad:?}");
        }
        assert_eq!(
            parsed(&["release", "build"]),
            Some(Ok(Command::Release {
                name: "build".into()
            }))
        );
        assert_eq!(
            parsed(&["run", "--", "make", "-j", "--quiet"]),
            run(&["make", "-j", "--quiet"])
        );
        assert_eq!(parsed(&["run", "sleep", "1"]), run(&["sleep", "1"]));
    }

    #[test]
    fn parses_links() {
        assert_eq!(
            parse_url("wick://start"),
            Ok(Intent::Start(Request::Indefinite))
        );
        assert_eq!(
            parse_url("wick://start/?minutes=60"),
            Ok(Intent::Start(Request::For { minutes: 60 }))
        );
        assert_eq!(
            parse_url("wick://start?until=09%3A05"),
            Ok(Intent::Start(Request::Until { hour: 9, minute: 5 }))
        );
        assert_eq!(parse_url("wick://stop"), Ok(Intent::Stop));
        assert_eq!(parse_url("wick://toggle"), Ok(Intent::Toggle));
        assert!(parse_url("wick://start?minutes=abc").is_err());
        assert!(parse_url("wick://start?minutes=5&until=10:00").is_err());
        assert!(parse_url("wick://explode").is_err());
        assert!(parse_url("https://example.com").is_err());
        assert_eq!(parse(&args(&["wick://stop"])), Ok(Some(Intent::Stop)));
    }
}
