//! Terminal interaction layer for Sync operations and command-driven review flows.
#![cfg_attr(not(feature = "tui"), allow(dead_code, unused_imports))]

use crate::common::Result;
#[cfg(feature = "tui")]
use crate::tui_shell;
use serde_json::Value;
use std::collections::BTreeSet;

use super::super::json::{require_json_array_field, require_json_object};
use super::super::plan_builder::{
    build_sync_alert_assessment_document, build_sync_plan_summary_document,
};

#[cfg(feature = "tui")]
use ratatui::style::{Color, Modifier, Style};
#[cfg(feature = "tui")]
use ratatui::text::{Line, Span};
#[cfg(feature = "tui")]
use ratatui::widgets::ListItem;

#[cfg(any(feature = "tui", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewableOperation {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) operation: Value,
}

#[cfg(any(feature = "tui", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewDiffLine {
    pub changed: bool,
    pub marker: char,
    pub content: String,
    pub highlight_range: Option<(usize, usize)>,
}

#[cfg(any(feature = "tui", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewDiffModel {
    pub title: String,
    pub action: String,
    pub live_lines: Vec<ReviewDiffLine>,
    pub desired_lines: Vec<ReviewDiffLine>,
}

#[cfg(any(feature = "tui", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiffPaneFocus {
    Live,
    Desired,
}

#[cfg(any(feature = "tui", test))]
type HighlightRange = Option<(usize, usize)>;

#[cfg(any(feature = "tui", test))]
pub(crate) struct DiffControlsState {
    pub selected: usize,
    pub total: usize,
    pub diff_focus: DiffPaneFocus,
    pub live_wrap_lines: bool,
    pub desired_wrap_lines: bool,
    pub live_diff_cursor: usize,
    pub live_horizontal_offset: usize,
    pub desired_diff_cursor: usize,
    pub desired_horizontal_offset: usize,
}

#[cfg(any(feature = "tui", test))]
fn operation_key(operation: &serde_json::Map<String, Value>) -> String {
    if let Some(action_id) = operation.get("actionId").and_then(Value::as_str) {
        return action_id.to_string();
    }
    format!(
        "{}::{}",
        operation
            .get("resourceKind")
            .or_else(|| operation.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        operation
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    )
}

#[cfg(any(feature = "tui", test))]
fn operation_label(operation: &serde_json::Map<String, Value>) -> String {
    format!(
        "{} {}",
        operation
            .get("resourceKind")
            .or_else(|| operation.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        operation
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    )
}

#[cfg(any(feature = "tui", test))]
fn operation_badge(action: &str) -> &'static str {
    match action {
        "would-create" => "CREATE",
        "would-update" => "UPDATE",
        "would-delete" => "DELETE",
        _ => "UNKNOWN",
    }
}

#[cfg(feature = "tui")]
pub(crate) fn operation_badge_color(action: &str) -> Color {
    match action {
        "would-create" => Color::Green,
        "would-update" => Color::Yellow,
        "would-delete" => Color::Red,
        _ => Color::DarkGray,
    }
}

#[cfg(feature = "tui")]
pub(crate) fn operation_row_color(action: &str) -> Color {
    match action {
        "would-create" => Color::LightGreen,
        "would-update" => Color::LightYellow,
        "would-delete" => Color::LightRed,
        _ => Color::Gray,
    }
}

#[cfg(feature = "tui")]
fn selection_mark(selected: bool) -> &'static str {
    if selected {
        "✓"
    } else {
        "·"
    }
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn operation_preview(item: &ReviewableOperation) -> Vec<String> {
    let object = match item.operation.as_object() {
        Some(object) => object,
        None => return vec!["Invalid operation payload".to_string()],
    };
    let action = object
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let kind = object
        .get("resourceKind")
        .or_else(|| object.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let identity = object
        .get("identity")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let changed_fields = object
        .get("changedFields")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "none".to_string());
    vec![
        format!("Action: {}", operation_badge(action)),
        format!("Kind: {kind}"),
        format!("Identity: {identity}"),
        format!("Changed: {changed_fields}"),
    ]
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn operation_detail_line_count(item: &ReviewableOperation) -> usize {
    build_review_operation_diff_model(&item.operation)
        .map(|model| model.live_lines.len().max(model.desired_lines.len()))
        .unwrap_or(0)
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn operation_changed_count(item: &ReviewableOperation) -> usize {
    build_review_operation_diff_model(&item.operation)
        .map(|model| model.live_lines.iter().filter(|line| line.changed).count())
        .unwrap_or(0)
}

#[cfg(any(feature = "tui", test))]
fn truncate_text(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    if max_chars <= 1 {
        return "…".to_string();
    }
    let kept = text.chars().take(max_chars - 1).collect::<String>();
    format!("{kept}…")
}

#[cfg(feature = "tui")]
pub(crate) fn build_checklist_line(
    item: &ReviewableOperation,
    index: usize,
    selected: bool,
    content_width: usize,
) -> Line<'static> {
    let action = item
        .operation
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let prefix = format!("{} {:>2}. ", selection_mark(selected), index + 1);
    let badge_text = format!("[{}]", operation_badge(action));
    let detail_rows = operation_detail_line_count(item);
    let changed = operation_changed_count(item);
    let row_label = if detail_rows == 1 { "row" } else { "rows" };
    let meta = format!("{detail_rows} {row_label} / {changed} changed");
    let reserved =
        prefix.chars().count() + 1 + badge_text.chars().count() + 1 + meta.chars().count();
    let label_width = content_width.saturating_sub(reserved).max(8);
    let label_text = truncate_text(&item.label, label_width);
    let current =
        prefix.chars().count() + badge_text.chars().count() + 1 + label_text.chars().count();
    let gap = content_width
        .saturating_sub(current + meta.chars().count())
        .max(1);

    Line::from(vec![
        Span::raw(prefix),
        Span::styled(
            badge_text,
            Style::default()
                .fg(operation_badge_color(action))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw(label_text),
        Span::raw(" ".repeat(gap)),
        Span::styled(meta, Style::default().fg(Color::DarkGray)),
    ])
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn selection_title_with_position(
    item: Option<&ReviewableOperation>,
    position: Option<usize>,
    total: Option<usize>,
) -> String {
    let Some(item) = item else {
        return "Selection".to_string();
    };
    let action = item
        .operation
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let identity = item
        .operation
        .get("identity")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    match (position, total) {
        (Some(position), Some(total)) if total > 0 => format!(
            "Selection {}/{} [{}] {identity}",
            position + 1,
            total,
            operation_badge(action)
        ),
        _ => format!("Selection [{}] {identity}", operation_badge(action)),
    }
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn collect_reviewable_operations(plan: &Value) -> Result<Vec<ReviewableOperation>> {
    let plan = require_json_object(plan, "Sync plan document")?;
    let operations = match plan.get("actions").or_else(|| plan.get("operations")) {
        Some(Value::Array(items)) => items,
        _ => require_json_array_field(plan, "operations", "Sync plan document")?,
    };
    Ok(operations
        .iter()
        .filter_map(Value::as_object)
        .filter(|operation| {
            matches!(
                operation.get("action").and_then(Value::as_str),
                Some("would-create" | "would-update" | "would-delete")
            )
        })
        .map(|operation| ReviewableOperation {
            key: operation_key(operation),
            label: operation_label(operation),
            operation: Value::Object(operation.clone()),
        })
        .collect())
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn filter_review_plan_operations(
    plan: &Value,
    selected_keys: &BTreeSet<String>,
) -> Result<Value> {
    let plan_object = require_json_object(plan, "Sync plan document")?;
    let operations = match plan_object
        .get("actions")
        .or_else(|| plan_object.get("operations"))
    {
        Some(Value::Array(items)) => items,
        _ => require_json_array_field(plan_object, "operations", "Sync plan document")?,
    };
    let filtered_operations = operations
        .iter()
        .filter(|item| {
            let Some(object) = item.as_object() else {
                return false;
            };
            match object.get("action").and_then(Value::as_str) {
                Some("would-create" | "would-update" | "would-delete") => {
                    selected_keys.contains(&operation_key(object))
                }
                _ => true,
            }
        })
        .cloned()
        .collect::<Vec<_>>();

    let mut filtered = plan_object.clone();
    filtered.insert(
        "summary".to_string(),
        build_sync_plan_summary_document(&filtered_operations),
    );
    filtered.insert(
        "alertAssessment".to_string(),
        build_sync_alert_assessment_document(&filtered_operations),
    );
    filtered.insert(
        "operations".to_string(),
        Value::Array(filtered_operations.clone()),
    );
    filtered.insert("actions".to_string(), Value::Array(filtered_operations));
    if let Ok(view) = crate::sync::workspace_preview_review_view::build_workspace_review_view(
        &Value::Object(filtered.clone()),
    ) {
        filtered.insert(
            "domains".to_string(),
            Value::Array(
                view.domains
                    .iter()
                    .map(|domain| domain.raw.clone())
                    .collect(),
            ),
        );
        filtered.insert(
            "blockedReasons".to_string(),
            Value::Array(
                view.blocked_reasons
                    .iter()
                    .map(|reason| Value::String(reason.clone()))
                    .collect(),
            ),
        );
        if let Some(summary) = filtered.get_mut("summary").and_then(Value::as_object_mut) {
            summary
                .entry("actionCount".to_string())
                .or_insert(Value::Number((view.summary.action_count as i64).into()));
            summary
                .entry("domainCount".to_string())
                .or_insert(Value::Number((view.summary.domain_count as i64).into()));
            summary
                .entry("sameCount".to_string())
                .or_insert(Value::Number((view.summary.same_count as i64).into()));
            summary
                .entry("blockedCount".to_string())
                .or_insert(Value::Number((view.summary.blocked_count as i64).into()));
            summary
                .entry("warningCount".to_string())
                .or_insert(Value::Number((view.summary.warning_count as i64).into()));
            if !summary.contains_key("blocked_reasons") {
                summary.insert(
                    "blocked_reasons".to_string(),
                    Value::Array(
                        view.blocked_reasons
                            .iter()
                            .map(|reason| Value::String(reason.clone()))
                            .collect(),
                    ),
                );
            }
        }
    }
    Ok(Value::Object(filtered))
}

#[cfg(any(feature = "tui", test))]
fn pretty_inline_json(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "null".to_string(),
        Some(Value::String(text)) => format!("{text:?}"),
        Some(other) => serde_json::to_string(other).unwrap_or_else(|_| "null".to_string()),
    }
}

#[cfg(any(feature = "tui", test))]
fn numbered_line(index: usize, content: String) -> String {
    format!("{:>3} | {content}", index + 1)
}

#[cfg(any(feature = "tui", test))]
fn diff_highlight_ranges(left: &str, right: &str) -> (HighlightRange, HighlightRange) {
    if left == right {
        return (None, None);
    }
    let left_bytes = left.as_bytes();
    let right_bytes = right.as_bytes();
    let mut prefix = 0usize;
    let min_len = left_bytes.len().min(right_bytes.len());
    while prefix < min_len && left_bytes[prefix] == right_bytes[prefix] {
        prefix += 1;
    }

    let mut left_suffix = left_bytes.len();
    let mut right_suffix = right_bytes.len();
    while left_suffix > prefix
        && right_suffix > prefix
        && left_bytes[left_suffix - 1] == right_bytes[right_suffix - 1]
    {
        left_suffix -= 1;
        right_suffix -= 1;
    }

    let left_range = if prefix == left_suffix {
        None
    } else {
        Some((prefix, left_suffix))
    };
    let right_range = if prefix == right_suffix {
        None
    } else {
        Some((prefix, right_suffix))
    };
    (left_range, right_range)
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn build_review_operation_diff_model(operation: &Value) -> Result<ReviewDiffModel> {
    let object = require_json_object(operation, "Sync review operation")?;
    let action = object
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let title = format!(
        "{} {}",
        object
            .get("resourceKind")
            .or_else(|| object.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        object
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    );
    let desired = object.get("desired").and_then(Value::as_object);
    let live = object.get("live").and_then(Value::as_object);
    let changed_fields = object
        .get("changedFields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| item.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let fields = if changed_fields.is_empty() {
        let mut combined = BTreeSet::new();
        if let Some(object) = live {
            combined.extend(object.keys().cloned());
        }
        if let Some(object) = desired {
            combined.extend(object.keys().cloned());
        }
        combined.into_iter().collect::<Vec<_>>()
    } else {
        changed_fields
    };
    if fields.is_empty() {
        return Ok(ReviewDiffModel {
            title,
            action,
            live_lines: vec![ReviewDiffLine {
                changed: false,
                marker: '=',
                content: numbered_line(0, "<no managed field changes>".to_string()),
                highlight_range: None,
            }],
            desired_lines: vec![ReviewDiffLine {
                changed: false,
                marker: '=',
                content: numbered_line(0, "<no managed field changes>".to_string()),
                highlight_range: None,
            }],
        });
    }
    let mut ordered_fields = fields
        .into_iter()
        .map(|field| {
            let live_value = live.and_then(|object| object.get(&field));
            let desired_value = desired.and_then(|object| object.get(&field));
            let changed = live_value != desired_value;
            (field, changed, live_value, desired_value)
        })
        .collect::<Vec<_>>();
    ordered_fields.sort_by_key(|(_, changed, _, _)| if *changed { 0 } else { 1 });

    let mut live_lines = Vec::new();
    let mut desired_lines = Vec::new();
    for (index, (field, changed, live_value, desired_value)) in
        ordered_fields.into_iter().enumerate()
    {
        let live_value_text = pretty_inline_json(live_value);
        let desired_value_text = pretty_inline_json(desired_value);
        let (live_range, desired_range) =
            diff_highlight_ranges(&live_value_text, &desired_value_text);
        let base_prefix = format!("{field}: ");
        let live_content = numbered_line(index, format!("{base_prefix}{live_value_text}"));
        let desired_content = numbered_line(index, format!("{base_prefix}{desired_value_text}"));
        let value_offset = numbered_line(index, base_prefix).len();

        live_lines.push(ReviewDiffLine {
            changed,
            marker: if changed { '-' } else { '=' },
            content: live_content,
            highlight_range: live_range
                .map(|(start, end)| (value_offset + start, value_offset + end)),
        });
        desired_lines.push(ReviewDiffLine {
            changed,
            marker: if changed { '+' } else { '=' },
            content: desired_content,
            highlight_range: desired_range
                .map(|(start, end)| (value_offset + start, value_offset + end)),
        });
    }
    Ok(ReviewDiffModel {
        title,
        action,
        live_lines,
        desired_lines,
    })
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn wrap_text_chunks(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    if text.is_empty() {
        return vec![String::new()];
    }
    let chars = text.chars().collect::<Vec<_>>();
    chars
        .chunks(width)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect()
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn clip_text_window(text: &str, offset: usize, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    text.chars().skip(offset).take(width).collect::<String>()
}

#[cfg(feature = "tui")]
pub(crate) fn render_diff_items(
    lines: &[ReviewDiffLine],
    color: Color,
    content_width: usize,
    wrap_lines: bool,
    horizontal_offset: usize,
) -> Vec<ListItem<'static>> {
    lines
        .iter()
        .map(|line| {
            let marker_style = if line.changed {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let content_style = if line.changed {
                Style::default().fg(color).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let wrapped = if wrap_lines {
                wrap_text_chunks(&line.content, content_width.max(1))
            } else {
                vec![clip_text_window(
                    &line.content,
                    horizontal_offset,
                    content_width.max(1),
                )]
            };
            let body = wrapped
                .into_iter()
                .enumerate()
                .map(|(index, chunk)| {
                    let marker = if index == 0 {
                        format!("{} ", line.marker)
                    } else {
                        "  ".to_string()
                    };
                    let marker_span = Span::styled(marker, marker_style);
                    let visible_highlight = if wrap_lines || index > 0 {
                        None
                    } else {
                        line.highlight_range.and_then(|(start, end)| {
                            let visible_start = start.max(horizontal_offset);
                            let visible_end = end.min(horizontal_offset + content_width.max(1));
                            if visible_start < visible_end {
                                Some((
                                    visible_start.saturating_sub(horizontal_offset),
                                    visible_end.saturating_sub(horizontal_offset),
                                ))
                            } else {
                                None
                            }
                        })
                    };
                    let content_span = match visible_highlight {
                        Some((start, end)) if start < end && end <= chunk.len() => {
                            let prefix = chunk[..start].to_string();
                            let middle = chunk[start..end].to_string();
                            let suffix = chunk[end..].to_string();
                            let mut spans = vec![marker_span, Span::raw(prefix)];
                            spans.push(Span::styled(
                                middle,
                                Style::default()
                                    .fg(color)
                                    .bg(Color::Black)
                                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                            ));
                            spans.push(Span::raw(suffix));
                            return Line::from(spans);
                        }
                        Some(_) if line.changed && index == 0 => Span::styled(
                            chunk,
                            Style::default()
                                .fg(color)
                                .bg(Color::Black)
                                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                        ),
                        _ if line.changed => Span::styled(chunk, content_style),
                        _ => Span::raw(chunk),
                    };
                    Line::from(vec![marker_span, content_span])
                })
                .collect::<Vec<_>>();
            ListItem::new(body)
        })
        .collect()
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn diff_pane_title(
    pane: &str,
    action: &str,
    title: &str,
    position: usize,
    total: usize,
) -> String {
    format!("{pane} {}/{} [{}] {title}", position + 1, total, action)
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn diff_scroll_max(model: &ReviewDiffModel, focus: DiffPaneFocus) -> usize {
    match focus {
        DiffPaneFocus::Live => model.live_lines.len().saturating_sub(1),
        DiffPaneFocus::Desired => model.desired_lines.len().saturating_sub(1),
    }
}

#[cfg(feature = "tui")]
pub(crate) fn build_diff_controls_lines(state: &DiffControlsState) -> Vec<Line<'static>> {
    let focus = match state.diff_focus {
        DiffPaneFocus::Live => "LIVE",
        DiffPaneFocus::Desired => "DESIRED",
    };
    vec![
        Line::from(vec![
            tui_shell::label("Item "),
            tui_shell::accent(
                format!("{}/{}", state.selected + 1, state.total),
                Color::White,
            ),
            Span::raw("  "),
            tui_shell::focus_label("Focus "),
            tui_shell::key_chip(focus, Color::Blue),
            Span::raw("  "),
            Span::styled(
                format!(
                    "Live wrap {}",
                    if state.live_wrap_lines { "ON" } else { "OFF" }
                ),
                Style::default().fg(Color::Red),
            ),
            Span::raw("  "),
            Span::styled(
                format!(
                    "Desired wrap {}",
                    if state.desired_wrap_lines {
                        "ON"
                    } else {
                        "OFF"
                    }
                ),
                Style::default().fg(Color::Green),
            ),
            Span::raw("  "),
            Span::styled(
                "w active  W both".to_string(),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled("Left/Right pan", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!(
                    "Live {} @{}",
                    state.live_diff_cursor + 1,
                    state.live_horizontal_offset
                ),
                Style::default().fg(Color::Red),
            ),
            Span::raw("  "),
            Span::styled(
                format!(
                    "Desired {} @{}",
                    state.desired_diff_cursor + 1,
                    state.desired_horizontal_offset
                ),
                Style::default().fg(Color::Green),
            ),
        ]),
        tui_shell::control_line(&[
            ("Tab", Color::Blue, "switch pane"),
            ("Up/Down", Color::Blue, "scroll"),
            ("[/]", Color::Blue, "item"),
            ("PgUp/PgDn", Color::Blue, "jump"),
        ]),
        tui_shell::control_line(&[
            ("Home/End", Color::Blue, "bounds"),
            ("Space", Color::Yellow, "keep/drop"),
            ("c", Color::Green, "confirm staged"),
            ("Esc/q", Color::Gray, "return"),
        ]),
    ]
}
