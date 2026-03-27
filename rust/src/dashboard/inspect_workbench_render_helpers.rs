#![cfg(feature = "tui")]
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};

use crate::interactive_browser::BrowserItem;

pub(crate) fn pane_block(label: &str, active: bool) -> Block<'static> {
    let mut block = Block::default().borders(Borders::ALL).title(if active {
        format!("{label} [Focused]")
    } else {
        label.to_string()
    });
    if active {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }
    block
}

pub(crate) fn item_color(kind: &str) -> Color {
    match kind {
        "dashboard-summary" | "dashboard-finding-summary" => Color::Yellow,
        "query" | "query-review" => Color::Cyan,
        "finding" => Color::LightRed,
        "datasource-usage" | "datasource-finding-coverage" => Color::LightGreen,
        _ => Color::Gray,
    }
}

pub(crate) fn group_color(kind: &str) -> Color {
    match kind {
        "overview" => Color::Yellow,
        "findings" => Color::LightRed,
        "queries" => Color::Cyan,
        "dependencies" => Color::LightGreen,
        _ => Color::Gray,
    }
}

pub(crate) fn item_badge_label(kind: &str) -> String {
    match kind {
        "dashboard-summary" => "DASHBOARD".to_string(),
        "dashboard-finding-summary" => "SUMMARY".to_string(),
        "query" => "QUERY".to_string(),
        "query-review" => "REVIEW".to_string(),
        "finding" => "FINDING".to_string(),
        "datasource-usage" => "DATASOURCE".to_string(),
        "datasource-finding-coverage" => "COVERAGE".to_string(),
        _ => kind.to_uppercase(),
    }
}

pub(crate) fn item_row_text(index: usize, item: &BrowserItem) -> String {
    format!(
        "{:>2}. [{}] {}  {}",
        index + 1,
        item_badge_label(&item.kind),
        item.title,
        item.meta
    )
}

pub(crate) fn slice_visible(value: &str, offset: usize, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    value.chars().skip(offset).take(width).collect()
}

pub(crate) fn control_line(items: &[(&str, Color, &str)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, (key, color, text)) in items.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(key_chip(key, *color));
        spans.push(plain(format!(" {text}")));
    }
    Line::from(spans)
}

pub(crate) fn key_chip(label: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", label),
        Style::default()
            .fg(Color::White)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

pub(crate) fn plain(value: impl Into<String>) -> Span<'static> {
    Span::styled(value.into(), Style::default().fg(Color::White))
}

pub(crate) fn compact_count_label(count: usize) -> String {
    if count > 99 {
        "99+".to_string()
    } else {
        format!("{count:>2}")
    }
}
