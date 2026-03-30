//! Governance row models and shared risk constants for inspect output.
use serde::Serialize;

/// Struct definition for GovernanceSummary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct GovernanceSummary {
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "queryRecordCount")]
    pub(crate) query_record_count: usize,
    #[serde(rename = "datasourceInventoryCount")]
    pub(crate) datasource_inventory_count: usize,
    #[serde(rename = "datasourceFamilyCount")]
    pub(crate) datasource_family_count: usize,
    #[serde(rename = "datasourceCoverageCount")]
    pub(crate) datasource_coverage_count: usize,
    #[serde(rename = "dashboardDatasourceEdgeCount")]
    pub(crate) dashboard_datasource_edge_count: usize,
    #[serde(rename = "datasourceRiskCoverageCount")]
    pub(crate) datasource_risk_coverage_count: usize,
    #[serde(rename = "highBlastRadiusDatasourceCount")]
    pub(crate) high_blast_radius_datasource_count: usize,
    #[serde(rename = "dashboardRiskCoverageCount")]
    pub(crate) dashboard_risk_coverage_count: usize,
    #[serde(rename = "mixedDatasourceDashboardCount")]
    pub(crate) mixed_datasource_dashboard_count: usize,
    #[serde(rename = "orphanedDatasourceCount")]
    pub(crate) orphaned_datasource_count: usize,
    #[serde(rename = "riskRecordCount")]
    pub(crate) risk_record_count: usize,
    #[serde(rename = "queryAuditCount")]
    pub(crate) query_audit_count: usize,
    #[serde(rename = "dashboardAuditCount")]
    pub(crate) dashboard_audit_count: usize,
}

/// Struct definition for DatasourceFamilyCoverageRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DatasourceFamilyCoverageRow {
    pub(crate) family: String,
    #[serde(rename = "datasourceTypes")]
    pub(crate) datasource_types: Vec<String>,
    #[serde(rename = "datasourceCount")]
    pub(crate) datasource_count: usize,
    #[serde(rename = "orphanedDatasourceCount")]
    pub(crate) orphaned_datasource_count: usize,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
}

/// Struct definition for DatasourceCoverageRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DatasourceCoverageRow {
    #[serde(rename = "datasourceUid")]
    pub(crate) datasource_uid: String,
    pub(crate) datasource: String,
    pub(crate) family: String,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "dashboardUids")]
    pub(crate) dashboard_uids: Vec<String>,
    #[serde(rename = "queryFields")]
    pub(crate) query_fields: Vec<String>,
    pub(crate) orphaned: bool,
}

/// Struct definition for DatasourceGovernanceRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DatasourceGovernanceRow {
    #[serde(rename = "datasourceUid")]
    pub(crate) datasource_uid: String,
    pub(crate) datasource: String,
    pub(crate) family: String,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "mixedDashboardCount")]
    pub(crate) mixed_dashboard_count: usize,
    #[serde(rename = "riskCount")]
    pub(crate) risk_count: usize,
    #[serde(rename = "riskKinds")]
    pub(crate) risk_kinds: Vec<String>,
    #[serde(rename = "folderCount")]
    pub(crate) folder_count: usize,
    #[serde(rename = "highBlastRadius")]
    pub(crate) high_blast_radius: bool,
    #[serde(rename = "crossFolder")]
    pub(crate) cross_folder: bool,
    #[serde(rename = "folderPaths")]
    pub(crate) folder_paths: Vec<String>,
    #[serde(rename = "dashboardUids")]
    pub(crate) dashboard_uids: Vec<String>,
    #[serde(rename = "dashboardTitles")]
    pub(crate) dashboard_titles: Vec<String>,
    pub(crate) orphaned: bool,
}

/// Struct definition for DashboardDependencyRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DashboardDependencyRow {
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    pub(crate) dashboard_title: String,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    #[serde(rename = "file")]
    pub(crate) file_path: String,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    #[serde(rename = "datasourceCount")]
    pub(crate) datasource_count: usize,
    #[serde(rename = "datasourceFamilyCount")]
    pub(crate) datasource_family_count: usize,
    #[serde(rename = "panelIds")]
    pub(crate) panel_ids: Vec<String>,
    pub(crate) datasources: Vec<String>,
    #[serde(rename = "datasourceFamilies")]
    pub(crate) datasource_families: Vec<String>,
    #[serde(rename = "queryFields")]
    pub(crate) query_fields: Vec<String>,
    #[serde(rename = "panelVariables")]
    pub(crate) panel_variables: Vec<String>,
    #[serde(rename = "queryVariables")]
    pub(crate) query_variables: Vec<String>,
    pub(crate) metrics: Vec<String>,
    pub(crate) functions: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
}

/// Struct definition for DashboardDatasourceEdgeRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DashboardDatasourceEdgeRow {
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    pub(crate) dashboard_title: String,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    #[serde(rename = "datasourceUid")]
    pub(crate) datasource_uid: String,
    pub(crate) datasource: String,
    #[serde(rename = "datasourceType")]
    pub(crate) datasource_type: String,
    pub(crate) family: String,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    #[serde(rename = "queryFields")]
    pub(crate) query_fields: Vec<String>,
    #[serde(rename = "queryVariables")]
    pub(crate) query_variables: Vec<String>,
    pub(crate) metrics: Vec<String>,
    pub(crate) functions: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
}

/// Struct definition for DashboardGovernanceRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DashboardGovernanceRow {
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    pub(crate) dashboard_title: String,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    #[serde(rename = "datasourceCount")]
    pub(crate) datasource_count: usize,
    #[serde(rename = "datasourceFamilyCount")]
    pub(crate) datasource_family_count: usize,
    pub(crate) datasources: Vec<String>,
    #[serde(rename = "datasourceFamilies")]
    pub(crate) datasource_families: Vec<String>,
    #[serde(rename = "mixedDatasource")]
    pub(crate) mixed_datasource: bool,
    #[serde(rename = "riskCount")]
    pub(crate) risk_count: usize,
    #[serde(rename = "riskKinds")]
    pub(crate) risk_kinds: Vec<String>,
}

/// Struct definition for GovernanceRiskRow.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct GovernanceRiskRow {
    pub(crate) kind: String,
    pub(crate) severity: String,
    pub(crate) category: String,
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "panelId")]
    pub(crate) panel_id: String,
    pub(crate) datasource: String,
    pub(crate) detail: String,
    pub(crate) recommendation: String,
}

/// Struct definition for QueryAuditRow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct QueryAuditRow {
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    pub(crate) dashboard_title: String,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    #[serde(rename = "panelId")]
    pub(crate) panel_id: String,
    #[serde(rename = "panelTitle")]
    pub(crate) panel_title: String,
    #[serde(rename = "refId")]
    pub(crate) ref_id: String,
    pub(crate) datasource: String,
    #[serde(rename = "datasourceUid")]
    pub(crate) datasource_uid: String,
    #[serde(rename = "datasourceFamily")]
    pub(crate) datasource_family: String,
    #[serde(rename = "aggregationDepth")]
    pub(crate) aggregation_depth: usize,
    #[serde(rename = "regexMatcherCount")]
    pub(crate) regex_matcher_count: usize,
    #[serde(rename = "estimatedSeriesRisk")]
    pub(crate) estimated_series_risk: String,
    #[serde(rename = "queryCostScore")]
    pub(crate) query_cost_score: usize,
    pub(crate) score: usize,
    pub(crate) severity: String,
    pub(crate) reasons: Vec<String>,
    pub(crate) recommendations: Vec<String>,
}

/// Struct definition for DashboardAuditRow.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct DashboardAuditRow {
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    pub(crate) dashboard_title: String,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    #[serde(rename = "refreshIntervalSeconds")]
    pub(crate) refresh_interval_seconds: Option<u64>,
    pub(crate) score: usize,
    pub(crate) severity: String,
    pub(crate) reasons: Vec<String>,
    pub(crate) recommendations: Vec<String>,
}

pub(crate) const GOVERNANCE_RISK_KIND_MIXED_DASHBOARD: &str = "mixed-datasource-dashboard";
pub(crate) const GOVERNANCE_RISK_KIND_DATASOURCE_HIGH_BLAST_RADIUS: &str =
    "datasource-high-blast-radius";
pub(crate) const GOVERNANCE_RISK_KIND_ORPHANED_DATASOURCE: &str = "orphaned-datasource";
pub(crate) const GOVERNANCE_RISK_KIND_UNKNOWN_DATASOURCE_FAMILY: &str = "unknown-datasource-family";
pub(crate) const GOVERNANCE_RISK_KIND_EMPTY_QUERY_ANALYSIS: &str = "empty-query-analysis";
pub(crate) const GOVERNANCE_RISK_KIND_BROAD_LOKI_SELECTOR: &str = "broad-loki-selector";
pub(crate) const GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE: &str = "dashboard-panel-pressure";
