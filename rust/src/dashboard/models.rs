//! Shared dashboard export/import data models.
//!
//! These structs define the serialized contract for dashboard export metadata,
//! index files, folder inventory, and datasource inventory.

use serde::{Deserialize, Serialize};

use crate::export_metadata::{ExportMetadataCapture, ExportMetadataPaths, ExportMetadataSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DashboardExportRootScopeKind {
    OrgRoot,
    AllOrgsRoot,
    WorkspaceRoot,
    Unknown,
}

impl DashboardExportRootScopeKind {
    pub(crate) fn is_aggregate(self) -> bool {
        matches!(self, Self::AllOrgsRoot | Self::WorkspaceRoot)
    }

    pub(crate) fn is_root(self) -> bool {
        !matches!(self, Self::Unknown)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardExportRootManifest {
    pub(crate) metadata: ExportMetadata,
    pub(crate) scope_kind: DashboardExportRootScopeKind,
}

impl DashboardExportRootManifest {
    pub(crate) fn from_metadata(metadata: ExportMetadata) -> Self {
        Self {
            scope_kind: Self::classify_scope_kind(&metadata),
            metadata,
        }
    }

    pub(crate) fn classify_scope_kind(metadata: &ExportMetadata) -> DashboardExportRootScopeKind {
        match metadata.scope_kind.as_deref() {
            Some("org-root") => DashboardExportRootScopeKind::OrgRoot,
            Some("all-orgs-root") => DashboardExportRootScopeKind::AllOrgsRoot,
            Some("workspace-root") => DashboardExportRootScopeKind::WorkspaceRoot,
            Some(_) => DashboardExportRootScopeKind::Unknown,
            None if metadata.variant == "all-orgs-root" => {
                DashboardExportRootScopeKind::AllOrgsRoot
            }
            None if metadata.variant == "root" && metadata.orgs.is_some() => {
                DashboardExportRootScopeKind::AllOrgsRoot
            }
            None if metadata.variant == "root" => DashboardExportRootScopeKind::OrgRoot,
            None => DashboardExportRootScopeKind::Unknown,
        }
    }

    pub(crate) fn with_scope_kind(mut self, scope_kind: DashboardExportRootScopeKind) -> Self {
        self.scope_kind = scope_kind;
        self
    }
}

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
    #[serde(rename = "scopeKind", skip_serializing_if = "Option::is_none", default)]
    pub scope_kind: Option<String>,
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
    #[serde(
        rename = "metadataVersion",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub metadata_version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub domain: Option<String>,
    #[serde(
        rename = "resourceKind",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub resource_kind: Option<String>,
    #[serde(
        rename = "bundleKind",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub bundle_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source: Option<ExportMetadataSource>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub capture: Option<ExportMetadataCapture>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub paths: Option<ExportMetadataPaths>,
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
