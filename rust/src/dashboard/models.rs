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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org: Option<String>,
    #[serde(rename = "orgId", skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
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
}

/// Struct definition for VariantIndexEntry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct VariantIndexEntry {
    pub uid: String,
    pub title: String,
    pub path: String,
    pub format: String,
    pub org: String,
    #[serde(rename = "orgId")]
    pub org_id: String,
}

/// Struct definition for RootExportVariants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct RootExportVariants {
    pub raw: Option<String>,
    pub prompt: Option<String>,
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
    pub kind: String,
    pub items: Vec<DashboardIndexItem>,
    pub variants: RootExportVariants,
    #[serde(default)]
    pub folders: Vec<FolderInventoryItem>,
}
