use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::{
    app::{App, CurrentFocus},
    util::map_block_color,
};

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let height = area.height.saturating_sub(2) as usize;
    let max_offset = app.log_lines.len().saturating_sub(height) as u64;
    app.logs_offset = app.logs_offset.min(max_offset);

    let title = app
        .selected_step
        .as_ref()
        .map(|s| format!("Logs - {}", s.name))
        .unwrap_or_else(|| "Logs".to_string());

    let placeholder = app.selected_step.is_some() && app.log_lines.is_empty();
    let mut lines: Vec<Line> = app
        .log_lines
        .iter()
        .skip(app.logs_offset as usize)
        .take(height)
        .map(|l| style_line(l))
        .collect();

    let shown = lines.len();
    if placeholder {
        lines.push(Line::styled(
            "No log output for this step (the job has not run yet, was skipped, or its logs expired)",
            Style::default().fg(Color::DarkGray),
        ));
    }

    let block = Block::default()
        .borders(Borders::all())
        .title(Line::from("[5]").left_aligned())
        .title(Line::from(title).centered())
        .title(
            Line::from(format!(
                "{}/{}",
                app.logs_offset as usize + shown,
                app.log_lines.len()
            ))
            .right_aligned(),
        )
        // style only the frame, so the log text keeps its own colors
        .border_style(map_block_color(app, CurrentFocus::Logs))
        .title_style(map_block_color(app, CurrentFocus::Logs));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Strips the timestamp from a raw GitHub log line, colors the `##[...]`
/// workflow commands and renders embedded ANSI escape sequences as styles.
fn style_line(raw: &str) -> Line<'static> {
    let text = strip_timestamp(raw.trim_start_matches('\u{feff}'));
    let (content, prefix, base) = if let Some(rest) = text.strip_prefix("##[group]") {
        (rest, "▶ ", Style::default().add_modifier(Modifier::BOLD))
    } else if text.starts_with("##[endgroup]") {
        return Line::default();
    } else if let Some(rest) = text.strip_prefix("##[error]") {
        (rest, "", Style::default().fg(Color::Red))
    } else if let Some(rest) = text.strip_prefix("##[warning]") {
        (rest, "", Style::default().fg(Color::Yellow))
    } else if let Some(rest) = text.strip_prefix("##[debug]") {
        (rest, "", Style::default().fg(Color::DarkGray))
    } else {
        (text, "", Style::default())
    };

    let mut spans = Vec::new();
    if !prefix.is_empty() {
        spans.push(Span::styled(prefix, base));
    }
    spans.extend(parse_ansi(content, base));
    Line::from(spans)
}

fn strip_timestamp(line: &str) -> &str {
    match line.split_once("Z ") {
        Some((ts, rest)) if ts.len() <= 30 && ts.starts_with(|c: char| c.is_ascii_digit()) => rest,
        _ => line,
    }
}

/// Converts text with ANSI SGR sequences into styled spans. Other escape
/// sequences and control characters are dropped.
fn parse_ansi(text: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut style = base;
    let mut buf = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => {
                if chars.peek() != Some(&'[') {
                    continue;
                }
                chars.next();
                let mut params = String::new();
                let mut last = None;
                for c in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&c) {
                        last = Some(c);
                        break;
                    }
                    params.push(c);
                }
                if last == Some('m') {
                    if !buf.is_empty() {
                        spans.push(Span::styled(std::mem::take(&mut buf), style));
                    }
                    style = apply_sgr(style, &params, base);
                }
            }
            // tabs would break ratatui's cell width accounting
            '\t' => buf.push_str("    "),
            c if c.is_control() => {}
            c => buf.push(c),
        }
    }
    if !buf.is_empty() {
        spans.push(Span::styled(buf, style));
    }
    spans
}

fn apply_sgr(mut style: Style, params: &str, base: Style) -> Style {
    let codes: Vec<u16> = if params.is_empty() {
        vec![0]
    } else {
        params
            .split([';', ':'])
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };

    let mut i = 0;
    while i < codes.len() {
        match codes[i] {
            0 => style = base,
            1 => style = style.add_modifier(Modifier::BOLD),
            2 => style = style.add_modifier(Modifier::DIM),
            3 => style = style.add_modifier(Modifier::ITALIC),
            4 => style = style.add_modifier(Modifier::UNDERLINED),
            7 => style = style.add_modifier(Modifier::REVERSED),
            9 => style = style.add_modifier(Modifier::CROSSED_OUT),
            22 => style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            23 => style = style.remove_modifier(Modifier::ITALIC),
            24 => style = style.remove_modifier(Modifier::UNDERLINED),
            27 => style = style.remove_modifier(Modifier::REVERSED),
            29 => style = style.remove_modifier(Modifier::CROSSED_OUT),
            n @ 30..=37 => style.fg = Some(ansi_color(n - 30)),
            n @ 90..=97 => style.fg = Some(ansi_color(n - 90 + 8)),
            39 => style.fg = base.fg,
            n @ 40..=47 => style.bg = Some(ansi_color(n - 40)),
            n @ 100..=107 => style.bg = Some(ansi_color(n - 100 + 8)),
            49 => style.bg = base.bg,
            n @ (38 | 48) => {
                let color = match codes.get(i + 1) {
                    Some(5) => codes.get(i + 2).map(|&c| Color::Indexed(c as u8)),
                    Some(2) => match codes.get(i + 2..i + 5) {
                        Some(&[r, g, b]) => Some(Color::Rgb(r as u8, g as u8, b as u8)),
                        _ => None,
                    },
                    _ => None,
                };
                i += if codes.get(i + 1) == Some(&2) { 4 } else { 2 };
                match (color, n) {
                    (Some(c), 38) => style.fg = Some(c),
                    (Some(c), _) => style.bg = Some(c),
                    _ => {}
                }
            }
            _ => {}
        }
        i += 1;
    }
    style
}

fn ansi_color(n: u16) -> Color {
    match n {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        8 => Color::DarkGray,
        9 => Color::LightRed,
        10 => Color::LightGreen,
        11 => Color::LightYellow,
        12 => Color::LightBlue,
        13 => Color::LightMagenta,
        14 => Color::LightCyan,
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_one_span() {
        let spans = parse_ansi("hello", Style::default());
        assert_eq!(spans, vec![Span::raw("hello")]);
    }

    #[test]
    fn colors_and_reset() {
        let spans = parse_ansi("\x1b[31;1merr\x1b[0m ok", Style::default());
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "err");
        assert_eq!(spans[0].style.fg, Some(Color::Red));
        assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        assert_eq!(spans[1].content, " ok");
        assert_eq!(spans[1].style, Style::default());
    }

    #[test]
    fn extended_colors() {
        let spans = parse_ansi("\x1b[38;5;208ma\x1b[48;2;1;2;3mb", Style::default());
        assert_eq!(spans[0].style.fg, Some(Color::Indexed(208)));
        assert_eq!(spans[1].style.fg, Some(Color::Indexed(208)));
        assert_eq!(spans[1].style.bg, Some(Color::Rgb(1, 2, 3)));
    }

    #[test]
    fn reset_restores_base_style() {
        let base = Style::default().fg(Color::Red);
        let spans = parse_ansi("\x1b[32ma\x1b[39mb", base);
        assert_eq!(spans[0].style.fg, Some(Color::Green));
        assert_eq!(spans[1].style.fg, Some(Color::Red));
    }

    #[test]
    fn non_sgr_sequences_and_control_chars_are_dropped() {
        let spans = parse_ansi("a\x1b[2Kb\rc\x1b", Style::default());
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(text, "abc");
    }
}
