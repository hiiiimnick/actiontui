use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Block, Borders},
};

use crate::tui::{
    app::{App, CurrentFocus},
    util::map_block_color,
};

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(map_block_color(app, CurrentFocus::Logs))
        .title(Line::from("[4]").left_aligned())
        .title(Line::from("Logs").left_aligned());

    frame.render_widget(block, area);
}
