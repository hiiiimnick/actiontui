use ratatui::style::Style;

use crate::tui::app::{App, CurrentFocus};

pub fn map_block_color(app: &App, expected: CurrentFocus) -> Style {
    if app.current_focus == expected {
        Style::light_blue(Style::new())
    } else {
        Style::default()
    }
}
