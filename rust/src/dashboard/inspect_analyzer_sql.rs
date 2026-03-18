//! SQL analyzer for dashboard query inspection.
//! Derives source/shape hints and keeps non-flattened row model output for cross-report use.
use serde_json::{Map, Value};

use super::inspect::{extract_sql_query_shape_hints, extract_sql_source_references, QueryAnalysis};

/// analyze query.
pub(crate) fn analyze_query(
    _panel: &Map<String, Value>,
    _target: &Map<String, Value>,
    _query_field: &str,
    query_text: &str,
) -> QueryAnalysis {
    QueryAnalysis {
        metrics: Vec::new(),
        functions: extract_sql_query_shape_hints(query_text),
        measurements: extract_sql_source_references(query_text),
        buckets: Vec::new(),
    }
}
