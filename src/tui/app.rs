//! Dashboard state and key handling.

use chrono::{Duration, Local, NaiveDateTime, NaiveTime};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::anim::{self, BookAnim, FPS};
use crate::{
    config::Config,
    paths::{Override, State},
    schedule::{Phase, phase_at},
    win::sync,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Field {
    In,
    Lunch,
    LunchLen,
    Out,
    Days,
    Relaunch,
    Enforce,
}

pub const FIELDS: [Field; 7] = [
    Field::In,
    Field::Lunch,
    Field::LunchLen,
    Field::Out,
    Field::Days,
    Field::Relaunch,
    Field::Enforce,
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Info,
    Good,
    Bad,
}

pub struct Toast {
    pub text: String,
    pub tone: Tone,
    pub until: u64,
}

pub struct App {
    pub saved: Config,
    pub draft: Config,
    pub field: usize,
    pub day_cursor: usize,
    pub frame: u64,
    pub books: [Option<BookAnim>; 7],
    pub toast: Option<Toast>,
    pub now: NaiveDateTime,
    pub lunch_until: Option<NaiveDateTime>,
    pub daemon: Option<State>,
    pub daemon_running: bool,
    pub quit: bool,
    confirm_quit: bool,
    seed: u64,
}

impl App {
    pub fn new(config: Config) -> Self {
        let mut app = Self {
            saved: config.clone(),
            draft: config,
            field: 0,
            day_cursor: 0,
            frame: 0,
            books: [None; 7],
            toast: None,
            now: now(),
            lunch_until: None,
            daemon: None,
            daemon_running: false,
            quit: false,
            confirm_quit: false,
            seed: 0x2545_F491_4F6C_DD1D,
        };
        app.refresh();
        app
    }

    pub fn phase(&self) -> Phase {
        phase_at(self.now, &self.saved, self.lunch_until)
    }

    pub fn dirty(&self) -> bool {
        self.saved != self.draft
    }

    pub fn selected(&self) -> Field {
        FIELDS[self.field]
    }

    /// Called once per animation frame.
    pub fn tick(&mut self) {
        self.frame += 1;
        if self.frame.is_multiple_of(FPS) {
            self.refresh();
        }
        if self.toast.as_ref().is_some_and(|t| self.frame >= t.until) {
            self.toast = None;
        }
        for slot in &mut self.books {
            if slot.is_some_and(|a| a.at(self.frame).is_none()) {
                *slot = None;
            }
        }
    }

    fn refresh(&mut self) {
        self.now = now();
        self.lunch_until = Override::load().active_lunch(self.now);
        self.daemon_running = sync::daemon_running();
        self.daemon = if self.daemon_running {
            State::load()
        } else {
            None
        };
    }

    fn toast(&mut self, text: impl Into<String>, tone: Tone) {
        self.toast = Some(Toast {
            text: text.into(),
            tone,
            until: self.frame + FPS * 3,
        });
    }

    fn rand(&mut self) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }

    fn animate(&mut self, book: usize, seq: anim::Seq) {
        self.books[book] = Some(BookAnim {
            seq,
            start: self.frame,
        });
    }

    /// A standing, idle book, picked at random.
    fn idle_standing_book(&mut self) -> Option<usize> {
        let idle: Vec<usize> = (0..7)
            .filter(|&i| self.draft.days[i] && self.books[i].is_none())
            .collect();
        if idle.is_empty() {
            return None;
        }
        let pick = self.rand() as usize % idle.len();
        Some(idle[pick])
    }

    pub fn key(&mut self, k: KeyEvent) {
        let quit_pending = std::mem::take(&mut self.confirm_quit);
        let shift = k.modifiers.contains(KeyModifiers::SHIFT);

        // Every keypress jostles the shelf a little.
        if let Some(i) = self.idle_standing_book() {
            self.animate(i, anim::WOBBLE);
        }

        match k.code {
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Char('q') => {
                if self.dirty() && !quit_pending {
                    self.confirm_quit = true;
                    self.toast("unsaved changes · q again to quit, s to save", Tone::Bad);
                } else {
                    self.quit = true;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.field = (self.field + FIELDS.len() - 1) % FIELDS.len()
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                self.field = (self.field + 1) % FIELDS.len()
            }
            KeyCode::Left | KeyCode::Char('-') => self.adjust(-1, shift),
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => self.adjust(1, shift),
            KeyCode::Char(' ') => match self.selected() {
                Field::Days => self.toggle_day(self.day_cursor),
                Field::Relaunch | Field::Enforce => self.adjust(1, false),
                _ => {}
            },
            KeyCode::Char(c @ '1'..='7') => self.toggle_day(c as usize - '1' as usize),
            KeyCode::Enter | KeyCode::Char('s') => {
                self.save();
            }
            KeyCode::Esc => {
                if self.dirty() {
                    self.draft = self.saved.clone();
                    self.toast("changes discarded", Tone::Info);
                }
            }
            KeyCode::Char('g') => self.start(),
            KeyCode::Char('x') => self.stop(),
            KeyCode::Char('l') => self.lunch(),
            _ => {}
        }
    }

    fn adjust(&mut self, dir: i64, big: bool) {
        let step = Duration::minutes(if big { 60 } else { 5 } * dir);
        let bump = |t: &mut NaiveTime| *t = t.overflowing_add_signed(step).0;
        match self.selected() {
            Field::In => bump(&mut self.draft.clock_in),
            Field::Out => bump(&mut self.draft.clock_out),
            Field::Lunch => bump(&mut self.draft.lunch_start),
            Field::LunchLen => {
                let m = self.draft.lunch_mins as i64 + if big { 30 } else { 5 } * dir;
                self.draft.lunch_mins = m.clamp(0, 180) as u32;
            }
            Field::Days => self.day_cursor = (self.day_cursor as i64 + dir).rem_euclid(7) as usize,
            Field::Relaunch => self.draft.relaunch_teams = !self.draft.relaunch_teams,
            Field::Enforce => self.draft.enforce_offline = !self.draft.enforce_offline,
        }
    }

    fn toggle_day(&mut self, i: usize) {
        self.day_cursor = i;
        self.draft.days[i] = !self.draft.days[i];
        self.animate(
            i,
            if self.draft.days[i] {
                anim::RISE
            } else {
                anim::FALL
            },
        );
    }

    fn save(&mut self) -> bool {
        if let Err(e) = self.draft.validate() {
            for i in 0..7 {
                if self.draft.days[i] {
                    self.animate(i, anim::WOBBLE);
                }
            }
            self.toast(e.to_string(), Tone::Bad);
            return false;
        }
        if !self.dirty() {
            self.toast("nothing to save", Tone::Info);
            return true;
        }
        match self.draft.save() {
            Ok(()) => {
                self.saved = self.draft.clone();
                if let Some(i) = self.idle_standing_book() {
                    self.animate(i, anim::TOPPLE);
                }
                self.toast("saved", Tone::Good);
                true
            }
            Err(e) => {
                self.toast(format!("{e:#}"), Tone::Bad);
                false
            }
        }
    }

    fn start(&mut self) {
        if self.dirty() && !self.save() {
            return;
        }
        if sync::daemon_running() {
            self.toast("the scheduler is already running", Tone::Info);
            return;
        }
        match sync::spawn_detached() {
            Ok(_) => {
                if let Some(i) = self.idle_standing_book() {
                    self.animate(i, anim::TOPPLE);
                }
                self.toast("scheduler started · you can close this window", Tone::Good);
            }
            Err(e) => self.toast(format!("couldn't start: {e}"), Tone::Bad),
        }
        self.refresh();
    }

    fn stop(&mut self) {
        if sync::request_stop() {
            self.toast("scheduler stopped", Tone::Info);
        } else {
            self.toast("the scheduler wasn't running", Tone::Info);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        self.refresh();
    }

    fn lunch(&mut self) {
        self.refresh();
        let ov = match self.phase() {
            Phase::Lunch if self.lunch_until.is_some() => {
                self.toast("back to work", Tone::Info);
                Override { lunch_until: None }
            }
            Phase::Working => {
                let mins = if self.saved.lunch_mins == 0 {
                    30
                } else {
                    self.saved.lunch_mins
                };
                let until = self.now + Duration::minutes(mins as i64);
                self.toast(format!("lunch until {}", until.format("%H:%M")), Tone::Good);
                Override {
                    lunch_until: Some(until),
                }
            }
            _ => {
                self.toast("lunch only works during work hours", Tone::Info);
                return;
            }
        };
        if let Err(e) = ov.save() {
            self.toast(format!("{e:#}"), Tone::Bad);
        }
        self.refresh();
    }
}

/// The current time. `CLOCK_IN_DEMO_TIME=HH:MM` pins the dashboard's clock,
/// for screenshots.
fn now() -> NaiveDateTime {
    let real = Local::now().naive_local();
    std::env::var("CLOCK_IN_DEMO_TIME")
        .ok()
        .and_then(|t| crate::config::parse_time(&t).ok())
        .map_or(real, |t| real.date().and_time(t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;

    fn press(app: &mut App, code: KeyCode) {
        app.key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn toggling_a_day_knocks_its_book_over() {
        let mut app = App::new(Config::default());
        press(&mut app, KeyCode::Char('3')); // Wednesday off
        assert!(!app.draft.days[2]);
        assert_eq!(app.books[2].unwrap().seq, anim::FALL);
        for _ in 0..anim::FALL.len() {
            app.tick();
        }
        assert!(app.books[2].is_none(), "book comes to rest");
        press(&mut app, KeyCode::Char('3'));
        assert_eq!(app.books[2].unwrap().seq, anim::RISE);
        assert!(!app.dirty());
    }

    #[test]
    fn adjusting_times_and_quitting_with_unsaved_changes() {
        let mut app = App::new(Config::default());
        press(&mut app, KeyCode::Right);
        assert_eq!(app.draft.clock_in, crate::config::t(9, 5));
        press(&mut app, KeyCode::Char('q'));
        assert!(!app.quit, "first q warns about unsaved changes");
        press(&mut app, KeyCode::Char('q'));
        assert!(app.quit);
    }
}
