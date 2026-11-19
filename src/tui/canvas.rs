//! A tiny cell grid for sprites. Writes outside the grid are ignored, so
//! animations can never panic on a small terminal.

use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    cells: Vec<(char, Style)>,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            cells: vec![(' ', Style::new()); w * h],
        }
    }

    pub fn put(&mut self, x: i32, y: i32, ch: char, style: Style) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.cells[y as usize * self.w + x as usize] = (ch, style);
        }
    }

    pub fn text(&mut self, x: i32, y: i32, s: &str, style: Style) {
        for (i, ch) in s.chars().enumerate() {
            self.put(x + i as i32, y, ch, style);
        }
    }

    /// Like `text`, but spaces leave what's underneath.
    pub fn sprite(&mut self, x: i32, y: i32, s: &str, style: Style) {
        for (i, ch) in s.chars().enumerate() {
            if ch != ' ' {
                self.put(x + i as i32, y, ch, style);
            }
        }
    }

    pub fn into_lines(self) -> Vec<Line<'static>> {
        self.cells
            .chunks(self.w.max(1))
            .map(|row| {
                let mut spans: Vec<Span> = Vec::new();
                let mut run = String::new();
                let mut style = row.first().map(|c| c.1).unwrap_or_default();
                for &(ch, s) in row {
                    if s != style && !run.is_empty() {
                        spans.push(Span::styled(std::mem::take(&mut run), style));
                    }
                    style = s;
                    run.push(ch);
                }
                spans.push(Span::styled(run, style));
                Line::from(spans)
            })
            .collect()
    }
}
