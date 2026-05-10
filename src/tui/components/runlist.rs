use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::{
    domain::models::workflow,
    tui::{
        app::{App, CurrentFocus},
        util::{map_block_color, map_status_to_span},
    },
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let list_items: Vec<ListItem> = app
        .runs
        .iter()
        .map(|run| {
            let name = run.name.clone();
            ListItem::new(Line::from(vec![
                map_status_to_span(run.status.clone(), run.conclusion.clone()),
                Span::raw(name),
            ]))
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
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().bg(Color::White).fg(Color::Black));

    frame.render_stateful_widget(list, area, &mut app.run_state);
}
