//! File locations, the daemon's shared state file and the log.

use anyhow::Result;
use chrono::{Local, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

use crate::schedule::Phase;

/// `CLOCK_IN_HOME` keeps every file in one folder, for portable installs and testing.
fn home_override() -> Option<PathBuf> {
    std::env::var_os("CLOCK_IN_HOME").map(PathBuf::from)
}

fn config_dir() -> PathBuf {
    home_override().unwrap_or_else(|| {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clock-in")
    })
}

fn data_dir() -> PathBuf {
    home_override().unwrap_or_else(|| {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clock-in")
    })
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn state_file() -> PathBuf {
    data_dir().join("state.json")
}

pub fn override_file() -> PathBuf {
    data_dir().join("override.json")
}

pub fn log_file() -> PathBuf {
    data_dir().join("clock-in.log")
}

/// Written by the daemon every tick; read by the TUI and `status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub pid: u32,
    pub phase: Phase,
    pub heartbeat: NaiveDateTime,
    pub last_nudge: Option<NaiveDateTime>,
    pub teams_running: bool,
}

/// Written by commands like `lunch now`; read by the daemon.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Override {
    pub lunch_until: Option<NaiveDateTime>,
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Option<T> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn write_json<T: Serialize>(path: PathBuf, value: &T) -> Result<()> {
    fs::create_dir_all(data_dir())?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, serde_json::to_string_pretty(value)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

impl State {
    pub fn load() -> Option<Self> {
        read_json(state_file())
    }

    pub fn save(&self) -> Result<()> {
        write_json(state_file(), self)
    }
}

impl Override {
    pub fn load() -> Self {
        read_json(override_file()).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        write_json(override_file(), self)
    }

    /// The lunch override, if it hasn't expired.
    pub fn active_lunch(&self, now: NaiveDateTime) -> Option<NaiveDateTime> {
        self.lunch_until.filter(|until| *until > now)
    }
}

const LOG_LIMIT: u64 = 512 * 1024;

pub fn log(msg: impl AsRef<str>) {
    let path = log_file();
    let _ = fs::create_dir_all(data_dir());
    if fs::metadata(&path).is_ok_and(|m| m.len() > LOG_LIMIT) {
        let _ = fs::rename(&path, path.with_extension("log.old"));
    }
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(
            f,
            "{} {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            msg.as_ref()
        );
    }
}
