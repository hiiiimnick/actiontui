use chrono::Utc;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::tui::{
    app::{App, CurrentFocus},
    util::{map_block_color, map_delta_time_to_duration, map_status_to_span},
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let local_time = Utc::now().to_rfc3339();
    let list_items: Vec<ListItem> = if let Some(selected) = &app.selected_job {
        selected
            .steps
            .iter()
            .map(|step| {
                let name = step.name.clone();
                let status_span = map_status_to_span(&step.status, step.conclusion.as_deref());

                let duration = if step.started_at != "" {
                    if step.completed_at != "" {
                        map_delta_time_to_duration(
                            step.started_at.as_str(),
                            step.completed_at.as_str(),
                        )
                    } else {
                        map_delta_time_to_duration(step.started_at.as_str(), &local_time)
                    }
                } else {
                    String::new()
                };

                let filler = " ".repeat(
                    area.width as usize - 2 - status_span.width() - name.len() - duration.len(),
                );
                return ListItem::new(Line::from(vec![
                    status_span,
                    Span::raw(name),
                    Span::raw(filler),
                    Span::raw(duration),
                ]));
            })
            .collect()
    } else {
        Vec::new()
    };

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(map_block_color(app, CurrentFocus::Steps))
                .title(Line::from("[4]").left_aligned())
                .title(Line::from("Steps").left_aligned()),
        )
        .style(Color::White)
        .highlight_style(Style::default().bg(Color::White).fg(Color::Black));

    frame.render_stateful_widget(list, area, &mut app.step_state);
}
