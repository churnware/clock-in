//! Pure schedule logic: which phase of the day it is, and what comes next.

use chrono::{Datelike, Duration, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    DayOff,
    BeforeWork,
    Working,
    Lunch,
    AfterWork,
}

impl Phase {
    /// Phases where Teams should look offline.
    pub fn is_off_hours(self) -> bool {
        matches!(self, Phase::AfterWork | Phase::DayOff)
    }

    pub fn label(self) -> &'static str {
        match self {
            Phase::DayOff => "day off",
            Phase::BeforeWork => "before work",
            Phase::Working => "working",
            Phase::Lunch => "lunch",
            Phase::AfterWork => "clocked out",
        }
    }
}

/// The phase at `now`. `lunch_until` is a manual "lunch now" override.
pub fn phase_at(now: NaiveDateTime, c: &Config, lunch_until: Option<NaiveDateTime>) -> Phase {
    if !c.works_on(now.weekday()) {
        return Phase::DayOff;
    }
    let t = now.time();
    if t < c.clock_in {
        Phase::BeforeWork
    } else if t >= c.clock_out {
        Phase::AfterWork
    } else if lunch_until.is_some_and(|until| now < until)
        || (c.lunch_mins > 0 && t >= c.lunch_start && t < c.lunch_end())
    {
        Phase::Lunch
    } else {
        Phase::Working
    }
}

/// The next moment the phase changes, and the phase it changes to.
/// `None` if no working days are configured.
pub fn next_transition(
    now: NaiveDateTime,
    c: &Config,
    lunch_until: Option<NaiveDateTime>,
) -> Option<(NaiveDateTime, Phase)> {
    let current = phase_at(now, c, lunch_until);
    let mut candidates = Vec::new();
    for offset in 0..=8 {
        let date = now.date() + Duration::days(offset);
        // Every midnight can flip between a working day and a day off.
        candidates.push(date.and_hms_opt(0, 0, 0)?);
        if c.works_on(date.weekday()) {
            candidates.push(date.and_time(c.clock_in));
            candidates.push(date.and_time(c.clock_out));
            if c.lunch_mins > 0 {
                candidates.push(date.and_time(c.lunch_start));
                candidates.push(date.and_time(c.lunch_end()));
            }
        }
    }
    if let Some(until) = lunch_until {
        candidates.push(until);
    }
    candidates.sort();
    candidates
        .into_iter()
        .filter(|at| *at > now)
        .map(|at| (at, phase_at(at, c, lunch_until)))
        .find(|(_, phase)| *phase != current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    // 2026-11-16 is a Monday.
    fn at(day: u32, h: u32, m: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 11, day)
            .unwrap()
            .and_hms_opt(h, m, 0)
            .unwrap()
    }

    #[test]
    fn phases_across_a_day() {
        let c = Config::default();
        assert_eq!(phase_at(at(16, 8, 59), &c, None), Phase::BeforeWork);
        assert_eq!(phase_at(at(16, 9, 0), &c, None), Phase::Working);
        assert_eq!(phase_at(at(16, 12, 29), &c, None), Phase::Working);
        assert_eq!(phase_at(at(16, 12, 30), &c, None), Phase::Lunch);
        assert_eq!(phase_at(at(16, 12, 59), &c, None), Phase::Lunch);
        assert_eq!(phase_at(at(16, 13, 0), &c, None), Phase::Working);
        assert_eq!(phase_at(at(16, 17, 29), &c, None), Phase::Working);
        assert_eq!(phase_at(at(16, 17, 30), &c, None), Phase::AfterWork);
        assert_eq!(phase_at(at(21, 10, 0), &c, None), Phase::DayOff); // Saturday
    }

    #[test]
    fn lunch_override() {
        let c = Config::default();
        let until = at(16, 10, 30);
        assert_eq!(phase_at(at(16, 10, 0), &c, Some(until)), Phase::Lunch);
        assert_eq!(phase_at(at(16, 10, 30), &c, Some(until)), Phase::Working);
        assert_eq!(
            next_transition(at(16, 10, 0), &c, Some(until)),
            Some((until, Phase::Working))
        );
    }

    #[test]
    fn next_transitions() {
        let c = Config::default();
        assert_eq!(
            next_transition(at(16, 7, 0), &c, None),
            Some((at(16, 9, 0), Phase::Working))
        );
        assert_eq!(
            next_transition(at(16, 9, 0), &c, None),
            Some((at(16, 12, 30), Phase::Lunch))
        );
        assert_eq!(
            next_transition(at(16, 12, 45), &c, None),
            Some((at(16, 13, 0), Phase::Working))
        );
        assert_eq!(
            next_transition(at(16, 15, 0), &c, None),
            Some((at(16, 17, 30), Phase::AfterWork))
        );
        // Friday evening -> Saturday becomes a day off at midnight.
        assert_eq!(
            next_transition(at(20, 18, 0), &c, None),
            Some((at(21, 0, 0), Phase::DayOff))
        );
        // Saturday -> Monday at midnight.
        assert_eq!(
            next_transition(at(21, 10, 0), &c, None),
            Some((at(23, 0, 0), Phase::BeforeWork))
        );
    }

    #[test]
    fn no_lunch() {
        let c = Config {
            lunch_mins: 0,
            ..Config::default()
        };
        assert_eq!(phase_at(at(16, 12, 40), &c, None), Phase::Working);
    }

    #[test]
    fn no_days() {
        let c = Config {
            days: [false; 7],
            ..Config::default()
        };
        assert_eq!(next_transition(at(16, 10, 0), &c, None), None);
    }
}
