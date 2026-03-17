//! Prometheus analyzer for dashboard query inspection.
//! Extracts metrics, measurement names, and bucket-like hints from panel query text.
use serde_json::{Map, Value};

use super::inspect::{
    extract_prometheus_metric_names, extract_query_buckets, extract_query_measurements,
    QueryAnalysis,
};

/// analyze query.
pub(crate) fn analyze_query(
    _panel: &Map<String, Value>,
    target: &Map<String, Value>,
    _query_field: &str,
    query_text: &str,
) -> QueryAnalysis {
    QueryAnalysis {
        metrics: extract_prometheus_metric_names(query_text),
        measurements: extract_query_measurements(target, query_text),
        buckets: extract_query_buckets(target, query_text),
    }
}
