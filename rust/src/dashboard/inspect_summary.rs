//! Summary aggregates for dashboard inspection reports.
//! Provides compact DTOs for folder/datasource/dashboard-level coverage metrics.
use serde::Serialize;

/// Struct definition for ExportFolderUsage.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportFolderUsage {
    pub(crate) path: String,
    pub(crate) dashboards: usize,
}

/// Struct definition for ExportDatasourceUsage.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportDatasourceUsage {
    pub(crate) datasource: String,
    pub(crate) reference_count: usize,
    pub(crate) dashboard_count: usize,
}

/// Struct definition for DatasourceInventorySummary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DatasourceInventorySummary {
    pub(crate) uid: String,
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) datasource_type: String,
    pub(crate) access: String,
    pub(crate) url: String,
    #[serde(rename = "isDefault")]
    pub(crate) is_default: String,
    pub(crate) org: String,
    #[serde(rename = "orgId")]
    pub(crate) org_id: String,
    #[serde(rename = "referenceCount")]
    pub(crate) reference_count: usize,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
}

/// Struct definition for MixedDashboardSummary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct MixedDashboardSummary {
    pub(crate) uid: String,
    pub(crate) title: String,
    pub(crate) folder_path: String,
    pub(crate) datasource_count: usize,
    pub(crate) datasources: Vec<String>,
}

/// Struct definition for ExportInspectionSummary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionSummary {
    pub(crate) input_dir: String,
    pub(crate) export_org: Option<String>,
    pub(crate) export_org_id: Option<String>,
    pub(crate) dashboard_count: usize,
    pub(crate) folder_count: usize,
    pub(crate) panel_count: usize,
    pub(crate) query_count: usize,
    pub(crate) datasource_inventory_count: usize,
    pub(crate) orphaned_datasource_count: usize,
    pub(crate) mixed_dashboard_count: usize,
    pub(crate) folder_paths: Vec<ExportFolderUsage>,
    pub(crate) datasource_usage: Vec<ExportDatasourceUsage>,
    pub(crate) datasource_inventory: Vec<DatasourceInventorySummary>,
    pub(crate) orphaned_datasources: Vec<DatasourceInventorySummary>,
    pub(crate) mixed_dashboards: Vec<MixedDashboardSummary>,
}

/// Struct definition for ExportInspectionSummaryJsonSummary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionSummaryJsonSummary {
    #[serde(rename = "exportOrg", skip_serializing_if = "Option::is_none")]
    pub(crate) export_org: Option<String>,
    #[serde(rename = "exportOrgId", skip_serializing_if = "Option::is_none")]
    pub(crate) export_org_id: Option<String>,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "folderCount")]
    pub(crate) folder_count: usize,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    #[serde(rename = "mixedDatasourceDashboardCount")]
    pub(crate) mixed_datasource_dashboard_count: usize,
    #[serde(rename = "datasourceInventoryCount")]
    pub(crate) datasource_inventory_count: usize,
    #[serde(rename = "orphanedDatasourceCount")]
    pub(crate) orphaned_datasource_count: usize,
}

/// Struct definition for ExportFolderUsageJsonRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportFolderUsageJsonRow {
    pub(crate) path: String,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
}

/// Struct definition for ExportDatasourceUsageJsonRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportDatasourceUsageJsonRow {
    pub(crate) name: String,
    #[serde(rename = "referenceCount")]
    pub(crate) reference_count: usize,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
}

/// Struct definition for MixedDashboardJsonRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct MixedDashboardJsonRow {
    pub(crate) uid: String,
    pub(crate) title: String,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    pub(crate) datasources: Vec<String>,
}

/// Struct definition for ExportInspectionSummaryDocument.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionSummaryDocument {
    pub(crate) summary: ExportInspectionSummaryJsonSummary,
    pub(crate) folders: Vec<ExportFolderUsageJsonRow>,
    pub(crate) datasources: Vec<ExportDatasourceUsageJsonRow>,
    #[serde(rename = "datasourceInventory")]
    pub(crate) datasource_inventory: Vec<DatasourceInventorySummary>,
    #[serde(rename = "orphanedDatasources")]
    pub(crate) orphaned_datasources: Vec<DatasourceInventorySummary>,
    #[serde(rename = "mixedDatasourceDashboards")]
    pub(crate) mixed_datasource_dashboards: Vec<MixedDashboardJsonRow>,
}

// Keep the machine-readable summary JSON contract aligned with the Python
// inspection document while allowing the internal Rust summary struct to retain
// snake_case field names that read naturally in table/text code paths.
/// Purpose: implementation note.
pub(crate) fn build_export_inspection_summary_document(
    summary: &ExportInspectionSummary,
) -> ExportInspectionSummaryDocument {
    ExportInspectionSummaryDocument {
        summary: ExportInspectionSummaryJsonSummary {
            export_org: summary.export_org.clone(),
            export_org_id: summary.export_org_id.clone(),
            dashboard_count: summary.dashboard_count,
            folder_count: summary.folder_count,
            panel_count: summary.panel_count,
            query_count: summary.query_count,
            mixed_datasource_dashboard_count: summary.mixed_dashboard_count,
            datasource_inventory_count: summary.datasource_inventory_count,
            orphaned_datasource_count: summary.orphaned_datasource_count,
        },
        folders: summary
            .folder_paths
            .iter()
            .map(|item| ExportFolderUsageJsonRow {
                path: item.path.clone(),
                dashboard_count: item.dashboards,
            })
            .collect(),
        datasources: summary
            .datasource_usage
            .iter()
            .map(|item| ExportDatasourceUsageJsonRow {
                name: item.datasource.clone(),
                reference_count: item.reference_count,
                dashboard_count: item.dashboard_count,
            })
            .collect(),
        datasource_inventory: summary.datasource_inventory.clone(),
        orphaned_datasources: summary.orphaned_datasources.clone(),
        mixed_datasource_dashboards: summary
            .mixed_dashboards
            .iter()
            .map(|item| MixedDashboardJsonRow {
                uid: item.uid.clone(),
                title: item.title.clone(),
                folder_path: item.folder_path.clone(),
                datasources: item.datasources.clone(),
            })
            .collect(),
    }
}

/// Build the table-friendly summary rows used by the inspect export renderer.
pub(crate) fn build_export_inspection_summary_rows(
    summary: &ExportInspectionSummary,
) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    if let Some(export_org) = &summary.export_org {
        rows.push(vec!["export_org".to_string(), export_org.clone()]);
    }
    if let Some(export_org_id) = &summary.export_org_id {
        rows.push(vec!["export_org_id".to_string(), export_org_id.clone()]);
    }
    rows.extend([
        vec![
            "dashboard_count".to_string(),
            summary.dashboard_count.to_string(),
        ],
        vec!["folder_count".to_string(), summary.folder_count.to_string()],
        vec!["panel_count".to_string(), summary.panel_count.to_string()],
        vec!["query_count".to_string(), summary.query_count.to_string()],
        vec![
            "datasource_inventory_count".to_string(),
            summary.datasource_inventory_count.to_string(),
        ],
        vec![
            "orphaned_datasource_count".to_string(),
            summary.orphaned_datasource_count.to_string(),
        ],
        vec![
            "mixed_datasource_dashboard_count".to_string(),
            summary.mixed_dashboard_count.to_string(),
        ],
    ]);
    rows
}
