#![cfg(feature = "tui")]
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use super::super::super::inspect_workbench_state::{
    InspectPane, InspectWorkbenchState, SearchDirection, SearchPromptState,
};
use super::super::inspect_workbench_render_helpers::{
    item_badge_label, key_chip, pane_block, plain,
};

pub(crate) fn render_detail_panel(
    frame: &mut ratatui::Frame,
    area: Rect,
    state: &InspectWorkbenchState,
) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(7),
            Constraint::Length(4),
        ])
        .split(area);
    let selected_item = state.selected_item();
    let mode_label = state
        .current_group()
        .map(|group| group.label.as_str())
        .unwrap_or("Overview");
    let mode_subtitle = state
        .current_group()
        .map(|group| group.subtitle.as_str())
        .unwrap_or("Inspect review");
    let view_label = state.current_view_label();
    let overview_lines = if let Some(item) = selected_item {
        vec![
            Line::from(vec![
                Span::styled(
                    " Mode ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    mode_label.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " View ",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(44, 92, 184))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    view_label.clone(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("Selected  {}  ", item_badge_label(&item.kind)),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    item.title.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Summary   ", Style::default().fg(Color::Gray)),
                Span::styled(item.meta.clone(), Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Source    ", Style::default().fg(Color::Gray)),
                Span::styled(
                    state.document.source_label.clone(),
                    Style::default().fg(Color::Green),
                ),
                Span::styled("   Context    ", Style::default().fg(Color::Gray)),
                Span::styled(mode_subtitle.to_string(), Style::default().fg(Color::White)),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled(
                    " Mode ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    mode_label.to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
                Span::styled(
                    " View ",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(44, 92, 184))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    view_label,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Selected  ", Style::default().fg(Color::Gray)),
                Span::styled("No row selected yet.", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Source    ", Style::default().fg(Color::Gray)),
                Span::styled(
                    state.document.source_label.clone(),
                    Style::default().fg(Color::Green),
                ),
                Span::styled("   Context    ", Style::default().fg(Color::Gray)),
                Span::styled(mode_subtitle.to_string(), Style::default().fg(Color::White)),
            ]),
        ]
    };
    frame.render_widget(
        Paragraph::new(overview_lines).block(pane_block("Overview", false)),
        sections[0],
    );

    render_focusable_lines(
        frame,
        sections[1],
        state.current_detail_lines(),
        pane_block("Facts", state.focus == InspectPane::Facts),
        state.focus == InspectPane::Facts,
        state.detail_cursor,
    );

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                key_chip("g", Color::Magenta),
                plain(" switch mode"),
                plain("   "),
                key_chip("v", Color::Magenta),
                plain(" switch mode view"),
            ]),
            Line::from(vec![
                key_chip("/", Color::Yellow),
                plain(" search forward"),
                plain("   "),
                key_chip("?", Color::Yellow),
                plain(" search backward"),
            ]),
        ])
        .block(Block::default().borders(Borders::ALL).title("Actions")),
        sections[2],
    );
}

fn render_focusable_lines(
    frame: &mut ratatui::Frame,
    area: Rect,
    lines: Vec<String>,
    block: Block<'static>,
    focused: bool,
    cursor: usize,
) {
    let inner = block.inner(area);
    let viewport_height = inner.height.max(1) as usize;
    let clamped_cursor = cursor.min(lines.len().saturating_sub(1));
    let start = clamped_cursor.saturating_sub(viewport_height.saturating_sub(1));
    let end = (start + viewport_height).min(lines.len());
    let visible = lines[start..end]
        .iter()
        .enumerate()
        .map(|(offset, line)| {
            let absolute = start + offset;
            let style = if focused && absolute == clamped_cursor {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(line.clone(), style)))
        })
        .collect::<Vec<_>>();
    frame.render_widget(List::new(visible).block(block), area);
}

pub(crate) fn render_search_prompt(frame: &mut ratatui::Frame, search: &SearchPromptState) {
    let area = centered_rect(frame.area(), 68, 5);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(search.query.clone()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(match search.direction {
                    SearchDirection::Forward => "Search /",
                    SearchDirection::Backward => "Search ?",
                })
                .style(Style::default().bg(Color::Rgb(18, 20, 26)))
                .border_style(Style::default().fg(Color::Yellow)),
        ),
        area,
    );
    let max_offset = area.width.saturating_sub(3) as usize;
    let offset = search.query.chars().count().min(max_offset) as u16;
    frame.set_cursor_position(Position::new(area.x + 1 + offset, area.y + 1));
}

pub(crate) fn render_full_detail_viewer(
    frame: &mut ratatui::Frame,
    state: &mut InspectWorkbenchState,
) {
    let area = centered_rect_percent(frame.area(), 84, 72);
    let container = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Rgb(16, 18, 24)))
        .border_style(Style::default().fg(Color::LightCyan));
    let inner = container.inner(area);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(6),
            Constraint::Length(2),
        ])
        .split(inner);
    let selected_badge = state
        .selected_item()
        .map(|item| item_badge_label(&item.kind))
        .unwrap_or_else(|| "DETAIL".to_string());
    let selected_title = state
        .selected_item()
        .map(|item| item.title.clone())
        .unwrap_or_else(|| "No item selected".to_string());
    let selected_meta = state
        .selected_item()
        .map(|item| item.meta.clone())
        .filter(|value: &String| !value.is_empty())
        .unwrap_or_else(|| "No summary available.".to_string());
    let body_block = Block::default()
        .borders(Borders::ALL)
        .title("Content")
        .border_style(Style::default().fg(Color::DarkGray));
    let body_inner = body_block.inner(sections[1]);
    let rendered_rows = viewer_rows(
        state.current_full_detail_lines(),
        body_inner.width.saturating_sub(1) as usize,
        state.modal.full_detail.wrapped,
    );
    state.sync_full_detail_row_mapping(
        rendered_rows
            .iter()
            .map(|row| row.logical_index)
            .collect::<Vec<_>>(),
    );
    let visible_height = body_inner.height.max(1) as usize;
    let max_scroll = rendered_rows.len().saturating_sub(visible_height.max(1));
    state.ensure_full_detail_focus_visible(visible_height);
    state.clamp_full_detail_scroll(max_scroll);
    let scroll_y = state.modal.full_detail.scroll;

    frame.render_widget(Clear, area);
    frame.render_widget(container, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    " Detail Viewer ",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(44, 92, 184))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    format!(" {} ", selected_badge),
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    selected_title,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Summary  ", Style::default().fg(Color::Gray)),
                Span::styled(selected_meta, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Wrap     ", Style::default().fg(Color::Gray)),
                Span::styled(
                    if state.modal.full_detail.wrapped {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    Style::default().fg(Color::Green),
                ),
            ]),
        ])
        .block(Block::default()),
        sections[0],
    );
    let visible_items = rendered_rows
        .into_iter()
        .skip(scroll_y)
        .take(visible_height)
        .map(|row| {
            let style = if row.logical_index == state.modal.full_detail.active_logical {
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(row.line).style(style)
        })
        .collect::<Vec<_>>();
    frame.render_widget(List::new(visible_items).block(body_block), sections[1]);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            key_chip("w", Color::Yellow),
            plain(" toggle wrap"),
            Span::raw("   "),
            key_chip("Up/Down", Color::Blue),
            plain(" scroll"),
            Span::raw("   "),
            key_chip("PgUp/PgDn", Color::Blue),
            plain(" jump"),
            Span::raw("   "),
            key_chip("Esc", Color::Gray),
            plain(" close"),
            Span::raw("   "),
            key_chip("Enter", Color::Gray),
            plain(" close"),
        ]))
        .style(Style::default().bg(Color::Rgb(16, 18, 24))),
        sections[2],
    );
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x
        + area
            .width
            .saturating_sub(width.min(area.width))
            .saturating_div(2);
    let y = area.y
        + area
            .height
            .saturating_sub(height.min(area.height))
            .saturating_div(2);
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn centered_rect_percent(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let width = area.width.saturating_mul(width_percent).saturating_div(100);
    let height = area
        .height
        .saturating_mul(height_percent)
        .saturating_div(100);
    centered_rect(area, width.max(32), height.max(12))
}

struct ViewerRenderRow {
    logical_index: usize,
    line: Line<'static>,
}

fn viewer_rows(lines: Vec<String>, width: usize, wrapped: bool) -> Vec<ViewerRenderRow> {
    lines
        .into_iter()
        .enumerate()
        .flat_map(|(logical_index, line)| {
            if line.trim().is_empty() {
                return vec![ViewerRenderRow {
                    logical_index,
                    line: Line::from(""),
                }];
            }
            if let Some((label, value)) = line.split_once(':') {
                let prefix = format!("{label:<16}: ");
                return wrap_labeled_viewer_line(&prefix, value.trim(), width, wrapped)
                    .into_iter()
                    .map(|line| ViewerRenderRow {
                        logical_index,
                        line,
                    })
                    .collect::<Vec<_>>();
            }
            wrap_plain_viewer_line(&line, width, wrapped)
                .into_iter()
                .map(|line| ViewerRenderRow {
                    logical_index,
                    line,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn wrap_labeled_viewer_line(
    prefix: &str,
    value: &str,
    width: usize,
    wrapped: bool,
) -> Vec<Line<'static>> {
    if !wrapped || width <= prefix.len().saturating_add(1) {
        return vec![Line::from(vec![
            Span::styled(
                prefix.to_string(),
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(value.to_string(), Style::default().fg(Color::White)),
        ])];
    }
    let first_width = width.saturating_sub(prefix.len()).max(1);
    let continuation_prefix = " ".repeat(prefix.len());
    let chunks = wrap_text_chunks(value, first_width);
    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            if index == 0 {
                Line::from(vec![
                    Span::styled(
                        prefix.to_string(),
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(chunk, Style::default().fg(Color::White)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(
                        continuation_prefix.clone(),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(chunk, Style::default().fg(Color::White)),
                ])
            }
        })
        .collect()
}

fn wrap_plain_viewer_line(line: &str, width: usize, wrapped: bool) -> Vec<Line<'static>> {
    if !wrapped || width == 0 {
        return vec![Line::from(Span::styled(
            line.to_string(),
            Style::default().fg(Color::White),
        ))];
    }
    wrap_text_chunks(line, width.max(1))
        .into_iter()
        .map(|chunk| Line::from(Span::styled(chunk, Style::default().fg(Color::White))))
        .collect()
}

fn wrap_text_chunks(value: &str, width: usize) -> Vec<String> {
    if width == 0 || value.is_empty() {
        return vec![value.to_string()];
    }
    let chars = value.chars().collect::<Vec<_>>();
    chars
        .chunks(width)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<_>>()
}
