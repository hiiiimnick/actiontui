use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem},
};

use crate::tui::{
    app::{App, CurrentFocus},
    util::map_block_color,
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let list_items: Vec<ListItem> = app
        .runs
        .iter()
        .map(|run| {
            let name = run.name.clone();
            ListItem::new(name.to_string())
        })
        .collect();

    let mut block = Block::default()
        .borders(Borders::ALL)
        .style(map_block_color(app, CurrentFocus::Runs))
        .title(Line::from("[2]").left_aligned())
        .title(Line::from("Runs").left_aligned());

    if let Some(workflow) = &app.selected_workflow {
        block = block.title(Line::from(workflow.name.to_string()).centered())
    }

    let list = List::new(list_items)
        .block(block)
        .style(Color::White)
        .highlight_style(Modifier::REVERSED);

    frame.render_stateful_widget(list, area, &mut app.run_state);
}
