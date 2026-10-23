use crate::core::session::{Request, MAX_MINUTES};

#[derive(Debug, PartialEq, Eq)]
pub enum Intent {
    Start(Request),
    Stop,
}

/// `wick start`, `wick start 45` (minutes) and `wick stop`, also forwarded to a running Wick.
pub fn parse(args: &[String]) -> Result<Option<Intent>, String> {
    let mut words = args
        .iter()
        .skip(1)
        .filter(|arg| !arg.starts_with("--") && !arg.starts_with("-psn"));
    match (
        words.next().map(String::as_str),
        words.next().map(String::as_str),
    ) {
        (None, _) => Ok(None),
        (Some("stop"), None) => Ok(Some(Intent::Stop)),
        (Some("start"), None) => Ok(Some(Intent::Start(Request::Indefinite))),
        (Some("start"), Some(minutes)) => match minutes.parse::<u32>() {
            Ok(minutes) if (1..=MAX_MINUTES).contains(&minutes) => {
                Ok(Some(Intent::Start(Request::For { minutes })))
            }
            _ => Err(format!("Expected minutes between 1 and {MAX_MINUTES}.")),
        },
        _ => Err("Usage: wick start [minutes] | wick stop".into()),
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
    fn parses_start_and_stop() {
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
        assert_eq!(parse(&args(&["stop"])), Ok(Some(Intent::Stop)));
        assert!(parse(&args(&["start", "0"])).is_err());
        assert!(parse(&args(&["start", "soon"])).is_err());
        assert!(parse(&args(&["sleep"])).is_err());
    }
}
