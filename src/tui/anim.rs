//! Sprites, drawn at 7 fps: the lucky cat clock and the shelf of day books.

use super::canvas::Canvas;
use super::theme::{self, Rgb};

pub const FPS: u64 = 7;

// ── Cat ──────────────────────────────────────────────────────────────────

pub const CAT_W: usize = 24;
pub const CAT_H: usize = 14;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mood {
    /// Paw beckons, eyes and tail tick like a pendulum clock.
    Working,
    /// Happy eyes, a steaming bowl, a lazy tail.
    Lunch,
    /// Time has stopped: everything freezes mid-swing and dims.
    Stopped,
}

const BODY: [&str; 11] = [
    r"  /\_________/\  ",
    r" /             \ ",
    r" |             | ", // eyes are drawn over this row
    r" |  =  >w<  =  | ",
    r"  \___________/  ",
    r"  /==== o ====\  ",
    r" |  .-------.  | ",
    r" |  |       |  | ", // clock face digits
    r" |  '-------'  | ",
    r" |_____________| ",
    r"  (__)     (__)  ",
];

const PAW_UP: [&str; 6] = [
    r"   __  ", r"  (  ) ", r"  |  | ", r"  |  | ", r"  /  / ", r" /  /  ",
];

const PAW_BECKON: [&str; 6] = [
    r"       ", r"   __  ", r"  (__) ", r"  |  | ", r"  /  / ", r" /  /  ",
];

const BOWL: [&str; 3] = [r"-------", r"\_____/", r"       "];
const STEAM: [&str; 2] = [r"  ) )  ", r"  ( (  "];

/// Pendulum position: -1 left, 0 centre, 1 right.
fn swing(frame: u64) -> i32 {
    const CYCLE: [i32; 8] = [-1, -1, -1, 0, 1, 1, 1, 0];
    CYCLE[(frame % 8) as usize]
}

/// Frozen mid-swing.
const STOPPED_SWING: i32 = 1;

pub fn cat(mood: Mood, frame: u64, face: &str) -> Canvas {
    let mut c = Canvas::new(CAT_W, CAT_H);
    let off = mood == Mood::Stopped;
    let tone = |rgb: Rgb| if off { theme::dim(rgb) } else { rgb };

    let pos = match mood {
        Mood::Working => swing(frame),
        Mood::Lunch => swing(frame / 2),
        Mood::Stopped => STOPPED_SWING,
    };

    // Body, shaded top to bottom like the Teams icon.
    for (y, row) in BODY.iter().enumerate() {
        let t = y as f32 / (BODY.len() - 1) as f32;
        let shade = theme::lerp(theme::SKY, theme::INDIGO, t);
        c.sprite(0, y as i32, row, theme::fg(tone(shade)));
    }
    c.text(7, 3, ">w<", theme::fg(tone(theme::LAVENDER)));
    c.text(8, 5, "o", theme::bold(tone(theme::LAVENDER)));

    // Eyes.
    let eye = match mood {
        Mood::Lunch => "( ^ )".to_string(),
        _ => {
            let mut s = ['(', ' ', ' ', ' ', ')'];
            s[(2 + pos) as usize] = '•';
            s.iter().collect()
        }
    };
    let eye_style = theme::bold(tone(theme::WHITE));
    c.text(3, 2, &eye, eye_style);
    c.text(9, 2, &eye, eye_style);

    // Clock face.
    c.text(
        6,
        7,
        face,
        theme::bold(if off { theme::MUTED } else { theme::WHITE }),
    );

    // Tail pendulum, swinging opposite the eyes.
    let tail = theme::fg(tone(theme::LAVENDER));
    let dir = -pos;
    for i in 0..3 {
        let ch = if i == 2 {
            '•'
        } else {
            match dir {
                -1 => '/',
                1 => '\\',
                _ => '|',
            }
        };
        c.put(8 + dir * i, 11 + i, ch, tail);
    }

    // Right-hand side: paw, bowl or sleepy z's.
    match mood {
        Mood::Working => {
            let paw = if (frame / 2).is_multiple_of(2) {
                &PAW_UP
            } else {
                &PAW_BECKON
            };
            for (y, row) in paw.iter().enumerate() {
                c.sprite(16, y as i32, row, theme::fg(theme::SKY));
            }
        }
        Mood::Lunch => {
            c.sprite(
                16,
                5,
                STEAM[((frame / 3) % 2) as usize],
                theme::fg(theme::MUTED),
            );
            for (y, row) in BOWL.iter().enumerate() {
                c.sprite(16, 6 + y as i32, row, theme::fg(theme::LAVENDER));
            }
        }
        Mood::Stopped => {
            for i in 0..3u64 {
                let p = ((frame / 2 + i * 3) % 9) as i32;
                let ch = if p < 3 { 'z' } else { 'Z' };
                let fade = theme::lerp(theme::SKY, theme::FAINT, p as f32 / 8.0);
                c.put(17 + p / 2, 4 - p / 2, ch, theme::fg(fade));
            }
        }
    }
    c
}

// ── Books ────────────────────────────────────────────────────────────────

pub const SLOT_W: i32 = 5;
pub const SLOT_GAP: i32 = 1;
pub const SHELF_W: usize = (7 * (SLOT_W + SLOT_GAP) - SLOT_GAP) as usize;
/// Tallest book, plus the shelf and the cursor row.
pub const SHELF_H: usize = 7;
const HEIGHTS: [i32; 7] = [4, 5, 4, 5, 4, 4, 5];
const LETTERS: [char; 7] = ['M', 'T', 'W', 'T', 'F', 'S', 'S'];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pose {
    Upright,
    Tilt,
    Lean,
    FlatHigh,
    Flat,
}

use Pose::*;

/// (pose, x offset) per frame.
pub type Seq = &'static [(Pose, i32)];

pub const FALL: Seq = &[(Tilt, 0), (Lean, 0), (FlatHigh, 0), (Flat, 0)];
pub const RISE: Seq = &[(Lean, 0), (Tilt, 0), (Upright, 0)];
pub const WOBBLE: Seq = &[(Upright, 1), (Tilt, 0), (Upright, -1), (Upright, 0)];
pub const TOPPLE: Seq = &[
    (Tilt, 0),
    (Lean, 0),
    (FlatHigh, 0),
    (Flat, 0),
    (Flat, 0),
    (Flat, 0),
    (Lean, 0),
    (Tilt, 0),
    (Upright, 0),
];

#[derive(Clone, Copy, Debug)]
pub struct BookAnim {
    pub seq: Seq,
    pub start: u64,
}

impl BookAnim {
    pub fn at(&self, frame: u64) -> Option<(Pose, i32)> {
        self.seq
            .get(frame.saturating_sub(self.start) as usize)
            .copied()
    }
}

pub struct ShelfView<'a> {
    pub days: [bool; 7],
    pub anims: &'a [Option<BookAnim>; 7],
    pub frame: u64,
    /// Day under the cursor, when the Days field is selected.
    pub cursor: Option<usize>,
}

pub fn shelf(v: &ShelfView) -> Canvas {
    let mut c = Canvas::new(SHELF_W, SHELF_H);
    let floor = SHELF_H as i32 - 3; // last row a book can occupy

    for i in 0..7 {
        let resting = if v.days[i] { Upright } else { Flat };
        let (pose, dx) = v.anims[i]
            .and_then(|a| a.at(v.frame))
            .unwrap_or((resting, 0));
        let x0 = i as i32 * (SLOT_W + SLOT_GAP) + dx;
        let rgb = if v.days[i] || pose != Flat {
            theme::BOOKS[i]
        } else {
            theme::dim(theme::BOOKS[i])
        };
        let spine = ratatui::style::Style::new()
            .bg(theme::color(rgb))
            .fg(theme::color(theme::WHITE));
        let band = spine.fg(theme::color(theme::lerp(rgb, theme::WHITE, 0.45)));
        let h = HEIGHTS[i];
        let letter = LETTERS[i];

        let fill = |c: &mut Canvas, cells: &[(i32, i32)]| {
            for &(x, y) in cells {
                c.put(x0 + x, y, ' ', spine);
            }
        };

        match pose {
            Upright => {
                let top = floor - h + 1;
                for y in top..=floor {
                    fill(&mut c, &[(1, y), (2, y), (3, y)]);
                }
                c.put(
                    x0 + 2,
                    top,
                    letter,
                    spine.add_modifier(ratatui::style::Modifier::BOLD),
                );
                c.text(x0 + 1, top + 2, "───", band);
            }
            Tilt => {
                let top = floor - h + 1;
                for y in top..=floor {
                    let lean = if y < top + h / 2 { 1 } else { 0 };
                    fill(&mut c, &[(1 + lean, y), (2 + lean, y), (3 + lean, y)]);
                }
                c.put(x0 + 3, top, letter, spine);
            }
            Lean => {
                fill(&mut c, &[(3, floor - 2), (4, floor - 2)]);
                fill(&mut c, &[(1, floor - 1), (2, floor - 1), (3, floor - 1)]);
                fill(&mut c, &[(0, floor), (1, floor)]);
            }
            FlatHigh | Flat => {
                let y = if pose == FlatHigh {
                    floor - 2
                } else {
                    floor - 1
                };
                for x in 0..SLOT_W {
                    fill(&mut c, &[(x, y), (x, y + 1)]);
                }
                c.put(x0 + 2, y, letter, spine);
                c.text(x0, y + 1, "─────", band);
            }
        }
    }

    // The shelf itself.
    let shelf_y = floor + 1;
    for x in 0..SHELF_W as i32 {
        c.put(x, shelf_y, '▀', theme::fg(theme::FAINT));
    }

    if let Some(i) = v.cursor {
        let x = i as i32 * (SLOT_W + SLOT_GAP) + 2;
        c.put(x, shelf_y + 1, '▲', theme::bold(theme::SKY));
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_rows_line_up() {
        assert!(BODY.iter().all(|r| r.chars().count() == 17));
        assert!(
            PAW_UP
                .iter()
                .chain(&PAW_BECKON)
                .all(|r| r.chars().count() == 7)
        );
        assert!(BOWL.iter().chain(&STEAM).all(|r| r.chars().count() == 7));
    }

    #[test]
    fn anims_end() {
        let a = BookAnim {
            seq: FALL,
            start: 10,
        };
        assert_eq!(a.at(10), Some((Tilt, 0)));
        assert_eq!(a.at(13), Some((Flat, 0)));
        assert_eq!(a.at(14), None);
    }

    /// Writes the working cat's frames as ANSI for utils/render_hero.py.
    /// Set CLOCK_IN_FRAMES to the output path, and optionally
    /// CLOCK_IN_FRAME_COUNT and CLOCK_IN_FACE.
    #[test]
    #[ignore]
    fn dump_hero_frames() {
        use ratatui::style::{Color, Modifier};
        use std::fmt::Write;

        let path = std::env::var("CLOCK_IN_FRAMES").expect("set CLOCK_IN_FRAMES");
        let count = std::env::var("CLOCK_IN_FRAME_COUNT").map_or(8, |n| n.parse().unwrap());
        let face = std::env::var("CLOCK_IN_FACE").unwrap_or_else(|_| "09:00".into());
        let mut out = String::new();
        for f in 0..count {
            for line in cat(Mood::Working, f, &face).into_lines() {
                for span in line.spans {
                    if let Some(Color::Rgb(r, g, b)) = span.style.fg {
                        write!(out, "\x1b[38;2;{r};{g};{b}m").unwrap();
                    }
                    if span.style.add_modifier.contains(Modifier::BOLD) {
                        out.push_str("\x1b[1m");
                    }
                    out.push_str(&span.content);
                    out.push_str("\x1b[0m");
                }
                out.push('\n');
            }
            out.push('\x0c'); // frame separator
        }
        std::fs::write(path, out).unwrap();
    }

    #[test]
    fn draws_every_mood_without_panicking() {
        for mood in [Mood::Working, Mood::Lunch, Mood::Stopped] {
            for f in 0..32 {
                cat(mood, f, "01:23").into_lines();
            }
        }
    }
}
