use chrono::{DateTime, Local, NaiveDateTime};
use ratatui::{
    style::{Color, Style},
    text::Span,
};

use crate::tui::app::{App, CurrentFocus};

pub fn map_block_color(app: &App, expected: CurrentFocus) -> Style {
    if app.current_focus == expected {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

pub fn map_status_to_span(status: &str, conclusion: Option<&str>) -> Span<'static> {
    match status {
        "queued" | "pending" | "requested" => {
            Span::styled("   ", Style::default().fg(Color::White))
        }
        "in_progress" => Span::styled("   ", Style::default().fg(Color::White)),
        "action_required" | "waiting" => Span::styled("   ", Style::default().fg(Color::Yellow)),
        "completed" => {
            if let Some(conclusion) = conclusion {
                match conclusion {
                    "success" | "neutral" => {
                        Span::styled("   ", Style::default().fg(Color::Green))
                    }
                    "failure" | "cancelled" | "skipped" | "timed_out" | "stale" => {
                        Span::styled("   ", Style::default().fg(Color::Red))
                    }
                    "action_required" => Span::styled(" ", Style::default().fg(Color::Yellow)),
                    _ => Span::styled("   ", Style::default().fg(Color::Red)),
                }
            } else {
                Span::styled("   ", Style::default().fg(Color::Red))
            }
        }
        _ => Span::styled("   ", Style::default().fg(Color::Red)),
    }
}

pub fn map_optional_time_to_string(optional_time: Option<DateTime<Local>>) -> String {
    if let Some(time) = optional_time {
        return time.format("%Y-%m-%d %H:%M:%S").to_string();
    }
    String::default()
}

pub fn map_delta_time_to_duration(start_time: &str, end_time: &str) -> String {
    let start = DateTime::parse_from_rfc3339(start_time).expect("Failed to parse Step timestamp");
    let end = DateTime::parse_from_rfc3339(end_time).expect("Failed to parse step timestamp");

    let duration = end.signed_duration_since(start);

    return format!(
        "{}m {}s",
        duration.num_minutes(),
        duration.num_seconds() % 60
    );
}
