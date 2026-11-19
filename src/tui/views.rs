//! Layout and drawing.

use chrono::{Datelike, NaiveTime, Timelike};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Padding, Paragraph},
};

use super::{
    anim::{self, CAT_H, CAT_W, FPS, Mood, SHELF_H, SHELF_W, ShelfView},
    app::{App, FIELDS, Field, Tone},
    canvas::Canvas,
    theme::{self, BRAND, FAINT, GREY, LAVENDER, MUTED, ROSE, SKY, WHITE, bold, fg},
};
use crate::{
    schedule::{Phase, next_transition},
    until,
};

pub fn draw(f: &mut Frame, app: &App, fps: Option<f64>) {
    let area = f.area();
    let phase = app.phase();
    let off = !matches!(phase, Phase::Working | Phase::Lunch);

    let mut block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(fg(if off { FAINT } else { BRAND }))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("clock-in", bold(WHITE)),
            Span::raw(" "),
        ]))
        .title(
            Line::from(Span::styled(
                format!(" {} ", app.now.format("%H:%M")),
                fg(MUTED),
            ))
            .right_aligned(),
        )
        .padding(Padding::new(3, 3, 1, 0));
    if let Some(keys) = footer(area.width.saturating_sub(4) as usize) {
        block = block.title_bottom(keys.centered());
    }
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 36 || inner.height < 14 {
        let msg = Paragraph::new("make the window a little bigger")
            .style(fg(MUTED))
            .alignment(Alignment::Center);
        let [mid] = Layout::vertical([Constraint::Length(1)])
            .flex(ratatui::layout::Flex::Center)
            .areas(inner);
        f.render_widget(msg, mid);
        return;
    }

    let wide = inner.width >= SHELF_W as u16 + 44 && inner.height >= 22;
    if wide {
        let [left, right] =
            Layout::horizontal([Constraint::Length(SHELF_W as u16), Constraint::Min(36)])
                .spacing(5)
                .areas(inner);
        draw_left(f, app, phase, left);
        draw_right(f, app, phase, right);
    } else {
        draw_right(f, app, phase, inner);
    }

    if let Some(fps) = fps {
        let text = format!("{fps:.1} fps · frame {}", app.frame);
        let w = text.chars().count() as u16;
        let r = Rect {
            x: inner.right().saturating_sub(w),
            y: inner.y,
            width: w.min(inner.width),
            height: 1,
        };
        f.render_widget(Paragraph::new(text).style(fg(ROSE)), r);
    }
}

/// Key hints that fit in `max` columns, dropping the most obvious first.
fn footer(max: usize) -> Option<Line<'static>> {
    let keys = [
        ("↑↓", "select"),
        ("←→", "adjust"),
        ("space", "toggle"),
        ("s", "save"),
        ("g", "start"),
        ("x", "stop"),
        ("l", "lunch"),
        ("q", "quit"),
    ];
    let width = |ks: &[(&str, &str)]| -> usize {
        ks.iter()
            .map(|(k, w)| k.chars().count() + w.len() + 3)
            .sum::<usize>()
            + 1
    };
    let skip = (0..keys.len()).find(|&n| width(&keys[n..]) <= max)?;
    let mut spans = vec![Span::raw(" ")];
    for (i, (k, what)) in keys[skip..].iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", fg(FAINT)));
        }
        spans.push(Span::styled(*k, bold(SKY)));
        spans.push(Span::styled(format!(" {what}"), fg(MUTED)));
    }
    spans.push(Span::raw(" "));
    Some(Line::from(spans))
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(w) / 2,
        y: area.y,
        width: w.min(area.width),
        height: h.min(area.height),
    }
}

fn draw_left(f: &mut Frame, app: &App, phase: Phase, area: Rect) {
    let [cat_area, label_area, _, shelf_area, _] = Layout::vertical([
        Constraint::Length(CAT_H as u16),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(SHELF_H as u16),
        Constraint::Min(0),
    ])
    .areas(area);

    let mood = match phase {
        Phase::Working => Mood::Working,
        Phase::Lunch => Mood::Lunch,
        _ => Mood::Stopped,
    };
    let next = next_transition(app.now, &app.saved, app.lunch_until);
    let face = match (phase, next) {
        (Phase::Working | Phase::Lunch, Some((at, _))) => {
            let mins = (at - app.now).num_minutes() + 1;
            let colon = if app.frame % FPS < 4 { ':' } else { ' ' };
            format!("{:02}{colon}{:02}", (mins / 60).min(99), mins % 60)
        }
        (Phase::AfterWork, _) => app.saved.clock_out.format("%H:%M").to_string(),
        _ => "--:--".into(),
    };
    let cat = anim::cat(mood, app.frame, &face);
    f.render_widget(
        Paragraph::new(cat.into_lines()),
        centered(cat_area, CAT_W as u16, CAT_H as u16),
    );

    let label = match (phase, next) {
        (Phase::Working, Some((at, Phase::Lunch))) => format!("lunch in {}", until(app.now, at)),
        (Phase::Working, Some((at, _))) => format!("clock-out in {}", until(app.now, at)),
        (Phase::Lunch, Some((at, _))) => format!("back at {}", at.format("%H:%M")),
        (Phase::AfterWork, _) => format!("time stopped at {}", app.saved.clock_out.format("%H:%M")),
        (_, Some((at, _))) => {
            let at = if at.time() == NaiveTime::MIN {
                at.date().and_time(app.saved.clock_in)
            } else {
                at
            };
            if at.date() == app.now.date() {
                format!("clocking in at {}", at.format("%H:%M"))
            } else {
                format!(
                    "back {} {}",
                    at.format("%a"),
                    app.saved.clock_in.format("%H:%M")
                )
            }
        }
        _ => "no working days set".into(),
    };
    let label_style = if matches!(mood, Mood::Stopped) {
        fg(MUTED)
    } else {
        fg(SKY)
    };
    f.render_widget(
        Paragraph::new(label)
            .style(label_style)
            .alignment(Alignment::Center),
        label_area,
    );

    let cursor = (app.selected() == Field::Days).then_some(app.day_cursor);
    let shelf = anim::shelf(&ShelfView {
        days: app.draft.days,
        anims: &app.books,
        frame: app.frame,
        cursor,
    });
    f.render_widget(Paragraph::new(shelf.into_lines()), shelf_area);
}

fn draw_right(f: &mut Frame, app: &App, phase: Phase, area: Rect) {
    if area.height < 20 {
        return draw_compact(f, app, phase, area);
    }
    let rows = Layout::vertical([
        Constraint::Length(1), // status badge
        Constraint::Length(1), // date
        Constraint::Length(1),
        Constraint::Length(3), // timeline
        Constraint::Length(1),
        Constraint::Length(4), // times
        Constraint::Length(1),
        Constraint::Length(1), // days
        Constraint::Length(1),
        Constraint::Length(2), // toggles
        Constraint::Length(1),
        Constraint::Length(1), // field hint
        Constraint::Length(1), // toast
        Constraint::Min(0),
    ])
    .split(area);

    f.render_widget(Paragraph::new(badge(app, phase)), rows[0]);
    let date = format!(
        "{} {} {}",
        app.now.format("%A"),
        app.now.day(),
        app.now.format("%B")
    );
    f.render_widget(Paragraph::new(date).style(fg(MUTED)), rows[1]);

    f.render_widget(
        Paragraph::new(timeline(app, rows[3].width as usize)),
        rows[3],
    );

    let field_lines =
        |fields: &[Field]| -> Vec<Line> { fields.iter().map(|&fl| field_line(app, fl)).collect() };
    f.render_widget(Paragraph::new(field_lines(&FIELDS[0..4])), rows[5]);
    f.render_widget(Paragraph::new(field_lines(&FIELDS[4..5])), rows[7]);
    f.render_widget(Paragraph::new(field_lines(&FIELDS[5..7])), rows[9]);

    f.render_widget(
        Paragraph::new(hint(app.selected())).style(fg(FAINT)),
        rows[11],
    );

    if let Some(t) = &app.toast {
        f.render_widget(Paragraph::new(toast_line(&t.text, t.tone)), rows[12]);
    }
}

fn badge(app: &App, phase: Phase) -> Line<'static> {
    let mut spans = Vec::new();
    if app.daemon_running {
        let (dot, text, color) = match phase {
            Phase::Working => ("●", "online", SKY),
            Phase::Lunch => ("◐", "lunch", LAVENDER),
            _ => ("○", "offline", MUTED),
        };
        spans.push(Span::styled(format!("{dot} {text}"), bold(color)));
        spans.push(Span::styled("  scheduler running", fg(FAINT)));
    } else {
        spans.push(Span::styled("◌ scheduler stopped", bold(ROSE)));
        spans.push(Span::styled("  press g to start", fg(FAINT)));
    }
    if app.dirty() {
        spans.push(Span::styled("   ● unsaved", fg(LAVENDER)));
    }
    Line::from(spans)
}

fn hint(field: Field) -> &'static str {
    match field {
        Field::In => "Teams shows you as available from here",
        Field::Lunch => "your break starts and Teams can go idle",
        Field::LunchLen => "0 turns the lunch break off",
        Field::Out => "Teams closes and you appear offline",
        Field::Days => "←→ pick a day · space or 1–7 to toggle",
        Field::Relaunch => "open Teams when the day starts",
        Field::Enforce => "close Teams again if it opens after hours",
    }
}

fn field_line(app: &App, field: Field) -> Line<'static> {
    let selected = app.selected() == field;
    let (d, s) = (&app.draft, &app.saved);
    let label = match field {
        Field::In => "Clock in",
        Field::Lunch => "Lunch",
        Field::LunchLen => "Lunch length",
        Field::Out => "Clock out",
        Field::Days => "Days",
        Field::Relaunch => "Open Teams",
        Field::Enforce => "Stay offline",
    };
    let changed = match field {
        Field::In => d.clock_in != s.clock_in,
        Field::Lunch => d.lunch_start != s.lunch_start,
        Field::LunchLen => d.lunch_mins != s.lunch_mins,
        Field::Out => d.clock_out != s.clock_out,
        Field::Days => d.days != s.days,
        Field::Relaunch => d.relaunch_teams != s.relaunch_teams,
        Field::Enforce => d.enforce_offline != s.enforce_offline,
    };

    let mut spans = vec![
        Span::styled(if selected { "› " } else { "  " }, bold(SKY)),
        Span::styled(
            format!("{label:<14}"),
            if selected { bold(WHITE) } else { fg(MUTED) },
        ),
    ];

    let pill = Style::new()
        .bg(theme::color(BRAND))
        .fg(theme::color(WHITE))
        .add_modifier(Modifier::BOLD);
    let value_style = if selected { pill } else { fg(GREY) };
    let arrows = |v: String| {
        if selected {
            format!(" ‹ {v} › ")
        } else {
            format!("   {v}   ")
        }
    };

    match field {
        Field::Days => {
            spans.push(Span::raw("   "));
            for (i, letter) in ['M', 'T', 'W', 'T', 'F', 'S', 'S'].iter().enumerate() {
                let on = d.days[i];
                let mut st = if on {
                    bold(theme::lerp(theme::BOOKS[i], WHITE, 0.5))
                } else {
                    fg(FAINT)
                };
                if selected && i == app.day_cursor {
                    st = st.bg(theme::color(BRAND));
                }
                spans.push(Span::styled(format!("{letter}"), st));
                spans.push(Span::raw(" "));
            }
        }
        Field::Relaunch | Field::Enforce => {
            let on = if field == Field::Relaunch {
                d.relaunch_teams
            } else {
                d.enforce_offline
            };
            let text = if on { "● on" } else { "○ off" };
            let st = if selected {
                pill
            } else if on {
                fg(SKY)
            } else {
                fg(MUTED)
            };
            spans.push(Span::styled(arrows(text.into()), st));
        }
        Field::LunchLen => {
            let text = if d.lunch_mins == 0 {
                "none".into()
            } else {
                format!("{} min", d.lunch_mins)
            };
            spans.push(Span::styled(arrows(text), value_style));
        }
        _ => {
            let t = match field {
                Field::In => d.clock_in,
                Field::Lunch => d.lunch_start,
                _ => d.clock_out,
            };
            let dim_lunch = field == Field::Lunch && d.lunch_mins == 0 && !selected;
            let st = if dim_lunch { fg(FAINT) } else { value_style };
            spans.push(Span::styled(arrows(t.format("%H:%M").to_string()), st));
        }
    }
    if changed {
        spans.push(Span::styled(" •", fg(LAVENDER)));
    }
    Line::from(spans)
}

/// Today's shape: off-hours, work, lunch, with a "now" marker.
fn timeline(app: &App, w: usize) -> Vec<Line<'static>> {
    let mut c = Canvas::new(w.max(1), 3);
    let d = &app.draft;
    let mins = |t: NaiveTime| (t.hour() * 60 + t.minute()) as i32;
    let start = (mins(d.clock_in) - 60).max(0);
    let end = (mins(d.clock_out) + 60).min(24 * 60 - 1);
    if w < 10 || end <= start {
        return c.into_lines();
    }
    let span = (end - start) as f32;
    let col = |m: i32| (((m - start) as f32 / span) * (w - 1) as f32).round() as i32;
    let today = d.works_on(app.now.weekday());

    for x in 0..w as i32 {
        let m = start + ((x as f32 / (w - 1) as f32) * span) as i32;
        let in_lunch = d.lunch_mins > 0 && m >= mins(d.lunch_start) && m < mins(d.lunch_end());
        let in_work = m >= mins(d.clock_in) && m < mins(d.clock_out);
        let (ch, rgb) = if in_lunch {
            ('━', LAVENDER)
        } else if in_work {
            ('━', theme::PERIWINKLE)
        } else {
            ('─', FAINT)
        };
        c.put(x, 1, ch, fg(if today { rgb } else { theme::dim(rgb) }));
    }

    // Now marker.
    let now_m = mins(app.now.time());
    if today && now_m >= start && now_m <= end {
        c.put(col(now_m), 0, '▼', bold(WHITE));
    }

    // Labels, left to right, skipping any that would collide.
    let mut labels = vec![(mins(d.clock_in), d.clock_in)];
    if d.lunch_mins > 0 {
        labels.push((mins(d.lunch_start), d.lunch_start));
    }
    labels.push((mins(d.clock_out), d.clock_out));
    let mut free_from = 0;
    for (m, t) in labels {
        let text = t.format("%H:%M").to_string();
        let x = (col(m) - 2).clamp(0, w as i32 - 5);
        if x >= free_from {
            c.text(x, 2, &text, fg(MUTED));
            free_from = x + 6;
        }
    }
    if !today {
        c.text(0, 0, "not a working day", fg(FAINT));
    }
    c.into_lines()
}

/// Short windows: the status, every field, and any message.
fn draw_compact(f: &mut Frame, app: &App, phase: Phase, area: Rect) {
    let mut lines = vec![badge(app, phase), Line::raw("")];
    lines.extend(FIELDS.iter().map(|&fl| field_line(app, fl)));
    lines.push(Line::raw(""));
    if let Some(t) = &app.toast {
        lines.push(toast_line(&t.text, t.tone));
    } else {
        lines.push(Line::styled(hint(app.selected()), fg(FAINT)));
    }
    f.render_widget(Paragraph::new(lines), area);
}

fn toast_line(text: &str, tone: Tone) -> Line<'static> {
    let color = match tone {
        Tone::Info => SKY,
        Tone::Good => LAVENDER,
        Tone::Bad => ROSE,
    };
    Line::from(vec![
        Span::styled("▸ ", bold(color)),
        Span::styled(text.to_string(), fg(color)),
    ])
}
