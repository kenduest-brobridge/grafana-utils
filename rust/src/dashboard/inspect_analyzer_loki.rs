//! Loki analyzer for dashboard query inspection.
//! Parses stream selectors, label matchers, pipeline operations, and obvious line filters.
use regex::Regex;
use serde_json::{Map, Value};

use super::inspect_query::QueryAnalysis;

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
    for selector in extract_loki_stream_selectors(query_text) {
        for captures in regex.captures_iter(&selector) {
            if let Some(value) = captures.get(1) {
                ordered_unique_push(&mut values, value.as_str());
            }
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
    let mut values = Vec::new();
    let bytes = query_text.as_bytes();
    let mut index = 0;
    let mut in_quotes = false;
    let mut escaped = false;
    let mut selector_depth = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if escaped {
            escaped = false;
            index += 1;
            continue;
        }
        match byte {
            b'\\' if in_quotes => {
                escaped = true;
                index += 1;
            }
            b'"' => {
                in_quotes = !in_quotes;
                index += 1;
            }
            b'{' if !in_quotes => {
                selector_depth += 1;
                index += 1;
            }
            b'}' if !in_quotes => {
                selector_depth = selector_depth.saturating_sub(1);
                index += 1;
            }
            b'|' | b'!' if !in_quotes && selector_depth == 0 => {
                let mut cursor = if byte == b'|' { index + 1 } else { index };
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                let Some(operator) = bytes.get(cursor).copied() else {
                    index += 1;
                    continue;
                };
                let (hint, separator_len) = match operator {
                    b'=' if byte == b'|' => ("line_filter_contains", 1),
                    b'~' if byte == b'|' => ("line_filter_regex", 1),
                    b'!' => match bytes.get(cursor + 1).copied() {
                        Some(b'=') => ("line_filter_not_contains", 2),
                        Some(b'~') => ("line_filter_not_regex", 2),
                        _ => {
                            index += 1;
                            continue;
                        }
                    },
                    _ => {
                        index += 1;
                        continue;
                    }
                };
                cursor += separator_len;
                while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                    cursor += 1;
                }
                if bytes.get(cursor) != Some(&b'"') {
                    index += 1;
                    continue;
                }
                let literal_start = cursor + 1;
                cursor += 1;
                let mut literal_escaped = false;
                while cursor < bytes.len() {
                    let current = bytes[cursor];
                    if literal_escaped {
                        literal_escaped = false;
                        cursor += 1;
                        continue;
                    }
                    match current {
                        b'\\' => {
                            literal_escaped = true;
                        }
                        b'"' => {
                            ordered_unique_push(&mut values, hint);
                            let literal = &query_text[literal_start..cursor];
                            if !literal.trim().is_empty() {
                                ordered_unique_push(&mut values, &format!("{hint}:{literal}"));
                            }
                            index = cursor + 1;
                            break;
                        }
                        _ => {}
                    }
                    cursor += 1;
                }
                if cursor >= bytes.len() {
                    break;
                }
            }
            _ => {
                index += 1;
            }
        }
    }
    values
}

fn extract_loki_range_windows(query_text: &str) -> Vec<String> {
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
            '[' if !in_quotes => {
                capture_start = Some(index + character.len_utf8());
            }
            ']' if !in_quotes => {
                if let Some(start) = capture_start.take() {
                    ordered_unique_push(&mut values, &query_text[start..index]);
                }
            }
            _ => {}
        }
    }
    values
}

fn split_loki_pipeline_segments(query_text: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut in_quotes = false;
    let mut escaped = false;
    let mut selector_depth = 0usize;
    let mut segment_start: Option<usize> = None;

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
                selector_depth += 1;
            }
            '}' if !in_quotes => {
                selector_depth = selector_depth.saturating_sub(1);
            }
            '|' if !in_quotes && selector_depth == 0 => {
                if let Some(start) = segment_start.take() {
                    let segment = query_text[start..index].trim();
                    if !segment.is_empty() {
                        segments.push(segment.to_string());
                    }
                }
                segment_start = Some(index + character.len_utf8());
            }
            _ => {}
        }
    }

    if let Some(start) = segment_start {
        let segment = query_text[start..].trim();
        if !segment.is_empty() {
            segments.push(segment.to_string());
        }
    }

    segments
}

fn extract_loki_pipeline_fields(query_text: &str) -> Vec<String> {
    let quoted_regex =
        Regex::new(r#""(?:\\.|[^"\\])*""#).expect("invalid hard-coded loki quoted regex");
    let unwrap_regex = Regex::new(r"(?i)\bunwrap\s+([A-Za-z_][A-Za-z0-9_.]*)\b")
        .expect("invalid hard-coded loki unwrap regex");
    let predicate_regex =
        Regex::new(r"(?i)\b([A-Za-z_][A-Za-z0-9_.]*)\b\s*(?:==|=~|!~|!=|>=|<=|=|>|<)")
            .expect("invalid hard-coded loki predicate regex");
    let stage_prefix_regex = Regex::new(
        r"(?i)^(?:json|logfmt|pattern|regexp|unpack|decolorize|drop|keep|label_format|line_format)\b",
    )
    .expect("invalid hard-coded loki stage prefix regex");
    let reserved = ["and", "or", "true", "false", "bool", "bytes", "duration"];

    let mut values = Vec::new();
    for segment in split_loki_pipeline_segments(query_text) {
        let sanitized_segment = quoted_regex.replace_all(&segment, "\"\"");
        for captures in unwrap_regex.captures_iter(&sanitized_segment) {
            if let Some(field) = captures.get(1) {
                ordered_unique_push(&mut values, field.as_str());
            }
        }

        let predicate_source = stage_prefix_regex
            .replace(&sanitized_segment, "")
            .trim()
            .to_string();
        if predicate_source.is_empty() {
            continue;
        }

        for captures in predicate_regex.captures_iter(&predicate_source) {
            let Some(field) = captures.get(1).map(|value| value.as_str()) else {
                continue;
            };
            if reserved
                .iter()
                .any(|keyword| keyword.eq_ignore_ascii_case(field))
            {
                continue;
            }
            ordered_unique_push(&mut values, field);
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
    for field in extract_loki_pipeline_fields(query_text) {
        ordered_unique_push(&mut measurements, &field);
    }
    QueryAnalysis {
        metrics: Vec::new(),
        functions: extract_loki_pipeline_metrics(query_text),
        measurements,
        buckets: extract_loki_range_windows(query_text),
    }
}
