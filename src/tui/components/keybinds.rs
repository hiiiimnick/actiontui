use chrono::Utc;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::domain::{RateLimit, RateLimitLevel};
use crate::tui::app::{App, CurrentFocus};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    // the picker lists the repositories of all accounts, none is active yet
    let mut indicator = Vec::new();
    if !app.in_picker {
        indicator.push(Span::styled(
            format!(" {}/{}", app.repo.owner, app.repo.repo),
            Style::default().fg(Color::Cyan),
        ));
        indicator.push(Span::styled(
            format!(" {} ", app.profile_name),
            Style::default().fg(Color::DarkGray),
        ));
        if let Some(rate) = app.workflowrepo.rate_limit() {
            indicator.push(rate_limit_span(&rate, Utc::now()));
        }
    }
    let indicator = Line::from(indicator);
    let [area, indicator_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(indicator.width() as u16),
    ])
    .areas(area);
    frame.render_widget(Paragraph::new(indicator), indicator_area);

    if let Some(message) = &app.message {
        let color = if message.is_error {
            Color::Red
        } else {
            Color::Green
        };
        let line = Line::styled(format!(" {}", message.text), Style::default().fg(color));
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    let global_mode = app.picker.is_some();
    let picker_hints: &[(&str, &str)] = match app.picker.as_ref() {
        Some(picker) if picker.searching => {
            &[("Enter", "done"), ("Esc", "clear"), ("C-u", "clear text")]
        }
        _ => &[
            ("j/k", "move"),
            ("g/G", "first/last"),
            ("Enter", "open"),
            ("/", "search"),
            ("Esc", "clear search"),
            ("q", "quit"),
        ],
    };
    if app.in_picker {
        render_hints(frame, area, picker_hints, &[]);
        return;
    }

    let context: &[(&str, &str)] = match app.current_focus {
        CurrentFocus::Workflows => &[
            ("j/k", "move"),
            ("Enter", "runs"),
            ("n", "new run"),
            ("r", "refresh"),
        ],
        CurrentFocus::Runs => &[
            ("j/k", "move"),
            ("Enter", "jobs"),
            ("n", "new run"),
            ("R", "rerun failed"),
            ("A", "rerun all"),
            ("a/d", "approve/reject"),
            ("r", "refresh"),
        ],
        CurrentFocus::Jobs => &[
            ("j/k", "move"),
            ("Enter", "steps"),
            ("R", "rerun job"),
            ("a/d", "approve/reject"),
            ("r", "refresh"),
        ],
        CurrentFocus::Steps => &[("j/k", "move"), ("Enter", "logs"), ("r", "refresh")],
        CurrentFocus::Logs => &[
            ("j/k", "scroll"),
            ("C-j/k", "scroll 10"),
            ("g/G", "top/bottom"),
        ],
    };
    let global: &[(&str, &str)] = if global_mode {
        &[("1-5", "focus"), ("Esc", "repositories"), ("q", "quit")]
    } else {
        &[("1-5", "focus"), ("q", "quit")]
    };
    render_hints(frame, area, context, global);
}

fn render_hints(frame: &mut Frame, area: Rect, context: &[(&str, &str)], global: &[(&str, &str)]) {
    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let spans: Vec<Span> = context
        .iter()
        .chain(global)
        .flat_map(|(key, desc)| {
            [
                Span::styled(format!(" {key} "), key_style),
                Span::styled(format!("{desc} "), Style::default().fg(Color::Gray)),
            ]
        })
        .collect();

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// `API 2%`: the used part of the quota, colored by how much is left. When the
/// quota runs low the time until it is renewed is shown too.
fn rate_limit_span(rate: &RateLimit, now: chrono::DateTime<Utc>) -> Span<'static> {
    let level = rate.level();
    let used = rate.used_percent();
    let mut text = if used > 0.0 && used < 0.5 {
        " API <1%".to_string()
    } else {
        format!(" API {used:.0}%")
    };
    if level != RateLimitLevel::Ok
        && let Some(left) = rate.time_until_reset(now)
    {
        text.push_str(&format!(" (resets in {}m)", (left.num_seconds() + 59) / 60));
    }
    text.push(' ');

    let color = match level {
        RateLimitLevel::Ok => Color::Gray,
        RateLimitLevel::Low => Color::Yellow,
        RateLimitLevel::Exhausted => Color::Red,
    };
    Span::styled(text, Style::default().fg(color))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate(remaining: u32, reset_in_secs: Option<i64>) -> (RateLimit, chrono::DateTime<Utc>) {
        let now = chrono::DateTime::from_timestamp(1_000, 0).unwrap();
        let reset_at = reset_in_secs.and_then(|s| chrono::DateTime::from_timestamp(1_000 + s, 0));
        (
            RateLimit {
                limit: 5000,
                remaining,
                reset_at,
            },
            now,
        )
    }

    #[test]
    fn shows_the_used_percentage() {
        let now = rate(0, None).1;
        let at = |remaining| {
            rate_limit_span(&rate(remaining, None).0, now)
                .content
                .to_string()
        };
        assert_eq!(at(4920), " API 2% ");
        assert_eq!(at(2500), " API 50% ");
        // never claims 0% while something is used
        assert_eq!(at(4999), " API <1% ");
    }

    #[test]
    fn healthy_quota_is_plain() {
        let (rate, now) = rate(5000, Some(600));
        let span = rate_limit_span(&rate, now);
        assert_eq!(span.content, " API 0% ");
        assert_eq!(span.style.fg, Some(Color::Gray));
    }

    #[test]
    fn low_quota_warns_and_shows_the_reset() {
        let (rate, now) = rate(300, Some(601));
        let span = rate_limit_span(&rate, now);
        assert_eq!(span.content, " API 94% (resets in 11m) ");
        assert_eq!(span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn exhausted_quota_is_red() {
        let (rate, now) = rate(0, Some(30));
        let span = rate_limit_span(&rate, now);
        assert_eq!(span.content, " API 100% (resets in 1m) ");
        assert_eq!(span.style.fg, Some(Color::Red));
    }

    #[test]
    fn low_quota_without_reset_time() {
        let (rate, now) = rate(10, None);
        assert_eq!(rate_limit_span(&rate, now).content, " API 100% ");
    }
}
