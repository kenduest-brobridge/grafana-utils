//! Flux analyzer for dashboard query inspection.
//! Extracts pipeline functions plus measurement/bucket hints for Flux query classification.
use regex::Regex;
use serde_json::{Map, Value};

use super::inspect::{
    extract_flux_pipeline_functions, extract_influxql_select_functions,
    extract_influxql_select_metrics, extract_influxql_time_windows, extract_query_buckets,
    extract_query_measurements,
};
use super::inspect_query::{ordered_unique_push, QueryAnalysis};

fn extract_flux_every_windows(query_text: &str) -> Vec<String> {
    let sanitized_query = Regex::new(r#""(?:\\.|[^"\\])*""#)
        .expect("invalid hard-coded flux quoted regex")
        .replace_all(query_text, "\"\"");
    let regex = Regex::new(
        r#"(?i)\b(?:aggregateWindow|window)\s*\([^)]*?\bevery\s*:\s*([0-9]+(?:ns|us|µs|ms|s|m|h|d|w|mo|y))\b"#,
    )
    .expect("invalid hard-coded flux window regex");
    let mut values = Vec::new();
    for captures in regex.captures_iter(&sanitized_query) {
        if let Some(value) = captures.get(1) {
            ordered_unique_push(&mut values, value.as_str());
        }
    }
    values
}

/// analyze query.
pub(crate) fn analyze_query(
    _panel: &Map<String, Value>,
    target: &Map<String, Value>,
    _query_field: &str,
    query_text: &str,
) -> QueryAnalysis {
    let trimmed = query_text.trim_start();
    let has_flux_pipeline =
        trimmed.starts_with("from(") || trimmed.starts_with("from (") || query_text.contains("|>");
    let mut metrics = Vec::new();
    let mut functions = if has_flux_pipeline {
        extract_flux_pipeline_functions(query_text)
    } else {
        Vec::new()
    };
    for value in extract_influxql_select_metrics(query_text) {
        ordered_unique_push(&mut metrics, &value);
    }
    for value in extract_influxql_select_functions(query_text) {
        ordered_unique_push(&mut functions, &value);
    }
    let mut buckets = extract_query_buckets(target, query_text);
    for value in extract_flux_every_windows(query_text) {
        ordered_unique_push(&mut buckets, &value);
    }
    for value in extract_influxql_time_windows(query_text) {
        ordered_unique_push(&mut buckets, &value);
    }
    QueryAnalysis {
        metrics,
        functions,
        measurements: extract_query_measurements(target, query_text),
        buckets,
    }
}
