use super::candle::Glyph;
use chrono::{DateTime, Duration, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};

pub const MAX_MINUTES: u32 = 7 * 24 * 60;

/// Times are Unix milliseconds so the frontend can count down without parsing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum Session {
    #[default]
    Off,
    #[serde(rename_all = "camelCase")]
    For { started_at: i64, ends_at: i64 },
    #[serde(rename_all = "camelCase")]
    Until { started_at: i64, ends_at: i64 },
    #[serde(rename_all = "camelCase")]
    Indefinite { started_at: i64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum Request {
    For { minutes: u32 },
    Until { hour: u32, minute: u32 },
    Indefinite,
}

/// The next occurrence of `hour:minute` after `now`, tomorrow if it has passed.
fn next_time<Tz: TimeZone>(now: &DateTime<Tz>, time: NaiveTime) -> Option<DateTime<Tz>> {
    let timezone = now.timezone();
    let mut day = now.date_naive();
    for _ in 0..3 {
        let candidate = day.and_time(time);
        let local = timezone
            .from_local_datetime(&candidate)
            .earliest()
            .or_else(|| {
                timezone
                    .from_local_datetime(&(candidate + Duration::hours(1)))
                    .earliest()
            });
        if let Some(local) = local.filter(|local| local > now) {
            return Some(local);
        }
        day = day.succ_opt()?;
    }
    None
}

impl Session {
    pub fn start<Tz: TimeZone>(request: Request, now: &DateTime<Tz>) -> Result<Self, String> {
        let started_at = now.timestamp_millis();
        match request {
            Request::Indefinite => Ok(Session::Indefinite { started_at }),
            Request::For { minutes } => {
                if minutes == 0 || minutes > MAX_MINUTES {
                    return Err("Choose between 1 minute and 7 days.".into());
                }
                Ok(Session::For {
                    started_at,
                    ends_at: started_at + i64::from(minutes) * 60_000,
                })
            }
            Request::Until { hour, minute } => {
                let time = NaiveTime::from_hms_opt(hour, minute, 0)
                    .ok_or("Choose a time between 00:00 and 23:59.")?;
                let ends_at = next_time(now, time)
                    .ok_or("That time does not exist today or tomorrow.")?
                    .timestamp_millis();
                Ok(Session::Until {
                    started_at,
                    ends_at,
                })
            }
        }
    }

    pub fn is_on(&self) -> bool {
        !matches!(self, Session::Off)
    }

    pub fn ends_at(&self) -> Option<i64> {
        match *self {
            Session::For { ends_at, .. } | Session::Until { ends_at, .. } => Some(ends_at),
            _ => None,
        }
    }

    pub fn remaining_ms(&self, now: i64) -> Option<i64> {
        self.ends_at().map(|ends_at| (ends_at - now).max(0))
    }

    /// True once a timed session has run out, including across sleep.
    pub fn expired(&self, now: i64) -> bool {
        self.ends_at().is_some_and(|ends_at| now >= ends_at)
    }

    pub fn fraction(&self, now: i64) -> Option<f64> {
        match *self {
            Session::For {
                started_at,
                ends_at,
            }
            | Session::Until {
                started_at,
                ends_at,
            } => {
                let total = (ends_at - started_at).max(1) as f64;
                Some(((ends_at - now) as f64 / total).clamp(0.0, 1.0))
            }
            _ => None,
        }
    }

    pub fn glyph(&self, now: i64, held: bool) -> Glyph {
        match self {
            Session::Off if held => Glyph::Held,
            Session::Off => Glyph::Off,
            Session::Indefinite { .. } => Glyph::Full,
            _ => Glyph::Remaining(self.fraction(now).unwrap_or(0.0)),
        }
    }
}

/// "42m", "1h 05m", "2d 3h": the countdown shown in the tray tooltip and title.
pub fn short_duration(ms: i64) -> String {
    let minutes = (ms.max(0) + 59_999) / 60_000;
    let (days, hours, minutes) = (minutes / 1440, minutes / 60 % 24, minutes % 60);
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn at(hour: u32, minute: u32) -> DateTime<FixedOffset> {
        FixedOffset::east_opt(5 * 3600)
            .unwrap()
            .with_ymd_and_hms(2026, 10, 23, hour, minute, 0)
            .unwrap()
    }

    #[test]
    fn timed_session_counts_down_and_expires() {
        let now = at(15, 0);
        let session = Session::start(Request::For { minutes: 60 }, &now).unwrap();
        let start = now.timestamp_millis();
        assert_eq!(session.remaining_ms(start), Some(3_600_000));
        assert_eq!(session.fraction(start + 900_000), Some(0.75));
        assert!(!session.expired(start + 3_599_999));
        assert!(session.expired(start + 3_600_000));
        assert_eq!(session.glyph(start, false), Glyph::Remaining(1.0));
    }

    #[test]
    fn time_after_sleep_never_goes_negative() {
        let now = at(15, 0);
        let session = Session::start(Request::For { minutes: 15 }, &now).unwrap();
        let after_wake = now.timestamp_millis() + 5 * 3_600_000;
        assert_eq!(session.remaining_ms(after_wake), Some(0));
        assert_eq!(session.fraction(after_wake), Some(0.0));
        assert!(session.expired(after_wake));
    }

    #[test]
    fn until_rolls_to_tomorrow_when_the_time_has_passed() {
        let now = at(15, 0);
        let today = Session::start(
            Request::Until {
                hour: 16,
                minute: 30,
            },
            &now,
        )
        .unwrap();
        assert_eq!(today.ends_at(), Some(at(16, 30).timestamp_millis()));
        let tomorrow = Session::start(Request::Until { hour: 9, minute: 0 }, &now).unwrap();
        assert_eq!(
            tomorrow.ends_at(),
            Some(at(9, 0).timestamp_millis() + 86_400_000)
        );
        let same = Session::start(
            Request::Until {
                hour: 15,
                minute: 0,
            },
            &now,
        )
        .unwrap();
        assert_eq!(
            same.ends_at(),
            Some(at(15, 0).timestamp_millis() + 86_400_000)
        );
    }

    #[test]
    fn until_midnight_crosses_the_date() {
        let now = at(23, 59);
        let session = Session::start(Request::Until { hour: 0, minute: 0 }, &now).unwrap();
        assert_eq!(session.remaining_ms(now.timestamp_millis()), Some(60_000));
    }

    #[test]
    fn rejects_out_of_range_requests() {
        let now = at(15, 0);
        assert!(Session::start(Request::For { minutes: 0 }, &now).is_err());
        assert!(Session::start(
            Request::For {
                minutes: MAX_MINUTES + 1
            },
            &now
        )
        .is_err());
        assert!(Session::start(
            Request::Until {
                hour: 24,
                minute: 0
            },
            &now
        )
        .is_err());
    }

    #[test]
    fn glyph_follows_mode() {
        assert_eq!(Session::Off.glyph(0, false), Glyph::Off);
        assert_eq!(Session::Off.glyph(0, true), Glyph::Held);
        assert_eq!(
            Session::Indefinite { started_at: 0 }.glyph(0, false),
            Glyph::Full
        );
    }

    #[test]
    fn short_duration_rounds_up_to_the_minute() {
        assert_eq!(short_duration(0), "0m");
        assert_eq!(short_duration(1), "1m");
        assert_eq!(short_duration(42 * 60_000), "42m");
        assert_eq!(short_duration(65 * 60_000), "1h 05m");
        assert_eq!(short_duration(51 * 3_600_000), "2d 3h");
    }

    #[test]
    fn serializes_for_the_frontend() {
        let json = serde_json::to_value(Session::For {
            started_at: 1,
            ends_at: 2,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "mode": "for", "startedAt": 1, "endsAt": 2 })
        );
        let request: Request =
            serde_json::from_value(serde_json::json!({ "mode": "until", "hour": 9, "minute": 30 }))
                .unwrap();
        assert_eq!(
            request,
            Request::Until {
                hour: 9,
                minute: 30
            }
        );
    }
}
