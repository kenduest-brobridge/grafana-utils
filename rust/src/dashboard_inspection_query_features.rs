//! Shared family-aware query feature extraction for dashboard inspection contracts.

use crate::dashboard_reference_models::{
    dedupe_strings, normalize_family_name, DashboardQueryReference,
};
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct QueryFeatureHints {
    pub(crate) metrics: Vec<String>,
    pub(crate) functions: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
    pub(crate) labels: Vec<String>,
}

pub(crate) fn build_query_features(
    row: &Value,
    reference: &DashboardQueryReference,
) -> crate::dashboard_reference_models::QueryFeatureSet {
    let mut hints = parse_query_text_families(reference);
    let analysis_hints = parse_features_from_object(row);

    merge_unique(&mut hints.metrics, analysis_hints.metrics);
    merge_unique(&mut hints.functions, analysis_hints.functions);
    merge_unique(&mut hints.measurements, analysis_hints.measurements);
    merge_unique(&mut hints.buckets, analysis_hints.buckets);
    merge_unique(&mut hints.labels, analysis_hints.labels);

    crate::dashboard_reference_models::QueryFeatureSet {
        metrics: dedupe_strings(&hints.metrics),
        functions: dedupe_strings(&hints.functions),
        measurements: dedupe_strings(&hints.measurements),
        buckets: dedupe_strings(&hints.buckets),
        labels: dedupe_strings(&hints.labels),
    }
}

pub(crate) fn parse_query_text_families(row: &DashboardQueryReference) -> QueryFeatureHints {
    let family = normalize_family_name(&row.datasource_type);
    let query = &row.query;
    match family.as_str() {
        "prometheus" | "graphite" | "victoriametrics" => parse_prometheus_query_features(query),
        "loki" => parse_loki_query_features(query),
        "flux" | "influxdb" => parse_flux_query_features(query),
        "mysql" | "postgres" | "postgresql" | "sql" => parse_sql_query_features(query),
        _ => parse_unknown_query_features(query),
    }
}

fn merge_unique(target: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        if !target.iter().any(|item| item == &value) {
            target.push(value);
        }
    }
}

fn ordered_unique_push(values: &mut Vec<String>, candidate: &str) {
    let value = candidate.trim();
    if value.is_empty() {
        return;
    }
    if !values.iter().any(|item| item == value) {
        values.push(value.to_string());
    }
}

fn quoted_captures(query_text: &str, pattern: &str) -> Vec<String> {
    let regex = Regex::new(pattern).expect("invalid hard-coded regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            values.push(value.as_str().to_string());
        }
    }
    values
}

fn strip_quoted_text(query_text: &str) -> String {
    let mut out = String::with_capacity(query_text.len());
    let mut in_quotes = false;
    let mut escaped = false;

    for c in query_text.chars() {
        if escaped {
            escaped = false;
            out.push(' ');
            continue;
        }
        if c == '\\' && in_quotes {
            escaped = true;
            out.push(' ');
            continue;
        }
        if c == '"' {
            in_quotes = !in_quotes;
            out.push(' ');
            continue;
        }
        if in_quotes {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn extract_quoted_aware_enclosed(query_text: &str, open: char, close: char) -> Vec<String> {
    let mut values = Vec::new();
    let mut in_quotes = false;
    let mut escaped = false;
    let mut depth = 0usize;
    let mut begin = None;

    for (index, value) in query_text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match value {
            '\\' if in_quotes => {
                escaped = true;
            }
            '"' => {
                in_quotes = !in_quotes;
            }
            c if c == open && !in_quotes => {
                if depth == 0 {
                    begin = Some(index + c.len_utf8());
                }
                depth += 1;
            }
            c if c == close && !in_quotes => {
                if depth > 0 {
                    depth -= 1;
                }
                if depth == 0 {
                    if let Some(start) = begin.take() {
                        ordered_unique_push(&mut values, &query_text[start..index]);
                    }
                }
            }
            _ => {}
        }
    }
    values
}

fn parse_prometheus_query_features(query_text: &str) -> QueryFeatureHints {
    let mut functions = Vec::new();
    let mut metrics = Vec::new();
    let mut buckets = Vec::new();

    for value in extract_prometheus_metric_names(query_text) {
        ordered_unique_push(&mut metrics, &value);
    }
    for value in extract_prometheus_functions(query_text) {
        ordered_unique_push(&mut functions, &value);
    }
    for value in extract_prometheus_range_windows(query_text) {
        ordered_unique_push(&mut buckets, &value);
    }

    QueryFeatureHints {
        metrics: dedupe_strings(&metrics),
        functions: dedupe_strings(&functions),
        measurements: Vec::new(),
        buckets: dedupe_strings(&buckets),
        labels: Vec::new(),
    }
}

fn parse_loki_query_features(query_text: &str) -> QueryFeatureHints {
    let mut functions = Vec::new();
    let mut measurements = Vec::new();
    let mut buckets = Vec::new();
    let mut labels = Vec::new();

    for value in extract_loki_selectors(query_text) {
        ordered_unique_push(&mut measurements, &format!("{{{}}}", value));
        for matcher in extract_loki_label_matchers(&value) {
            ordered_unique_push(&mut measurements, &matcher);
            ordered_unique_push(&mut labels, &matcher);
        }
    }
    for value in extract_loki_pipeline_functions(query_text) {
        ordered_unique_push(&mut functions, &value);
    }
    for hint in extract_loki_filter_hints(query_text) {
        ordered_unique_push(&mut functions, &hint);
    }
    for value in extract_loki_range_windows(query_text) {
        ordered_unique_push(&mut buckets, &value);
    }

    QueryFeatureHints {
        metrics: Vec::new(),
        functions: dedupe_strings(&functions),
        measurements: dedupe_strings(&measurements),
        buckets: dedupe_strings(&buckets),
        labels: dedupe_strings(&labels),
    }
}

fn parse_flux_query_features(query_text: &str) -> QueryFeatureHints {
    let mut functions = Vec::new();
    let mut measurements = Vec::new();
    let mut buckets = Vec::new();

    if query_text.to_lowercase().trim_start().contains("from(") || query_text.contains("|>") {
        for value in extract_flux_pipeline_functions(query_text) {
            ordered_unique_push(&mut functions, &value);
        }
        for value in extract_flux_buckets(query_text) {
            ordered_unique_push(&mut buckets, &value);
        }
        for value in extract_flux_source_references(query_text) {
            ordered_unique_push(&mut measurements, &value);
        }
    }

    QueryFeatureHints {
        metrics: Vec::new(),
        functions: dedupe_strings(&functions),
        measurements: dedupe_strings(&measurements),
        buckets: dedupe_strings(&buckets),
        labels: Vec::new(),
    }
}

fn parse_sql_query_features(query_text: &str) -> QueryFeatureHints {
    QueryFeatureHints {
        metrics: Vec::new(),
        functions: extract_sql_query_shape_hints(query_text),
        measurements: extract_sql_source_references(query_text),
        buckets: Vec::new(),
        labels: Vec::new(),
    }
}

fn parse_unknown_query_features(query_text: &str) -> QueryFeatureHints {
    let sanitized = strip_quoted_text(&query_text.to_lowercase());
    let mut functions = Vec::new();
    for capture in quoted_captures(&sanitized, r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(") {
        ordered_unique_push(&mut functions, &capture);
    }
    QueryFeatureHints {
        metrics: Vec::new(),
        functions: dedupe_strings(&functions),
        measurements: Vec::new(),
        buckets: Vec::new(),
        labels: Vec::new(),
    }
}

fn extract_prometheus_metric_names(query_text: &str) -> Vec<String> {
    let query_text = strip_quoted_text(query_text);
    let token_regex =
        Regex::new(r"[A-Za-z_:][A-Za-z0-9_:]*").expect("invalid hard-coded metric regex");
    let quoted_name_regex =
        Regex::new(r#"\"(?:\\.|[^\"\\])*\""#).expect("invalid hard-coded quoted regex");
    let matcher_regex = Regex::new(r"\{[^{}]*\}").expect("invalid hard-coded matcher regex");
    let vector_matching_regex = Regex::new(r"\b(?:by|without|on|ignoring)\s*\(\s*[^)]*\)")
        .expect("invalid hard-coded vector matching regex");
    let group_modifier_regex = Regex::new(r"\b(?:group_left|group_right)\s*(?:\(\s*[^)]*\))?")
        .expect("invalid hard-coded group modifier regex");
    let mut values = Vec::new();
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

    let query = quoted_name_regex.replace_all(&query_text, "\"\"");
    for capture in quoted_captures(&query, r#"__name__\s*=\s*\"([A-Za-z_:][A-Za-z0-9_:]*)\""#) {
        ordered_unique_push(&mut values, &capture);
    }
    let sanitized_query = vector_matching_regex.replace_all(&query, " ").into_owned();
    let sanitized_query = group_modifier_regex
        .replace_all(&sanitized_query, " ")
        .into_owned();
    let sanitized_query = matcher_regex
        .replace_all(&sanitized_query, "{}")
        .into_owned();

    for token in token_regex.find_iter(&sanitized_query) {
        let start = token.start();
        let end = token.end();
        let previous = sanitized_query[..start]
            .chars()
            .next_back()
            .map(|value| value.is_ascii_alphanumeric() || value == '_' || value == ':')
            .unwrap_or(false);
        if previous {
            continue;
        }
        let next = sanitized_query[end..]
            .chars()
            .next()
            .map(|value| value.is_ascii_alphanumeric() || value == '_' || value == ':')
            .unwrap_or(false);
        if next {
            continue;
        }
        let token = token.as_str();
        if reserved_words.contains(&token) {
            continue;
        }
        let trailing = sanitized_query[end..].trim_start();
        if trailing.starts_with('(')
            || ["=", "!=", "=~", "!~"]
                .iter()
                .any(|operator| trailing.starts_with(operator))
        {
            continue;
        }
        ordered_unique_push(&mut values, token);
    }
    values
}

fn extract_prometheus_functions(query_text: &str) -> Vec<String> {
    let function_regex = Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")
        .expect("invalid hard-coded promql function regex");
    let mut values = Vec::new();
    for captures in function_regex.captures_iter(&strip_quoted_text(query_text)) {
        if let Some(value) = captures.get(1) {
            let name = value.as_str();
            if ![
                "by",
                "without",
                "on",
                "ignoring",
                "group_left",
                "group_right",
            ]
            .contains(&name)
            {
                ordered_unique_push(&mut values, name);
            }
        }
    }
    values
}

fn extract_prometheus_range_windows(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for value in extract_quoted_aware_enclosed(query_text, '[', ']') {
        ordered_unique_push(&mut values, value.trim());
    }
    values
}

fn extract_loki_selectors(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for value in extract_quoted_aware_enclosed(query_text, '{', '}') {
        let candidate = value.trim();
        if candidate.is_empty() {
            continue;
        }
        if !candidate.contains('=') && !candidate.contains('!') {
            continue;
        }
        ordered_unique_push(&mut values, candidate);
    }
    values
}

fn extract_loki_label_matchers(selector: &str) -> Vec<String> {
    let mut values = Vec::new();
    for value in quoted_captures(
        selector,
        r#"([A-Za-z_][A-Za-z0-9_]*\s*(?:=|!=|=~|!~)\s*(?:\"(?:\\.|[^\"\\])*\"|'[^']*'|[0-9A-Za-z_.-]+))"#,
    ) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

fn extract_loki_pipeline_functions(query_text: &str) -> Vec<String> {
    let sanitized_query = strip_quoted_text(query_text);
    let function_regex = Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(")
        .expect("invalid hard-coded loki function regex");
    let stage_regex = Regex::new(r"\|\s*([A-Za-z_][A-Za-z0-9_]*)(?:\s|\(|$)")
        .expect("invalid hard-coded loki stage regex");
    let mut functions = Vec::new();
    for captures in function_regex.captures_iter(&sanitized_query) {
        if let Some(value) = captures.get(1) {
            let name = value.as_str();
            if matches!(
                name,
                "by" | "without" | "on" | "ignoring" | "group_left" | "group_right"
            ) {
                continue;
            }
            ordered_unique_push(&mut functions, name);
        }
    }
    for captures in stage_regex.captures_iter(&sanitized_query) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut functions, value.as_str());
        }
    }
    functions
}

fn extract_loki_filter_hints(query_text: &str) -> Vec<String> {
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
                let (token, separator_len) = match operator {
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
                cursor += 1;
                let literal_start = cursor;
                let mut escaped_literal = false;
                while cursor < bytes.len() {
                    if escaped_literal {
                        escaped_literal = false;
                        cursor += 1;
                        continue;
                    }
                    match bytes[cursor] {
                        b'\\' => {
                            escaped_literal = true;
                            cursor += 1;
                        }
                        b'"' => {
                            ordered_unique_push(&mut values, token);
                            let literal = &query_text[literal_start..cursor];
                            ordered_unique_push(&mut values, &format!("{token}:{literal}"));
                            index = cursor + 1;
                            break;
                        }
                        _ => {
                            cursor += 1;
                        }
                    }
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
    extract_quoted_aware_enclosed(query_text, '[', ']')
}

fn extract_flux_pipeline_functions(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let trimmed = query_text.trim_start().to_lowercase();
    if trimmed.starts_with("from(") || query_text.contains("|>") {
        let regex = Regex::new(r#"(?i)\b([A-Za-z_][A-Za-z0-9_]*)\s*\("#)
            .expect("invalid hard-coded flux function regex");
        let first_stage = Regex::new(r#"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*\("#)
            .expect("invalid hard-coded flux first-stage regex");
        if let Some(matches) = first_stage.captures(&trimmed) {
            if let Some(value) = matches.get(1) {
                ordered_unique_push(&mut values, value.as_str());
            }
        }
        for captured in regex.captures_iter(query_text) {
            if let Some(value) = captured.get(1) {
                ordered_unique_push(&mut values, value.as_str());
            }
        }
        let pipeline_regex = Regex::new(r#"\|>\s*([A-Za-z_][A-Za-z0-9_]*)(?:\(|\s|$)"#)
            .expect("invalid hard-coded flux pipeline regex");
        for captured in pipeline_regex.captures_iter(query_text) {
            if let Some(value) = captured.get(1) {
                ordered_unique_push(&mut values, value.as_str());
            }
        }
    }
    values
}

fn extract_flux_buckets(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for value in quoted_captures(
        &Regex::new(r#""(?:\\.|[^"\\])*""#)
            .expect("invalid hard-coded quoted regex")
            .replace_all(query_text, "\"\""),
        r#"from\(\s*bucket\s*:\s*\"([^\"]+)\"\)"#,
    ) {
        ordered_unique_push(&mut values, &value);
    }
    for value in quoted_captures(
        &strip_quoted_text(query_text),
        r"(?i)\bevery\s*:\s*([0-9]+(?:ns|us|µs|ms|s|m|h|d|w|mo|y))",
    ) {
        ordered_unique_push(&mut values, &value);
    }
    for value in quoted_captures(
        &strip_quoted_text(query_text),
        r"(?i)\btime\s*\(\s*([^)]+?)\s*\)",
    ) {
        if !value.trim().is_empty() {
            ordered_unique_push(&mut values, &value);
        }
    }
    values
}

fn extract_flux_source_references(query_text: &str) -> Vec<String> {
    extract_influxql_select_metrics(query_text)
}

fn extract_sql_source_references(query_text: &str) -> Vec<String> {
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
        r#"(?i)\b(?:from|join|update|into|delete\s+from)\s+((?:[A-Za-z_][A-Za-z0-9_$]*|\"[^\"]+\"|`[^`]+`|\[[^\]]+\])(?:\s*\.\s*(?:[A-Za-z_][A-Za-z0-9_$]*|\"[^\"]+\"|`[^`]+`|\[[^\]]+\])){0,2})"#,
    ) {
        let normalized = normalize_sql_identifier(&value);
        if !normalized.is_empty() && !cte_names.contains(&normalized.to_ascii_lowercase()) {
            ordered_unique_push(&mut values, &normalized);
        }
    }
    values
}

fn extract_sql_query_shape_hints(query_text: &str) -> Vec<String> {
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
        let regex = Regex::new(pattern).expect("invalid hard-coded shape regex");
        if regex.is_match(&lowered) {
            ordered_unique_push(&mut values, name);
        }
    }
    values
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

fn strip_sql_comments(query_text: &str) -> String {
    let block_regex = Regex::new(r"(?s)/\*.*?\*/").expect("invalid hard-coded sql comment regex");
    let line_regex = Regex::new(r"--[^\n]*").expect("invalid hard-coded sql line comment regex");
    let without_blocks = block_regex.replace_all(query_text, " ");
    line_regex.replace_all(&without_blocks, " ").into_owned()
}

fn extract_influxql_select_metrics(query_text: &str) -> Vec<String> {
    let query_text = strip_sql_comments(query_text);
    let query_text = Regex::new(r#"(?is)\s*select\s+(.*?)\s+\bfrom\b"#)
        .expect("invalid hard-coded influxql select regex")
        .captures(&query_text)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().trim().to_string())
        .unwrap_or_default();
    if query_text.is_empty() {
        return Vec::new();
    }
    let mut values = Vec::new();
    for value in quoted_captures(&query_text, r#""([^"]+)""#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

fn parse_features_from_object(row: &Value) -> QueryFeatureHints {
    let mut metrics = Vec::new();
    let mut functions = Vec::new();
    let mut measurements = Vec::new();
    let mut buckets = Vec::new();
    let mut labels = Vec::new();

    if let Some(analysis) = row.get("analysis").and_then(Value::as_object) {
        let collect = |key: &str, target: &mut Vec<String>| {
            if let Some(items) = analysis.get(key).and_then(Value::as_array) {
                for item in items {
                    if let Some(text) = item.as_str() {
                        target.push(text.to_string());
                    }
                }
            }
        };
        collect("metrics", &mut metrics);
        collect("functions", &mut functions);
        collect("measurements", &mut measurements);
        collect("buckets", &mut buckets);
        collect("labels", &mut labels);
    }

    if metrics.is_empty() {
        if let Some(items) = row.get("metrics").and_then(Value::as_array) {
            for item in items {
                if let Some(text) = item.as_str() {
                    metrics.push(text.to_string());
                }
            }
        }
    }
    if functions.is_empty() {
        if let Some(items) = row.get("functions").and_then(Value::as_array) {
            for item in items {
                if let Some(text) = item.as_str() {
                    functions.push(text.to_string());
                }
            }
        }
    }
    if measurements.is_empty() {
        if let Some(items) = row.get("measurements").and_then(Value::as_array) {
            for item in items {
                if let Some(text) = item.as_str() {
                    measurements.push(text.to_string());
                }
            }
        }
    }
    if buckets.is_empty() {
        if let Some(items) = row.get("buckets").and_then(Value::as_array) {
            for item in items {
                if let Some(text) = item.as_str() {
                    buckets.push(text.to_string());
                }
            }
        }
    }

    QueryFeatureHints {
        metrics: dedupe_strings(&metrics),
        functions: dedupe_strings(&functions),
        measurements: dedupe_strings(&measurements),
        buckets: dedupe_strings(&buckets),
        labels: dedupe_strings(&labels),
    }
}
