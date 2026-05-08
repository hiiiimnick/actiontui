use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, List, ListItem},
};

use crate::tui::{
    app::{App, CurrentFocus},
    util::map_block_color,
};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let list_items: Vec<ListItem> = app
        .workflows
        .iter()
        .map(|workflow| {
            let name = workflow.name.clone();
            ListItem::new(format!("{}", name))
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .style(map_block_color(app, CurrentFocus::Workflow))
            .title(Line::from("[1]").left_aligned())
            .title(Line::from("Workflows").centered()),
    );

    frame.render_widget(list, area);
}
