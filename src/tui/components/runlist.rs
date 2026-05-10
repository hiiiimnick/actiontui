use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::{
    domain::Run,
    tui::{
        app::{App, CurrentFocus},
        util::{map_block_color, map_status_to_span},
    },
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let area_width = area.width as usize;
    let list_items: Vec<ListItem> = app
        .runs
        .iter()
        .map(|run| {
            let status = map_status_to_span(run.status.clone(), run.conclusion.clone());

            ListItem::new(Line::from(vec![
                status,
                Span::raw(format_line(run, area_width)),
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

fn format_line(run: &Run, line_size: usize) -> String {
    let time = run.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
    let title_padding = " ".repeat(
        (line_size / 2)
            .saturating_sub(run.head_branch.len() / 2)
            .saturating_sub(run.display_title.len())
            .saturating_sub(5),
    );
    let mut branch_padding = " ".repeat(
        (line_size / 2)
            .saturating_sub(run.head_branch.len() / 2)
            .saturating_sub(time.len()),
    );
    let total_len = 5
        + run.display_title.len()
        + title_padding.len()
        + run.head_branch.len()
        + branch_padding.len()
        + time.len();

    if total_len >= line_size {
        let to_remove = total_len - line_size + 1;
        branch_padding.truncate(branch_padding.len().saturating_sub(to_remove));
    }
    format!(
        "  {}{}{}{}{}",
        run.display_title, title_padding, run.head_branch, branch_padding, time
    )
}
