use regex::Regex;
use serde_json::Value;

use super::super::string_field;

pub(super) fn is_sql_family(family: &str) -> bool {
    matches!(
        family.trim().to_ascii_lowercase().as_str(),
        "mysql" | "postgres" | "mssql" | "sql"
    )
}

pub(super) fn query_uses_time_filter(query_text: &str) -> bool {
    let lowered = query_text.trim().to_ascii_lowercase();
    lowered.contains("$__timefilter(")
        || lowered.contains("$__unixepochfilter(")
        || lowered.contains("$timefilter")
}

pub(super) fn parse_duration_seconds(value: &str) -> Option<usize> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("off") {
        return None;
    }
    let mut digits = String::new();
    let mut suffix = String::new();
    for character in trimmed.chars() {
        if character.is_ascii_digit() && suffix.is_empty() {
            digits.push(character);
        } else if !character.is_whitespace() {
            suffix.push(character);
        }
    }
    let number = digits.parse::<usize>().ok()?;
    let multiplier = match suffix.to_ascii_lowercase().as_str() {
        "ms" => 0,
        "s" | "" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 60 * 60 * 24,
        "w" => 60 * 60 * 24 * 7,
        _ => return None,
    };
    Some(number.saturating_mul(multiplier))
}

pub(super) fn prometheus_query_is_broad(query: &Value) -> bool {
    let query_text = string_field(query, "query");
    let family = string_field(query, "datasourceFamily");
    if !family.eq_ignore_ascii_case("prometheus")
        || query_text.is_empty()
        || query_text.contains('{')
        || query_text.contains(' ')
        || query_text.contains('(')
        || query_text.contains('[')
    {
        return false;
    }
    let metrics = query
        .get("metrics")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<&str>>()
        })
        .unwrap_or_default();
    metrics.len() == 1 && metrics[0] == query_text
}

pub(super) fn query_uses_regex_matchers(query_text: &str) -> bool {
    query_text.contains("=~") || query_text.contains("!~")
}

pub(super) fn query_uses_unscoped_loki_search(query: &Value) -> bool {
    if !string_field(query, "datasourceFamily").eq_ignore_ascii_case("loki") {
        return false;
    }
    let functions = query
        .get("functions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<&str>>()
        })
        .unwrap_or_default();
    let has_line_filter = functions
        .iter()
        .any(|value| value.starts_with("line_filter_"));
    if !has_line_filter {
        return false;
    }
    let measurements = query
        .get("measurements")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<&str>>()
        })
        .unwrap_or_default();
    !measurements.is_empty()
        && measurements
            .iter()
            .all(|value| *value == "{}" || !value.contains('=') || value.contains(".*"))
}

pub(super) fn query_dashboard_refresh_seconds(query: &Value) -> Option<usize> {
    for key in [
        "dashboardRefreshSeconds",
        "refreshIntervalSeconds",
        "refreshSeconds",
    ] {
        if let Some(seconds) = query.get(key).and_then(Value::as_u64) {
            return Some(seconds as usize);
        }
    }
    match query.get("refresh") {
        Some(Value::Number(number)) => number.as_u64().map(|value| value as usize),
        Some(Value::String(value)) => parse_duration_seconds(value),
        _ => None,
    }
}

pub(super) fn loki_query_is_broad(query_text: &str) -> bool {
    let lowered = query_text.trim().to_ascii_lowercase();
    lowered.contains("=~\".*\"")
        || lowered.contains("=~\".+\"")
        || lowered.contains("|~\".*\"")
        || lowered.contains("|~\".+\"")
        || lowered.contains("{}")
}

fn value_array_len(record: &Value, key: &str) -> usize {
    record
        .get(key)
        .and_then(Value::as_array)
        .map(|values| values.len())
        .unwrap_or(0)
}

pub(super) fn score_query_complexity(query: &Value) -> usize {
    let query_text = string_field(query, "query");
    if query_text.is_empty() {
        return 0;
    }
    let token_pattern = Regex::new(
        r"\b(sum|avg|min|max|count|rate|increase|histogram_quantile|label_replace|topk|bottomk|join|union|group by|order by)\b",
    )
    .unwrap();
    let lowered = query_text.to_ascii_lowercase();
    let mut score = 1usize;
    if query_text.len() > 80 {
        score += 1;
    }
    if query_text.len() > 160 {
        score += 1;
    }
    score += std::cmp::min(3, token_pattern.find_iter(&query_text).count());
    score += std::cmp::min(2, lowered.matches('|').count());
    if query_text.contains("=~") || query_text.contains("!~") {
        score += 1;
    }
    if query_text.contains('(') && query_text.contains(')') {
        score += std::cmp::min(2, query_text.matches('(').count() / 2);
    }
    score += std::cmp::min(2, value_array_len(query, "metrics"));
    score += std::cmp::min(1, value_array_len(query, "measurements"));
    score += std::cmp::min(1, value_array_len(query, "buckets"));
    score
}
