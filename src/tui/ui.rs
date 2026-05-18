use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::tui::{
    app::App,
    components::{joblist, logs, runlist, workflowlist},
};

pub fn ui(app: &mut App, frame: &mut Frame) {
    let main_rects = Layout::vertical([
        Constraint::Max(3),
        Constraint::Percentage(30),
        Constraint::Min(12),
        Constraint::Max(1),
    ])
    .split(frame.area());

    let horizontal_split =
        Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
            .split(main_rects[2]);

    workflowlist::render(app, frame, main_rects[0]);
    runlist::render(app, frame, main_rects[1]);
    joblist::render(app, frame, horizontal_split[0]);
    logs::render(app, frame, horizontal_split[1]);
}
