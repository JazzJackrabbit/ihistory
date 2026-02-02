use chrono::{DateTime, Local, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::search::SearchResult;

const INPUT_HEIGHT: u16 = 3;
const PREVIEW_HEIGHT: u16 = 8;
const PREVIEW_LINES: usize = 6;
const HELP_HEIGHT: u16 = 1;

const COLOR_ACCENT: Color = Color::Cyan;
const COLOR_MUTED: Color = Color::DarkGray;
const COLOR_TEXT: Color = Color::White;
const COLOR_MATCH: Color = Color::Yellow;
const COLOR_SELECTED_BG: Color = Color::Rgb(40, 44, 52);

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

pub struct UI;

impl UI {
    pub fn new() -> Self {
        Self
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
        );

        if let Some(cmd) = selected_command {
            self.render_preview(frame, chunks[2], cmd);
        } else {
            self.render_empty_preview(frame, chunks[2]);
        }
        self.render_help_bar(frame, chunks[3]);

        new_offset
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect, command: &str) {
        let inner_width = area.width.saturating_sub(2) as usize;
        let wrapped = textwrap::wrap(command, inner_width);
        let lines: Vec<Line> = wrapped
            .iter()
            .take(PREVIEW_LINES)
            .map(|s| Line::from(Span::styled(s.to_string(), Style::default().fg(COLOR_TEXT))))
            .collect();

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
    ) -> usize {
        let visible_height = area.height.saturating_sub(2) as usize;

        let mut new_offset = scroll_offset;
        if selected_index < new_offset {
            new_offset = selected_index;
        } else if selected_index >= new_offset + visible_height {
            new_offset = selected_index.saturating_sub(visible_height - 1);
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
            Span::styled(" navigate  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Enter", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" select  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Tab", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" run  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Ctrl+D", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" delete  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Esc", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" cancel", Style::default().fg(COLOR_MUTED)),
        ]));

        frame.render_widget(help, area);
    }
}

fn render_command_line(
    command: &str,
    match_indices: &[usize],
    timestamp: Option<i64>,
    is_selected: bool,
    available_width: usize,
    now: i64,
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
    let time_width = time_str.as_ref().map(|s| s.len() + 2).unwrap_or(0);
    let prefix_width = 2;
    let max_cmd_width = available_width.saturating_sub(prefix_width + time_width);
    let display_len = max_cmd_width.saturating_sub(3).min(command.len());
    let needs_truncation = command.len() > max_cmd_width.saturating_sub(3);

    let mut spans = Vec::with_capacity(8);
    spans.push(Span::styled(
        if is_selected { "> " } else { "  " },
        prefix_style,
    ));

    let cmd_bytes = command.as_bytes();

    if match_indices.is_empty() || match_indices.iter().all(|&i| i >= display_len) {
        spans.push(Span::styled(
            command[..display_len].to_string(),
            normal_style,
        ));
    } else {
        let mut last_end = 0;
        for &idx in match_indices {
            if idx >= display_len || idx >= cmd_bytes.len() {
                break;
            }
            if idx > last_end {
                if let Ok(text) = std::str::from_utf8(&cmd_bytes[last_end..idx]) {
                    spans.push(Span::styled(text.to_string(), normal_style));
                }
            }
            if let Ok(ch) = std::str::from_utf8(&cmd_bytes[idx..idx + 1]) {
                spans.push(Span::styled(ch.to_string(), match_style));
            }
            last_end = idx + 1;
        }
        if last_end < display_len {
            let end = display_len.min(cmd_bytes.len());
            if let Ok(text) = std::str::from_utf8(&cmd_bytes[last_end..end]) {
                spans.push(Span::styled(text.to_string(), normal_style));
            }
        }
    }

    if needs_truncation {
        spans.push(Span::styled("...", normal_style));
    }

    if let Some(time) = time_str {
        let current_len: usize = spans.iter().map(|s| s.content.len()).sum();
        let padding_needed = available_width.saturating_sub(current_len + time.len());
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
