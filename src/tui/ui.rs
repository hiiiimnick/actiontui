use ratatui::{
    Frame,
    layout::{Constraint, Layout},
};

use crate::tui::{
    app::App,
    components::{joblist, logs, runlist, steps, workflowlist},
};

pub fn ui(app: &mut App, frame: &mut Frame) {
    let main_rects = Layout::vertical([
        Constraint::Max(3),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Min(8),
        Constraint::Max(1),
    ])
    .split(frame.area());

    let horizontal_split =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_rects[2]);

    workflowlist::render(app, frame, main_rects[0]);
    runlist::render(app, frame, main_rects[1]);
    joblist::render(app, frame, horizontal_split[0]);
    steps::render(app, frame, horizontal_split[1]);
    logs::render(app, frame, main_rects[3]);
}
