#![cfg(feature = "tui")]
use ratatui::style::Color;
use ratatui::text::{Line, Span};

use crate::interactive_browser::BrowserItem;
use crate::tui_shell;

pub(crate) fn pane_block(label: &str, active: bool) -> ratatui::widgets::Block<'static> {
    tui_shell::pane_block(label, active, Color::Cyan, Color::Reset)
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
    tui_shell::control_line(items)
}

pub(crate) fn key_chip(label: &str, color: Color) -> Span<'static> {
    tui_shell::key_chip(label, color)
}

pub(crate) fn plain(value: impl Into<String>) -> Span<'static> {
    tui_shell::plain(value)
}

pub(crate) fn compact_count_label(count: usize) -> String {
    if count > 99 {
        "99+".to_string()
    } else {
        format!("{count:>2}")
    }
}
