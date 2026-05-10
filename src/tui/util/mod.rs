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

pub fn map_status_to_span(status: String, conclusion: Option<String>) -> Span<'static> {
    match status.as_str() {
        "queued" | "pending" | "requested" => Span::styled(" ", Style::default().fg(Color::White)),
        "in_progress" => Span::styled(" ", Style::default().fg(Color::White)),
        "action_required" | "waiting" => Span::styled(" ", Style::default().fg(Color::Yellow)),
        "completed" => {
            if let Some(conclusion) = conclusion {
                match conclusion.as_str() {
                    "success" | "neutral" => Span::styled(" ", Style::default().fg(Color::Green)),
                    "failure" | "cancelled" | "skipped" | "timed_out" | "stale" => {
                        Span::styled(" ", Style::default().fg(Color::Red))
                    }
                    "action_required" => Span::styled(" ", Style::default().fg(Color::Yellow)),
                    _ => Span::styled(" ", Style::default().fg(Color::Red)),
                }
            } else {
                Span::styled(" ", Style::default().fg(Color::Red))
            }
        }
        _ => Span::styled(" ", Style::default().fg(Color::Red)),
    }
}
