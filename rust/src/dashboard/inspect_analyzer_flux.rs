//! Flux analyzer for dashboard query inspection.
//! Extracts pipeline functions plus measurement/bucket hints for Flux query classification.
use serde_json::{Map, Value};

use super::inspect::{
    extract_flux_pipeline_functions, extract_influxql_select_functions, extract_influxql_select_metrics,
    extract_influxql_time_windows, extract_query_buckets, extract_query_measurements,
    ordered_unique_push, QueryAnalysis,
};

/// analyze query.
pub(crate) fn analyze_query(
    _panel: &Map<String, Value>,
    target: &Map<String, Value>,
    _query_field: &str,
    query_text: &str,
) -> QueryAnalysis {
    let trimmed = query_text.trim_start();
    let mut metrics = if trimmed.starts_with("from(")
        || trimmed.starts_with("from (")
        || query_text.contains("|>")
    {
        Vec::new()
    } else {
        Vec::new()
    };
    let mut functions = if trimmed.starts_with("from(")
        || trimmed.starts_with("from (")
        || query_text.contains("|>")
    {
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
