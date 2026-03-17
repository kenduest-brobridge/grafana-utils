//! Flux analyzer for dashboard query inspection.
//! Extracts pipeline functions plus measurement/bucket hints for Flux query classification.
use serde_json::{Map, Value};

use super::inspect::{
    extract_flux_pipeline_functions, extract_query_buckets, extract_query_measurements,
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
        metrics: extract_flux_pipeline_functions(query_text),
        measurements: extract_query_measurements(target, query_text),
        buckets: extract_query_buckets(target, query_text),
    }
}
