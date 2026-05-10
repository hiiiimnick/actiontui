use std::fmt::format;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::tui::{
    app::{App, CurrentFocus},
    util::{map_block_color, map_status_to_span},
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let area_width = area.width as usize;
    let list_items: Vec<ListItem> = app
        .runs
        .iter()
        .map(|run| {
            let status = map_status_to_span(run.status.clone(), run.conclusion.clone());
            let time = run.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
            let padding_total = area_width.saturating_sub(
                5_usize + run.display_title.len() + run.head_branch.len() + time.len(),
            );

            let padding_status = " ".repeat((5_usize).saturating_sub(status.content.len()));
            let padding_per_item = " ".repeat(padding_total / 2);

            ListItem::new(Line::from(vec![
                status,
                Span::raw(format!(
                    "{}{}{}{}{}{}",
                    padding_status,
                    run.display_title,
                    padding_per_item,
                    run.head_branch,
                    padding_per_item,
                    time,
                )),
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
