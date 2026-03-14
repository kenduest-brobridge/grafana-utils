//! Summary aggregates for dashboard inspection reports.
//! Provides compact DTOs for folder/datasource/dashboard-level coverage metrics.
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportFolderUsage {
    pub(crate) path: String,
    pub(crate) dashboards: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportDatasourceUsage {
    pub(crate) datasource: String,
    pub(crate) reference_count: usize,
    pub(crate) dashboard_count: usize,
}

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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct MixedDashboardSummary {
    pub(crate) uid: String,
    pub(crate) title: String,
    pub(crate) folder_path: String,
    pub(crate) datasource_count: usize,
    pub(crate) datasources: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionSummary {
    pub(crate) import_dir: String,
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionSummaryJsonSummary {
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportFolderUsageJsonRow {
    pub(crate) path: String,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportDatasourceUsageJsonRow {
    pub(crate) name: String,
    #[serde(rename = "referenceCount")]
    pub(crate) reference_count: usize,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct MixedDashboardJsonRow {
    pub(crate) uid: String,
    pub(crate) title: String,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    pub(crate) datasources: Vec<String>,
}

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
pub(crate) fn build_export_inspection_summary_document(
    summary: &ExportInspectionSummary,
) -> ExportInspectionSummaryDocument {
    ExportInspectionSummaryDocument {
        summary: ExportInspectionSummaryJsonSummary {
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
