use ratatui::{
    Frame,
    layout::Rect,
    style::Styled,
    text::Line,
    widgets::{Block, Borders},
};

use crate::tui::{
    app::{App, CurrentFocus},
    util::map_block_color,
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::all())
        .title(Line::from("[5]").left_aligned())
        .title(Line::from("Logs").centered())
        .set_style(map_block_color(app, CurrentFocus::Logs));

    frame.render_widget(block, area);
}
