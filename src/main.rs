#[cfg(not(windows))]
compile_error!("clock-in drives the Windows Teams client and only builds on Windows");

mod config;
mod daemon;
mod paths;
mod schedule;
mod tui;
mod win;

use anyhow::{Result, bail};
use chrono::{Duration, Local, NaiveDateTime};
use clap::{Parser, Subcommand, ValueEnum};

use config::Config;
use paths::{Override, State};
use schedule::{Phase, next_transition, phase_at};
use win::sync;

#[derive(Parser)]
#[command(
    name = "clock-in",
    version,
    about = "Set when Microsoft Teams shows you online."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
    /// Show a frame-rate overlay in the dashboard.
    #[arg(long, hide = true)]
    debug_fps: bool,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start the background scheduler.
    Start,
    /// Stop the background scheduler.
    Stop,
    /// Print the current status on one line.
    Status,
    /// Change the schedule.
    Set {
        /// Clock-in time, e.g. 09:00.
        #[arg(long = "in", value_name = "HH:MM")]
        clock_in: Option<String>,
        /// Clock-out time, e.g. 17:30.
        #[arg(long = "out", value_name = "HH:MM")]
        clock_out: Option<String>,
        /// Lunch start time, e.g. 12:30.
        #[arg(long, value_name = "HH:MM")]
        lunch: Option<String>,
        /// Lunch length in minutes (0 for no lunch).
        #[arg(long, value_name = "MINS")]
        lunch_mins: Option<u32>,
        /// Working days, e.g. mon-fri or mon,wed,fri.
        #[arg(long)]
        days: Option<String>,
        /// Launch Teams at clock-in.
        #[arg(long, value_name = "BOOL")]
        relaunch_teams: Option<bool>,
        /// Keep Teams closed after hours.
        #[arg(long, value_name = "BOOL")]
        enforce_offline: Option<bool>,
    },
    /// Start or end a lunch break now.
    Lunch {
        #[arg(value_enum)]
        action: LunchAction,
    },
    /// Run the scheduler in this console (used by `start`).
    #[command(hide = true)]
    Daemon,
}

#[derive(Clone, Copy, ValueEnum)]
enum LunchAction {
    Now,
    End,
}

fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("clock-in: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        None => tui::run(cli.debug_fps),
        Some(Cmd::Daemon) => daemon::run(),
        Some(Cmd::Start) => start(),
        Some(Cmd::Stop) => {
            if sync::request_stop() {
                println!("clock-in stopped");
            } else {
                println!("clock-in wasn't running");
            }
            Ok(())
        }
        Some(Cmd::Status) => {
            println!("{}", status_line());
            Ok(())
        }
        Some(Cmd::Set {
            clock_in,
            clock_out,
            lunch,
            lunch_mins,
            days,
            relaunch_teams,
            enforce_offline,
        }) => {
            let mut c = Config::load()?;
            if let Some(v) = clock_in {
                c.clock_in = config::parse_time(&v)?;
            }
            if let Some(v) = clock_out {
                c.clock_out = config::parse_time(&v)?;
            }
            if let Some(v) = lunch {
                c.lunch_start = config::parse_time(&v)?;
            }
            if let Some(v) = lunch_mins {
                c.lunch_mins = v;
            }
            if let Some(v) = days {
                c.days = config::parse_days(&v)?;
            }
            if let Some(v) = relaunch_teams {
                c.relaunch_teams = v;
            }
            if let Some(v) = enforce_offline {
                c.enforce_offline = v;
            }
            c.save()?;
            println!("{}", summary(&c));
            Ok(())
        }
        Some(Cmd::Lunch { action }) => {
            let c = Config::load()?;
            let now = Local::now().naive_local();
            let ov = match action {
                LunchAction::Now => {
                    if phase_at(now, &c, None) != Phase::Working {
                        bail!("lunch only makes sense during work hours");
                    }
                    let mins = if c.lunch_mins == 0 { 30 } else { c.lunch_mins };
                    let until = now + Duration::minutes(mins as i64);
                    println!("lunch until {}", until.format("%H:%M"));
                    Override {
                        lunch_until: Some(until),
                    }
                }
                LunchAction::End => {
                    println!("back to work");
                    Override { lunch_until: None }
                }
            };
            ov.save()
        }
    }
}

pub fn start() -> Result<()> {
    if sync::daemon_running() {
        println!("clock-in is already running");
        return Ok(());
    }
    Config::load()?.validate()?;
    sync::spawn_detached()?;
    for _ in 0..20 {
        if sync::daemon_running() {
            println!("clock-in started · {}", status_line());
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    bail!(
        "the scheduler didn't start; see {}",
        paths::log_file().display()
    )
}

pub fn summary(c: &Config) -> String {
    let days: Vec<_> = config::DAY_NAMES
        .iter()
        .zip(c.days)
        .filter_map(|(d, on)| on.then_some(*d))
        .collect();
    let lunch = if c.lunch_mins == 0 {
        "no lunch".to_string()
    } else {
        format!(
            "lunch {} for {}m",
            c.lunch_start.format("%H:%M"),
            c.lunch_mins
        )
    };
    format!(
        "{}–{} · {} · {}",
        c.clock_in.format("%H:%M"),
        c.clock_out.format("%H:%M"),
        lunch,
        days.join(" ")
    )
}

pub fn until(now: NaiveDateTime, at: NaiveDateTime) -> String {
    let mins = (at - now).num_minutes().max(0);
    match (mins / 60, mins % 60) {
        (0, m) => format!("{m}m"),
        (h, 0) if h < 48 => format!("{h}h"),
        (h, m) if h < 48 => format!("{h}h{m:02}m"),
        (h, _) => format!("{}d", h / 24),
    }
}

fn status_line() -> String {
    let c = Config::load().unwrap_or_default();
    let now = Local::now().naive_local();
    let lunch = Override::load().active_lunch(now);
    let phase = phase_at(now, &c, lunch);
    let badge = match phase {
        Phase::Working => "● online",
        Phase::Lunch => "◐ lunch",
        _ => "○ offline",
    };
    let next = match next_transition(now, &c, lunch) {
        Some((at, p)) => format!(
            " · {} at {} (in {})",
            p.label(),
            at.format("%H:%M"),
            until(now, at)
        ),
        None => String::new(),
    };
    let daemon = if sync::daemon_running() {
        match State::load() {
            Some(s) => format!("running (pid {})", s.pid),
            None => "running".into(),
        }
    } else {
        "not running".into()
    };
    let label = match phase {
        Phase::Lunch => String::new(),
        p => format!(" · {}", p.label()),
    };
    format!("{badge}{label}{next} · scheduler {daemon}")
}
