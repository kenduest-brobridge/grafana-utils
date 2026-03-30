#![cfg(feature = "tui")]

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

pub(crate) fn header_block(title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title.to_string())
        .border_style(Style::default().fg(Color::LightBlue))
}

pub(crate) fn footer_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title("Status & Controls")
        .style(Style::default().bg(Color::Rgb(16, 22, 30)))
        .border_style(Style::default().fg(Color::LightBlue))
}

pub(crate) fn pane_block(title: &str, focused: bool, accent: Color, bg: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(if focused {
            format!("{title} [Focused]")
        } else {
            title.to_string()
        })
        .style(Style::default().bg(bg))
        .border_style(Style::default().fg(if focused { accent } else { Color::Gray }))
        .title_style(
            Style::default()
                .fg(Color::White)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        )
}

pub(crate) fn key_chip(label: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {label} "),
        Style::default()
            .fg(Color::White)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

pub(crate) fn plain(value: impl Into<String>) -> Span<'static> {
    Span::styled(value.into(), Style::default().fg(Color::White))
}

pub(crate) fn label(value: impl Into<String>) -> Span<'static> {
    Span::styled(
        value.into(),
        Style::default()
            .fg(Color::LightCyan)
            .add_modifier(Modifier::BOLD),
    )
}

pub(crate) fn accent(value: impl Into<String>, color: Color) -> Span<'static> {
    Span::styled(
        value.into(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

pub(crate) struct SummaryCell {
    label: String,
    value: String,
    color: Color,
}

pub(crate) fn summary_cell(
    label: impl Into<String>,
    value: impl Into<String>,
    color: Color,
) -> SummaryCell {
    SummaryCell {
        label: label.into(),
        value: value.into(),
        color,
    }
}

pub(crate) fn summary_line(items: &[SummaryCell]) -> Line<'static> {
    let cell_width = items
        .iter()
        .map(|item| item.label.chars().count() + item.value.chars().count() + 2)
        .max()
        .unwrap_or(0);
    let mut spans = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        let used_width = item.label.chars().count() + item.value.chars().count() + 2;
        let trailing_padding = cell_width.saturating_sub(used_width);
        spans.push(label(format!("{} ", item.label)));
        spans.push(accent(item.value.clone(), item.color));
        if trailing_padding > 0 {
            spans.push(Span::raw(" ".repeat(trailing_padding)));
        }
    }
    Line::from(spans)
}

pub(crate) fn status_line(status: impl Into<String>) -> Line<'static> {
    Line::from(Span::styled(
        status.into(),
        Style::default().fg(Color::Gray),
    ))
}

pub(crate) fn control_line(items: &[(&str, Color, &str)]) -> Line<'static> {
    let key_width = items
        .iter()
        .map(|(key, _, _)| key.chars().count())
        .max()
        .unwrap_or(0);
    let body_width = items
        .iter()
        .map(|(_, _, text)| text.chars().count())
        .max()
        .unwrap_or(0);
    let cell_width = key_width + body_width + 3;
    let mut spans = Vec::new();
    for (index, (key, color, text)) in items.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        let padded_key = format!("{key:<key_width$}");
        let text_span = format!(" {:<body_width$}", text);
        let used_width = key_width + body_width + 3;
        let trailing_padding = cell_width.saturating_sub(used_width);
        spans.push(key_chip(&padded_key, *color));
        spans.push(plain(text_span));
        if trailing_padding > 0 {
            spans.push(Span::raw(" ".repeat(trailing_padding)));
        }
    }
    Line::from(spans)
}

pub(crate) fn build_header(title: &str, lines: Vec<Line<'static>>) -> Paragraph<'static> {
    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(header_block(title))
}

pub(crate) fn build_footer_controls(lines: Vec<Line<'static>>) -> Paragraph<'static> {
    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(footer_block())
        .style(Style::default().bg(Color::Rgb(16, 22, 30)).fg(Color::White))
}

pub(crate) fn build_footer(
    mut lines: Vec<Line<'static>>,
    status: impl Into<String>,
) -> Paragraph<'static> {
    lines.push(status_line(status));
    build_footer_controls(lines)
}

pub(crate) fn centered_rect(area: Rect, width_percent: u16, height_percent: u16) -> Rect {
    let width = area.width.saturating_mul(width_percent).saturating_div(100);
    let height = area
        .height
        .saturating_mul(height_percent)
        .saturating_div(100);
    let width = width.max(32).min(area.width);
    let height = height.max(8).min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width).saturating_div(2),
        y: area.y + area.height.saturating_sub(height).saturating_div(2),
        width,
        height,
    }
}

pub(crate) fn render_overlay(
    frame: &mut ratatui::Frame,
    title: &str,
    lines: Vec<Line<'static>>,
    accent: Color,
) {
    let area = centered_rect(frame.area(), 76, 48);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title.to_string())
                .style(Style::default().bg(Color::Rgb(18, 20, 26)))
                .border_style(Style::default().fg(accent))
                .title_style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
        ),
        area,
    );
}
