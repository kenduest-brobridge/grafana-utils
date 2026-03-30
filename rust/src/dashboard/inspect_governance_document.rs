//! Governance document assembly for dashboard inspect output.
//! Keeps dashboard dependency, datasource edge, and summary orchestration out of the facade.
use std::collections::{BTreeMap, BTreeSet};

use super::super::inspect_report::{
    normalize_query_report, ExportInspectionQueryReport, ExportInspectionQueryRow,
};
use super::{
    build_dashboard_audit_rows, build_datasource_coverage_rows,
    build_datasource_family_coverage_rows, build_datasource_governance_rows,
    build_governance_risk_rows, build_inventory_lookup, build_query_audit_rows,
    dashboard_dependency_normalize_family_list, dashboard_dependency_unique_strings,
    DashboardDatasourceEdgeRow, DashboardDependencyRow, DashboardGovernanceRow,
    ExportInspectionGovernanceDocument, ExportInspectionSummary,
    GOVERNANCE_RISK_KIND_MIXED_DASHBOARD,
};
use crate::dashboard::inspect_family::normalize_family_name;

#[derive(Clone, Debug)]
pub(crate) struct ResolvedDatasourceIdentity {
    pub(crate) uid: String,
    pub(crate) name: String,
    pub(crate) datasource_type: String,
}

pub(crate) fn resolve_datasource_identity(
    row: &ExportInspectionQueryRow,
    inventory_by_uid: &BTreeMap<String, (String, String, String)>,
    inventory_by_name: &BTreeMap<String, (String, String, String)>,
) -> ResolvedDatasourceIdentity {
    let normalized_family = normalize_family_name(&row.datasource_type);
    let datasource_type = if matches!(normalized_family.as_str(), "search" | "tracing") {
        row.datasource_type.clone()
    } else {
        "unknown".to_string()
    };
    if !row.datasource_uid.trim().is_empty() {
        if let Some((uid, name, datasource_type)) = inventory_by_uid.get(&row.datasource_uid) {
            return ResolvedDatasourceIdentity {
                uid: uid.clone(),
                name: name.clone(),
                datasource_type: datasource_type.clone(),
            };
        }
    }
    if !row.datasource.trim().is_empty() {
        if let Some((uid, name, datasource_type)) = inventory_by_uid
            .get(&row.datasource)
            .or_else(|| inventory_by_name.get(&row.datasource))
        {
            return ResolvedDatasourceIdentity {
                uid: uid.clone(),
                name: name.clone(),
                datasource_type: datasource_type.clone(),
            };
        }
    }
    if !row.datasource_uid.trim().is_empty() {
        return ResolvedDatasourceIdentity {
            uid: row.datasource_uid.clone(),
            name: if row.datasource.trim().is_empty() {
                row.datasource_uid.clone()
            } else {
                row.datasource.clone()
            },
            datasource_type,
        };
    }
    if !row.datasource.trim().is_empty() {
        return ResolvedDatasourceIdentity {
            uid: row.datasource.clone(),
            name: row.datasource.clone(),
            datasource_type,
        };
    }
    ResolvedDatasourceIdentity {
        uid: "unknown".to_string(),
        name: "unknown".to_string(),
        datasource_type,
    }
}

/// Fold grouped query rows into dashboard dependencies without changing the
/// per-target row contract that the query report already established.
pub(crate) fn build_dashboard_dependency_rows(
    report: &ExportInspectionQueryReport,
) -> Vec<DashboardDependencyRow> {
    let normalized = normalize_query_report(report);
    normalized
        .dashboards
        .into_iter()
        .map(|dashboard| {
            let dashboard_uid = dashboard.dashboard_uid;
            let dashboard_title = dashboard.dashboard_title;
            let folder_path = dashboard.folder_path;
            let file_path = dashboard.file_path;
            let panel_count = dashboard.panels.len();
            let query_count = dashboard
                .panels
                .iter()
                .map(|panel| panel.queries.len())
                .sum::<usize>();
            let datasources = dashboard.datasources;
            let datasource_families =
                dashboard_dependency_normalize_family_list(&dashboard.datasource_families);
            let panel_ids = dashboard_dependency_unique_strings(
                dashboard.panels.iter().map(|panel| panel.panel_id.clone()),
            );
            let query_fields = dashboard_dependency_unique_strings(
                dashboard
                    .panels
                    .iter()
                    .flat_map(|panel| panel.query_fields.iter().cloned()),
            );
            let panel_variables =
                dashboard_dependency_unique_strings(dashboard.panels.iter().flat_map(|panel| {
                    panel
                        .queries
                        .iter()
                        .flat_map(|row| row.panel_variables.iter().cloned())
                }));
            let query_variables =
                dashboard_dependency_unique_strings(dashboard.panels.iter().flat_map(|panel| {
                    panel
                        .queries
                        .iter()
                        .flat_map(|row| row.query_variables.iter().cloned())
                }));
            let metrics =
                dashboard_dependency_unique_strings(dashboard.panels.iter().flat_map(|panel| {
                    panel
                        .queries
                        .iter()
                        .flat_map(|row| row.metrics.iter().cloned())
                }));
            let functions =
                dashboard_dependency_unique_strings(dashboard.panels.iter().flat_map(|panel| {
                    panel
                        .queries
                        .iter()
                        .flat_map(|row| row.functions.iter().cloned())
                }));
            let measurements =
                dashboard_dependency_unique_strings(dashboard.panels.iter().flat_map(|panel| {
                    panel
                        .queries
                        .iter()
                        .flat_map(|row| row.measurements.iter().cloned())
                }));
            let buckets =
                dashboard_dependency_unique_strings(dashboard.panels.iter().flat_map(|panel| {
                    panel
                        .queries
                        .iter()
                        .flat_map(|row| row.buckets.iter().cloned())
                }));

            DashboardDependencyRow {
                dashboard_uid,
                dashboard_title,
                folder_path,
                file_path,
                panel_count,
                query_count,
                datasource_count: datasources.len(),
                datasource_family_count: datasource_families.len(),
                panel_ids,
                datasources,
                datasource_families,
                query_fields,
                panel_variables,
                query_variables,
                metrics,
                functions,
                measurements,
                buckets,
            }
        })
        .collect()
}

/// Derive dashboard governance from the dependency rows plus risk records so mixed
/// datasource and pressure signals share one source of truth.
pub(crate) fn build_dashboard_governance_rows(
    report: &ExportInspectionQueryReport,
    risk_records: &[super::GovernanceRiskRow],
) -> Vec<DashboardGovernanceRow> {
    let mut risk_by_dashboard = BTreeMap::<String, (BTreeSet<String>, usize)>::new();
    for risk in risk_records {
        if risk.dashboard_uid.trim().is_empty() {
            continue;
        }
        let entry = risk_by_dashboard
            .entry(risk.dashboard_uid.clone())
            .or_insert_with(|| (BTreeSet::new(), 0usize));
        entry.0.insert(risk.kind.clone());
        entry.1 += 1;
    }

    let mut rows = build_dashboard_dependency_rows(report)
        .into_iter()
        .map(|row| {
            let (risk_kinds, risk_count) = risk_by_dashboard
                .remove(&row.dashboard_uid)
                .map(|(kinds, count)| (kinds.into_iter().collect::<Vec<String>>(), count))
                .unwrap_or_else(|| (Vec::new(), 0usize));
            DashboardGovernanceRow {
                dashboard_uid: row.dashboard_uid,
                dashboard_title: row.dashboard_title,
                folder_path: row.folder_path,
                panel_count: row.panel_count,
                query_count: row.query_count,
                datasource_count: row.datasource_count,
                datasource_family_count: row.datasource_family_count,
                datasources: row.datasources,
                datasource_families: row.datasource_families,
                mixed_datasource: risk_kinds
                    .iter()
                    .any(|kind| kind == GOVERNANCE_RISK_KIND_MIXED_DASHBOARD),
                risk_count,
                risk_kinds,
            }
        })
        .collect::<Vec<DashboardGovernanceRow>>();

    rows.sort_by(|left, right| {
        right
            .risk_count
            .cmp(&left.risk_count)
            .then(right.mixed_datasource.cmp(&left.mixed_datasource))
            .then(right.query_count.cmp(&left.query_count))
            .then(left.dashboard_uid.cmp(&right.dashboard_uid))
    });
    rows
}

/// Produce dashboard-to-datasource edge rows for governance and topology consumers from
/// the normalized report.
pub(crate) fn build_dashboard_datasource_edge_rows(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> Vec<DashboardDatasourceEdgeRow> {
    let (inventory_by_uid, inventory_by_name) = build_inventory_lookup(summary);
    let mut edges = BTreeMap::<
        (String, String),
        (
            String,
            String,
            String,
            String,
            String,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            usize,
        ),
    >::new();
    for row in &report.queries {
        let identity = resolve_datasource_identity(row, &inventory_by_uid, &inventory_by_name);
        let edge = edges
            .entry((row.dashboard_uid.clone(), identity.uid.clone()))
            .or_insert_with(|| {
                (
                    row.dashboard_title.clone(),
                    row.folder_path.clone(),
                    identity.name.clone(),
                    identity.datasource_type.clone(),
                    normalize_family_name(&identity.datasource_type),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    0usize,
                )
            });
        edge.5
            .insert(format!("{}:{}", row.dashboard_uid, row.panel_id));
        if !row.query_field.trim().is_empty() {
            edge.6.insert(row.query_field.clone());
        }
        edge.7.extend(row.query_variables.iter().cloned());
        edge.8.extend(row.metrics.iter().cloned());
        edge.9.extend(row.functions.iter().cloned());
        edge.10.extend(row.measurements.iter().cloned());
        edge.11.extend(row.buckets.iter().cloned());
        edge.12 += 1;
    }
    edges
        .into_iter()
        .map(
            |(
                (dashboard_uid, datasource_uid),
                (
                    dashboard_title,
                    folder_path,
                    datasource,
                    datasource_type,
                    family,
                    panel_keys,
                    query_fields,
                    query_variables,
                    metrics,
                    functions,
                    measurements,
                    buckets,
                    query_count,
                ),
            )| DashboardDatasourceEdgeRow {
                dashboard_uid,
                dashboard_title,
                folder_path,
                datasource_uid,
                datasource,
                datasource_type,
                family,
                panel_count: panel_keys.len(),
                query_count,
                query_fields: query_fields.into_iter().collect(),
                query_variables: query_variables.into_iter().collect(),
                metrics: metrics.into_iter().collect(),
                functions: functions.into_iter().collect(),
                measurements: measurements.into_iter().collect(),
                buckets: buckets.into_iter().collect(),
            },
        )
        .collect()
}

/// Project the normalized summary/report pair into the stable governance JSON document
/// used by both live and export inspect flows.
pub(crate) fn build_export_inspection_governance_document(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> ExportInspectionGovernanceDocument {
    let datasource_families = build_datasource_family_coverage_rows(summary, report);
    let dashboard_dependencies = build_dashboard_dependency_rows(report);
    let query_audits = build_query_audit_rows(summary, report);
    let dashboard_audits = build_dashboard_audit_rows(report);
    let risk_records = build_governance_risk_rows(summary, report);
    let dashboard_governance = build_dashboard_governance_rows(report, &risk_records);
    let dashboard_datasource_edges = build_dashboard_datasource_edge_rows(summary, report);
    let datasource_governance = build_datasource_governance_rows(summary, report);
    let datasources = build_datasource_coverage_rows(summary, report);
    ExportInspectionGovernanceDocument {
        summary: super::GovernanceSummary {
            dashboard_count: summary.dashboard_count,
            query_record_count: report.summary.report_row_count,
            datasource_inventory_count: summary.datasource_inventory_count,
            datasource_family_count: datasource_families.len(),
            datasource_coverage_count: datasources.len(),
            dashboard_datasource_edge_count: dashboard_datasource_edges.len(),
            datasource_risk_coverage_count: datasource_governance
                .iter()
                .filter(|row| row.risk_count != 0)
                .count(),
            high_blast_radius_datasource_count: datasource_governance
                .iter()
                .filter(|row| row.high_blast_radius)
                .count(),
            dashboard_risk_coverage_count: dashboard_governance
                .iter()
                .filter(|row| row.risk_count != 0)
                .count(),
            mixed_datasource_dashboard_count: summary.mixed_dashboard_count,
            orphaned_datasource_count: summary
                .datasource_inventory
                .iter()
                .filter(|item| item.reference_count == 0 && item.dashboard_count == 0)
                .count(),
            risk_record_count: risk_records.len(),
            query_audit_count: query_audits.len(),
            dashboard_audit_count: dashboard_audits.iter().filter(|row| row.score != 0).count(),
        },
        datasource_families,
        dashboard_dependencies,
        dashboard_governance,
        dashboard_datasource_edges,
        datasource_governance,
        datasources,
        risk_records,
        query_audits,
        dashboard_audits,
    }
}
