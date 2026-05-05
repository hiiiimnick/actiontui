use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Borders},
};

use crate::tui::app::App;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default())
        .title(Line::from("[2]").left_aligned())
        .title(Line::from("Dispatch").centered());

    frame.render_widget(block, area);
}
