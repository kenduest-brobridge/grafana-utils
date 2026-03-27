//! Governance report facade for inspect mode.
//! Keeps the stable row models in a sibling module while document assembly lives here.
use serde::Serialize;

use super::inspect_render::render_simple_table;

#[path = "inspect_governance_coverage.rs"]
mod inspect_governance_coverage;
#[path = "inspect_governance_document.rs"]
mod inspect_governance_document;
#[path = "inspect_governance_risk.rs"]
mod inspect_governance_risk;
#[path = "inspect_governance_rows.rs"]
mod inspect_governance_rows;

#[allow(unused_imports)]
pub(crate) use super::inspect_report::ExportInspectionQueryReport;
#[allow(unused_imports)]
pub(crate) use super::ExportInspectionSummary;
pub(crate) use inspect_governance_coverage::{
    build_datasource_coverage_rows, build_datasource_family_coverage_rows,
    build_datasource_governance_rows, build_inventory_lookup,
    dashboard_dependency_normalize_family_list, dashboard_dependency_unique_strings,
    normalize_family_name,
};
#[allow(unused_imports)]
pub(crate) use inspect_governance_document::{
    build_dashboard_datasource_edge_rows, build_dashboard_dependency_rows,
    build_dashboard_governance_rows, build_export_inspection_governance_document,
    resolve_datasource_identity,
};
#[cfg(test)]
pub(crate) use inspect_governance_risk::governance_risk_spec;
pub(crate) use inspect_governance_risk::{
    build_dashboard_audit_rows, build_governance_risk_rows, build_query_audit_rows,
    find_broad_loki_selector,
};
pub(crate) use inspect_governance_rows::{
    DashboardAuditRow, DashboardDatasourceEdgeRow, DashboardDependencyRow, DashboardGovernanceRow,
    DatasourceCoverageRow, DatasourceFamilyCoverageRow, DatasourceGovernanceRow, GovernanceRiskRow,
    GovernanceSummary, QueryAuditRow, GOVERNANCE_RISK_KIND_BROAD_LOKI_SELECTOR,
    GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE, GOVERNANCE_RISK_KIND_EMPTY_QUERY_ANALYSIS,
    GOVERNANCE_RISK_KIND_MIXED_DASHBOARD, GOVERNANCE_RISK_KIND_ORPHANED_DATASOURCE,
    GOVERNANCE_RISK_KIND_UNKNOWN_DATASOURCE_FAMILY,
};

/// Struct definition for ExportInspectionGovernanceDocument.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionGovernanceDocument {
    pub(crate) summary: GovernanceSummary,
    #[serde(rename = "datasourceFamilies")]
    pub(crate) datasource_families: Vec<DatasourceFamilyCoverageRow>,
    #[serde(rename = "dashboardDependencies")]
    pub(crate) dashboard_dependencies: Vec<DashboardDependencyRow>,
    #[serde(rename = "dashboardGovernance")]
    pub(crate) dashboard_governance: Vec<DashboardGovernanceRow>,
    #[serde(rename = "dashboardDatasourceEdges")]
    pub(crate) dashboard_datasource_edges: Vec<DashboardDatasourceEdgeRow>,
    #[serde(rename = "datasourceGovernance")]
    pub(crate) datasource_governance: Vec<DatasourceGovernanceRow>,
    pub(crate) datasources: Vec<DatasourceCoverageRow>,
    #[serde(rename = "riskRecords")]
    pub(crate) risk_records: Vec<GovernanceRiskRow>,
    #[serde(rename = "queryAudits")]
    pub(crate) query_audits: Vec<QueryAuditRow>,
    #[serde(rename = "dashboardAudits")]
    pub(crate) dashboard_audits: Vec<DashboardAuditRow>,
}

/// Render the already-normalized governance document into text rows without recomputing
/// risk logic or re-reading files.
pub(crate) fn render_governance_table_report(
    import_dir: &str,
    document: &ExportInspectionGovernanceDocument,
) -> Vec<String> {
    let mut lines = vec![
        format!("Export inspection governance: {import_dir}"),
        String::new(),
    ];

    lines.push("# Summary".to_string());
    lines.extend(render_simple_table(
        &[
            "DASHBOARDS",
            "QUERIES",
            "FAMILIES",
            "DATASOURCES",
            "DASHBOARD_DATASOURCE_EDGES",
            "DATASOURCES_WITH_RISKS",
            "DASHBOARDS_WITH_RISKS",
            "MIXED_DASHBOARDS",
            "ORPHANED_DATASOURCES",
            "RISKS",
        ],
        &[vec![
            document.summary.dashboard_count.to_string(),
            document.summary.query_record_count.to_string(),
            document.summary.datasource_family_count.to_string(),
            document.summary.datasource_coverage_count.to_string(),
            document.summary.dashboard_datasource_edge_count.to_string(),
            document.summary.datasource_risk_coverage_count.to_string(),
            document.summary.dashboard_risk_coverage_count.to_string(),
            document
                .summary
                .mixed_datasource_dashboard_count
                .to_string(),
            document.summary.orphaned_datasource_count.to_string(),
            document.summary.risk_record_count.to_string(),
        ]],
        true,
    ));

    lines.push(String::new());
    lines.push("# Datasource Families".to_string());
    let family_rows = document
        .datasource_families
        .iter()
        .map(|row| {
            vec![
                row.family.clone(),
                row.datasource_types.join(","),
                row.datasource_count.to_string(),
                row.orphaned_datasource_count.to_string(),
                row.dashboard_count.to_string(),
                row.panel_count.to_string(),
                row.query_count.to_string(),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if family_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "FAMILY",
                "TYPES",
                "DATASOURCES",
                "ORPHANED_DATASOURCES",
                "DASHBOARDS",
                "PANELS",
                "QUERIES",
            ],
            &family_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Dashboard Dependencies".to_string());
    let dashboard_rows = document
        .dashboard_dependencies
        .iter()
        .map(|row| {
            vec![
                row.dashboard_uid.clone(),
                row.dashboard_title.clone(),
                row.folder_path.clone(),
                row.panel_count.to_string(),
                row.query_count.to_string(),
                row.datasource_count.to_string(),
                row.datasource_family_count.to_string(),
                row.datasources.join(","),
                row.datasource_families.join(","),
                row.query_fields.join(","),
                row.metrics.join(","),
                row.functions.join(","),
                row.measurements.join(","),
                row.buckets.join(","),
                row.file_path.clone(),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if dashboard_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "DASHBOARD_UID",
                "TITLE",
                "FOLDER_PATH",
                "PANELS",
                "QUERIES",
                "DATASOURCE_COUNT",
                "DATASOURCE_FAMILY_COUNT",
                "DATASOURCES",
                "FAMILIES",
                "QUERY_FIELDS",
                "METRICS",
                "FUNCTIONS",
                "MEASUREMENTS",
                "BUCKETS",
                "FILE",
            ],
            &dashboard_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Dashboard Governance".to_string());
    let dashboard_governance_rows = document
        .dashboard_governance
        .iter()
        .map(|row| {
            let datasources = if row.datasources.is_empty() {
                "(none)".to_string()
            } else {
                row.datasources.join(",")
            };
            let datasource_families = if row.datasource_families.is_empty() {
                "(none)".to_string()
            } else {
                row.datasource_families.join(",")
            };
            let risk_kinds = if row.risk_kinds.is_empty() {
                "(none)".to_string()
            } else {
                row.risk_kinds.join(",")
            };
            vec![
                row.dashboard_uid.clone(),
                row.dashboard_title.clone(),
                row.folder_path.clone(),
                row.panel_count.to_string(),
                row.query_count.to_string(),
                row.datasource_count.to_string(),
                row.datasource_family_count.to_string(),
                datasources,
                datasource_families,
                if row.mixed_datasource {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
                row.risk_count.to_string(),
                risk_kinds,
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if dashboard_governance_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "DASHBOARD_UID",
                "TITLE",
                "FOLDER_PATH",
                "PANELS",
                "QUERIES",
                "DATASOURCE_COUNT",
                "DATASOURCE_FAMILY_COUNT",
                "DATASOURCES",
                "FAMILIES",
                "MIXED_DATASOURCE",
                "RISKS",
                "RISK_KINDS",
            ],
            &dashboard_governance_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Dashboard Datasource Edges".to_string());
    let edge_rows = document
        .dashboard_datasource_edges
        .iter()
        .map(|row| {
            vec![
                row.dashboard_uid.clone(),
                row.dashboard_title.clone(),
                row.folder_path.clone(),
                row.datasource_uid.clone(),
                row.datasource.clone(),
                row.datasource_type.clone(),
                row.family.clone(),
                row.panel_count.to_string(),
                row.query_count.to_string(),
                row.query_fields.join(","),
                row.metrics.join(","),
                row.functions.join(","),
                row.measurements.join(","),
                row.buckets.join(","),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if edge_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "DASHBOARD_UID",
                "TITLE",
                "FOLDER_PATH",
                "DATASOURCE_UID",
                "DATASOURCE",
                "DATASOURCE_TYPE",
                "FAMILY",
                "PANELS",
                "QUERIES",
                "QUERY_FIELDS",
                "METRICS",
                "FUNCTIONS",
                "MEASUREMENTS",
                "BUCKETS",
            ],
            &edge_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Datasource Governance".to_string());
    let datasource_governance_rows = document
        .datasource_governance
        .iter()
        .map(|row| {
            let dashboard_uids = if row.dashboard_uids.is_empty() {
                "(none)".to_string()
            } else {
                row.dashboard_uids.join(",")
            };
            let risk_kinds = if row.risk_kinds.is_empty() {
                "(none)".to_string()
            } else {
                row.risk_kinds.join(",")
            };
            vec![
                row.datasource_uid.clone(),
                row.datasource.clone(),
                row.family.clone(),
                row.query_count.to_string(),
                row.dashboard_count.to_string(),
                row.panel_count.to_string(),
                row.mixed_dashboard_count.to_string(),
                row.risk_count.to_string(),
                risk_kinds,
                dashboard_uids,
                if row.orphaned {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if datasource_governance_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "UID",
                "DATASOURCE",
                "FAMILY",
                "QUERIES",
                "DASHBOARDS",
                "PANELS",
                "MIXED_DASHBOARDS",
                "RISKS",
                "RISK_KINDS",
                "DASHBOARD_UIDS",
                "ORPHANED",
            ],
            &datasource_governance_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Datasources".to_string());
    let datasource_rows = document
        .datasources
        .iter()
        .map(|row| {
            let dashboard_uids = if row.dashboard_uids.is_empty() {
                "(none)".to_string()
            } else {
                row.dashboard_uids.join(",")
            };
            let query_fields = if row.query_fields.is_empty() {
                "(none)".to_string()
            } else {
                row.query_fields.join(",")
            };
            vec![
                row.datasource_uid.clone(),
                row.datasource.clone(),
                row.family.clone(),
                row.query_count.to_string(),
                row.dashboard_count.to_string(),
                row.panel_count.to_string(),
                dashboard_uids,
                query_fields,
                if row.orphaned {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if datasource_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "UID",
                "DATASOURCE",
                "FAMILY",
                "QUERIES",
                "DASHBOARDS",
                "PANELS",
                "DASHBOARD_UIDS",
                "QUERY_FIELDS",
                "ORPHANED",
            ],
            &datasource_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Risks".to_string());
    let risk_rows = document
        .risk_records
        .iter()
        .map(|row| {
            vec![
                row.severity.clone(),
                row.category.clone(),
                row.kind.clone(),
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                row.datasource.clone(),
                row.detail.clone(),
                row.recommendation.clone(),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if risk_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "SEVERITY",
                "CATEGORY",
                "KIND",
                "DASHBOARD_UID",
                "PANEL_ID",
                "DATASOURCE",
                "DETAIL",
                "RECOMMENDATION",
            ],
            &risk_rows,
            true,
        ));
    }
    lines
}
