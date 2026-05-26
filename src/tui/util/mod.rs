use chrono::{DateTime, Local};
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

pub fn map_optional_time_to_string(optinal_time: Option<DateTime<Local>>) -> String {
    if let Some(time) = optinal_time {
        return time.format("%Y-%m-%d %H:%M:%S").to_string();
    }
    String::default()
}
