use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Styled},
    text::Line,
    widgets::{Block, Borders, List, ListItem},
};

use crate::{
    domain::models::step,
    tui::{
        app::{App, CurrentFocus},
        util::map_block_color,
    },
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let selected_run = app.selected_run.as_ref();
    let current_focus = &app.current_focus;

    if *current_focus == CurrentFocus::Steps {
        render_steps(app, frame, area);
    } else {
        render_jobs(app, frame, area);
    }
    //
    // let list_items: Vec<ListItem> = app
    //     .workflows
    //     .iter()
    //     .map(|workflow| {
    //         let name = workflow.name.clone();
    //
    //         if let Some(selected) = selected_workflow
    //             && selected == workflow
    //         {
    //             return ListItem::new(format!("> {}", name)).style(Color::Yellow);
    //         }
    //         ListItem::new(name)
    //     })
    //     .collect();
    //
    // let list = List::new(list_items)
    //     .block(
    //         Block::default()
    //             .borders(Borders::ALL)
    //             .style(map_block_color(app, CurrentFocus::Jobs))
    //             .title(Line::from("[3]").left_aligned())
    //             .title(Line::from("Jobs").centered())
    //             .title(Line::from("Steps").centered()),
    //     )
    //     .style(Color::White)
    //     .highlight_style(Modifier::REVERSED);
    //
    // frame.render_stateful_widget(list, area, &mut app.workflow_state);
}

fn render_steps(app: &mut App, frame: &mut Frame, area: Rect) {
    let selected_job = app.selected_job.clone();
    let list_items: Vec<ListItem> = if let Some(job) = selected_job {
        job.steps
            .iter()
            .map(|step| ListItem::new(step.name.clone()))
            .collect()
    } else {
        Vec::new()
    };
}

fn render_jobs(app: &mut App, frame: &mut Frame, area: Rect) {
    let selected_job = app.selected_job.as_ref();
    let list_items: Vec<ListItem> = app
        .jobs
        .iter()
        .map(|job| {
            let name = job.name.clone();
            if let Some(selected) = selected_job
                && selected == job
            {
                return ListItem::new(format!("> {}", name)).style(Color::Yellow);
            }
            ListItem::new(name)
        })
        .collect();

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Line::from("[3]").left_aligned())
                .title(
                    Line::from("Jobs")
                        .style(map_block_color(app, CurrentFocus::Jobs))
                        .centered(),
                )
                .title(Line::from("Steps").centered()),
        )
        .style(Color::White)
        .highlight_style(Modifier::REVERSED);

    frame.render_stateful_widget(list, area, &mut app.job_state);
}
