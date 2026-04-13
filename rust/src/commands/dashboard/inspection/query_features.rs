//! Shared family-aware query feature extraction for dashboard inspection contracts.

#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
use crate::dashboard_reference_models::{dedupe_strings, DashboardQueryReference};

#[path = "query_signatures.rs"]
mod dashboard_inspection_query_signatures;

pub(crate) use dashboard_inspection_query_signatures::parse_query_text_families;

#[derive(Debug, Clone)]
pub(crate) struct QueryFeatureHints {
    pub(crate) metrics: Vec<String>,
    pub(crate) functions: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
    pub(crate) labels: Vec<String>,
}

#[cfg(test)]
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

#[cfg(test)]
fn merge_unique(target: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        if !target.iter().any(|item| item == &value) {
            target.push(value);
        }
    }
}

#[cfg(test)]
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
