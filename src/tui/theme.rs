//! Teams accent colours, with a 256-colour fallback.

use ratatui::style::{Color, Modifier, Style};
use std::sync::OnceLock;

pub type Rgb = (u8, u8, u8);

// Teams brand palette.
pub const BRAND: Rgb = (0x4B, 0x53, 0xBC);
pub const INDIGO: Rgb = (0x5B, 0x5F, 0xC7); // interface purple
pub const CLASSIC: Rgb = (0x62, 0x64, 0xA7);
pub const PERIWINKLE: Rgb = (0x7B, 0x83, 0xEB); // light purple
pub const DARK: Rgb = (0x46, 0x47, 0x75);
pub const GREY: Rgb = (0xF5, 0xF5, 0xF5);
pub const WHITE: Rgb = (0xFF, 0xFF, 0xFF);

// Highlights from the Teams icon's gradient.
pub const SKY: Rgb = (0x8A, 0xB4, 0xF8);
pub const LAVENDER: Rgb = (0xA3, 0x8C, 0xF8);

// Derived text tones.
pub const MUTED: Rgb = (0xA4, 0xA6, 0xD0);
pub const FAINT: Rgb = DARK;
pub const ROSE: Rgb = (0xF2, 0x8F, 0xB1);

/// Book spine colours, picked to sit next to each other.
pub const BOOKS: [Rgb; 7] = [BRAND, PERIWINKLE, INDIGO, LAVENDER, CLASSIC, DARK, SKY];

fn truecolor() -> bool {
    static TC: OnceLock<bool> = OnceLock::new();
    *TC.get_or_init(|| {
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        if colorterm.contains("truecolor") || colorterm.contains("24bit") {
            return true;
        }
        // Windows Terminal, VS Code and modern conhost all do 24-bit colour;
        // only fall back when a terminal advertises 256 colours and nothing more.
        !std::env::var("TERM").unwrap_or_default().contains("256")
    })
}

pub fn color((r, g, b): Rgb) -> Color {
    if truecolor() {
        Color::Rgb(r, g, b)
    } else {
        let q = |v: u8| ((v as u16 * 5 + 127) / 255) as u8;
        Color::Indexed(16 + 36 * q(r) + 6 * q(g) + q(b))
    }
}

pub fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)).round() as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// Dim a colour toward the deep indigo, for "time stopped".
pub fn dim(c: Rgb) -> Rgb {
    lerp(c, (0x24, 0x22, 0x4A), 0.6)
}

pub fn fg(c: Rgb) -> Style {
    Style::new().fg(color(c))
}

pub fn bold(c: Rgb) -> Style {
    fg(c).add_modifier(Modifier::BOLD)
}
