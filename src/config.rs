//! User schedule, stored as TOML in `%APPDATA%\clock-in\config.toml`.

use anyhow::{Context, Result, bail};
use chrono::{NaiveTime, Weekday};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::paths;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(with = "hhmm")]
    pub clock_in: NaiveTime,
    #[serde(with = "hhmm")]
    pub clock_out: NaiveTime,
    #[serde(with = "hhmm")]
    pub lunch_start: NaiveTime,
    /// Length of the lunch break in minutes. 0 disables lunch.
    pub lunch_mins: u32,
    #[serde(with = "days")]
    pub days: [bool; 7],
    /// Launch Teams when the work day starts, if it isn't running.
    pub relaunch_teams: bool,
    /// Keep Teams closed after clock-out and on days off.
    pub enforce_offline: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            clock_in: t(9, 0),
            clock_out: t(17, 30),
            lunch_start: t(12, 30),
            lunch_mins: 30,
            days: [true, true, true, true, true, false, false],
            relaunch_teams: true,
            enforce_offline: true,
        }
    }
}

pub fn t(h: u32, m: u32) -> NaiveTime {
    NaiveTime::from_hms_opt(h, m, 0).expect("valid time")
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = paths::config_file();
        match fs::read_to_string(&path) {
            Ok(text) => {
                toml::from_str(&text).with_context(|| format!("reading {}", path.display()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    pub fn save(&self) -> Result<()> {
        self.validate()?;
        let path = paths::config_file();
        fs::create_dir_all(path.parent().expect("config dir"))?;
        fs::write(&path, toml::to_string_pretty(self)?)
            .with_context(|| format!("writing {}", path.display()))
    }

    pub fn validate(&self) -> Result<()> {
        if self.clock_out <= self.clock_in {
            bail!("clock-out must be after clock-in");
        }
        if self.lunch_mins > 0
            && (self.lunch_start < self.clock_in || self.lunch_end() > self.clock_out)
        {
            bail!("lunch must fit between clock-in and clock-out");
        }
        Ok(())
    }

    pub fn lunch_end(&self) -> NaiveTime {
        self.lunch_start + chrono::Duration::minutes(self.lunch_mins as i64)
    }

    pub fn works_on(&self, day: Weekday) -> bool {
        self.days[day.num_days_from_monday() as usize]
    }
}

pub const DAY_NAMES: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/// Parse `mon-fri`, `mon,wed,fri`, `sat-sun`, `mon-wed,fri`.
pub fn parse_days(spec: &str) -> Result<[bool; 7]> {
    let idx = |s: &str| {
        let s = s.trim().to_ascii_lowercase();
        DAY_NAMES
            .iter()
            .position(|d| s.starts_with(d))
            .with_context(|| format!("unknown day `{s}`"))
    };
    let mut out = [false; 7];
    for part in spec.split(',').filter(|p| !p.trim().is_empty()) {
        match part.split_once('-') {
            Some((a, b)) => {
                let (a, b) = (idx(a)?, idx(b)?);
                let mut i = a;
                loop {
                    out[i] = true;
                    if i == b {
                        break;
                    }
                    i = (i + 1) % 7;
                }
            }
            None => out[idx(part)?] = true,
        }
    }
    Ok(out)
}

pub fn parse_time(s: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M")
        .with_context(|| format!("`{s}` isn't a time like 09:00"))
}

mod hhmm {
    use chrono::NaiveTime;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(t: &NaiveTime, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&t.format("%H:%M").to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveTime, D::Error> {
        let s = String::deserialize(d)?;
        super::parse_time(&s).map_err(D::Error::custom)
    }
}

mod days {
    use super::DAY_NAMES;
    use serde::{Deserialize, Deserializer, Serializer, de::Error, ser::SerializeSeq};

    pub fn serialize<S: Serializer>(days: &[bool; 7], s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(None)?;
        for (on, name) in days.iter().zip(DAY_NAMES) {
            if *on {
                seq.serialize_element(name)?;
            }
        }
        seq.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[bool; 7], D::Error> {
        let names = Vec::<String>::deserialize(d)?;
        super::parse_days(&names.join(",")).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_toml() {
        let c = Config::default();
        let text = toml::to_string_pretty(&c).unwrap();
        assert!(text.contains("clock_in = \"09:00\""));
        assert!(text.contains("\"mon\""));
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), c);
    }

    #[test]
    fn parses_day_ranges() {
        assert_eq!(
            parse_days("mon-fri").unwrap(),
            [true, true, true, true, true, false, false]
        );
        assert_eq!(
            parse_days("sat-mon").unwrap(),
            [true, false, false, false, false, true, true]
        );
        assert_eq!(
            parse_days("tue,thu").unwrap(),
            [false, true, false, true, false, false, false]
        );
        assert!(parse_days("funday").is_err());
    }

    #[test]
    fn rejects_bad_schedules() {
        let c = Config {
            clock_out: t(8, 0),
            ..Config::default()
        };
        assert!(c.validate().is_err());

        let c = Config {
            lunch_start: t(17, 15),
            ..Config::default()
        };
        assert!(c.validate().is_err());

        let c = Config {
            lunch_start: t(3, 0),
            lunch_mins: 0,
            ..Config::default()
        };
        assert!(c.validate().is_ok());
    }
}
