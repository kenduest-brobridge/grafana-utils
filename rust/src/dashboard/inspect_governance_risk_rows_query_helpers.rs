use crate::dashboard::inspect_report::ExportInspectionQueryRow;

use super::super::inspect_governance_risk_spec::parse_duration_seconds;
use crate::dashboard::inspect_family::normalize_family_name;

pub(super) fn query_uses_broad_prometheus_selector(row: &ExportInspectionQueryRow) -> bool {
    if normalize_family_name(&row.datasource_type) != "prometheus" {
        return false;
    }
    let query_text = row.query_text.trim();
    if query_text.is_empty() || !row.measurements.is_empty() || query_text.contains('{') {
        return false;
    }
    if row.metrics.len() != 1
        || query_text.contains(' ')
        || query_text.contains('(')
        || query_text.contains('[')
    {
        return false;
    }
    row.metrics[0].trim() == query_text
}

pub(super) fn query_uses_prometheus_regex(row: &ExportInspectionQueryRow) -> bool {
    normalize_family_name(&row.datasource_type) == "prometheus"
        && (row.query_text.contains("=~") || row.query_text.contains("!~"))
}

pub(super) fn prometheus_regex_matcher_count(row: &ExportInspectionQueryRow) -> usize {
    if normalize_family_name(&row.datasource_type) != "prometheus" {
        return 0;
    }
    row.query_text.matches("=~").count() + row.query_text.matches("!~").count()
}

pub(super) fn query_uses_high_cardinality_prometheus_regex(row: &ExportInspectionQueryRow) -> bool {
    if normalize_family_name(&row.datasource_type) != "prometheus" {
        return false;
    }
    const HIGH_CARDINALITY_LABELS: [&str; 8] = [
        "instance",
        "pod",
        "container",
        "endpoint",
        "path",
        "uri",
        "name",
        "id",
    ];
    HIGH_CARDINALITY_LABELS.iter().any(|label| {
        row.query_text.contains(&format!("{label}=~"))
            || row.query_text.contains(&format!("{label}!~"))
    })
}

pub(super) fn prometheus_aggregation_depth(row: &ExportInspectionQueryRow) -> usize {
    if normalize_family_name(&row.datasource_type) != "prometheus" {
        return 0;
    }
    const AGGREGATORS: [&str; 11] = [
        "sum",
        "avg",
        "min",
        "max",
        "count",
        "group",
        "count_values",
        "quantile",
        "topk",
        "bottomk",
        "stddev",
    ];
    row.functions
        .iter()
        .filter(|function: &&String| AGGREGATORS.contains(&function.as_str()))
        .count()
}

pub(super) fn prometheus_estimated_series_risk(
    broad_selector: bool,
    regex_matcher_count: usize,
    high_cardinality_regex: bool,
    aggregation_depth: usize,
    largest_bucket_seconds: Option<u64>,
) -> String {
    let mut score = 0usize;
    if broad_selector {
        score += 2;
    }
    if regex_matcher_count != 0 {
        score += 1;
    }
    if high_cardinality_regex {
        score += 2;
    }
    if aggregation_depth >= 2 {
        score += 1;
    }
    if largest_bucket_seconds.unwrap_or(0) >= 60 * 60 {
        score += 2;
    }
    match score {
        0 => "low".to_string(),
        1 | 2 => "medium".to_string(),
        3 | 4 => "high".to_string(),
        _ => "critical".to_string(),
    }
}

pub(super) fn largest_bucket_seconds(row: &ExportInspectionQueryRow) -> Option<u64> {
    row.buckets
        .iter()
        .filter_map(|bucket| parse_duration_seconds(bucket))
        .max()
}

fn loki_selector_has_concrete_matcher(selector: &str) -> bool {
    let inner = selector
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}');
    if inner.is_empty() {
        return false;
    }
    for matcher in split_loki_selector_matchers(inner) {
        let matcher = matcher.trim();
        if matcher.is_empty() {
            continue;
        }
        if let Some((_, value)) = matcher.split_once("=~") {
            if !loki_regex_is_wildcard(value) {
                return true;
            }
            continue;
        }
        if matcher.contains("!~") || matcher.contains("!=") || matcher.contains('=') {
            return true;
        }
    }
    false
}

pub(super) fn extract_loki_stream_selectors(query_text: &str) -> Vec<String> {
    let mut selectors = Vec::new();
    let mut start = 0usize;
    while let Some(open) = query_text[start..].find('{') {
        let open = start + open;
        let Some(close) = query_text[open..].find('}') else {
            break;
        };
        let close = open + close;
        selectors.push(query_text[open..=close].to_string());
        start = close + 1;
    }
    selectors
}

fn split_loki_selector_matchers(selector: &str) -> Vec<String> {
    let mut matchers = Vec::new();
    let mut depth = 0usize;
    let mut last = 0usize;
    for (index, character) in selector.char_indices() {
        match character {
            '"' => {
                depth ^= 1;
            }
            ',' if depth == 0 => {
                matchers.push(selector[last..index].to_string());
                last = index + 1;
            }
            _ => {}
        }
    }
    if last <= selector.len() {
        matchers.push(selector[last..].to_string());
    }
    matchers
}

fn loki_regex_is_wildcard(value: &str) -> bool {
    let trimmed = value.trim().trim_matches('"');
    trimmed == ".*" || trimmed == ".+"
}

fn loki_selector_is_broad(selector: &str) -> bool {
    let inner = selector
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}');
    if inner.is_empty() {
        return true;
    }
    let mut saw_matcher = false;
    for matcher in split_loki_selector_matchers(inner) {
        let matcher = matcher.trim();
        if matcher.is_empty() {
            continue;
        }
        saw_matcher = true;
        if let Some((_, value)) = matcher.split_once("=~") {
            if !loki_regex_is_wildcard(value) {
                return false;
            }
            continue;
        }
        if matcher.contains("!~") || matcher.contains("!=") || matcher.contains('=') {
            return false;
        }
        return false;
    }
    saw_matcher
}

pub(crate) fn find_broad_loki_selector(query_text: &str) -> Option<String> {
    extract_loki_stream_selectors(query_text)
        .into_iter()
        .find(|selector| loki_selector_is_broad(selector))
}

pub(super) fn query_uses_unscoped_loki_search(row: &ExportInspectionQueryRow) -> bool {
    if normalize_family_name(&row.datasource_type) != "loki" {
        return false;
    }
    let has_line_filter = row.functions.iter().any(|function: &String| {
        function.starts_with("line_filter_")
            || function.contains("pattern")
            || function.contains("regexp")
    });
    if !has_line_filter {
        return false;
    }
    let selectors = extract_loki_stream_selectors(&row.query_text);
    !selectors.is_empty()
        && selectors
            .iter()
            .all(|selector| !loki_selector_has_concrete_matcher(selector))
}
