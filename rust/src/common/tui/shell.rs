#![cfg(feature = "tui")]

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::{
    layout::{Margin, Rect},
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
    let title_bg = if focused { accent } else { bg };
    let title_fg = if focused { Color::Black } else { Color::White };
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
                .fg(title_fg)
                .bg(title_bg)
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

pub(crate) fn focus_label(value: impl Into<String>) -> Span<'static> {
    Span::styled(
        value.into(),
        Style::default()
            .fg(Color::Black)
            .bg(Color::Blue)
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
    build_control_line(items, &vec![(key_width, body_width); items.len()])
}

pub(crate) fn control_grid(rows: &[Vec<(&str, Color, &str)>]) -> Vec<Line<'static>> {
    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![(0usize, 0usize); column_count];
    for row in rows {
        for (index, (key, _, text)) in row.iter().enumerate() {
            widths[index].0 = widths[index].0.max(key.chars().count());
            widths[index].1 = widths[index].1.max(text.chars().count());
        }
    }
    rows.iter()
        .map(|row| build_control_line(row, &widths))
        .collect()
}

fn build_control_line(items: &[(&str, Color, &str)], widths: &[(usize, usize)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, (key, color, text)) in items.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        let (key_width, body_width) = widths.get(index).copied().unwrap_or_default();
        let cell_width = key_width + body_width + 3;
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
        .block(footer_block())
        .style(Style::default().bg(Color::Rgb(16, 22, 30)).fg(Color::White))
}

pub(crate) fn footer_height(control_line_count: usize) -> u16 {
    control_line_count
        .saturating_add(3)
        .max(4)
        .min(u16::MAX as usize) as u16
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

pub(crate) fn centered_fixed_rect(area: Rect, width_percent: u16, height: u16) -> Rect {
    let width = area
        .width
        .saturating_mul(width_percent)
        .saturating_div(100)
        .max(32)
        .min(area.width);
    let height = height.max(5).min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width).saturating_div(2),
        y: area.y + area.height.saturating_sub(height).saturating_div(2),
        width,
        height,
    }
}

pub(crate) fn dialog_block(title: impl Into<String>, accent: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title.into())
        .style(Style::default().bg(Color::Rgb(16, 22, 30)))
        .border_style(Style::default().fg(accent))
        .title_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(24, 78, 140))
                .add_modifier(Modifier::BOLD),
        )
}

pub(crate) fn render_dialog_shell(
    frame: &mut ratatui::Frame,
    title: impl Into<String>,
    width_percent: u16,
    height: u16,
    accent: Color,
) -> Rect {
    let area = centered_fixed_rect(frame.area(), width_percent, height);
    frame.render_widget(Clear, area);
    frame.render_widget(dialog_block(title, accent), area);
    area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    })
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

#[cfg(test)]
mod tests {
    use super::{centered_fixed_rect, footer_height};
    use ratatui::layout::Rect;

    #[test]
    fn footer_height_accounts_for_status_line_and_borders() {
        assert_eq!(footer_height(0), 4);
        assert_eq!(footer_height(1), 4);
        assert_eq!(footer_height(3), 6);
    }

    #[test]
    fn centered_fixed_rect_places_dialog_in_middle() {
        let area = Rect::new(0, 0, 120, 40);
        let dialog = centered_fixed_rect(area, 50, 10);

        assert_eq!(dialog, Rect::new(30, 15, 60, 10));
    }
}
