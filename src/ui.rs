use chrono::{DateTime, Local, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::search::SearchResult;
use crate::syntax::{classify_with, Class, CommandIndex};

const INPUT_HEIGHT: u16 = 3;
const PREVIEW_HEIGHT: u16 = 8;
const PREVIEW_LINES: usize = 6;
const HELP_HEIGHT: u16 = 1;

const COLOR_ACCENT: Color = Color::Cyan;
const COLOR_MUTED: Color = Color::DarkGray;
const COLOR_TEXT: Color = Color::White;
const COLOR_MATCH: Color = Color::Yellow;
const COLOR_SELECTED_BG: Color = Color::Rgb(40, 44, 52);
// Distinct from COLOR_ACCENT on purpose: the chrome owns teal/cyan, the
// command word in results owns blue, so UI and content never blur together.
const COLOR_SYNTAX_CMD: Color = Color::Rgb(137, 180, 250);

// Syntax palette: deliberately quiet. The command word carries the accent,
// strings and variables get one muted hue each, structure fades back.
fn class_style(class: Class) -> Style {
    match class {
        Class::Command => Style::default().fg(COLOR_SYNTAX_CMD),
        Class::Flag => Style::default().fg(Color::Gray),
        Class::Str => Style::default().fg(Color::Green),
        Class::Var => Style::default().fg(Color::Magenta),
        Class::Operator => Style::default().fg(COLOR_MUTED),
        Class::Plain => Style::default().fg(COLOR_TEXT),
    }
}

fn format_relative_time(timestamp: Option<i64>, now: i64) -> Option<String> {
    let ts = timestamp?;
    let dt = DateTime::from_timestamp(ts, 0)?;
    let now_dt = DateTime::from_timestamp(now, 0)?;
    let duration = now_dt.signed_duration_since(dt);

    let seconds = duration.num_seconds();
    if seconds < 0 {
        return Some("future".to_string());
    }

    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    Some(match () {
        _ if seconds < 60 => "just now".to_string(),
        _ if minutes < 60 => format!("{}m ago", minutes),
        _ if hours < 24 => format!("{}h ago", hours),
        _ if days == 1 => "yesterday".to_string(),
        _ if days < 7 => format!("{} days", days),
        _ if days < 30 => {
            let weeks = days / 7;
            if weeks == 1 {
                "last week".to_string()
            } else {
                format!("{} weeks", weeks)
            }
        }
        _ if days < 365 => {
            let local: DateTime<Local> = dt.into();
            local.format("%b %d").to_string()
        }
        _ => {
            let local: DateTime<Local> = dt.into();
            local.format("%b %Y").to_string()
        }
    })
}

pub struct UI {
    commands: CommandIndex,
}

impl UI {
    pub fn new() -> Self {
        Self {
            commands: CommandIndex::from_path(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        frame: &mut Frame,
        query: &str,
        results: &[SearchResult],
        selected_index: usize,
        scroll_offset: usize,
        list_state: &mut ListState,
        status_message: Option<&str>,
        syntax_enabled: bool,
    ) -> usize {
        let selected_command = results.get(selected_index).map(|r| &r.entry.command);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(INPUT_HEIGHT),
                Constraint::Min(1),
                Constraint::Length(PREVIEW_HEIGHT),
                Constraint::Length(HELP_HEIGHT),
            ])
            .split(frame.area());

        self.render_input(frame, chunks[0], query);
        let new_offset = self.render_results(
            frame,
            chunks[1],
            results,
            selected_index,
            scroll_offset,
            list_state,
            status_message,
            syntax_enabled,
        );

        if let Some(cmd) = selected_command {
            self.render_preview(frame, chunks[2], cmd, syntax_enabled);
        } else {
            self.render_empty_preview(frame, chunks[2]);
        }
        self.render_help_bar(frame, chunks[3]);

        new_offset
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect, command: &str, syntax_enabled: bool) {
        let inner_width = area.width.saturating_sub(2) as usize;
        let lines = preview_lines(command, inner_width, syntax_enabled, Some(&self.commands));

        let preview = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_MUTED))
                .title(" Preview "),
        );

        frame.render_widget(preview, area);
    }

    fn render_empty_preview(&self, frame: &mut Frame, area: Rect) {
        let preview = Paragraph::new("").block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_MUTED))
                .title(" Preview "),
        );

        frame.render_widget(preview, area);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect, query: &str) {
        let input_text = Line::from(vec![
            Span::styled("> ", Style::default().fg(COLOR_ACCENT)),
            Span::raw(query),
            Span::styled("_", Style::default().fg(Color::Gray)),
        ]);

        let input = Paragraph::new(input_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_ACCENT))
                .title(" ihistory "),
        );

        frame.render_widget(input, area);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_results(
        &self,
        frame: &mut Frame,
        area: Rect,
        results: &[SearchResult],
        selected_index: usize,
        scroll_offset: usize,
        list_state: &mut ListState,
        status_message: Option<&str>,
        syntax_enabled: bool,
    ) -> usize {
        let visible_height = area.height.saturating_sub(2) as usize;

        let mut new_offset = scroll_offset;
        if selected_index < new_offset {
            new_offset = selected_index;
        } else if selected_index >= new_offset + visible_height {
            new_offset = selected_index.saturating_sub(visible_height.saturating_sub(1));
        }
        new_offset = new_offset.min(results.len().saturating_sub(visible_height));

        let start = new_offset;
        let end = (start + visible_height).min(results.len());
        let available_width = area.width.saturating_sub(4) as usize;
        let now = Utc::now().timestamp();

        let items: Vec<ListItem> = results[start..end]
            .iter()
            .enumerate()
            .map(|(i, result)| {
                let actual_index = start + i;
                let is_selected = actual_index == selected_index;

                let line = render_command_line(
                    &result.entry.command,
                    &result.indices,
                    result.entry.timestamp,
                    is_selected,
                    available_width,
                    now,
                    syntax_enabled,
                    Some(&self.commands),
                );

                let style = if is_selected {
                    Style::default()
                        .bg(COLOR_SELECTED_BG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                ListItem::new(line).style(style)
            })
            .collect();

        let (title, border_style) = if let Some(msg) = status_message {
            (format!(" {} ", msg), Style::default().fg(Color::Red))
        } else {
            (
                format!(" {} results ", results.len()),
                Style::default().fg(COLOR_MUTED),
            )
        };

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        );

        list_state.select(Some(selected_index - start));
        frame.render_stateful_widget(list, area, list_state);
        new_offset
    }

    fn render_help_bar(&self, frame: &mut Frame, area: Rect) {
        let help = Paragraph::new(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" move ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Enter", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" select ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Tab", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" run ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Ctrl+D", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" hide ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Ctrl+T", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" toggle color ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Esc", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" quit", Style::default().fg(COLOR_MUTED)),
        ]));

        frame.render_widget(help, area);
    }
}

/// Wraps the command for the preview pane, carrying syntax colors across
/// wrapped lines. textwrap only elides whitespace at break points, so a
/// pointer into the original chars recovers each wrapped char's class.
fn preview_lines(
    command: &str,
    width: usize,
    syntax_enabled: bool,
    index: Option<&CommandIndex>,
) -> Vec<Line<'static>> {
    let wrapped = textwrap::wrap(command, width.max(1));
    let plain = Style::default().fg(COLOR_TEXT);
    if !syntax_enabled {
        return wrapped
            .iter()
            .take(PREVIEW_LINES)
            .map(|s| Line::from(Span::styled(s.to_string(), plain)))
            .collect();
    }

    let chars: Vec<char> = command.chars().collect();
    let classes = classify_with(command, index);
    let mut pos = 0usize;
    wrapped
        .iter()
        .take(PREVIEW_LINES)
        .map(|piece| {
            let mut spans = Vec::new();
            let mut run = String::new();
            let mut run_style = plain;
            for pc in piece.chars() {
                while pos < chars.len() && chars[pos] != pc {
                    pos += 1;
                }
                let style = if pos < chars.len() {
                    class_style(classes[pos])
                } else {
                    plain
                };
                if pos < chars.len() {
                    pos += 1;
                }
                if style != run_style && !run.is_empty() {
                    spans.push(Span::styled(std::mem::take(&mut run), run_style));
                }
                run_style = style;
                run.push(pc);
            }
            if !run.is_empty() {
                spans.push(Span::styled(run, run_style));
            }
            Line::from(spans)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn render_command_line(
    command: &str,
    match_indices: &[usize],
    timestamp: Option<i64>,
    is_selected: bool,
    available_width: usize,
    now: i64,
    syntax_enabled: bool,
    index: Option<&CommandIndex>,
) -> Line<'static> {
    let prefix_style = Style::default().fg(if is_selected {
        COLOR_ACCENT
    } else {
        COLOR_MUTED
    });
    let normal_style = Style::default().fg(COLOR_TEXT);
    let match_style = Style::default()
        .fg(COLOR_MATCH)
        .add_modifier(Modifier::BOLD);
    let time_style = Style::default().fg(COLOR_MUTED);

    let time_str = format_relative_time(timestamp, now);
    let time_width = time_str.as_ref().map(|s| s.width() + 2).unwrap_or(0);
    let prefix_width = 2;
    let max_cmd_width = available_width.saturating_sub(prefix_width + time_width);

    // The command is truncated by display width, never by byte offset: byte
    // slicing panics inside multi-byte characters, and column math has to
    // count double-width glyphs as two cells. The matcher hands out char
    // indices, so highlighting walks chars as well.
    let budget = max_cmd_width.saturating_sub(3);
    let matches: std::collections::HashSet<usize> = match_indices.iter().copied().collect();
    let classes = classify_with(command, index);
    let mut kept: Vec<(char, Style)> = Vec::new();
    let mut used_width = 0;
    let mut needs_truncation = false;
    for (i, ch) in command.chars().enumerate() {
        let w = ch.width().unwrap_or(0);
        if used_width + w > budget {
            needs_truncation = true;
            break;
        }
        used_width += w;
        // Three layers, in precedence order: the match highlight wins over
        // the syntax color, which replaces the flat text color.
        let style = if matches.contains(&i) {
            match_style
        } else if syntax_enabled {
            class_style(classes[i])
        } else {
            normal_style
        };
        kept.push((ch, style));
    }

    let mut spans = Vec::with_capacity(8);
    spans.push(Span::styled(
        if is_selected { "> " } else { "  " },
        prefix_style,
    ));

    // Consecutive chars with the same styling collapse into one span.
    let mut run = String::new();
    let mut run_style = normal_style;
    for &(ch, style) in &kept {
        if style != run_style && !run.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut run), run_style));
        }
        run_style = style;
        run.push(ch);
    }
    if !run.is_empty() {
        spans.push(Span::styled(run, run_style));
    }

    if needs_truncation {
        spans.push(Span::styled("...", normal_style));
    }

    if let Some(time) = time_str {
        let current_width: usize = spans.iter().map(|s| s.content.as_ref().width()).sum();
        let padding_needed = available_width.saturating_sub(current_width + time.width());
        if padding_needed > 0 {
            spans.push(Span::raw(" ".repeat(padding_needed)));
        }
        spans.push(Span::styled(time, time_style));
    }

    Line::from(spans)
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn matched_text(line: &Line) -> String {
        line.spans
            .iter()
            .filter(|s| s.style.fg == Some(COLOR_MATCH))
            .map(|s| s.content.as_ref())
            .collect()
    }

    #[test]
    fn truncates_non_ascii_without_panicking() {
        // Byte-based truncation used to slice inside 'ö' and panic.
        let cmd = "gít cömmit -m 'ünïcöde chängé' --amend --no-verify";
        let line = render_command_line(cmd, &[], None, false, 24, 0, true, None);
        let text = text_of(&line);
        assert!(text.contains("..."));
        assert!(text.starts_with("  gít cömmit"));
    }

    #[test]
    fn highlights_char_indices_not_bytes() {
        // "é" is two bytes; a byte-based highlighter would mark the wrong
        // columns for every index after it.
        let cmd = "économie status";
        let indices = [0, 9, 10]; // é, s, t (char positions)
        let line = render_command_line(cmd, &indices, None, false, 80, 0, true, None);
        assert_eq!(matched_text(&line), "ést");
    }

    #[test]
    fn counts_wide_glyphs_as_two_cells() {
        // Four CJK chars occupy eight cells; a char-counting truncation
        // would overrun the column budget.
        let cmd = "echo 日本語検索 && ls";
        let line = render_command_line(cmd, &[], None, false, 16, 0, true, None);
        let text = text_of(&line);
        assert!(text.contains("..."));
        let width: usize = text.width();
        assert!(width <= 16, "rendered width {} exceeds budget", width);
    }

    #[test]
    fn pads_the_timestamp_by_display_width() {
        let now = 1_700_000_000;
        let ascii = render_command_line("ls -la", &[], Some(now - 30), false, 40, now, true, None);
        let unicode =
            render_command_line("ls -lä", &[], Some(now - 30), false, 40, now, true, None);
        assert_eq!(text_of(&ascii).width(), text_of(&unicode).width());
    }

    #[test]
    fn preview_carries_colors_across_wrapped_lines() {
        // The quoted string spans the wrap point; both halves stay Str-green.
        let cmd = r#"git commit -m "a rather long commit message here""#;
        let lines = preview_lines(cmd, 30, true, None);
        assert!(lines.len() >= 2);
        let first_git = lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("git"))
            .unwrap();
        assert_eq!(first_git.style.fg, Some(COLOR_SYNTAX_CMD));
        let tail = lines.last().unwrap();
        for span in tail.spans.iter().filter(|s| !s.content.trim().is_empty()) {
            assert_eq!(span.style.fg, Some(Color::Green), "span {:?}", span.content);
        }
    }

    #[test]
    fn preview_toggle_off_is_flat() {
        let lines = preview_lines("git status", 40, false, None);
        assert_eq!(lines[0].spans[0].style.fg, Some(COLOR_TEXT));
    }

    #[test]
    fn toggle_off_renders_flat_color() {
        let line = render_command_line(
            "brew update && brew upgrade",
            &[],
            None,
            false,
            80,
            0,
            false,
            None,
        );
        for span in line.spans.iter().skip(1) {
            assert_eq!(span.style.fg, Some(COLOR_TEXT), "span {:?}", span.content);
        }
    }

    #[test]
    fn fits_short_commands_without_ellipsis() {
        let line = render_command_line("ls", &[], None, false, 40, 0, true, None);
        assert!(!text_of(&line).contains("..."));
    }
}
