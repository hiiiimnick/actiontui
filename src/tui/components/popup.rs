use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::domain::InputKind;
use crate::tui::{
    app::{App, Mode},
    run_form::{Field, RunForm},
};

/// The popup of the current mode: the run form or a confirmation.
pub fn render(app: &App, frame: &mut Frame) {
    match (&app.mode, &app.run_form, &app.confirmation) {
        (Mode::Input, Some(form), _) => render_form(form, frame),
        (Mode::Confirm, _, Some(confirmation)) => render_confirmation(&confirmation.prompt, frame),
        _ => {}
    }
}

fn render_confirmation(prompt: &str, frame: &mut Frame) {
    let width = (prompt.chars().count() as u16 + 4).max(30);
    let area = centered(frame.area(), width, 3);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Line::from(" Confirm ").centered())
        .title_bottom(Line::from(" y yes  n no ").right_aligned());
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(Line::from(prompt.to_string()).centered()).block(block),
        area,
    );
}

fn render_form(form: &RunForm, frame: &mut Frame) {
    // one row per field (as many as fit) plus one row for description/notice
    let visible = form
        .fields
        .len()
        .min(frame.area().height.saturating_sub(5).max(1) as usize);
    let area = centered(frame.area(), 70, visible as u16 + 3);
    let start = (form.selected + 1).saturating_sub(visible);

    let hints = if form.editing {
        " Esc/Enter done  Ctrl-u clear "
    } else {
        " j/k move  i edit  h/l change  Enter run  Esc cancel "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Line::from(format!(" Run '{}' ", form.workflow_name)).centered())
        .title_bottom(Line::from(hints).right_aligned());
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let label_width = form
        .fields
        .iter()
        .map(|f| f.label.chars().count())
        .max()
        .unwrap_or(0);

    let mut lines: Vec<Line> = form
        .fields
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(i, field)| field_line(form, i, field, label_width))
        .collect();
    lines.push(footer_line(form));
    frame.render_widget(Paragraph::new(lines), inner);

    if form.editing {
        let prefix = prefix_width(label_width);
        let value_width = form.fields[form.selected].value.chars().count();
        let x = (inner.x as usize + prefix + value_width).min(inner.right() as usize - 1);
        let y = inner.y + (form.selected - start) as u16;
        frame.set_cursor_position(Position::new(x as u16, y));
    }
}

/// Width of `> label* ` in front of the value, see `field_line`.
fn prefix_width(label_width: usize) -> usize {
    2 + label_width + 1 + 1
}

fn field_line<'a>(form: &RunForm, index: usize, field: &Field, label_width: usize) -> Line<'a> {
    let selected = index == form.selected;
    let marker = if selected { "> " } else { "  " };
    let star = if field.required { "*" } else { " " };
    let value = match &field.kind {
        InputKind::Boolean => if field.value == "true" { "[x]" } else { "[ ]" }.to_string(),
        InputKind::Choice(_) => format!("< {} >", field.value),
        InputKind::Text | InputKind::Number => field.value.clone(),
    };

    let style = if selected {
        let color = if form.editing {
            Color::Yellow
        } else {
            Color::White
        };
        Style::default().fg(Color::Black).bg(color)
    } else {
        Style::default()
    };
    Line::from(vec![
        Span::styled(
            format!("{marker}{:<label_width$}{star} ", field.label),
            style.add_modifier(Modifier::BOLD),
        ),
        Span::styled(value, style),
    ])
    .style(style)
}

fn footer_line<'a>(form: &RunForm) -> Line<'a> {
    if let Some(notice) = &form.notice {
        return Line::styled(notice.clone(), Style::default().fg(Color::Red));
    }
    let description = form.fields[form.selected].description.clone();
    Line::styled(
        description.unwrap_or_default(),
        Style::default().fg(Color::Gray),
    )
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let [row] = Layout::vertical([Constraint::Length(height.min(area.height))])
        .flex(Flex::Center)
        .areas(area);
    let [rect] = Layout::horizontal([Constraint::Length(width.min(area.width))])
        .flex(Flex::Center)
        .areas(row);
    rect
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::WorkflowInput;
    use ratatui::{Terminal, backend::TestBackend};

    fn form() -> RunForm {
        RunForm::new(
            1,
            "Deploy".into(),
            "main".into(),
            vec![
                WorkflowInput {
                    name: "environment".into(),
                    description: Some("Where to deploy".into()),
                    required: true,
                    default: Some("staging".into()),
                    kind: InputKind::Choice(vec!["dev".into(), "staging".into()]),
                },
                WorkflowInput {
                    name: "dry_run".into(),
                    description: None,
                    required: false,
                    default: Some("true".into()),
                    kind: InputKind::Boolean,
                },
                WorkflowInput {
                    name: "version".into(),
                    description: None,
                    required: false,
                    default: Some("1.2".into()),
                    kind: InputKind::Text,
                },
            ],
            None,
        )
    }

    fn draw(form: &RunForm, height: u16) -> (Vec<String>, Position) {
        let mut terminal = Terminal::new(TestBackend::new(80, height)).unwrap();
        terminal.draw(|f| render_form(form, f)).unwrap();
        let cursor = terminal.get_cursor_position().unwrap();
        let buffer = terminal.backend().buffer().clone();
        let rows = (0..height)
            .map(|y| {
                (0..80)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect();
        (rows, cursor)
    }

    #[test]
    fn renders_every_kind_of_field() {
        let (rows, _) = draw(&form(), 12);
        let text = rows.join("\n");
        assert!(text.contains("Run 'Deploy'"));
        assert!(text.contains("> branch"));
        assert!(text.contains("environment* < staging >"), "{text}");
        assert!(text.contains("dry_run      [x]"), "{text}");
        assert!(text.contains("version      1.2"), "{text}");
        assert!(text.contains("j/k move  i edit  h/l change"));
    }

    #[test]
    fn shows_description_of_selected_field() {
        let mut form = form();
        form.selected = 1;
        let (rows, _) = draw(&form, 12);
        assert!(rows.join("\n").contains("Where to deploy"));
    }

    #[test]
    fn cursor_is_placed_after_the_edited_value() {
        let mut form = form();
        form.selected = 3;
        form.editing = true;
        let (rows, cursor) = draw(&form, 12);
        let row = &rows[cursor.y as usize];
        // `find` is a byte offset, the border characters are multibyte
        let value_start = row[..row.find("1.2").unwrap()].chars().count();
        assert_eq!(cursor.x as usize, value_start + 3, "{row:?}");
    }

    #[test]
    fn scrolls_to_keep_the_selected_field_visible() {
        let inputs = (0..10)
            .map(|i| WorkflowInput {
                name: format!("input{i}"),
                description: None,
                required: false,
                default: None,
                kind: InputKind::Text,
            })
            .collect();
        let mut form = RunForm::new(1, "W".into(), "main".into(), inputs, None);
        form.selected = 10;
        let (rows, _) = draw(&form, 8);
        let text = rows.join("\n");
        assert!(text.contains("> input9"), "{text}");
        assert!(!text.contains("branch"), "{text}");
    }

    #[test]
    fn renders_the_confirmation() {
        let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
        terminal
            .draw(|f| render_confirmation("Rerun all jobs of 'CI'?", f))
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let text: String = (0..10)
            .flat_map(|y| (0..80).map(move |x| (x, y)))
            .map(|(x, y)| buffer[(x, y)].symbol().to_string())
            .collect();
        assert!(text.contains("Rerun all jobs of 'CI'?"));
        assert!(text.contains("y yes  n no"));
    }

    #[test]
    fn fits_a_tiny_terminal_without_panicking() {
        draw(&form(), 3);
    }
}
