use serde::{Deserialize, Serialize};

/// Days are 0 = Monday … 6 = Sunday. Times are "HH:MM" in local time. An end at or before the
/// start runs overnight into the next day, and belongs to the day it starts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schedule {
    pub days: Vec<u8>,
    pub start: String,
    pub end: String,
}

/// Minutes since midnight for "HH:MM".
pub fn minutes(value: &str) -> Option<u32> {
    let (hours, minutes) = value.split_once(':')?;
    if hours.is_empty() || hours.len() > 2 || minutes.len() != 2 {
        return None;
    }
    let (hours, minutes): (u32, u32) = (hours.parse().ok()?, minutes.parse().ok()?);
    (hours < 24 && minutes < 60).then_some(hours * 60 + minutes)
}

impl Schedule {
    pub fn validate(&self) -> Result<(), String> {
        let days_ok = !self.days.is_empty()
            && self.days.len() <= 7
            && self.days.iter().all(|day| *day < 7)
            && self
                .days
                .iter()
                .enumerate()
                .all(|(index, day)| !self.days[..index].contains(day));
        if !days_ok {
            return Err("Choose at least one day.".into());
        }
        match (minutes(&self.start), minutes(&self.end)) {
            (Some(start), Some(end)) if start != end => Ok(()),
            (Some(_), Some(_)) => Err("Start and end can't be the same time.".into()),
            _ => Err("Use times like 09:00.".into()),
        }
    }

    /// Whether `weekday` (0 = Monday) at `minute` past midnight falls inside the schedule.
    pub fn contains(&self, weekday: u8, minute: u32) -> bool {
        let (Some(start), Some(end)) = (minutes(&self.start), minutes(&self.end)) else {
            return false;
        };
        let today = self.days.contains(&weekday);
        if start < end {
            return today && (start..end).contains(&minute);
        }
        let yesterday = self.days.contains(&((weekday + 6) % 7));
        (today && minute >= start) || (yesterday && minute < end)
    }

    /// "Weekdays", "Every day", "Mon, Wed, Fri".
    pub fn days_label(&self) -> String {
        let mut days = self.days.clone();
        days.sort_unstable();
        match days.as_slice() {
            [0, 1, 2, 3, 4, 5, 6] => "Every day".into(),
            [0, 1, 2, 3, 4] => "Weekdays".into(),
            [5, 6] => "Weekends".into(),
            _ => days
                .iter()
                .map(|day| ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][*day as usize])
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schedule(days: &[u8], start: &str, end: &str) -> Schedule {
        Schedule {
            days: days.to_vec(),
            start: start.into(),
            end: end.into(),
        }
    }

    #[test]
    fn parses_clock_times() {
        assert_eq!(minutes("09:00"), Some(540));
        assert_eq!(minutes("9:05"), Some(545));
        assert_eq!(minutes("23:59"), Some(1439));
        assert_eq!(minutes("24:00"), None);
        assert_eq!(minutes("9"), None);
        assert_eq!(minutes("09:5"), None);
    }

    #[test]
    fn daytime_window_on_chosen_days() {
        let work = schedule(&[0, 1, 2, 3, 4], "09:00", "18:00");
        assert!(work.contains(0, 540));
        assert!(work.contains(4, 1079));
        assert!(!work.contains(4, 1080));
        assert!(!work.contains(2, 539));
        assert!(!work.contains(5, 600));
    }

    #[test]
    fn overnight_window_belongs_to_the_start_day() {
        let nights = schedule(&[4], "22:00", "06:00");
        assert!(nights.contains(4, 22 * 60));
        assert!(nights.contains(5, 5 * 60 + 59));
        assert!(!nights.contains(5, 6 * 60));
        assert!(!nights.contains(4, 5 * 60));
        let sunday = schedule(&[6], "23:00", "01:00");
        assert!(sunday.contains(0, 30));
    }

    #[test]
    fn validation_and_labels() {
        assert!(schedule(&[0], "09:00", "18:00").validate().is_ok());
        assert!(schedule(&[], "09:00", "18:00").validate().is_err());
        assert!(schedule(&[7], "09:00", "18:00").validate().is_err());
        assert!(schedule(&[1, 1], "09:00", "18:00").validate().is_err());
        assert!(schedule(&[1], "09:00", "09:00").validate().is_err());
        assert!(schedule(&[1], "9am", "18:00").validate().is_err());
        assert_eq!(
            schedule(&[4, 0, 1, 2, 3], "09:00", "18:00").days_label(),
            "Weekdays"
        );
        assert_eq!(
            schedule(&[0, 1, 2, 3, 4, 5, 6], "09:00", "18:00").days_label(),
            "Every day"
        );
        assert_eq!(schedule(&[5, 6], "09:00", "18:00").days_label(), "Weekends");
        assert_eq!(schedule(&[0, 2], "09:00", "18:00").days_label(), "Mon, Wed");
    }
}
