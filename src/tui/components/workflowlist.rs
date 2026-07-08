use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders},
};
use tui_widget_list::{ListBuilder, ListView, ScrollAxis};

use crate::tui::{
    app::{App, CurrentFocus},
    util::map_block_color,
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    if app.workflows.is_empty() {
        return;
    }

    let builder = ListBuilder::new(|context| {
        let workflow = &app.workflows[context.index % app.workflows.len()];
        let default_line = Line::from(Span::styled(
            format!(" {} ", workflow.name),
            Style::default().fg(Color::White),
        ));

        let line_length = (workflow.name.len() + 2) as u16;
        if let Some(selected_id) = app.selected_workflow_id
            && workflow.id == selected_id
        {
            return (
                Line::from(Span::styled(
                    format!("[{}]", workflow.name),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                line_length,
            );
        } else if context.is_selected {
            return (default_line.add_modifier(Modifier::REVERSED), line_length);
        }

        (default_line, line_length)
    });

    let list = ListView::new(builder, app.workflows.len())
        .scroll_axis(ScrollAxis::Horizontal)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(map_block_color(app, CurrentFocus::Workflows))
                .title(Line::from("[1]").left_aligned())
                .title(Line::from("Workflows").left_aligned()),
        );

    frame.render_stateful_widget(list, area, &mut app.workflow_state);
}
