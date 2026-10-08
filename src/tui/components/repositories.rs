use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::tui::{
    app::App,
    repo_picker::{RepoEntry, RepoPicker},
};

/// The searchable list of all repositories, in place of the panes.
pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    if let Some(picker) = app.picker.as_mut() {
        render_picker(picker, frame, area);
    }
}

fn render_picker(picker: &mut RepoPicker, frame: &mut Frame, area: Rect) {
    let notice_rows = picker.notices.len().min(4) as u16;
    let [list_area, notices_area, search_area] = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(notice_rows),
        Constraint::Length(1),
    ])
    .areas(area);

    let visible = picker.visible();
    // inside the borders, the highlight takes no extra columns
    let width = list_area.width.saturating_sub(2) as usize;
    let items: Vec<ListItem> = visible
        .iter()
        .map(|entry| ListItem::new(entry_line(entry, width)))
        .collect();
    let title = format!(" Repositories ({}/{}) ", visible.len(), picker.total());
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .title(Line::from(title).centered()),
        )
        .highlight_style(Style::default().bg(Color::White).fg(Color::Black));
    frame.render_stateful_widget(list, list_area, &mut picker.state);

    let notices: Vec<Line> = picker
        .notices
        .iter()
        .take(4)
        .map(|n| Line::styled(n.clone(), Style::default().fg(Color::Red)))
        .collect();
    frame.render_widget(Paragraph::new(notices), notices_area);

    let search = if picker.searching || !picker.query.is_empty() {
        Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(picker.query.clone()),
        ])
    } else {
        Line::styled("press / to search", Style::default().fg(Color::DarkGray))
    };
    frame.render_widget(Paragraph::new(search), search_area);
    if picker.searching {
        let x = search_area.x + 1 + picker.query.chars().count() as u16;
        frame.set_cursor_position(Position::new(
            x.min(search_area.right().saturating_sub(1)),
            search_area.y,
        ));
    }
}

/// `owner/repo  description .......... profile`, cut to `width` columns.
fn entry_line(entry: &RepoEntry, width: usize) -> Line<'static> {
    let lock = if entry.summary.private { "🔒" } else { "  " };
    let name = format!(" {lock} {}", entry.full_name());
    let profile = format!("{} ", entry.profile);
    let description = entry.summary.description.clone().unwrap_or_default();

    let used = name.chars().count() + 2 + profile.chars().count();
    let room = width.saturating_sub(used + 1);
    let description: String = if description.chars().count() > room {
        description
            .chars()
            .take(room.saturating_sub(1))
            .chain(std::iter::once('…'))
            .collect()
    } else {
        description
    };
    let filler = width.saturating_sub(used + description.chars().count());

    Line::from(vec![
        Span::styled(name, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(description, Style::default().fg(Color::Gray)),
        Span::raw(" ".repeat(filler)),
        Span::styled(profile, Style::default().fg(Color::DarkGray)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Repository, RepositorySummary};
    use ratatui::{Terminal, backend::TestBackend};

    fn entry(profile: &str, owner: &str, repo: &str, description: Option<&str>) -> RepoEntry {
        RepoEntry {
            profile: profile.into(),
            summary: RepositorySummary {
                repository: Repository::new("github.com".into(), owner.into(), repo.into()),
                private: false,
                description: description.map(Into::into),
            },
        }
    }

    fn draw(picker: &mut RepoPicker, width: u16, height: u16) -> (Vec<String>, Position) {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| render_picker(picker, f, f.area()))
            .unwrap();
        let cursor = terminal.get_cursor_position().unwrap();
        let buffer = terminal.backend().buffer().clone();
        let rows = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect();
        (rows, cursor)
    }

    fn picker() -> RepoPicker {
        RepoPicker::new(
            vec![
                entry(
                    "personal",
                    "me",
                    "actiontui",
                    Some("A TUI for GitHub Actions"),
                ),
                entry("work", "acme", "api", None),
            ],
            vec!["broken: Bad credentials".into()],
        )
    }

    #[test]
    fn shows_repositories_profiles_and_the_count() {
        let (rows, _) = draw(&mut picker(), 70, 10);
        let text = rows.join("\n");
        assert!(text.contains("Repositories (2/2)"), "{text}");
        assert!(text.contains("me/actiontui"), "{text}");
        assert!(text.contains("A TUI for GitHub Actions"), "{text}");
        assert!(text.contains("personal"), "{text}");
        assert!(text.contains("acme/api"), "{text}");
        assert!(text.contains("broken: Bad credentials"), "{text}");
        assert!(text.contains("press / to search"), "{text}");
    }

    #[test]
    fn the_search_filters_and_shows_the_cursor() {
        let mut picker = picker();
        picker.searching = true;
        picker.query = "acme".into();
        let (rows, cursor) = draw(&mut picker, 70, 10);
        let text = rows.join("\n");
        assert!(text.contains("Repositories (1/2)"), "{text}");
        assert!(!text.contains("me/actiontui"), "{text}");
        let last = rows.last().unwrap();
        assert!(last.starts_with("/acme"), "{last:?}");
        assert_eq!((cursor.x, cursor.y), (5, 9));
    }

    #[test]
    fn long_descriptions_are_cut_and_the_profile_stays_visible() {
        let mut picker = RepoPicker::new(
            vec![entry("work", "acme", "api", Some(&"very long ".repeat(20)))],
            Vec::new(),
        );
        let (rows, _) = draw(&mut picker, 50, 6);
        let line = rows.iter().find(|r| r.contains("acme/api")).unwrap();
        assert!(line.contains('…'), "{line}");
        assert!(line.contains("work"), "{line}");
    }

    #[test]
    fn fits_a_tiny_terminal_without_panicking() {
        draw(&mut picker(), 20, 3);
    }
}
