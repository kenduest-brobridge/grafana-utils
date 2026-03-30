//! Offline dependency report models for future richer inspection modes.
//!
//! This module is a standalone contract builder for dependency-oriented
//! inspection artifacts and intentionally keeps runtime behavior local to staged
//! document shapes.

use crate::dashboard::DatasourceInventoryItem;
use crate::dashboard::ExportInspectionQueryRow;
#[cfg(test)]
use crate::dashboard_inspection_query_features::build_query_features;
use crate::dashboard_inspection_query_features::parse_query_text_families;
#[cfg(test)]
use crate::dashboard_reference_models::build_query_reference_payload;
use crate::dashboard_reference_models::{
    dedupe_strings, normalize_family_name, DashboardQueryReference, QueryFeatureSet,
};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const HIGH_BLAST_RADIUS_DASHBOARD_THRESHOLD: usize = 3;
const HIGH_BLAST_RADIUS_FOLDER_THRESHOLD: usize = 2;

fn has_high_blast_radius(dashboard_count: usize, folder_count: usize) -> bool {
    dashboard_count >= HIGH_BLAST_RADIUS_DASHBOARD_THRESHOLD
        || (dashboard_count >= 2 && folder_count >= HIGH_BLAST_RADIUS_FOLDER_THRESHOLD)
}

/// Struct definition for DependencyUsageSummary.
#[derive(Debug, Clone)]
pub struct DependencyUsageSummary {
    pub datasource_identity: String,
    pub datasource_uid: String,
    pub datasource_type: String,
    pub family: String,
    pub query_count: usize,
    pub dashboard_count: usize,
    pub panel_count: usize,
    pub reference_count: usize,
    pub query_fields: Vec<String>,
    pub folder_count: usize,
    pub high_blast_radius: bool,
    pub cross_folder: bool,
    pub folder_paths: Vec<String>,
    pub dashboard_titles: Vec<String>,
}

impl DependencyUsageSummary {
    /// as json.
    pub fn as_json(&self) -> Value {
        // Call graph (hierarchy): this function is used in related modules.
        // Upstream callers: 無
        // Downstream callees: 無

        json!({
            "datasource": self.datasource_identity,
            "datasourceUid": self.datasource_uid,
            "datasourceType": self.datasource_type,
            "family": self.family,
            "queryCount": self.query_count,
            "dashboardCount": self.dashboard_count,
            "panelCount": self.panel_count,
            "referenceCount": self.reference_count,
            "queryFields": self.query_fields,
            "folderCount": self.folder_count,
            "highBlastRadius": self.high_blast_radius,
            "crossFolder": self.cross_folder,
            "folderPaths": self.folder_paths,
            "dashboardTitles": self.dashboard_titles,
        })
    }
}

/// Struct definition for DependencyOrphanSummary.
#[derive(Debug, Clone)]
pub struct DependencyOrphanSummary {
    pub uid: String,
    pub name: String,
    pub datasource_type: String,
    pub family: String,
    pub access: String,
    pub url: String,
    pub database: String,
    pub default_bucket: String,
    pub organization: String,
    pub index_pattern: String,
    pub is_default: String,
    pub org: String,
    pub org_id: String,
}

impl DependencyOrphanSummary {
    /// as json.
    pub fn as_json(&self) -> Value {
        json!({
            "uid": self.uid,
            "name": self.name,
            "type": self.datasource_type,
            "family": self.family,
            "access": self.access,
            "url": self.url,
            "database": self.database,
            "defaultBucket": self.default_bucket,
            "organization": self.organization,
            "indexPattern": self.index_pattern,
            "isDefault": self.is_default,
            "org": self.org,
            "orgId": self.org_id,
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
    pub orphaned: Vec<DependencyOrphanSummary>,
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
                .map(|item| item.as_json())
                .collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DashboardDependencySummary {
    pub(crate) dashboard_uid: String,
    pub(crate) dashboard_title: String,
    pub(crate) query_count: usize,
    pub(crate) panel_count: usize,
    pub(crate) datasource_count: usize,
    pub(crate) datasource_family_count: usize,
    pub(crate) query_fields: Vec<String>,
    pub(crate) metrics: Vec<String>,
    pub(crate) functions: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
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

#[derive(Debug, Clone)]
struct DependencyQueryInput {
    reference: DashboardQueryReference,
    feature: QueryFeatureSet,
}

fn query_signature_key(row: &DashboardQueryReference) -> String {
    format!("{}|{}|{}", row.dashboard_uid, row.panel_id, row.ref_id)
}

#[cfg(test)]
fn build_dependency_query_input_from_value(row: &Value) -> Option<DependencyQueryInput> {
    let reference = build_query_reference_payload(row)?;
    let feature = build_query_features(row, &reference);
    Some(DependencyQueryInput { reference, feature })
}

fn build_dependency_query_input_from_report_row(
    row: &ExportInspectionQueryRow,
) -> DependencyQueryInput {
    let reference = DashboardQueryReference {
        dashboard_uid: row.dashboard_uid.clone(),
        dashboard_title: row.dashboard_title.clone(),
        folder_path: row.folder_path.clone(),
        panel_id: row.panel_id.clone(),
        panel_title: row.panel_title.clone(),
        panel_type: row.panel_type.clone(),
        ref_id: row.ref_id.clone(),
        datasource_uid: row.datasource_uid.clone(),
        datasource_name: row.datasource_name.clone(),
        datasource_type: row.datasource_type.clone(),
        datasource_family: row.datasource_family.clone(),
        file: row.file_path.clone(),
        query_field: row.query_field.clone(),
        query: row.query_text.clone(),
    };
    let mut hints = parse_query_text_families(&reference);
    hints.metrics.extend(row.metrics.clone());
    hints.functions.extend(row.functions.clone());
    hints.measurements.extend(row.measurements.clone());
    hints.buckets.extend(row.buckets.clone());
    DependencyQueryInput {
        reference,
        feature: QueryFeatureSet {
            metrics: dedupe_strings(&hints.metrics),
            functions: dedupe_strings(&hints.functions),
            measurements: dedupe_strings(&hints.measurements),
            buckets: dedupe_strings(&hints.buckets),
            labels: dedupe_strings(&hints.labels),
        },
    }
}

fn build_offline_dependency_contract_document_from_query_inputs(
    query_inputs: Vec<DependencyQueryInput>,
    datasource_inventory: &[DatasourceInventoryItem],
) -> OfflineDependencyReportDocument {
    let mut queries = Vec::new();
    let mut query_features = BTreeMap::new();
    let mut dashboard_dependencies = BTreeMap::<String, DashboardDependencyAccumulator>::new();
    let mut usage = BTreeMap::<
        String,
        (
            DependencyUsageSummary,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
        ),
    >::new();
    let mut query_fields = BTreeMap::<String, BTreeSet<String>>::new();
    let mut dashboard_uids = BTreeSet::new();
    let mut panel_keys = BTreeSet::new();

    for input in query_inputs {
        let DependencyQueryInput { reference, feature } = input;
        let key = query_signature_key(&reference);
        let QueryFeatureSet {
            metrics,
            functions,
            measurements,
            buckets,
            labels: _labels,
        } = feature.clone();
        dashboard_uids.insert(reference.dashboard_uid.clone());
        panel_keys.insert(format!(
            "{}:{}",
            reference.dashboard_uid, reference.panel_id
        ));
        query_features.insert(key, feature);
        let datasource_identity = if reference.datasource_name.is_empty() {
            reference.datasource_uid.clone()
        } else {
            reference.datasource_name.clone()
        };
        let datasource_key = if reference.datasource_uid.is_empty() {
            datasource_identity.clone()
        } else {
            reference.datasource_uid.clone()
        };
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
        let fields = query_fields.entry(datasource_key.clone()).or_default();
        fields.insert(reference.query_field.clone());

        let summary_entry = usage.entry(datasource_key.clone()).or_insert((
            DependencyUsageSummary {
                datasource_identity: datasource_identity.clone(),
                datasource_uid: reference.datasource_uid.clone(),
                datasource_type: reference.datasource_type.clone(),
                family: reference.datasource_family.clone(),
                query_count: 0,
                dashboard_count: 0,
                panel_count: 0,
                reference_count: 0,
                query_fields: Vec::new(),
                folder_count: 0,
                high_blast_radius: false,
                cross_folder: false,
                folder_paths: Vec::new(),
                dashboard_titles: Vec::new(),
            },
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
            BTreeSet::new(),
        ));
        summary_entry.0.datasource_identity = datasource_identity.clone();
        if summary_entry.0.datasource_uid.is_empty() {
            summary_entry.0.datasource_uid = reference.datasource_uid.clone();
        }
        if summary_entry.0.datasource_type.is_empty() {
            summary_entry.0.datasource_type = reference.datasource_type.clone();
        }
        if summary_entry.0.family.is_empty() {
            summary_entry.0.family = reference.datasource_family.clone();
        }
        summary_entry.0.query_count += 1;
        summary_entry.0.reference_count += 1;
        summary_entry.0.query_fields = fields.iter().cloned().collect();
        summary_entry.1.insert(reference.dashboard_uid.clone());
        summary_entry.2.insert(format!(
            "{}:{}",
            reference.dashboard_uid, reference.panel_id
        ));
        if !reference.folder_path.trim().is_empty() {
            summary_entry.3.insert(reference.folder_path.clone());
        }
        if !reference.dashboard_title.trim().is_empty() {
            summary_entry.4.insert(reference.dashboard_title.clone());
        }
        summary_entry.0.dashboard_count = summary_entry.1.len();
        summary_entry.0.panel_count = summary_entry.2.len();
        summary_entry.0.folder_count = summary_entry.3.len();
        summary_entry.0.high_blast_radius = has_high_blast_radius(
            summary_entry.0.dashboard_count,
            summary_entry.0.folder_count,
        );
        summary_entry.0.cross_folder = summary_entry.3.len() > 1;
        summary_entry.0.folder_paths = summary_entry.3.iter().cloned().collect();
        summary_entry.0.dashboard_titles = summary_entry.4.iter().cloned().collect();
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
        orphaned.push(DependencyOrphanSummary {
            uid,
            name,
            datasource_type: item.datasource_type.clone(),
            family: normalize_family_name(&item.datasource_type),
            access: item.access.clone(),
            url: item.url.clone(),
            database: item.database.clone(),
            default_bucket: item.default_bucket.clone(),
            organization: item.organization.clone(),
            index_pattern: item.index_pattern.clone(),
            is_default: item.is_default.clone(),
            org: item.org.clone(),
            org_id: item.org_id.clone(),
        });
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
        .map(|(summary, _, _, _, _)| summary)
        .collect::<Vec<_>>();
    usage_rows.sort_by(|left, right| {
        left.datasource_identity
            .cmp(&right.datasource_identity)
            .then_with(|| left.datasource_uid.cmp(&right.datasource_uid))
    });

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
}

fn build_offline_dependency_contract_document(
    query_inputs: Vec<DependencyQueryInput>,
    datasource_inventory: &[DatasourceInventoryItem],
) -> OfflineDependencyReportDocument {
    build_offline_dependency_contract_document_from_query_inputs(query_inputs, datasource_inventory)
}

pub(crate) fn build_offline_dependency_contract_document_from_report_rows(
    query_report_rows: &[ExportInspectionQueryRow],
    datasource_inventory: &[DatasourceInventoryItem],
) -> OfflineDependencyReportDocument {
    let query_inputs = query_report_rows
        .iter()
        .map(build_dependency_query_input_from_report_row)
        .collect::<Vec<DependencyQueryInput>>();
    build_offline_dependency_contract_document(query_inputs, datasource_inventory)
}

#[cfg(test)]
pub(crate) fn build_offline_dependency_contract(
    query_report_rows: &[Value],
    datasource_inventory: &[DatasourceInventoryItem],
) -> Value {
    let query_inputs = query_report_rows
        .iter()
        .filter_map(build_dependency_query_input_from_value)
        .collect::<Vec<DependencyQueryInput>>();
    build_offline_dependency_contract_document(query_inputs, datasource_inventory).as_json()
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_offline_dependency_contract_from_report_rows(
    query_report_rows: &[ExportInspectionQueryRow],
    datasource_inventory: &[DatasourceInventoryItem],
) -> Value {
    build_offline_dependency_contract_document_from_report_rows(
        query_report_rows,
        datasource_inventory,
    )
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
                    "folderPath": "General",
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
                    "folderPath": "General",
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
                    "folderPath": "Operations",
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
        assert_eq!(document["datasourceUsage"][0]["folderCount"], json!(2));
        assert_eq!(
            document["datasourceUsage"][0]["highBlastRadius"],
            json!(true)
        );
        assert_eq!(document["datasourceUsage"][0]["crossFolder"], json!(true));
        assert_eq!(
            document["datasourceUsage"][0]["folderPaths"],
            json!(["General", "Operations"])
        );
        assert_eq!(
            document["datasourceUsage"][0]["dashboardTitles"],
            json!(["Dash A", "Dash B"])
        );
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

    #[test]
    fn build_offline_dependency_contract_surfaces_richer_usage_and_orphan_details() {
        let document = build_offline_dependency_contract(
            &[json!({
                "dashboardUid": "dash-a",
                "dashboardTitle": "Dash A",
                "folderPath": "General",
                "panelId": "7",
                "panelTitle": "Panel",
                "panelType": "timeseries",
                "refId": "A",
                "datasource": "Prometheus Main",
                "datasourceUid": "prom-main",
                "datasourceType": "prometheus",
                "file": "dash-a.json",
                "queryField": "expr",
                "query": "up"
            })],
            &[
                DatasourceInventoryItem {
                    uid: "prom-main".to_string(),
                    name: "Prometheus Main".to_string(),
                    datasource_type: "prometheus".to_string(),
                    access: "proxy".to_string(),
                    url: "http://prometheus:9090".to_string(),
                    database: String::new(),
                    default_bucket: String::new(),
                    organization: String::new(),
                    index_pattern: String::new(),
                    is_default: "true".to_string(),
                    org: "Main Org.".to_string(),
                    org_id: "1".to_string(),
                },
                DatasourceInventoryItem {
                    uid: "orphan-main".to_string(),
                    name: "Orphan Main".to_string(),
                    datasource_type: "grafana-postgresql-datasource".to_string(),
                    access: "proxy".to_string(),
                    url: "postgresql://postgres:5432/orphan".to_string(),
                    database: "orphan_db".to_string(),
                    default_bucket: String::new(),
                    organization: String::new(),
                    index_pattern: String::new(),
                    is_default: "false".to_string(),
                    org: "Main Org.".to_string(),
                    org_id: "1".to_string(),
                },
            ],
        );

        assert_eq!(document["summary"]["orphanedDatasourceCount"], json!(1));
        assert_eq!(
            document["datasourceUsage"][0]["datasourceUid"],
            json!("prom-main")
        );
        assert_eq!(
            document["datasourceUsage"][0]["datasourceType"],
            json!("prometheus")
        );
        assert_eq!(
            document["datasourceUsage"][0]["queryFields"],
            json!(["expr"])
        );
        assert_eq!(document["datasourceUsage"][0]["folderCount"], json!(1));
        assert_eq!(
            document["datasourceUsage"][0]["highBlastRadius"],
            json!(false)
        );
        assert_eq!(document["datasourceUsage"][0]["crossFolder"], json!(false));
        assert_eq!(
            document["datasourceUsage"][0]["folderPaths"],
            json!(["General"])
        );
        assert_eq!(
            document["datasourceUsage"][0]["dashboardTitles"],
            json!(["Dash A"])
        );
        assert_eq!(
            document["orphanedDatasources"][0]["uid"],
            json!("orphan-main")
        );
        assert_eq!(
            document["orphanedDatasources"][0]["name"],
            json!("Orphan Main")
        );
        assert_eq!(
            document["orphanedDatasources"][0]["family"],
            json!("postgresql")
        );
        assert_eq!(
            document["orphanedDatasources"][0]["database"],
            json!("orphan_db")
        );
    }
}
