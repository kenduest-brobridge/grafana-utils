use regex::Regex;
use serde_json::{Map, Value};

use crate::common::string_field;
use crate::dashboard::inspect_query::ordered_unique_push;

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

fn quoted_captures(text: &str, pattern: &str) -> Vec<String> {
    let regex = Regex::new(pattern).expect("invalid hard-coded query report regex");
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

pub(crate) fn extract_query_field_and_text(target: &Map<String, Value>) -> (String, String) {
    for key in [
        "expr",
        "expression",
        "query",
        "logql",
        "rawSql",
        "sql",
        "rawQuery",
    ] {
        if let Some(value) = target.get(key).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return (key.to_string(), trimmed.to_string());
            }
        }
    }
    let synthesized = synthesize_influx_builder_query(target);
    if !synthesized.is_empty() {
        return ("builder".to_string(), synthesized);
    }
    (String::new(), String::new())
}

fn first_step_param(step: &Map<String, Value>) -> String {
    step.get("params")
        .and_then(Value::as_array)
        .and_then(|params| params.first())
        .map(|value| match value {
            Value::String(text) => text.trim().to_string(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

fn render_influx_select_chain(chain: &Value) -> String {
    let Some(steps) = chain.as_array() else {
        return String::new();
    };
    let mut expression = String::new();
    for step in steps {
        let Some(step_object) = step.as_object() else {
            continue;
        };
        let step_type = string_field(step_object, "type", "");
        let param = first_step_param(step_object);
        match step_type.as_str() {
            "field" => {
                if !param.is_empty() {
                    expression = format!("\"{param}\"");
                }
            }
            "math" => {
                if !param.is_empty() {
                    if expression.is_empty() {
                        expression = param;
                    } else {
                        expression.push_str(&param);
                    }
                }
            }
            "alias" => {}
            "" => {}
            _ => {
                if !expression.is_empty() {
                    expression = format!("{step_type}({expression})");
                } else if !param.is_empty() {
                    expression = format!("{step_type}({param})");
                } else {
                    expression = format!("{step_type}()");
                }
            }
        }
    }
    expression.trim().to_string()
}

fn render_influx_group_by_clause(group_by: Option<&Value>) -> String {
    let Some(items) = group_by.and_then(Value::as_array) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for item in items {
        let Some(group_object) = item.as_object() else {
            continue;
        };
        let group_type = string_field(group_object, "type", "");
        let param = first_step_param(group_object);
        let rendered = match group_type.as_str() {
            "time" if !param.is_empty() => format!("time({param})"),
            "fill" if !param.is_empty() => format!("fill({param})"),
            "tag" if !param.is_empty() => format!("\"{param}\""),
            _ if !group_type.is_empty() && !param.is_empty() => format!("{group_type}({param})"),
            _ if !group_type.is_empty() => group_type,
            _ => String::new(),
        };
        if !rendered.is_empty() {
            parts.push(rendered);
        }
    }
    parts.join(", ")
}

fn render_influx_where_clause(tags: Option<&Value>) -> String {
    let Some(items) = tags.and_then(Value::as_array) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for item in items {
        let Some(tag_object) = item.as_object() else {
            continue;
        };
        let key = string_field(tag_object, "key", "");
        let operator = string_field(tag_object, "operator", "=");
        let value = string_field(tag_object, "value", "");
        if key.is_empty() || value.is_empty() {
            continue;
        }
        let condition = string_field(tag_object, "condition", "").to_ascii_uppercase();
        if !parts.is_empty() && (condition == "AND" || condition == "OR") {
            parts.push(condition);
        }
        parts.push(format!("\"{key}\" {operator} {value}"));
    }
    parts.join(" ")
}

fn synthesize_influx_builder_query(target: &Map<String, Value>) -> String {
    let measurement = string_field(target, "measurement", "");
    let select_parts = target
        .get("select")
        .and_then(Value::as_array)
        .map(|chains| {
            chains
                .iter()
                .map(render_influx_select_chain)
                .filter(|value| !value.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    if measurement.is_empty() && select_parts.is_empty() {
        return String::new();
    }
    let mut query = format!(
        "SELECT {}",
        if select_parts.is_empty() {
            "*".to_string()
        } else {
            select_parts.join(", ")
        }
    );
    if !measurement.is_empty() {
        query.push_str(&format!(" FROM \"{measurement}\""));
    }
    let where_clause = render_influx_where_clause(target.get("tags"));
    if !where_clause.is_empty() {
        query.push_str(&format!(" WHERE {where_clause}"));
    }
    let group_by_clause = render_influx_group_by_clause(target.get("groupBy"));
    if !group_by_clause.is_empty() {
        query.push_str(&format!(" GROUP BY {group_by_clause}"));
    }
    query
}

pub(crate) fn extract_metric_names(query_text: &str) -> Vec<String> {
    if query_text.trim().is_empty() {
        return Vec::new();
    }
    let token_regex =
        Regex::new(r"[A-Za-z_:][A-Za-z0-9_:]*").expect("invalid hard-coded metric regex");
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
    for capture in quoted_captures(query_text, r#"__name__\s*=\s*"([A-Za-z_:][A-Za-z0-9_:]*)""#) {
        values.insert(capture);
    }
    for matched in token_regex.find_iter(query_text) {
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
    let token_regex =
        Regex::new(r"[A-Za-z_:][A-Za-z0-9_:]*").expect("invalid hard-coded metric regex");
    let quoted_regex =
        Regex::new(r#""(?:\\.|[^"\\])*""#).expect("invalid hard-coded quoted string regex");
    let vector_matching_regex = Regex::new(r"\b(?:by|without|on|ignoring)\s*\(\s*[^)]*\)")
        .expect("invalid hard-coded promql vector matching regex");
    let group_modifier_regex = Regex::new(r"\b(?:group_left|group_right)\s*(?:\(\s*[^)]*\))?")
        .expect("invalid hard-coded promql group modifier regex");
    let matcher_regex = Regex::new(r"\{[^{}]*\}").expect("invalid hard-coded promql matcher regex");
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
    for capture in quoted_captures(query_text, r#"__name__\s*=\s*"([A-Za-z_:][A-Za-z0-9_:]*)""#) {
        values.insert(capture);
    }
    let sanitized_query = quoted_regex.replace_all(query_text, "\"\"");
    let sanitized_query = vector_matching_regex.replace_all(&sanitized_query, " ");
    let sanitized_query = group_modifier_regex.replace_all(&sanitized_query, " ");
    let sanitized_query = matcher_regex.replace_all(&sanitized_query, "{}");
    for matched in token_regex.find_iter(&sanitized_query) {
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
    for value in quoted_captures(query_text, r#"(?i)\bFROM\s+"?([A-Za-z0-9_.:-]+)"?"#) {
        values.insert(value);
    }
    for value in quoted_captures(query_text, r#"_measurement\s*==\s*"([^"]+)""#) {
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
    for value in quoted_captures(query_text, r#"from\s*\(\s*bucket\s*:\s*"([^"]+)""#) {
        ordered_unique_push(&mut values, &value);
    }
    for value in extract_influxql_time_windows(query_text) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

pub(crate) fn extract_prometheus_range_windows(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for value in quoted_captures(query_text, r#"\[([^\[\]]+)\]"#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

pub(crate) fn extract_influxql_time_windows(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    if !query_text.to_ascii_lowercase().contains("group by") {
        return values;
    }
    for value in quoted_captures(query_text, r#"(?i)\btime\s*\(\s*([^)]+?)\s*\)"#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

fn extract_influxql_select_clause(query_text: &str) -> Option<String> {
    let query_text = strip_sql_comments(query_text);
    let regex = Regex::new(r#"(?is)^\s*select\s+(.*?)\s+\bfrom\b"#)
        .expect("invalid hard-coded influxql select regex");
    regex
        .captures(&query_text)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn extract_influxql_select_metrics(query_text: &str) -> Vec<String> {
    let Some(select_clause) = extract_influxql_select_clause(query_text) else {
        return Vec::new();
    };
    let select_clause = Regex::new(r#"(?i)\bas\s+"[^"]+""#)
        .expect("invalid hard-coded influxql alias regex")
        .replace_all(&select_clause, "")
        .into_owned();
    let mut values = Vec::new();
    for value in quoted_captures(&select_clause, r#""([^"]+)""#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

pub(crate) fn extract_influxql_select_functions(query_text: &str) -> Vec<String> {
    let Some(select_clause) = extract_influxql_select_clause(query_text) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for value in quoted_captures(&select_clause, r#"\b([A-Za-z_][A-Za-z0-9_]*)\s*\("#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

pub(crate) fn extract_prometheus_functions(query_text: &str) -> Vec<String> {
    let regex = Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")
        .expect("invalid hard-coded promql function regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(query_text) {
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
    if let Some(value) = quoted_captures(query_text, r#"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\("#)
        .into_iter()
        .next()
    {
        ordered_unique_push(&mut values, &value);
    }
    for value in quoted_captures(query_text, r#"\|>\s*([A-Za-z_][A-Za-z0-9_]*)\s*\("#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

fn strip_sql_comments(query_text: &str) -> String {
    let block_regex = Regex::new(r"(?s)/\*.*?\*/").expect("invalid hard-coded sql comment regex");
    let line_regex = Regex::new(r"--[^\n]*").expect("invalid hard-coded sql line comment regex");
    let without_blocks = block_regex.replace_all(query_text, " ");
    line_regex.replace_all(&without_blocks, " ").into_owned()
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
    let cte_names = quoted_captures(
        &query_text,
        r#"(?i)\bwith\s+([A-Za-z_][A-Za-z0-9_$]*)\s+as\s*\("#,
    )
    .into_iter()
    .map(|value| value.to_ascii_lowercase())
    .collect::<std::collections::BTreeSet<String>>();
    let mut values = Vec::new();
    for value in quoted_captures(
        &query_text,
        r#"(?i)\b(?:from|join|update|into|delete\s+from)\s+((?:[A-Za-z_][A-Za-z0-9_$]*|"[^"]+"|`[^`]+`|\[[^\]]+\])(?:\s*\.\s*(?:[A-Za-z_][A-Za-z0-9_$]*|"[^"]+"|`[^`]+`|\[[^\]]+\])){0,2})"#,
    ) {
        let normalized = normalize_sql_identifier(&value);
        if !normalized.is_empty() && !cte_names.contains(&normalized.to_ascii_lowercase()) {
            ordered_unique_push(&mut values, &normalized);
        }
    }
    values
}

pub(crate) fn extract_sql_query_shape_hints(query_text: &str) -> Vec<String> {
    let lowered = strip_sql_comments(query_text).to_ascii_lowercase();
    let patterns = [
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
    ];
    let mut values = Vec::new();
    for (name, pattern) in patterns {
        let regex = Regex::new(pattern).expect("invalid hard-coded sql shape regex");
        if regex.is_match(&lowered) {
            values.push(name.to_string());
        }
    }
    values
}
