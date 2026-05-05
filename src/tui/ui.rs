use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
};

use crate::tui::{
    app::{App, CurrentScreen},
    components::{dispatch, logs, runlist, workflowlist},
};

pub fn ui(app: &App, frame: &mut Frame) {
    match app.current_screen {
        CurrentScreen::RepoList => {}
        CurrentScreen::WorkflowManager => {
            workflow_manager_layout(app, frame);
        }
    }
}
fn workflow_manager_layout(app: &App, frame: &mut Frame) {
    let main_rects = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Max(30), Constraint::Min(40)])
        .split(frame.area());
    let left_rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_rects[0]);
    let right_rects = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
        .split(main_rects[1]);

    workflowlist::render(app, frame, left_rects[0]);
    dispatch::render(app, frame, left_rects[1]);
    runlist::render(app, frame, right_rects[1]);
    logs::render(app, frame, right_rects[0]);
}
