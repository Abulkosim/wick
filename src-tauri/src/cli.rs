use crate::core::{
    schedule::minutes as clock_minutes,
    session::{Request, MAX_MINUTES},
};

#[derive(Debug, PartialEq, Eq)]
pub enum Intent {
    Start(Request),
    Stop,
    Toggle,
    Status,
}

pub const USAGE: &str =
    "Usage: wick start [minutes | HH:MM] | wick stop | wick toggle | wick status";

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
