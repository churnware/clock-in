//! The animated dashboard. Sprites advance at a fixed 7 fps; key presses
//! redraw immediately.

mod anim;
mod app;
mod canvas;
#[cfg(test)]
mod demo;
mod theme;
mod views;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use crate::config::Config;
use app::App;

pub fn run(debug_fps: bool) -> Result<()> {
    let config = Config::load()?;
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, App::new(config), debug_fps);
    ratatui::restore();
    result
}

fn event_loop(
    terminal: &mut ratatui::DefaultTerminal,
    mut app: App,
    debug_fps: bool,
) -> Result<()> {
    let frame_len = Duration::from_nanos(1_000_000_000 / anim::FPS);
    let mut next_frame = Instant::now() + frame_len;
    let mut ticks: VecDeque<Instant> = VecDeque::new();

    while !app.quit {
        let fps = debug_fps.then(|| match (ticks.front(), ticks.back()) {
            (Some(a), Some(b)) if ticks.len() > 1 => {
                (ticks.len() - 1) as f64 / b.duration_since(*a).as_secs_f64()
            }
            _ => 0.0,
        });
        terminal.draw(|f| views::draw(f, &app, fps))?;

        let wait = next_frame.saturating_duration_since(Instant::now());
        if event::poll(wait)? {
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => app.key(k),
                _ => {} // resize and friends just redraw
            }
        }

        let now = Instant::now();
        if now >= next_frame {
            app.tick();
            next_frame += frame_len;
            // After a stall (sleep, a slow terminal), don't race to catch up.
            if now > next_frame {
                next_frame = now + frame_len;
            }
            ticks.push_back(now);
            if ticks.len() > 15 {
                ticks.pop_front();
            }
        }
    }
    Ok(())
}
