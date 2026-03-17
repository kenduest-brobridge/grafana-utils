//! Offline dependency report models for future richer inspection modes.
//!
//! This module is a standalone contract builder for dependency-oriented
//! inspection artifacts and intentionally keeps runtime behavior local to staged
//! document shapes.

use crate::dashboard::DatasourceInventoryItem;
use crate::dashboard_reference_models::{
    build_query_reference_payload, dedupe_strings, normalize_family_name, DashboardQueryReference,
    QueryFeatureSet,
};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Struct definition for DependencyUsageSummary.
#[derive(Debug, Clone)]
pub struct DependencyUsageSummary {
    pub datasource_identity: String,
    pub family: String,
    pub query_count: usize,
    pub dashboard_count: usize,
    pub reference_count: usize,
    pub query_fields: Vec<String>,
}

impl DependencyUsageSummary {
    /// as json.
    pub fn as_json(&self) -> Value {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: 無

        json!({
            "datasource": self.datasource_identity,
            "family": self.family,
            "queryCount": self.query_count,
            "dashboardCount": self.dashboard_count,
            "referenceCount": self.reference_count,
            "queryFields": self.query_fields,
        })
    }
}

/// Struct definition for OfflineDependencyReportDocument.
#[derive(Debug, Clone)]
pub struct OfflineDependencyReportDocument {
    pub summary: BTreeMap<String, Value>,
    pub queries: Vec<DashboardQueryReference>,
    pub query_features: BTreeMap<String, QueryFeatureSet>,
    pub usage: Vec<DependencyUsageSummary>,
    pub orphaned: Vec<String>,
}

impl OfflineDependencyReportDocument {
    /// as json.
    pub fn as_json(&self) -> Value {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: dashboard_inspection_dependency_contract.rs:query_signature_key

        let queries: Vec<Value> = self
            .queries
            .iter()
            .map(|query| {
                let feature = self
                    .query_features
                    .get(&query_signature_key(query))
                    .cloned()
                    .unwrap_or_else(QueryFeatureSet::blank);
                let mut record = Map::new();
                record.insert(
                    "dashboardUid".to_string(),
                    Value::String(query.dashboard_uid.clone()),
                );
                record.insert(
                    "dashboardTitle".to_string(),
                    Value::String(query.dashboard_title.clone()),
                );
                record.insert("panelId".to_string(), Value::String(query.panel_id.clone()));
                record.insert(
                    "panelTitle".to_string(),
                    Value::String(query.panel_title.clone()),
                );
                record.insert(
                    "panelType".to_string(),
                    Value::String(query.panel_type.clone()),
                );
                record.insert("refId".to_string(), Value::String(query.ref_id.clone()));
                record.insert(
                    "datasource".to_string(),
                    Value::String(query.datasource_name.clone()),
                );
                record.insert(
                    "datasourceUid".to_string(),
                    Value::String(query.datasource_uid.clone()),
                );
                record.insert(
                    "datasourceType".to_string(),
                    Value::String(query.datasource_type.clone()),
                );
                record.insert(
                    "datasourceFamily".to_string(),
                    Value::String(query.datasource_family.clone()),
                );
                record.insert("file".to_string(), Value::String(query.file.clone()));
                record.insert(
                    "queryField".to_string(),
                    Value::String(query.query_field.clone()),
                );
                record.insert("query".to_string(), Value::String(query.query.clone()));
                record.insert(
                    "analysis".to_string(),
                    json!({
                        "metrics": feature.metrics,
                        "measurements": feature.measurements,
                        "buckets": feature.buckets,
                        "labels": feature.labels,
                    }),
                );
                Value::Object(record)
            })
            .collect();

        json!({
            "kind": "grafana-utils-dashboard-dependency-contract",
            "summary": serde_json::to_value(&self.summary).unwrap_or_else(|_| json!({})),
            "queryCount": self.queries.len(),
            "datasourceCount": self.usage.len(),
            "orphanedDatasourceCount": self.orphaned.len(),
            "queries": queries,
            "datasourceUsage": self.usage.iter().map(|item| item.as_json()).collect::<Vec<_>>(),
            "orphanedDatasources": self
                .orphaned
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone)]
struct QueryFeatureHints {
    metrics: Vec<String>,
    measurements: Vec<String>,
    buckets: Vec<String>,
    labels: Vec<String>,
}

fn query_signature_key(row: &DashboardQueryReference) -> String {
    format!("{}|{}|{}", row.dashboard_uid, row.panel_id, row.ref_id)
}

fn parse_query_text_families(row: &DashboardQueryReference) -> QueryFeatureHints {
// Call graph (hierarchy): this function is used in related modules.
// Upstream callers: 無
// Downstream callees: dashboard_reference_models.rs:dedupe_strings, dashboard_reference_models.rs:normalize_family_name

    let family = normalize_family_name(&row.datasource_type);
    let query = row.query.to_lowercase();
    let mut metrics = Vec::new();
    let mut measurements = Vec::new();
    let mut buckets = Vec::new();
    let mut labels = Vec::new();

    if ["prometheus", "graphite", "victoriametrics"].contains(&family.as_str()) {
        for capture in row.query.split(&[' ', '\n', '\t', ';'][..]) {
            let value = capture.trim();
            if value.is_empty() {
                continue;
            }
            if value.ends_with(")") && value.chars().all(|c| !c.is_ascii_digit()) {
                continue;
            }
            if value
                .chars()
                .next()
                .map(|value| value.is_ascii_alphabetic())
                .unwrap_or(false)
            {
                metrics.push(value.to_string());
            }
        }
        if query.contains("rate(") {
            metrics.push("rate".to_string());
        }
        if query.contains("sum(") {
            metrics.push("sum".to_string());
        }
    }

    if family == "loki" {
        let mut selectors = BTreeSet::new();
        for segment in query.split('|') {
            if let Some(begin) = segment.find('{') {
                if let Some(end) = segment.find('}') {
                    selectors.insert(segment[begin + 1..end].to_string());
                }
            }
        }
        labels = selectors.into_iter().collect();
        if query.contains("|") {
            buckets.push("pipeline".to_string());
        }
    }

    if family == "flux" || family == "influxdb" {
        if query.contains("from(") {
            measurements.push("from".to_string());
        }
        if query.contains("window(") || query.contains("range(") {
            buckets.push("window".to_string());
        }
    }

    if ["mysql", "postgresql", "postgres", "sql"].contains(&family.as_str()) {
        for keyword in ["from", "join", "where", "group by"] {
            if query.contains(keyword) {
                measurements.push(keyword.to_string());
            }
        }
    }

    QueryFeatureHints {
        metrics: dedupe_strings(&metrics),
        measurements: dedupe_strings(&measurements),
        buckets: dedupe_strings(&buckets),
        labels: dedupe_strings(&labels),
    }
}

/// Purpose: implementation note.
pub(crate) fn build_offline_dependency_contract(
    query_report_rows: &[Value],
    datasource_inventory: &[DatasourceInventoryItem],
) -> Value {
    let mut queries = Vec::new();
    let mut query_features = BTreeMap::new();
    let mut usage = BTreeMap::<String, DependencyUsageSummary>::new();
    let mut query_fields = BTreeMap::<String, BTreeSet<String>>::new();

    for row in query_report_rows {
        let Some(reference) = build_query_reference_payload(row) else {
            continue;
        };
        let key = query_signature_key(&reference);
        let hint = parse_query_text_families(&reference);
        query_features.insert(
            key,
            QueryFeatureSet {
                metrics: hint.metrics,
                measurements: hint.measurements,
                buckets: hint.buckets,
                labels: hint.labels,
            },
        );
        let fields = query_fields
            .entry(reference.datasource_name.clone())
            .or_default();
        fields.insert(reference.query_field.clone());

        let summary_entry =
            usage
                .entry(reference.datasource_name.clone())
                .or_insert(DependencyUsageSummary {
                    datasource_identity: reference.datasource_name.clone(),
                    family: reference.datasource_family.clone(),
                    query_count: 0,
                    dashboard_count: 0,
                    reference_count: 0,
                    query_fields: Vec::new(),
                });
        summary_entry.query_count += 1;
        summary_entry.reference_count += 1;
        summary_entry.query_fields = fields.iter().cloned().collect();
        summary_entry.dashboard_count += 1;
        queries.push(reference);
    }

    let mut used = BTreeSet::new();
    for key in usage.keys() {
        used.insert(key.clone());
    }

    let mut orphaned = Vec::new();
    for item in datasource_inventory {
        let uid = item.uid.trim().to_string();
        let name = item.name.trim().to_string();
        if !uid.is_empty() && used.contains(&uid) {
            continue;
        }
        if !name.is_empty() && used.contains(&name) {
            continue;
        }
        if !uid.is_empty() {
            orphaned.push(uid);
            continue;
        }
        if !name.is_empty() {
            orphaned.push(name);
        }
    }

    let mut usage_rows = usage.into_values().collect::<Vec<_>>();
    usage_rows.sort_by(|left, right| left.datasource_identity.cmp(&right.datasource_identity));

    let mut summary = BTreeMap::new();
    summary.insert("queryCount".to_string(), Value::from(queries.len()));
    summary.insert(
        "dashboardCount".to_string(),
        Value::from(
            usage_rows
                .iter()
                .map(|item| item.dashboard_count)
                .sum::<usize>() as u64,
        ),
    );
    summary.insert(
        "datasourceCount".to_string(),
        Value::from(usage_rows.len() as u64),
    );
    summary.insert(
        "orphanedDatasourceCount".to_string(),
        Value::from(orphaned.len() as u64),
    );

    OfflineDependencyReportDocument {
        summary,
        queries,
        query_features,
        usage: usage_rows,
        orphaned,
    }
    .as_json()
}
