//! Governance report facade for inspect mode.
//! Keeps the stable row models in a sibling module while document assembly lives here.
use serde::Serialize;

#[path = "inspect_governance_coverage.rs"]
mod inspect_governance_coverage;
#[path = "inspect_governance_document.rs"]
mod inspect_governance_document;
#[path = "inspect_governance_render.rs"]
mod inspect_governance_render;
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
};
#[allow(unused_imports)]
pub(crate) use inspect_governance_document::{
    build_dashboard_datasource_edge_rows, build_dashboard_dependency_rows,
    build_dashboard_governance_rows, build_export_inspection_governance_document,
    resolve_datasource_identity,
};
pub(crate) use inspect_governance_render::render_governance_table_report;
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
    GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE,
    GOVERNANCE_RISK_KIND_DATASOURCE_HIGH_BLAST_RADIUS, GOVERNANCE_RISK_KIND_EMPTY_QUERY_ANALYSIS,
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
