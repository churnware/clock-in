//! Frame dumps for the README animations in utils/. Run through
//! `cargo test -- --ignored`, with the output path in CLOCK_IN_FRAMES and the
//! frame count in CLOCK_IN_FRAME_COUNT.

use chrono::NaiveDateTime;
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    style::{Color, Modifier},
    widgets::Paragraph,
};
use std::fmt::Write;

use super::{
    anim::{self, CAT_H, CAT_W, Mood},
    app::App,
    views,
};
use crate::config::Config;

/// Each frame as ANSI text, separated by form feeds.
fn dump(w: u16, h: u16, mut draw: impl FnMut(&mut ratatui::Frame, u64)) {
    let path = std::env::var("CLOCK_IN_FRAMES").expect("set CLOCK_IN_FRAMES");
    let count: u64 = std::env::var("CLOCK_IN_FRAME_COUNT").map_or(8, |n| n.parse().unwrap());
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    let mut out = String::new();
    for frame in 0..count {
        terminal.draw(|f| draw(f, frame)).unwrap();
        ansi(terminal.backend().buffer(), &mut out);
        out.push('\x0c');
    }
    std::fs::write(path, out).unwrap();
}

fn ansi(buf: &Buffer, out: &mut String) {
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            if let Color::Rgb(r, g, b) = cell.fg {
                write!(out, "\x1b[38;2;{r};{g};{b}m").unwrap();
            }
            if let Color::Rgb(r, g, b) = cell.bg {
                write!(out, "\x1b[48;2;{r};{g};{b}m").unwrap();
            }
            if cell.modifier.contains(Modifier::BOLD) {
                out.push_str("\x1b[1m");
            }
            out.push_str(cell.symbol());
            out.push_str("\x1b[0m");
        }
        out.push('\n');
    }
}

/// The working cat on its own. CLOCK_IN_FACE sets the time on its clock.
#[test]
#[ignore]
fn dump_cat_frames() {
    let face = std::env::var("CLOCK_IN_FACE").unwrap_or_else(|_| "09:00".into());
    dump(CAT_W as u16, CAT_H as u16, |f, frame| {
        let cat = anim::cat(Mood::Working, frame, &face);
        f.render_widget(Paragraph::new(cat.into_lines()), f.area());
    });
}

/// The whole dashboard on the default schedule with the scheduler running,
/// at CLOCK_IN_DEMO_AT ("YYYY-MM-DD HH:MM") and CLOCK_IN_COLS x CLOCK_IN_ROWS.
#[test]
#[ignore]
fn dump_dashboard_frames() {
    let var = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("set {k}"));
    let mut app = App::new(Config::default());
    app.now = NaiveDateTime::parse_from_str(&var("CLOCK_IN_DEMO_AT"), "%Y-%m-%d %H:%M").unwrap();
    app.lunch_until = None;
    app.daemon_running = true;
    let (cols, rows) = (
        var("CLOCK_IN_COLS").parse().unwrap(),
        var("CLOCK_IN_ROWS").parse().unwrap(),
    );
    dump(cols, rows, |f, frame| {
        app.frame = frame;
        views::draw(f, &app, None);
    });
}
