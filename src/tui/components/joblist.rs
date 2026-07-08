use std::vec;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::tui::{
    app::{App, CurrentFocus},
    util::{map_block_color, map_status_to_span},
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let selected_job = app.selected_job.as_ref();
    let list_items: Vec<ListItem> = app
        .jobs
        .iter()
        .map(|job| {
            let name = &job.name;
            let status_span = map_status_to_span(&job.status, job.conclusion.as_deref());
            if let Some(selected) = selected_job
                && selected == job
            {
                return ListItem::new(Line::from(vec![
                    status_span,
                    Span::raw(format!("[{}]", name)).style(Color::Yellow),
                ]));
            }
            ListItem::new(Line::from(vec![status_span, Span::raw(name)]))
        })
        .collect();

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(map_block_color(app, CurrentFocus::Jobs))
                .title(Line::from("[3]").left_aligned())
                .title(
                    Line::from("Jobs")
                        .style(map_block_color(app, CurrentFocus::Jobs))
                        .centered(),
                ),
        )
        .style(Color::White)
        .highlight_style(Style::default().bg(Color::White).fg(Color::Black));

    frame.render_stateful_widget(list, area, &mut app.job_state);
}
