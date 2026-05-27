use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::{
    domain::Run,
    tui::{
        app::{App, CurrentFocus},
        util::{map_block_color, map_status_to_span},
    },
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(map_block_color(app, CurrentFocus::Runs))
        .title(Line::from("[2]").left_aligned())
        .title(Line::from("Runs").left_aligned());

    let inner_area = block.inner(area);
    let inner_width = (inner_area.width - 4) as usize;

    let list_items: Vec<ListItem> = app
        .runs
        .iter()
        .map(|run| {
            let status = map_status_to_span(&run.status, run.conclusion.as_deref());

            if let Some(selected) = app.selected_run_id
                && run.id == selected
            {
                let line_content = format_line(run, inner_width - 2);
                return ListItem::new(Line::from(vec![
                    status,
                    Span::raw("[").style(Color::Yellow),
                    Span::raw(line_content).style(Color::Yellow),
                    Span::raw("]").style(Color::Yellow),
                ]));
            }

            let line_content = format_line(run, inner_width);

            ListItem::new(Line::from(vec![status, Span::raw(line_content)]))
        })
        .collect();

    let list = List::new(list_items)
        .block(block)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().bg(Color::White).fg(Color::Black));

    frame.render_stateful_widget(list, area, &mut app.run_state);
}

fn format_line(run: &Run, available_width: usize) -> String {
    let time: String = if let Some(op_time) = run.created_at {
        op_time.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        String::from("")
    };
    let time_len = time.chars().count();
    let gap_len = 1;

    // Fixed points:
    // Start: [Status(2)][Prefix(2)] (indices 0-3)
    // End: [Time(19)] (indices available_width-19 to available_width-1)
    let title_start = 0;
    let time_start = available_width.saturating_sub(time_len);

    if time_start <= title_start + gap_len {
        // Extremely narrow: just show title prefix or nothing
        return "  ".to_string();
    }

    let mut title = run.display_title.clone();
    let mut branch = run.head_branch.clone();

    let total_middle_space = time_start.saturating_sub(title_start);

    // Reserve some space for title if branch is huge
    let max_branch_len = total_middle_space.saturating_sub(gap_len + 5);
    if branch.chars().count() > max_branch_len && max_branch_len > 0 {
        branch = branch
            .chars()
            .take(max_branch_len.saturating_sub(1))
            .collect::<String>()
            + "…";
    }
    let actual_branch_len = branch.chars().count();

    // Target branch start (centered in the whole line)
    let mut branch_start = (available_width / 2).saturating_sub(actual_branch_len / 2);

    // Keep branch within bounds [title_start + gap + min_title(1), time_start - branch_len]
    if branch_start + actual_branch_len > time_start {
        branch_start = time_start.saturating_sub(actual_branch_len);
    }
    if branch_start < title_start + gap_len + 1 {
        branch_start = title_start + gap_len + 1;
    }

    // Truncate title to fit before branch
    let max_title_len = branch_start.saturating_sub(title_start + gap_len);
    if title.chars().count() > max_title_len {
        if max_title_len > 0 {
            title = title
                .chars()
                .take(max_title_len.saturating_sub(1))
                .collect::<String>()
                + "…";
        } else {
            title = String::new();
        }
    }

    let actual_title_len = title.chars().count();
    let title_padding_len = branch_start.saturating_sub(title_start + actual_title_len + gap_len);
    let branch_padding_len = time_start.saturating_sub(branch_start + actual_branch_len);

    format!(
        "{} {}{}{}{}",
        title,
        " ".repeat(title_padding_len),
        branch,
        " ".repeat(branch_padding_len),
        time
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_format_line_space_before_branch() {
        let run = Run {
            id: 1,
            name: "test".to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            workflow_id: 1,
            html_url: "url".to_string(),
            created_at: chrono::Local
                .with_ymd_and_hms(2024, 5, 23, 12, 0, 0)
                .unwrap(),
            display_title: "A very long title that would previously have no space".to_string(),
            head_branch: "main".to_string(),
        };

        // Test with a small line size to force title_padding to be empty
        let line = format_line(&run, 60);

        // Find position of "main" in characters
        let pos = line
            .chars()
            .collect::<Vec<_>>()
            .windows(4)
            .position(|w| w == ['m', 'a', 'i', 'n'])
            .unwrap();
        let char_before_branch = line.chars().nth(pos - 1).unwrap();

        assert_eq!(
            char_before_branch, ' ',
            "Should have a space before the branch name"
        );
    }

    #[test]
    fn test_format_line_unicode_alignment() {
        let run1 = Run {
            id: 1,
            display_title: "Short title".to_string(),
            head_branch: "main".to_string(),
            created_at: chrono::Local
                .with_ymd_and_hms(2024, 5, 23, 12, 0, 0)
                .unwrap(),
            ..Default::default()
        };
        let run2 = Run {
            id: 2,
            display_title: "Title with ellipsis…".to_string(), // … is 3 bytes but 1 char
            head_branch: "main".to_string(),
            created_at: chrono::Local
                .with_ymd_and_hms(2024, 5, 23, 12, 0, 0)
                .unwrap(),
            ..Default::default()
        };

        let line1 = format_line(&run1, 100);
        let line2 = format_line(&run2, 100);

        // Find position of "main" in characters
        let pos1 = line1
            .chars()
            .collect::<Vec<_>>()
            .windows(4)
            .position(|w| w == ['m', 'a', 'i', 'n'])
            .unwrap();
        let pos2 = line2
            .chars()
            .collect::<Vec<_>>()
            .windows(4)
            .position(|w| w == ['m', 'a', 'i', 'n'])
            .unwrap();

        assert_eq!(
            pos1, pos2,
            "Branch should be visually aligned (same character position) regardless of unicode characters in title"
        );

        // Also check that there is a space before "main"
        assert_eq!(line1.chars().nth(pos1 - 1).unwrap(), ' ');
        assert_eq!(line2.chars().nth(pos2 - 1).unwrap(), ' ');
    }
}
