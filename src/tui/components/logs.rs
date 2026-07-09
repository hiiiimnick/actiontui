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
    let list_items: Vec<ListItem> = if let Some(selected) = &app.selected_job {
        selected
            .steps
            .iter()
            .map(|step| {
                let name = step.name.clone();
                let status_span = map_status_to_span(&step.status, step.conclusion.as_deref());

                return ListItem::new(Line::from(vec![status_span, Span::raw(name)]));
            })
            .collect()
    } else {
        Vec::new()
    };

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(map_block_color(app, CurrentFocus::Logs))
                .title(Line::from("[4]").left_aligned())
                .title(Line::from("Logs").left_aligned()),
        )
        .style(Color::White)
        .highlight_style(Style::default().bg(Color::White).fg(Color::Black));

    frame.render_stateful_widget(list, area, &mut app.step_state);
}
