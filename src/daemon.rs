//! The background loop: keep Teams active during work hours, let it idle at
//! lunch, and close it at clock-out.

use anyhow::{Context, Result, bail};
use chrono::Local;
use std::{fs, time::Duration};

use crate::{
    config::Config,
    paths::{self, Override, State, log},
    schedule::{Phase, phase_at},
    win::{idle, sync, teams},
};

const TICK: Duration = Duration::from_secs(15);
/// Nudge once the user has been idle this long. Teams goes Away at 5 minutes.
const NUDGE_AFTER: Duration = Duration::from_secs(60);

pub fn run() -> Result<()> {
    let Some(_lock) = sync::claim_daemon() else {
        bail!("clock-in is already running");
    };
    let stop = sync::StopEvent::create().context("creating stop event")?;
    log(format!("daemon started (pid {})", std::process::id()));

    let mut config = Config::load().unwrap_or_default();
    let mut prev: Option<Phase> = None;
    let mut last_nudge = None;

    loop {
        // Pick up edits made from the TUI or `clock-in set`.
        match Config::load() {
            Ok(c) => config = c,
            Err(e) => log(format!("keeping previous config: {e:#}")),
        }
        let now = Local::now().naive_local();
        let lunch = Override::load().active_lunch(now);
        let phase = phase_at(now, &config, lunch);

        if prev != Some(phase) {
            log(format!("phase: {}", phase.label()));
            on_enter(phase, prev, &config);
            prev = Some(phase);
        }

        match phase {
            Phase::Working if idle::idle_for() >= NUDGE_AFTER => {
                if idle::nudge() {
                    last_nudge = Some(now);
                }
            }
            p if p.is_off_hours() && config.enforce_offline && teams::is_running() => {
                log("teams reopened after hours; closing it");
                teams::quit();
            }
            _ => {}
        }

        let state = State {
            pid: std::process::id(),
            phase,
            heartbeat: now,
            last_nudge,
            teams_running: teams::is_running(),
        };
        if let Err(e) = state.save() {
            log(format!("writing state: {e:#}"));
        }

        if stop.wait(TICK) {
            break;
        }
    }

    let _ = fs::remove_file(paths::state_file());
    log("daemon stopped");
    Ok(())
}

fn on_enter(phase: Phase, prev: Option<Phase>, config: &Config) {
    match phase {
        // Start of the day, or the daemon was started mid-shift.
        Phase::Working if prev != Some(Phase::Lunch) => {
            if config.relaunch_teams && !teams::is_running() {
                log("launching teams");
                if let Err(e) = teams::launch() {
                    log(format!("launching teams failed: {e}"));
                }
            }
        }
        // The moment of clock-out. Starting the daemon after hours only
        // closes Teams when enforce_offline is on (handled by the loop).
        Phase::AfterWork if prev.is_some() => {
            let n = teams::quit();
            log(format!("clocked out; closed {n} teams process(es)"));
        }
        _ => {}
    }
}
