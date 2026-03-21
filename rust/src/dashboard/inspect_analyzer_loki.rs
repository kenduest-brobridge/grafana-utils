//! Loki analyzer for dashboard query inspection.
//! Parses stream selectors, label matchers, pipeline operations, and obvious line filters.
use regex::Regex;
use serde_json::{Map, Value};

use super::inspect::QueryAnalysis;

fn ordered_unique_push(values: &mut Vec<String>, candidate: &str) {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return;
    }
    if !values.iter().any(|value| value == trimmed) {
        values.push(trimmed.to_string());
    }
}

fn extract_loki_stream_selectors(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut in_quotes = false;
    let mut escaped = false;
    let mut capture_start: Option<usize> = None;
    for (index, character) in query_text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' if in_quotes => {
                escaped = true;
            }
            '"' => {
                in_quotes = !in_quotes;
            }
            '{' if !in_quotes => {
                capture_start = Some(index);
            }
            '}' if !in_quotes => {
                if let Some(start) = capture_start.take() {
                    ordered_unique_push(
                        &mut values,
                        &query_text[start..index + character.len_utf8()],
                    );
                }
            }
            _ => {}
        }
    }
    values
}

fn extract_loki_label_matchers(query_text: &str) -> Vec<String> {
    let regex = Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*\s*(?:=|!=|=~|!~)\s*"(?:\\.|[^"\\])*")"#)
        .expect("invalid hard-coded loki label matcher regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut values, value.as_str());
        }
    }
    values
}

fn extract_loki_pipeline_metrics(query_text: &str) -> Vec<String> {
    let quoted_regex =
        Regex::new(r#""(?:\\.|[^"\\])*""#).expect("invalid hard-coded loki quoted regex");
    let function_regex = Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")
        .expect("invalid hard-coded loki function regex");
    let aggregation_regex =
        Regex::new(r"\b(sum|min|max|avg|count|topk|bottomk|count_values|quantile)\b")
            .expect("invalid hard-coded loki aggregation regex");
    let stage_regex = Regex::new(r"\|\s*([A-Za-z_][A-Za-z0-9_]*)(?:\s|\(|$)")
        .expect("invalid hard-coded loki stage regex");
    let sanitized_query = quoted_regex.replace_all(query_text, "\"\"");
    let mut values = Vec::new();
    for captures in aggregation_regex.captures_iter(&sanitized_query) {
        if let Some(value) = captures.get(1) {
            if let Some(full_match) = captures.get(0) {
                let trailing = sanitized_query[full_match.end()..].trim_start();
                if trailing.starts_with('(')
                    || trailing.starts_with("by ")
                    || trailing.starts_with("without ")
                    || trailing.starts_with("by(")
                    || trailing.starts_with("without(")
                {
                    ordered_unique_push(&mut values, value.as_str());
                }
            }
        }
    }
    for captures in function_regex.captures_iter(&sanitized_query) {
        if let Some(value) = captures.get(1) {
            let name = value.as_str();
            if matches!(
                name,
                "by" | "without" | "on" | "ignoring" | "group_left" | "group_right"
            ) {
                continue;
            }
            ordered_unique_push(&mut values, name);
        }
    }
    for captures in stage_regex.captures_iter(&sanitized_query) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut values, value.as_str());
        }
    }
    for value in extract_loki_line_filter_hints(query_text) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

fn extract_loki_line_filter_hints(query_text: &str) -> Vec<String> {
    let quoted_regex =
        Regex::new(r#""(?:\\.|[^"\\])*""#).expect("invalid hard-coded loki quoted regex");
    let sanitized_query = quoted_regex.replace_all(query_text, "\"\"");
    let regex = Regex::new(r#"\|\s*(=|~)\s*"(?:\\.|[^"\\])*""#)
        .expect("invalid hard-coded loki line filter regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(&sanitized_query) {
        let Some(operator) = captures.get(1).map(|item| item.as_str()) else {
            continue;
        };
        let hint = match operator {
            "=" => "line_filter_contains",
            "~" => "line_filter_regex",
            _ => continue,
        };
        ordered_unique_push(&mut values, hint);
    }
    values
}

fn extract_loki_range_windows(query_text: &str) -> Vec<String> {
    let regex = Regex::new(r"\[([^\]]+)\]").expect("invalid hard-coded loki range window regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut values, value.as_str());
        }
    }
    values
}

/// analyze query.
pub(crate) fn analyze_query(
    _panel: &Map<String, Value>,
    _target: &Map<String, Value>,
    _query_field: &str,
    query_text: &str,
) -> QueryAnalysis {
    let mut measurements = extract_loki_stream_selectors(query_text);
    for matcher in extract_loki_label_matchers(query_text) {
        ordered_unique_push(&mut measurements, &matcher);
    }
    QueryAnalysis {
        metrics: Vec::new(),
        functions: extract_loki_pipeline_metrics(query_text),
        measurements,
        buckets: extract_loki_range_windows(query_text),
    }
}
