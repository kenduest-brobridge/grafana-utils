use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

use crate::dashboard::inspect_query::ordered_unique_push;

#[path = "inspect_extract_query_builder.rs"]
mod inspect_extract_query_builder;

pub(crate) use inspect_extract_query_builder::extract_query_field_and_text;

static NAME_MATCHER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"__name__\s*=\s*"([A-Za-z_:][A-Za-z0-9_:]*)""#)
        .expect("invalid hard-coded query report regex")
});
static METRIC_TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z_:][A-Za-z0-9_:]*").expect("invalid hard-coded metric regex")
});
static QUOTED_PROMQL_STRING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#""(?:\\.|[^"\\])*""#).expect("invalid hard-coded quoted string regex")
});
static VECTOR_MATCHING_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:by|without|on|ignoring)\s*\(\s*[^)]*\)")
        .expect("invalid hard-coded promql vector matching regex")
});
static GROUP_MODIFIER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:group_left|group_right)\s*(?:\(\s*[^)]*\))?")
        .expect("invalid hard-coded promql group modifier regex")
});
static PROMQL_MATCHER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{[^{}]*\}").expect("invalid hard-coded promql matcher regex"));
static INFLUXQL_SELECT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)^\s*select\s+(.*?)\s+\bfrom\b"#)
        .expect("invalid hard-coded influxql select regex")
});
static INFLUXQL_ALIAS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\bas\s+"[^"]+""#).expect("invalid hard-coded influxql alias regex")
});
static PROMQL_FUNCTION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")
        .expect("invalid hard-coded promql function regex")
});
static SQL_BLOCK_COMMENT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)/\*.*?\*/").expect("invalid hard-coded sql comment regex"));
static SQL_LINE_COMMENT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"--[^\n]*").expect("invalid hard-coded sql line comment regex"));
static INFLUXQL_FROM_MEASUREMENT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\bFROM\s+"?([A-Za-z0-9_.:-]+)"?"#)
        .expect("invalid hard-coded query report regex")
});
static INFLUX_MEASUREMENT_EQUALS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"_measurement\s*==\s*"([^"]+)""#).expect("invalid hard-coded query report regex")
});
static FLUX_BUCKET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"from\s*\(\s*bucket\s*:\s*"([^"]+)""#)
        .expect("invalid hard-coded query report regex")
});
static PROMETHEUS_RANGE_WINDOW_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\[([^\[\]]+)\]"#).expect("invalid hard-coded query report regex")
});
static INFLUXQL_TIME_WINDOW_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\btime\s*\(\s*([^)]+?)\s*\)"#).expect("invalid hard-coded query report regex")
});
static DOUBLE_QUOTED_CAPTURE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""([^"]+)""#).expect("invalid hard-coded query report regex"));
static FLUX_FIRST_PIPE_FUNCTION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\("#)
        .expect("invalid hard-coded query report regex")
});
static FLUX_PIPE_FUNCTION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\|>\s*([A-Za-z_][A-Za-z0-9_]*)\s*\("#)
        .expect("invalid hard-coded query report regex")
});
static SQL_CTE_NAME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\bwith\s+([A-Za-z_][A-Za-z0-9_$]*)\s+as\s*\("#)
        .expect("invalid hard-coded query report regex")
});
static SQL_SOURCE_REFERENCE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b(?:from|join|update|into|delete\s+from)\s+((?:[A-Za-z_][A-Za-z0-9_$]*|"[^"]+"|`[^`]+`|\[[^\]]+\])(?:\s*\.\s*(?:[A-Za-z_][A-Za-z0-9_$]*|"[^"]+"|`[^`]+`|\[[^\]]+\])){0,2})"#)
        .expect("invalid hard-coded query report regex")
});
static SQL_SHAPE_REGEXES: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    [
        ("with", r"\bwith\b"),
        ("select", r"\bselect\b"),
        ("insert", r"\binsert\s+into\b"),
        ("update", r"\bupdate\b"),
        ("delete", r"\bdelete\s+from\b"),
        ("distinct", r"\bdistinct\b"),
        ("join", r"\bjoin\b"),
        ("where", r"\bwhere\b"),
        ("group_by", r"\bgroup\s+by\b"),
        ("having", r"\bhaving\b"),
        ("order_by", r"\border\s+by\b"),
        ("limit", r"\blimit\b"),
        ("top", r"\btop\s+\d+\b"),
        ("union", r"\bunion(?:\s+all)?\b"),
        ("window", r"\bover\s*\("),
        ("subquery", r"\b(?:from|join)\s*\("),
    ]
    .into_iter()
    .map(|(name, pattern)| {
        (
            name,
            Regex::new(pattern).expect("invalid hard-coded sql shape regex"),
        )
    })
    .collect()
});

fn string_list_field(target: &Map<String, Value>, key: &str) -> Vec<String> {
    target
        .get(key)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

fn quoted_captures(text: &str, regex: &Regex) -> Vec<String> {
    let mut values = std::collections::BTreeSet::new();
    for captures in regex.captures_iter(text) {
        if let Some(value) = captures.get(1).map(|item| item.as_str().trim()) {
            if !value.is_empty() {
                values.insert(value.to_string());
            }
        }
    }
    values.into_iter().collect()
}

pub(crate) fn extract_metric_names(query_text: &str) -> Vec<String> {
    if query_text.trim().is_empty() {
        return Vec::new();
    }
    let mut values = std::collections::BTreeSet::new();
    let reserved_words = [
        "and",
        "bool",
        "by",
        "group_left",
        "group_right",
        "ignoring",
        "offset",
        "on",
        "or",
        "unless",
        "without",
    ];
    for capture in quoted_captures(query_text, &NAME_MATCHER_REGEX) {
        values.insert(capture);
    }
    for matched in METRIC_TOKEN_REGEX.find_iter(query_text) {
        let start = matched.start();
        let end = matched.end();
        let previous = query_text[..start].chars().next_back();
        if previous
            .map(|value| value.is_ascii_alphanumeric() || value == '_' || value == ':')
            .unwrap_or(false)
        {
            continue;
        }
        let next = query_text[end..].chars().next();
        if next
            .map(|value| value.is_ascii_alphanumeric() || value == '_' || value == ':')
            .unwrap_or(false)
        {
            continue;
        }
        let token = matched.as_str();
        if reserved_words.contains(&token) {
            continue;
        }
        if query_text[end..].trim_start().starts_with('(') {
            continue;
        }
        values.insert(token.to_string());
    }
    values.into_iter().collect()
}

pub(crate) fn extract_prometheus_metric_names(query_text: &str) -> Vec<String> {
    if query_text.trim().is_empty() {
        return Vec::new();
    }
    let mut values = std::collections::BTreeSet::new();
    let reserved_words = [
        "and",
        "bool",
        "by",
        "group_left",
        "group_right",
        "ignoring",
        "offset",
        "on",
        "or",
        "unless",
        "without",
        "sum",
        "min",
        "max",
        "avg",
        "count",
        "stddev",
        "stdvar",
        "bottomk",
        "topk",
        "quantile",
        "count_values",
        "rate",
        "irate",
        "increase",
        "delta",
        "idelta",
        "deriv",
        "predict_linear",
        "holt_winters",
        "sort",
        "sort_desc",
        "label_replace",
        "label_join",
        "histogram_quantile",
        "clamp_max",
        "clamp_min",
        "abs",
        "absent",
        "ceil",
        "floor",
        "ln",
        "log2",
        "log10",
        "round",
        "scalar",
        "vector",
        "year",
        "month",
        "day_of_month",
        "day_of_week",
        "hour",
        "minute",
        "time",
    ];
    for capture in quoted_captures(query_text, &NAME_MATCHER_REGEX) {
        values.insert(capture);
    }
    let sanitized_query = QUOTED_PROMQL_STRING_REGEX.replace_all(query_text, "\"\"");
    let sanitized_query = VECTOR_MATCHING_REGEX.replace_all(&sanitized_query, " ");
    let sanitized_query = GROUP_MODIFIER_REGEX.replace_all(&sanitized_query, " ");
    let sanitized_query = PROMQL_MATCHER_REGEX.replace_all(&sanitized_query, "{}");
    for matched in METRIC_TOKEN_REGEX.find_iter(&sanitized_query) {
        let start = matched.start();
        let end = matched.end();
        let previous = sanitized_query[..start].chars().next_back();
        if previous
            .map(|value| value.is_ascii_alphanumeric() || value == '_' || value == ':')
            .unwrap_or(false)
        {
            continue;
        }
        let next = sanitized_query[end..].chars().next();
        if next
            .map(|value| value.is_ascii_alphanumeric() || value == '_' || value == ':')
            .unwrap_or(false)
        {
            continue;
        }
        let token = matched.as_str();
        if reserved_words.contains(&token) {
            continue;
        }
        let trailing = sanitized_query[end..].trim_start();
        if trailing.starts_with('(') {
            continue;
        }
        if ["=", "!=", "=~", "!~"]
            .iter()
            .any(|operator| trailing.starts_with(operator))
        {
            continue;
        }
        values.insert(token.to_string());
    }
    values.into_iter().collect()
}

pub(crate) fn extract_query_measurements(
    target: &Map<String, Value>,
    query_text: &str,
) -> Vec<String> {
    let mut values = std::collections::BTreeSet::new();
    if let Some(measurement) = target.get("measurement").and_then(Value::as_str) {
        let trimmed = measurement.trim();
        if !trimmed.is_empty() {
            values.insert(trimmed.to_string());
        }
    }
    for value in string_list_field(target, "measurements") {
        values.insert(value);
    }
    for value in quoted_captures(query_text, &INFLUXQL_FROM_MEASUREMENT_REGEX) {
        values.insert(value);
    }
    for value in quoted_captures(query_text, &INFLUX_MEASUREMENT_EQUALS_REGEX) {
        values.insert(value);
    }
    values.into_iter().collect()
}

pub(crate) fn extract_query_buckets(target: &Map<String, Value>, query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    if let Some(bucket) = target.get("bucket").and_then(Value::as_str) {
        let trimmed = bucket.trim();
        if !trimmed.is_empty() {
            ordered_unique_push(&mut values, trimmed);
        }
    }
    for value in string_list_field(target, "buckets") {
        ordered_unique_push(&mut values, &value);
    }
    for value in quoted_captures(query_text, &FLUX_BUCKET_REGEX) {
        ordered_unique_push(&mut values, &value);
    }
    for value in extract_influxql_time_windows(query_text) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

pub(crate) fn extract_prometheus_range_windows(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for value in quoted_captures(query_text, &PROMETHEUS_RANGE_WINDOW_REGEX) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

pub(crate) fn extract_influxql_time_windows(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    if !query_text.to_ascii_lowercase().contains("group by") {
        return values;
    }
    for value in quoted_captures(query_text, &INFLUXQL_TIME_WINDOW_REGEX) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

fn extract_influxql_select_clause(query_text: &str) -> Option<String> {
    let query_text = strip_sql_comments(query_text);
    INFLUXQL_SELECT_REGEX
        .captures(&query_text)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn extract_influxql_select_metrics(query_text: &str) -> Vec<String> {
    let Some(select_clause) = extract_influxql_select_clause(query_text) else {
        return Vec::new();
    };
    let select_clause = INFLUXQL_ALIAS_REGEX
        .replace_all(&select_clause, "")
        .into_owned();
    let mut values = Vec::new();
    for value in quoted_captures(&select_clause, &DOUBLE_QUOTED_CAPTURE_REGEX) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

pub(crate) fn extract_influxql_select_functions(query_text: &str) -> Vec<String> {
    let Some(select_clause) = extract_influxql_select_clause(query_text) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for value in quoted_captures(&select_clause, &PROMQL_FUNCTION_REGEX) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

pub(crate) fn extract_prometheus_functions(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for captures in PROMQL_FUNCTION_REGEX.captures_iter(query_text) {
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
    values
}

pub(crate) fn extract_flux_pipeline_functions(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    if let Some(value) = quoted_captures(query_text, &FLUX_FIRST_PIPE_FUNCTION_REGEX)
        .into_iter()
        .next()
    {
        ordered_unique_push(&mut values, &value);
    }
    for value in quoted_captures(query_text, &FLUX_PIPE_FUNCTION_REGEX) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

fn strip_sql_comments(query_text: &str) -> String {
    let without_blocks = SQL_BLOCK_COMMENT_REGEX.replace_all(query_text, " ");
    SQL_LINE_COMMENT_REGEX
        .replace_all(&without_blocks, " ")
        .into_owned()
}

fn normalize_sql_identifier(value: &str) -> String {
    value
        .split('.')
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                return None;
            }
            let normalized = if trimmed.len() >= 2
                && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
                    || (trimmed.starts_with('`') && trimmed.ends_with('`'))
                    || (trimmed.starts_with('[') && trimmed.ends_with(']')))
            {
                &trimmed[1..trimmed.len() - 1]
            } else {
                trimmed
            };
            let normalized = normalized.trim();
            if normalized.is_empty() {
                None
            } else {
                Some(normalized.to_string())
            }
        })
        .collect::<Vec<String>>()
        .join(".")
}

pub(crate) fn extract_sql_source_references(query_text: &str) -> Vec<String> {
    let query_text = strip_sql_comments(query_text);
    if query_text.trim().is_empty() {
        return Vec::new();
    }
    let cte_names = quoted_captures(&query_text, &SQL_CTE_NAME_REGEX)
        .into_iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<std::collections::BTreeSet<String>>();
    let mut values = Vec::new();
    for value in quoted_captures(&query_text, &SQL_SOURCE_REFERENCE_REGEX) {
        let normalized = normalize_sql_identifier(&value);
        if !normalized.is_empty() && !cte_names.contains(&normalized.to_ascii_lowercase()) {
            ordered_unique_push(&mut values, &normalized);
        }
    }
    values
}

pub(crate) fn extract_sql_query_shape_hints(query_text: &str) -> Vec<String> {
    let lowered = strip_sql_comments(query_text).to_ascii_lowercase();
    let mut values = Vec::new();
    for (name, regex) in SQL_SHAPE_REGEXES.iter() {
        if regex.is_match(&lowered) {
            values.push(name.to_string());
        }
    }
    values
}
