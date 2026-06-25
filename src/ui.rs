use chrono::{DateTime, Local, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, AppMode};
use crate::search::SearchResult;

const INPUT_HEIGHT: u16 = 3;
const PREVIEW_HEIGHT: u16 = 8;
const PREVIEW_LINES: usize = (PREVIEW_HEIGHT - 2) as usize;
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

use std::rc::Rc;

fn main_layout(area: Rect) -> Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(INPUT_HEIGHT),
            Constraint::Min(1),
            Constraint::Length(PREVIEW_HEIGHT),
            Constraint::Length(HELP_HEIGHT),
        ])
        .split(area)
}

fn list_block(count: usize, label: &str, status_message: Option<&str>) -> Block<'static> {
    let (title, border_style) = if let Some(msg) = status_message {
        (format!(" {} ", msg), Style::default().fg(Color::Yellow))
    } else {
        (
            format!(" {} {} ", count, label),
            Style::default().fg(COLOR_MUTED),
        )
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

pub fn render(frame: &mut Frame, app: &App) {
    match app.mode() {
        AppMode::Search | AppMode::AliasCreate => {
            render_search_view(frame, app);
            if app.mode() == AppMode::AliasCreate {
                render_alias_modal(
                    frame,
                    app.alias_input(),
                    app.alias_target_command().unwrap_or(""),
                    app.status_message(),
                );
            }
        }
        AppMode::Aliases => {
            render_aliases_view(frame, app);
        }
        AppMode::AliasModify => {
            render_aliases_view(frame, app);
            render_alias_modify_modal(
                frame,
                app.alias_modify_name().unwrap_or(""),
                app.alias_modify_input(),
                app.status_message(),
            );
        }
    }
}

fn render_search_view(frame: &mut Frame, app: &App) {
    let mode = app.mode();
    let chunks = main_layout(frame.area());

    render_input(frame, chunks[0], app.search_nav().query(), mode);
    render_results(
        frame,
        chunks[1],
        app.results(),
        app.search_nav().selected(),
        app.search_nav().scroll_offset(),
        app.status_message(),
    );
    render_preview(frame, chunks[2], app.selected_command_preview());
    render_help_bar(frame, chunks[3], mode);
}

fn render_preview(frame: &mut Frame, area: Rect, command: Option<&str>) {
    let lines: Vec<Line> = if let Some(cmd) = command {
        let inner_width = area.width.saturating_sub(2) as usize;
        textwrap::wrap(cmd, inner_width)
            .iter()
            .take(PREVIEW_LINES)
            .map(|s| Line::from(Span::styled(s.to_string(), Style::default().fg(COLOR_TEXT))))
            .collect()
    } else {
        Vec::new()
    };

    let preview = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_MUTED))
            .title(" Preview "),
    );

    frame.render_widget(preview, area);
}

fn render_input(frame: &mut Frame, area: Rect, query: &str, mode: AppMode) {
    let tab_indicator = match mode {
        AppMode::Search | AppMode::AliasCreate => " [History] Aliases ",
        AppMode::Aliases | AppMode::AliasModify => " History [Aliases] ",
    };

    let input_text = Line::from(vec![
        Span::styled("> ", Style::default().fg(COLOR_ACCENT)),
        Span::raw(query),
        Span::styled("_", Style::default().fg(Color::Gray)),
    ]);

    let input = Paragraph::new(input_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_ACCENT))
            .title(" ihistory ")
            .title_bottom(Line::from(tab_indicator).right_aligned()),
    );

    frame.render_widget(input, area);
}

fn render_results(
    frame: &mut Frame,
    area: Rect,
    results: &[SearchResult],
    selected_index: usize,
    scroll_offset: usize,
    status_message: Option<&str>,
) {
    let visible_height = area.height.saturating_sub(2) as usize;

    let start = scroll_offset;
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

    let list = List::new(items).block(list_block(results.len(), "results", status_message));

    let mut list_state = ListState::default();
    list_state.select(Some(selected_index - start));
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_help_bar(frame: &mut Frame, area: Rect, mode: AppMode) {
    let spans = match mode {
        AppMode::Search => vec![
            Span::styled("↑↓", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" navigate  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Enter", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" select  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Tab", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" run  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Ctrl+D", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" delete  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Ctrl+A", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" alias  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Ctrl+T", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" aliases  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Esc", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" cancel", Style::default().fg(COLOR_MUTED)),
        ],
        AppMode::AliasCreate => vec![
            Span::styled("Enter", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" save  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Esc", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" cancel", Style::default().fg(COLOR_MUTED)),
        ],
        AppMode::Aliases => vec![
            Span::styled("Ctrl+T", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" history  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Enter", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" select  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Tab", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" run  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Ctrl+D", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" delete  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("r", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" rename  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("m", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" modify  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Esc", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" quit", Style::default().fg(COLOR_MUTED)),
        ],
        AppMode::AliasModify => vec![
            Span::styled("Enter", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" save  ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Esc", Style::default().fg(COLOR_ACCENT)),
            Span::styled(" cancel", Style::default().fg(COLOR_MUTED)),
        ],
    };

    let help = Paragraph::new(Line::from(spans));
    frame.render_widget(help, area);
}

fn render_alias_modal(
    frame: &mut Frame,
    alias_input: &str,
    target_command: &str,
    status_message: Option<&str>,
) {
    let area = frame.area();
    let modal_width = area.width.saturating_sub(6).max(40);
    let inner_width = modal_width.saturating_sub(4) as usize;
    let wrapped_cmd = textwrap::wrap(target_command, inner_width.saturating_sub(9));
    let cmd_lines = wrapped_cmd.len().max(1);
    let modal_height = (3 + cmd_lines + 3 + if status_message.is_some() { 2 } else { 0 }) as u16;
    let modal_height = modal_height.min(area.height.saturating_sub(4));
    let modal_area = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_area);

    let mut lines: Vec<Line> = Vec::new();

    if let Some(first) = wrapped_cmd.first() {
        lines.push(Line::from(vec![
            Span::styled("Command: ", Style::default().fg(COLOR_MUTED)),
            Span::styled(first.to_string(), Style::default().fg(COLOR_TEXT)),
        ]));
    }
    for wrapped_line in wrapped_cmd.iter().skip(1) {
        lines.push(Line::from(vec![
            Span::styled("         ", Style::default().fg(COLOR_MUTED)),
            Span::styled(wrapped_line.to_string(), Style::default().fg(COLOR_TEXT)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Name: > ", Style::default().fg(COLOR_ACCENT)),
        Span::raw(alias_input),
        Span::styled("_", Style::default().fg(Color::Gray)),
    ]));

    if let Some(msg) = status_message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            msg.to_string(),
            Style::default().fg(Color::Yellow),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("    Enter", Style::default().fg(COLOR_ACCENT)),
        Span::styled(" save    ", Style::default().fg(COLOR_MUTED)),
        Span::styled("Esc", Style::default().fg(COLOR_ACCENT)),
        Span::styled(" cancel", Style::default().fg(COLOR_MUTED)),
    ]));

    let modal = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_ACCENT))
            .title(" Create Alias "),
    );

    frame.render_widget(modal, modal_area);
}

fn render_alias_modify_modal(
    frame: &mut Frame,
    alias_name: &str,
    modify_input: &str,
    status_message: Option<&str>,
) {
    let area = frame.area();
    let modal_width = area.width.saturating_sub(6).max(40);
    let inner_width = modal_width.saturating_sub(4) as usize;
    let wrapped_input = textwrap::wrap(modify_input, inner_width.saturating_sub(5));
    let input_lines = wrapped_input.len().max(1);
    let modal_height = (3 + input_lines + 1 + if status_message.is_some() { 2 } else { 0 }) as u16;
    let modal_height = modal_height.min(area.height.saturating_sub(4));
    let modal_area = centered_rect(modal_width, modal_height, area);

    frame.render_widget(Clear, modal_area);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("Alias: ", Style::default().fg(COLOR_MUTED)),
        Span::styled(alias_name.to_string(), Style::default().fg(COLOR_ACCENT)),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Cmd: ", Style::default().fg(COLOR_ACCENT)),
        Span::raw(modify_input),
        Span::styled("_", Style::default().fg(Color::Gray)),
    ]));

    if let Some(msg) = status_message {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            msg.to_string(),
            Style::default().fg(Color::Yellow),
        )));
    }

    let modal = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_ACCENT))
            .title(" Modify Command "),
    );

    frame.render_widget(modal, modal_area);
}

fn render_aliases_view(frame: &mut Frame, app: &App) {
    let mode = app.mode();
    let chunks = main_layout(frame.area());

    render_input(frame, chunks[0], app.alias_nav().query(), mode);
    render_alias_list(frame, chunks[1], app);
    render_preview(frame, chunks[2], app.selected_alias_preview());
    render_help_bar(frame, chunks[3], mode);
}

fn render_alias_list(frame: &mut Frame, area: Rect, app: &App) {
    let visible_height = area.height.saturating_sub(2) as usize;
    let filtered = app.filtered_aliases();
    let selected_index = app.alias_nav().selected();
    let scroll_offset = app.alias_nav().scroll_offset();
    let alias_editing = app.alias_editing();
    let alias_edit_input = app.alias_edit_input();

    let start = scroll_offset;
    let end = (start + visible_height).min(filtered.len());
    let available_width = area.width.saturating_sub(4) as usize;

    let items: Vec<ListItem> = filtered[start..end]
        .iter()
        .enumerate()
        .map(|(i, (orig_idx, alias))| {
            let actual_index = start + i;
            let is_selected = actual_index == selected_index;
            let is_editing = alias_editing == Some(*orig_idx);

            let line = if is_editing {
                render_alias_edit_line(
                    alias_edit_input,
                    &alias.command,
                    is_selected,
                    available_width,
                )
            } else {
                render_alias_line(&alias.name, &alias.command, is_selected, available_width)
            };

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

    let list = List::new(items).block(list_block(filtered.len(), "aliases", app.status_message()));

    let mut list_state = ListState::default();
    if !filtered.is_empty() {
        list_state.select(Some(selected_index - start));
    }
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn render_alias_line(
    name: &str,
    command: &str,
    is_selected: bool,
    available_width: usize,
) -> Line<'static> {
    let prefix = if is_selected { "> " } else { "  " };
    let prefix_style = Style::default().fg(if is_selected {
        COLOR_ACCENT
    } else {
        COLOR_MUTED
    });
    let name_style = Style::default().fg(COLOR_ACCENT);
    let cmd_style = Style::default().fg(COLOR_MUTED);

    let prefix_width = 2;
    let separator = "  ";
    let name_display = if name.len() > 16 {
        format!("{}..", &name[..14])
    } else {
        name.to_string()
    };
    let name_width = name_display.len();
    let sep_width = separator.len();
    let max_cmd_width = available_width.saturating_sub(prefix_width + name_width + sep_width);

    let cmd_single = command.replace('\n', " ");
    let cmd_display = if cmd_single.len() > max_cmd_width {
        format!("{}...", &cmd_single[..max_cmd_width.saturating_sub(3)])
    } else {
        cmd_single
    };

    Line::from(vec![
        Span::styled(prefix.to_string(), prefix_style),
        Span::styled(name_display, name_style),
        Span::styled(separator.to_string(), cmd_style),
        Span::styled(cmd_display, cmd_style),
    ])
}

fn render_alias_edit_line(
    edit_input: &str,
    command: &str,
    is_selected: bool,
    available_width: usize,
) -> Line<'static> {
    let prefix = if is_selected { "> " } else { "  " };
    let prefix_style = Style::default().fg(if is_selected {
        COLOR_ACCENT
    } else {
        COLOR_MUTED
    });
    let cmd_style = Style::default().fg(COLOR_MUTED);

    let prefix_width = 2;
    let separator = "  ";
    let name_width = edit_input.len() + 1; // +1 for cursor
    let sep_width = separator.len();
    let max_cmd_width = available_width.saturating_sub(prefix_width + name_width + sep_width);

    let cmd_single = command.replace('\n', " ");
    let cmd_display = if cmd_single.len() > max_cmd_width {
        format!("{}...", &cmd_single[..max_cmd_width.saturating_sub(3)])
    } else {
        cmd_single
    };

    Line::from(vec![
        Span::styled(prefix.to_string(), prefix_style),
        Span::styled(edit_input.to_string(), Style::default().fg(COLOR_MATCH)),
        Span::styled("_", Style::default().fg(Color::Gray)),
        Span::styled(separator.to_string(), cmd_style),
        Span::styled(cmd_display, cmd_style),
    ])
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
