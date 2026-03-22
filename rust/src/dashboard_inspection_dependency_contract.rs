//! Offline dependency report models for future richer inspection modes.
//!
//! This module is a standalone contract builder for dependency-oriented
//! inspection artifacts and intentionally keeps runtime behavior local to staged
//! document shapes.

use crate::dashboard::DatasourceInventoryItem;
use crate::dashboard_inspection_query_features::build_query_features;
use crate::dashboard_reference_models::{
    build_query_reference_payload, DashboardQueryReference, QueryFeatureSet,
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
    pub panel_count: usize,
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
            "panelCount": self.panel_count,
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
    pub(crate) dashboard_dependencies: Vec<DashboardDependencySummary>,
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
                        "functions": feature.functions,
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
            "dashboardDependencies": self
                .dashboard_dependencies
                .iter()
                .map(|item| item.as_json())
                .collect::<Vec<_>>(),
            "orphanedDatasources": self
                .orphaned
                .iter()
                .map(|value| Value::String(value.clone()))
                .collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DashboardDependencySummary {
    dashboard_uid: String,
    dashboard_title: String,
    query_count: usize,
    panel_count: usize,
    datasource_count: usize,
    datasource_family_count: usize,
    query_fields: Vec<String>,
    metrics: Vec<String>,
    functions: Vec<String>,
    measurements: Vec<String>,
    buckets: Vec<String>,
}

impl DashboardDependencySummary {
    fn as_json(&self) -> Value {
        json!({
            "dashboardUid": self.dashboard_uid.clone(),
            "dashboardTitle": self.dashboard_title.clone(),
            "queryCount": self.query_count,
            "panelCount": self.panel_count,
            "datasourceCount": self.datasource_count,
            "datasourceFamilyCount": self.datasource_family_count,
            "queryFields": self.query_fields,
            "metrics": self.metrics,
            "functions": self.functions,
            "measurements": self.measurements,
            "buckets": self.buckets,
        })
    }
}

#[derive(Debug, Clone)]
struct DashboardDependencyAccumulator {
    dashboard_title: String,
    query_count: usize,
    panel_keys: BTreeSet<String>,
    datasource_identities: BTreeSet<String>,
    datasource_families: BTreeSet<String>,
    query_fields: BTreeSet<String>,
    metrics: BTreeSet<String>,
    functions: BTreeSet<String>,
    measurements: BTreeSet<String>,
    buckets: BTreeSet<String>,
}

fn query_signature_key(row: &DashboardQueryReference) -> String {
    format!("{}|{}|{}", row.dashboard_uid, row.panel_id, row.ref_id)
}

pub(crate) fn build_offline_dependency_contract(
    query_report_rows: &[Value],
    datasource_inventory: &[DatasourceInventoryItem],
) -> Value {
    let mut queries = Vec::new();
    let mut query_features = BTreeMap::new();
    let mut dashboard_dependencies = BTreeMap::<String, DashboardDependencyAccumulator>::new();
    let mut usage =
        BTreeMap::<String, (DependencyUsageSummary, BTreeSet<String>, BTreeSet<String>)>::new();
    let mut query_fields = BTreeMap::<String, BTreeSet<String>>::new();
    let mut dashboard_uids = BTreeSet::new();
    let mut panel_keys = BTreeSet::new();

    for row in query_report_rows {
        let Some(reference) = build_query_reference_payload(row) else {
            continue;
        };
        let key = query_signature_key(&reference);
        let hint = build_query_features(row, &reference);
        let QueryFeatureSet {
            metrics,
            functions,
            measurements,
            buckets,
            labels: _labels,
        } = hint.clone();
        dashboard_uids.insert(reference.dashboard_uid.clone());
        panel_keys.insert(format!(
            "{}:{}",
            reference.dashboard_uid, reference.panel_id
        ));
        query_features.insert(key, hint);
        let dashboard_entry = dashboard_dependencies
            .entry(reference.dashboard_uid.clone())
            .or_insert(DashboardDependencyAccumulator {
                dashboard_title: reference.dashboard_title.clone(),
                query_count: 0,
                panel_keys: BTreeSet::new(),
                datasource_identities: BTreeSet::new(),
                datasource_families: BTreeSet::new(),
                query_fields: BTreeSet::new(),
                metrics: BTreeSet::new(),
                functions: BTreeSet::new(),
                measurements: BTreeSet::new(),
                buckets: BTreeSet::new(),
            });
        dashboard_entry.query_count += 1;
        dashboard_entry.panel_keys.insert(format!(
            "{}:{}",
            reference.dashboard_uid, reference.panel_id
        ));
        dashboard_entry
            .datasource_identities
            .insert(reference.datasource_name.clone());
        dashboard_entry
            .datasource_families
            .insert(reference.datasource_family.clone());
        dashboard_entry
            .query_fields
            .insert(reference.query_field.clone());
        dashboard_entry.metrics.extend(metrics);
        dashboard_entry.functions.extend(functions);
        dashboard_entry.measurements.extend(measurements);
        dashboard_entry.buckets.extend(buckets);
        let fields = query_fields
            .entry(reference.datasource_name.clone())
            .or_default();
        fields.insert(reference.query_field.clone());

        let summary_entry = usage.entry(reference.datasource_name.clone()).or_insert((
            DependencyUsageSummary {
                datasource_identity: reference.datasource_name.clone(),
                family: reference.datasource_family.clone(),
                query_count: 0,
                dashboard_count: 0,
                panel_count: 0,
                reference_count: 0,
                query_fields: Vec::new(),
            },
            BTreeSet::new(),
            BTreeSet::new(),
        ));
        summary_entry.0.query_count += 1;
        summary_entry.0.reference_count += 1;
        summary_entry.0.query_fields = fields.iter().cloned().collect();
        summary_entry.1.insert(reference.dashboard_uid.clone());
        summary_entry.2.insert(format!(
            "{}:{}",
            reference.dashboard_uid, reference.panel_id
        ));
        summary_entry.0.dashboard_count = summary_entry.1.len();
        summary_entry.0.panel_count = summary_entry.2.len();
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

    let dashboard_dependencies = dashboard_dependencies
        .into_iter()
        .map(|(dashboard_uid, summary)| DashboardDependencySummary {
            dashboard_uid,
            dashboard_title: summary.dashboard_title,
            query_count: summary.query_count,
            panel_count: summary.panel_keys.len(),
            datasource_count: summary.datasource_identities.len(),
            datasource_family_count: summary.datasource_families.len(),
            query_fields: summary.query_fields.into_iter().collect(),
            metrics: summary.metrics.into_iter().collect(),
            functions: summary.functions.into_iter().collect(),
            measurements: summary.measurements.into_iter().collect(),
            buckets: summary.buckets.into_iter().collect(),
        })
        .collect::<Vec<_>>();

    let mut usage_rows = usage
        .into_values()
        .map(|(summary, _, _)| summary)
        .collect::<Vec<_>>();
    usage_rows.sort_by(|left, right| left.datasource_identity.cmp(&right.datasource_identity));

    let mut summary = BTreeMap::new();
    summary.insert("queryCount".to_string(), Value::from(queries.len()));
    summary.insert(
        "dashboardCount".to_string(),
        Value::from(dashboard_uids.len() as u64),
    );
    summary.insert(
        "panelCount".to_string(),
        Value::from(panel_keys.len() as u64),
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
        dashboard_dependencies,
        usage: usage_rows,
        orphaned,
    }
    .as_json()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::DatasourceInventoryItem;
    use serde_json::json;

    #[test]
    fn build_offline_dependency_contract_reports_unique_dashboard_and_panel_counts() {
        let document = build_offline_dependency_contract(
            &[
                json!({
                    "dashboardUid": "dash-a",
                    "dashboardTitle": "Dash A",
                    "panelId": "1",
                    "panelTitle": "Panel One",
                    "panelType": "timeseries",
                    "refId": "A",
                    "datasource": "Prometheus Main",
                    "datasourceUid": "prom-main",
                    "datasourceType": "prometheus",
                    "file": "dash-a.json",
                    "queryField": "expr",
                    "query": "up"
                }),
                json!({
                    "dashboardUid": "dash-a",
                    "dashboardTitle": "Dash A",
                    "panelId": "1",
                    "panelTitle": "Panel One",
                    "panelType": "timeseries",
                    "refId": "B",
                    "datasource": "Prometheus Main",
                    "datasourceUid": "prom-main",
                    "datasourceType": "prometheus",
                    "file": "dash-a.json",
                    "queryField": "expr",
                    "query": "sum(rate(up[5m]))"
                }),
                json!({
                    "dashboardUid": "dash-b",
                    "dashboardTitle": "Dash B",
                    "panelId": "2",
                    "panelTitle": "Panel Two",
                    "panelType": "timeseries",
                    "refId": "A",
                    "datasource": "Prometheus Main",
                    "datasourceUid": "prom-main",
                    "datasourceType": "prometheus",
                    "file": "dash-b.json",
                    "queryField": "expr",
                    "query": "rate(http_requests_total[5m])"
                }),
            ],
            &[DatasourceInventoryItem {
                uid: "prom-main".to_string(),
                name: "Prometheus Main".to_string(),
                datasource_type: "prometheus".to_string(),
                access: String::new(),
                url: String::new(),
                database: String::new(),
                default_bucket: String::new(),
                organization: String::new(),
                index_pattern: String::new(),
                is_default: String::new(),
                org: String::new(),
                org_id: String::new(),
            }],
        );

        assert_eq!(document["summary"]["queryCount"], json!(3));
        assert_eq!(document["summary"]["dashboardCount"], json!(2));
        assert_eq!(document["summary"]["panelCount"], json!(2));
        assert_eq!(document["summary"]["datasourceCount"], json!(1));
        assert_eq!(document["datasourceUsage"][0]["queryCount"], json!(3));
        assert_eq!(document["datasourceUsage"][0]["dashboardCount"], json!(2));
        assert_eq!(document["datasourceUsage"][0]["panelCount"], json!(2));
        assert_eq!(
            document["datasourceUsage"][0]["queryFields"],
            json!(["expr"])
        );
    }

    #[test]
    fn build_offline_dependency_contract_parses_loki_family_features() {
        let document = build_offline_dependency_contract(
            &[json!({
                "dashboardUid": "dash-loki",
                "dashboardTitle": "Loki Dash",
                "panelId": "7",
                "panelTitle": "Logs",
                "panelType": "logs",
                "refId": "A",
                "datasource": "Loki Main",
                "datasourceUid": "loki-main",
                "datasourceType": "loki",
                "file": "dash-loki.json",
                "queryField": "expr",
                "query": "{job=\"api\",namespace=~\"prod-.*\"} |= \"a|~b\" | line_format \"{{.message}}\" |~ \"timeout\" | json"
            })],
            &[DatasourceInventoryItem {
                uid: "loki-main".to_string(),
                name: "Loki Main".to_string(),
                datasource_type: "loki".to_string(),
                access: String::new(),
                url: String::new(),
                database: String::new(),
                default_bucket: String::new(),
                organization: String::new(),
                index_pattern: String::new(),
                is_default: String::new(),
                org: String::new(),
                org_id: String::new(),
            }],
        );

        let analysis = &document["queries"][0]["analysis"];
        let mut measurements = analysis["measurements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        measurements.sort();
        let mut functions = analysis["functions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        functions.sort();
        let mut labels = analysis["labels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        labels.sort();

        assert_eq!(
            measurements,
            vec![
                "job=\"api\"",
                "namespace=~\"prod-.*\"",
                "{job=\"api\",namespace=~\"prod-.*\"}"
            ]
        );
        assert_eq!(
            functions,
            vec![
                "json",
                "line_filter_contains",
                "line_filter_contains:a|~b",
                "line_filter_regex",
                "line_filter_regex:timeout",
                "line_format"
            ]
        );
        assert_eq!(labels, vec!["job=\"api\"", "namespace=~\"prod-.*\""]);
    }

    #[test]
    fn parse_sql_features_extracts_shape_and_source() {
        let document = build_offline_dependency_contract(
            &[json!({
                "dashboardUid": "dash-sql",
                "dashboardTitle": "SQL Dash",
                "panelId": "3",
                "panelTitle": "SQL",
                "panelType": "table",
                "refId": "A",
                "datasource": "PG Main",
                "datasourceUid": "pg-main",
                "datasourceType": "postgres",
                "file": "dash-sql.json",
                "queryField": "rawQuery",
                "query": "WITH recent AS (SELECT id FROM users WHERE active = true) SELECT p.id, u.name FROM posts p JOIN users u ON p.user_id = u.id WHERE p.created_at > now() - INTERVAL '1 day' LIMIT 10"
            })],
            &[DatasourceInventoryItem {
                uid: "pg-main".to_string(),
                name: "PG Main".to_string(),
                datasource_type: "postgres".to_string(),
                access: String::new(),
                url: String::new(),
                database: String::new(),
                default_bucket: String::new(),
                organization: String::new(),
                index_pattern: String::new(),
                is_default: String::new(),
                org: String::new(),
                org_id: String::new(),
            }],
        );

        let analysis = &document["queries"][0]["analysis"];
        let mut functions = analysis["functions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        functions.sort();
        let mut measurements = analysis["measurements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        measurements.sort();

        assert!(functions.binary_search(&"with".to_string()).is_ok());
        assert!(functions.binary_search(&"select".to_string()).is_ok());
        assert!(functions.binary_search(&"join".to_string()).is_ok());
        assert!(functions.binary_search(&"limit".to_string()).is_ok());
        assert!(measurements.contains(&"posts".to_string()));
        assert!(measurements.contains(&"users".to_string()));
        assert!(!measurements.contains(&"recent".to_string()));
    }
}
