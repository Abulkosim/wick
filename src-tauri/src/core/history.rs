use chrono::{DateTime, Days, NaiveDate, TimeZone};
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::{collections::BTreeMap, fs, io::Read, path::Path};

pub const FILE: &str = "history.json";
const VERSION: u64 = 1;
const MAX_BYTES: u64 = 64 * 1024;
pub const KEEP_DAYS: u64 = 30;
pub const SHOWN_DAYS: u64 = 7;
const DAY_MS: u64 = 86_400_000;
/// Windows keeps counting its monotonic clock through sleep, so a long silence between ticks
/// (which come every 5 s while awake) is sleep there.
const WINDOWS_GAP_MS: i64 = 120_000;

/// Milliseconds Wick held the Mac awake, per local calendar day.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct History {
    days: BTreeMap<NaiveDate, u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Day {
    /// "2026-10-26", local time.
    pub date: String,
    pub minutes: u32,
}

fn local_date<Tz: TimeZone>(ms: i64, tz: &Tz) -> Option<NaiveDate> {
    DateTime::from_timestamp_millis(ms).map(|time| time.with_timezone(tz).date_naive())
}

/// The first instant of `date`, even where midnight is skipped by a DST change.
fn day_start<Tz: TimeZone>(date: NaiveDate, tz: &Tz) -> Option<i64> {
    (0..3)
        .filter_map(|hour| date.and_hms_opt(hour, 0, 0))
        .find_map(|time| tz.from_local_datetime(&time).earliest())
        .map(|time| time.timestamp_millis())
}

impl History {
    /// Adds the span from `start` to `end`, split at local midnights.
    pub fn add<Tz: TimeZone>(&mut self, start: i64, end: i64, tz: &Tz) {
        let mut cursor = start;
        while cursor < end {
            let Some(date) = local_date(cursor, tz) else {
                return;
            };
            let next = date
                .checked_add_days(Days::new(1))
                .and_then(|next| day_start(next, tz))
                .filter(|next| *next > cursor)
                .unwrap_or(end)
                .min(end);
            *self.days.entry(date).or_default() += (next - cursor) as u64;
            cursor = next;
        }
    }

    /// Drops days older than the last `KEEP_DAYS`, today included.
    pub fn prune(&mut self, today: NaiveDate) {
        if let Some(oldest) = today.checked_sub_days(Days::new(KEEP_DAYS - 1)) {
            self.days.retain(|date, _| *date >= oldest);
        }
    }

    /// The last `count` days ending today, oldest first, with empty days filled in.
    pub fn recent(&self, today: NaiveDate, count: u64) -> Vec<Day> {
        (0..count)
            .rev()
            .filter_map(|back| today.checked_sub_days(Days::new(back)))
            .map(|date| Day {
                date: date.format("%Y-%m-%d").to_string(),
                minutes: (self.days.get(&date).copied().unwrap_or(0) / 60_000) as u32,
            })
            .collect()
    }

    pub fn to_json(&self) -> Value {
        let days: Map<String, Value> = self
            .days
            .iter()
            .map(|(date, ms)| (date.format("%Y-%m-%d").to_string(), json!(ms)))
            .collect();
        json!({ "version": VERSION, "days": days })
    }

    /// Keeps every readable day; an unreadable file is an error so the caller can set it aside.
    pub fn from_json(value: Value) -> Result<Self, String> {
        let damaged = || "Wick's history file is damaged or unsupported.".to_string();
        let Value::Object(map) = value else {
            return Err(damaged());
        };
        if map.get("version").and_then(Value::as_u64) != Some(VERSION) {
            return Err(damaged());
        }
        let Some(Value::Object(days)) = map.get("days") else {
            return Err(damaged());
        };
        Ok(Self {
            days: days
                .iter()
                .filter_map(|(date, ms)| {
                    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
                    let ms = ms.as_u64().filter(|ms| *ms <= DAY_MS + 3_600_000)?;
                    Some((date, ms))
                })
                .collect(),
        })
    }
}

/// A missing file is an empty history. A damaged one is moved aside, never overwritten.
pub fn load(path: &Path) -> History {
    let read = || -> Result<Option<Value>, String> {
        let mut file = match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let mut bytes = Vec::new();
        file.by_ref()
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("too large".into());
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| error.to_string())
    };
    match read().and_then(|value| value.map(History::from_json).transpose()) {
        Ok(history) => history.unwrap_or_default(),
        Err(_) => {
            if let Ok(target) = super::store::backup_path(path) {
                let _ = fs::rename(path, target);
            }
            History::default()
        }
    }
}

pub fn save(path: &Path, history: &History) -> Result<(), String> {
    super::store::write_json(path, &history.to_json())
}

pub fn remove(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err("Wick could not clear its history.".into())
        }
        _ => Ok(()),
    }
}

/// Counts the time Wick holds the Mac awake. The wall clock places it on a day; the monotonic
/// clock, which stops while a Mac sleeps, keeps sleep out.
#[derive(Debug, Default)]
pub struct Tracker {
    mark: Option<(i64, u64)>,
}

impl Tracker {
    /// Moves the mark to now and returns the awake span since the last call, if any.
    pub fn advance(&mut self, holding: bool, wall: i64, monotonic: u64) -> Option<(i64, i64)> {
        let span = self.mark.and_then(|(since, since_monotonic)| {
            let counted = (wall - since)
                .max(0)
                .min(monotonic.saturating_sub(since_monotonic) as i64);
            let slept = cfg!(windows) && counted > WINDOWS_GAP_MS;
            (counted > 0 && !slept).then_some((wall - counted, wall))
        });
        self.mark = holding.then_some((wall, monotonic));
        span
    }
}

/// The local calendar day at `ms`.
pub fn local_day(ms: i64) -> NaiveDate {
    local_date(ms, &chrono::Local).unwrap_or_default()
}

/// Milliseconds on a clock that stops while the computer sleeps (on macOS).
pub fn monotonic_ms() -> u64 {
    static BASE: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    BASE.get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").unwrap()
    }

    fn at(tz: &FixedOffset, value: &str) -> i64 {
        tz.from_local_datetime(
            &chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M").unwrap(),
        )
        .unwrap()
        .timestamp_millis()
    }

    #[test]
    fn spans_split_at_local_midnight() {
        let tz = FixedOffset::east_opt(5 * 3600).unwrap();
        let mut history = History::default();
        history.add(
            at(&tz, "2026-10-25 23:30"),
            at(&tz, "2026-10-26 01:15"),
            &tz,
        );
        history.add(
            at(&tz, "2026-10-26 09:00"),
            at(&tz, "2026-10-26 09:01"),
            &tz,
        );
        let days = history.recent(date("2026-10-26"), 3);
        assert_eq!(
            days,
            [
                Day {
                    date: "2026-10-24".into(),
                    minutes: 0
                },
                Day {
                    date: "2026-10-25".into(),
                    minutes: 30
                },
                Day {
                    date: "2026-10-26".into(),
                    minutes: 76
                },
            ]
        );
    }

    #[test]
    fn prune_keeps_thirty_days() {
        let tz = FixedOffset::east_opt(0).unwrap();
        let mut history = History::default();
        for value in ["2026-09-26 10:00", "2026-09-27 10:00", "2026-10-26 10:00"] {
            history.add(at(&tz, value), at(&tz, value) + 60_000, &tz);
        }
        history.prune(date("2026-10-26"));
        assert_eq!(
            history.days.keys().copied().collect::<Vec<_>>(),
            [date("2026-09-27"), date("2026-10-26")]
        );
    }

    #[test]
    fn json_round_trip_keeps_readable_days() {
        let tz = FixedOffset::east_opt(0).unwrap();
        let mut history = History::default();
        history.add(
            at(&tz, "2026-10-26 10:00"),
            at(&tz, "2026-10-26 12:00"),
            &tz,
        );
        assert_eq!(History::from_json(history.to_json()).unwrap(), history);
        let partial = History::from_json(json!({
            "version": 1,
            "days": { "2026-10-26": 60000, "yesterday": 5, "2026-10-25": -1, "2026-10-24": 999999999999u64 }
        }))
        .unwrap();
        assert_eq!(partial.recent(date("2026-10-26"), 1)[0].minutes, 1);
        assert_eq!(partial.days.len(), 1);
        assert!(History::from_json(json!({ "version": 2, "days": {} })).is_err());
        assert!(History::from_json(json!([])).is_err());
    }

    #[test]
    fn damaged_files_are_set_aside() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(FILE);
        assert_eq!(load(&path), History::default());
        fs::write(&path, "{ damaged").unwrap();
        assert_eq!(load(&path), History::default());
        assert!(!path.exists());
        let kept: Vec<_> = fs::read_dir(directory.path())
            .unwrap()
            .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect();
        assert_eq!(kept, ["{ damaged"]);
        let tz = FixedOffset::east_opt(0).unwrap();
        let mut history = History::default();
        history.add(0, 90_000, &tz);
        save(&path, &history).unwrap();
        assert_eq!(load(&path), history);
        remove(&path).unwrap();
        remove(&path).unwrap();
        assert_eq!(load(&path), History::default());
    }

    #[test]
    fn tracker_counts_held_time_and_skips_sleep() {
        let mut tracker = Tracker::default();
        assert_eq!(tracker.advance(true, 1_000, 10), None);
        assert_eq!(tracker.advance(true, 6_000, 5_010), Some((1_000, 6_000)));
        // Slept for an hour: the wall clock moved, the monotonic clock barely did.
        assert_eq!(
            tracker.advance(true, 3_606_000, 8_010),
            Some((3_603_000, 3_606_000))
        );
        assert_eq!(
            tracker.advance(false, 3_611_000, 13_010),
            Some((3_606_000, 3_611_000))
        );
        assert_eq!(tracker.advance(false, 3_700_000, 99_000), None);
        assert_eq!(tracker.advance(true, 3_800_000, 100_000), None);
        // A clock set back counts nothing.
        assert_eq!(tracker.advance(true, 3_000_000, 105_000), None);
    }
}
