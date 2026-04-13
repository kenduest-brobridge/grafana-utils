//! Prometheus analyzer for dashboard query inspection.
//! Extracts metrics, measurement names, and bucket-like hints from panel query text.
use serde_json::{Map, Value};

use super::inspect::{
    extract_prometheus_functions, extract_prometheus_metric_names,
    extract_prometheus_range_windows, extract_query_buckets, extract_query_measurements,
};
use super::inspect_query::{ordered_unique_push, QueryAnalysis};

/// analyze query.
pub(crate) fn analyze_query(
    _panel: &Map<String, Value>,
    target: &Map<String, Value>,
    _query_field: &str,
    query_text: &str,
) -> QueryAnalysis {
    let mut buckets = extract_query_buckets(target, query_text);
    for value in extract_prometheus_range_windows(query_text) {
        ordered_unique_push(&mut buckets, &value);
    }
    QueryAnalysis {
        metrics: extract_prometheus_metric_names(query_text),
        functions: extract_prometheus_functions(query_text),
        measurements: extract_query_measurements(target, query_text),
        buckets,
    }
}
