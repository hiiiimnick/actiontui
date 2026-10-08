use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::app::{App, CurrentFocus};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
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
            ("r", "refresh"),
        ],
        CurrentFocus::Jobs => &[("j/k", "move"), ("Enter", "steps"), ("r", "refresh")],
        CurrentFocus::Steps => &[("j/k", "move"), ("Enter", "logs"), ("r", "refresh")],
        CurrentFocus::Logs => &[
            ("j/k", "scroll"),
            ("C-j/k", "scroll 10"),
            ("g/G", "top/bottom"),
        ],
    };
    let global: &[(&str, &str)] = &[("1-5", "focus"), ("q", "quit")];

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
