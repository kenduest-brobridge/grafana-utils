//! Inspection path for Core resources: analysis, extraction, and report shaping.

use crate::dashboard_reference_models::{
    dedupe_strings, normalize_family_name, DashboardQueryReference,
};
use regex::Regex;

use super::QueryFeatureHints;

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

fn extract_prometheus_metric_names(query_text: &str) -> Vec<String> {
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

fn extract_prometheus_functions(query_text: &str) -> Vec<String> {
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

fn extract_prometheus_range_windows(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    for value in quoted_captures(query_text, r#"\[([^\[\]]+)\]"#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

fn extract_loki_selectors(query_text: &str) -> Vec<String> {
    let regex = Regex::new(r"\{([^{}]*)\}").expect("invalid hard-coded loki selector regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            let selector = value.as_str().trim();
            if !selector.is_empty()
                && (selector.contains("=~")
                    || selector.contains("!~")
                    || selector.contains("!=")
                    || selector.contains('='))
            {
                ordered_unique_push(&mut values, selector);
            }
        }
    }
    values
}

fn extract_loki_label_matchers(selector: &str) -> Vec<String> {
    let regex = Regex::new(r#"([A-Za-z_][A-Za-z0-9_]*)\s*(=|!=|=~|!~)\s*"([^"]*)""#)
        .expect("invalid hard-coded loki matcher regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(selector) {
        if let (Some(label), Some(op), Some(value)) =
            (captures.get(1), captures.get(2), captures.get(3))
        {
            ordered_unique_push(
                &mut values,
                &format!("{}{}\"{}\"", label.as_str(), op.as_str(), value.as_str()),
            );
        }
    }
    values
}

fn extract_loki_pipeline_functions(query_text: &str) -> Vec<String> {
    let regex = Regex::new(r"\|\s*([A-Za-z_][A-Za-z0-9_]*)(?:\([^)]*\))?")
        .expect("invalid hard-coded loki pipeline regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut values, value.as_str());
        }
    }
    values
}

fn extract_loki_filter_hints(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let bytes = query_text.as_bytes();
    let mut index = 0usize;
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
    for value in quoted_captures(query_text, r#"\[([^\[\]]+)\]"#) {
        ordered_unique_push(&mut values, &value);
    }
    values
}

fn extract_flux_pipeline_functions(query_text: &str) -> Vec<String> {
    let regex = Regex::new(r"\|\s*([A-Za-z_][A-Za-z0-9_]*)\s*\(")
        .expect("invalid hard-coded flux pipeline regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut values, value.as_str());
        }
    }
    values
}

fn extract_flux_buckets(query_text: &str) -> Vec<String> {
    let regex = Regex::new(r#"from\(\s*bucket:\s*"([^"]+)""#)
        .expect("invalid hard-coded flux bucket regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut values, value.as_str());
        }
    }
    values
}

fn extract_flux_source_references(query_text: &str) -> Vec<String> {
    let regex = Regex::new(r#"(?i)\bfrom\(\s*bucket:\s*"([^"]+)""#)
        .expect("invalid hard-coded flux source regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut values, value.as_str());
        }
    }
    values
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
        r#"(?i)\b(?:from|join|update|into|delete\s+from)\s+((?:[A-Za-z_][A-Za-z0-9_$]*|"[^"]+"|`[^`]+`|\[[^\]]+\])(?:\s*\.\s*(?:[A-Za-z_][A-Za-z0-9_$]*|"[^"]+"|`[^`]+`|\[[^\]]+\])){0,2})"#,
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
        let regex = Regex::new(pattern).expect("invalid hard-coded sql shape regex");
        if regex.is_match(&lowered) {
            values.push(name.to_string());
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
