//! Elasticsearch/OpenSearch analyzer for dashboard query inspection.
//! Keeps Lucene-style queries out of the generic metric scraper and extracts stable field hints.
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

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

fn search_exists_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r#"(?i)(?:^|[^A-Za-z0-9_.-])_exists_\s*:\s*([@A-Za-z_][A-Za-z0-9_.-]*)\b"#)
            .expect("invalid hard-coded search exists regex")
    })
}

fn search_field_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        // Keep this narrow: only obvious `field:value` clauses with a simple value form.
        // This avoids trying to parse full Lucene/OpenSearch syntax and keeps URL-like text
        // or other complex expressions from being treated as stable field references.
        Regex::new(
            r#"(?i)(?:^|[^A-Za-z0-9_.-])([@A-Za-z_][A-Za-z0-9_.-]*)\s*:\s*(?:"(?:\\.|[^"\\])*"|\[[^\]]*\]|\{[^}]*\}|\([^\)]*\)|[A-Za-z0-9_*?.-]+)"#,
        )
        .expect("invalid hard-coded search field regex")
    })
}

fn extract_search_field_references(query_text: &str) -> Vec<String> {
    let trimmed = query_text.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Vec::new();
    }
    let mut values = Vec::new();
    for captures in search_exists_regex().captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut values, value.as_str());
        }
    }
    for captures in search_field_regex().captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            let field = value.as_str();
            if field.eq_ignore_ascii_case("_exists_") {
                continue;
            }
            ordered_unique_push(&mut values, field);
        }
    }
    values
}

fn is_tracing_field_name(field: &str) -> bool {
    matches!(
        field.to_ascii_lowercase().as_str(),
        "service.name" | "span.name" | "resource.service.name" | "trace.id" | "traceid"
    )
}

/// Detect conservative search-style query signatures.
///
/// Keep this narrow so explicit Lucene/OpenSearch field clauses can route to the
/// search analyzer without pulling in JSON DSL or tracing field-only queries.
pub(crate) fn query_text_looks_like_search(query_text: &str) -> bool {
    let trimmed = query_text.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return false;
    }
    for captures in search_exists_regex().captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            if !is_tracing_field_name(value.as_str()) {
                return true;
            }
        }
    }
    for captures in search_field_regex().captures_iter(query_text) {
        if let Some(value) = captures.get(1) {
            let field = value.as_str();
            if field.eq_ignore_ascii_case("_exists_") {
                continue;
            }
            if !is_tracing_field_name(field) {
                return true;
            }
        }
    }
    false
}

/// analyze query.
pub(crate) fn analyze_query(
    _panel: &Map<String, Value>,
    _target: &Map<String, Value>,
    _query_field: &str,
    query_text: &str,
) -> QueryAnalysis {
    QueryAnalysis {
        metrics: Vec::new(),
        functions: Vec::new(),
        measurements: extract_search_field_references(query_text),
        buckets: Vec::new(),
    }
}
