//! Shared dashboard export/import data models.
//!
//! These structs define the serialized contract for dashboard export metadata,
//! index files, folder inventory, and datasource inventory.

use serde::{Deserialize, Serialize};

/// Struct definition for ExportMetadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ExportMetadata {
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(
        rename = "toolVersion",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub tool_version: Option<String>,
    pub kind: String,
    pub variant: String,
    #[serde(rename = "dashboardCount")]
    pub dashboard_count: u64,
    #[serde(rename = "indexFile")]
    pub index_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(rename = "foldersFile", skip_serializing_if = "Option::is_none")]
    pub folders_file: Option<String>,
    #[serde(rename = "datasourcesFile", skip_serializing_if = "Option::is_none")]
    pub datasources_file: Option<String>,
    #[serde(rename = "permissionsFile", skip_serializing_if = "Option::is_none")]
    pub permissions_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
    #[serde(rename = "orgId", skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    #[serde(rename = "orgCount", skip_serializing_if = "Option::is_none")]
    pub org_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orgs: Option<Vec<ExportOrgSummary>>,
}

/// Struct definition for ExportOrgSummary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ExportOrgSummary {
    pub org: String,
    #[serde(rename = "orgId")]
    pub org_id: String,
    #[serde(rename = "dashboardCount")]
    pub dashboard_count: u64,
    #[serde(rename = "datasourceCount", skip_serializing_if = "Option::is_none")]
    pub datasource_count: Option<u64>,
    #[serde(
        rename = "usedDatasourceCount",
        skip_serializing_if = "Option::is_none"
    )]
    pub used_datasource_count: Option<u64>,
    #[serde(rename = "usedDatasources", skip_serializing_if = "Option::is_none")]
    pub used_datasources: Option<Vec<ExportDatasourceUsageSummary>>,
    #[serde(rename = "exportDir", skip_serializing_if = "Option::is_none")]
    pub export_dir: Option<String>,
}

/// Struct definition for ExportDatasourceUsageSummary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ExportDatasourceUsageSummary {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub datasource_type: Option<String>,
}

/// Struct definition for DashboardIndexItem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DashboardIndexItem {
    pub uid: String,
    pub title: String,
    #[serde(rename = "folderTitle")]
    pub folder_title: String,
    pub org: String,
    #[serde(rename = "orgId")]
    pub org_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provisioning_path: Option<String>,
}

/// Struct definition for VariantIndexEntry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct VariantIndexEntry {
    pub uid: String,
    pub title: String,
    pub path: String,
    pub format: String,
    #[serde(default)]
    pub org: String,
    #[serde(rename = "orgId")]
    #[serde(default)]
    pub org_id: String,
}

/// Struct definition for RootExportVariants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RootExportVariants {
    pub raw: Option<String>,
    pub prompt: Option<String>,
    pub provisioning: Option<String>,
}

/// Struct definition for FolderInventoryItem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct FolderInventoryItem {
    pub uid: String,
    pub title: String,
    pub path: String,
    #[serde(rename = "parentUid", skip_serializing_if = "Option::is_none")]
    pub parent_uid: Option<String>,
    pub org: String,
    #[serde(rename = "orgId")]
    pub org_id: String,
}

/// Struct definition for DatasourceInventoryItem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DatasourceInventoryItem {
    pub uid: String,
    pub name: String,
    #[serde(rename = "type")]
    pub datasource_type: String,
    pub access: String,
    pub url: String,
    #[serde(default)]
    pub database: String,
    #[serde(rename = "defaultBucket", default)]
    pub default_bucket: String,
    #[serde(default)]
    pub organization: String,
    #[serde(rename = "indexPattern", default)]
    pub index_pattern: String,
    #[serde(rename = "isDefault")]
    pub is_default: String,
    pub org: String,
    #[serde(rename = "orgId")]
    pub org_id: String,
}

/// Struct definition for RootExportIndex.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RootExportIndex {
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(
        rename = "toolVersion",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub tool_version: Option<String>,
    pub kind: String,
    pub items: Vec<DashboardIndexItem>,
    pub variants: RootExportVariants,
    #[serde(default)]
    pub folders: Vec<FolderInventoryItem>,
}
