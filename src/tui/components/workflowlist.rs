use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier},
    text::Line,
    widgets::{Block, Borders, List, ListItem},
};

use crate::tui::{
    app::{App, CurrentFocus},
    util::map_block_color,
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let selected_workflow = app.selected_workflow.as_ref();
    let list_items: Vec<ListItem> = app
        .workflows
        .iter()
        .map(|workflow| {
            let name = workflow.name.clone();

            if let Some(selected) = selected_workflow
                && selected == workflow
            {
                return ListItem::new(format!("> {}", name)).style(Color::Yellow);
            }
            ListItem::new(name)
        })
        .collect();

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(map_block_color(app, CurrentFocus::Workflows))
                .title(Line::from("[1]").left_aligned())
                .title(Line::from("Workflows").centered()),
        )
        .style(Color::White)
        .highlight_style(Modifier::REVERSED);

    frame.render_stateful_widget(list, area, &mut app.workflow_state);
}
