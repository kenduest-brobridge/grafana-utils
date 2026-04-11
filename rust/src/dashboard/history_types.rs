use crate::common::SharedDiffSummary;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[allow(dead_code)]
pub(crate) const BROWSE_HISTORY_RESTORE_MESSAGE: &str =
    "Restored by grafana-utils dashboard browse";
pub(crate) const DASHBOARD_HISTORY_RESTORE_MESSAGE: &str =
    "Restored by grafana-util dashboard history";
pub(crate) const DASHBOARD_HISTORY_LIST_KIND: &str = "grafana-util-dashboard-history-list";
pub(crate) const DASHBOARD_HISTORY_RESTORE_KIND: &str = "grafana-util-dashboard-history-restore";
pub(crate) const DASHBOARD_HISTORY_EXPORT_KIND: &str = "grafana-util-dashboard-history-export";
pub(crate) const DASHBOARD_HISTORY_INVENTORY_KIND: &str =
    "grafana-util-dashboard-history-inventory";
pub(crate) const DASHBOARD_HISTORY_DIFF_KIND: &str = "grafana-util-dashboard-history-diff";
pub(crate) const HISTORY_RESTORE_PROMPT_LIMIT: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DashboardHistoryVersion {
    pub version: i64,
    pub created: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DashboardHistoryListDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    #[serde(rename = "dashboardUid")]
    pub dashboard_uid: String,
    #[serde(rename = "versionCount")]
    pub version_count: usize,
    pub versions: Vec<DashboardHistoryVersion>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct DashboardHistoryRestoreDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    pub mode: String,
    #[serde(rename = "dashboardUid")]
    pub dashboard_uid: String,
    #[serde(rename = "currentVersion")]
    pub current_version: i64,
    #[serde(rename = "restoreVersion")]
    pub restore_version: i64,
    #[serde(rename = "currentTitle")]
    pub current_title: String,
    #[serde(rename = "restoredTitle")]
    pub restored_title: String,
    #[serde(rename = "targetFolderUid", skip_serializing_if = "Option::is_none")]
    pub target_folder_uid: Option<String>,
    #[serde(rename = "createsNewRevision")]
    pub creates_new_revision: bool,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct DashboardHistoryExportVersion {
    pub version: i64,
    pub created: String,
    #[serde(rename = "createdBy")]
    pub created_by: String,
    pub message: String,
    pub dashboard: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct DashboardHistoryExportDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    #[serde(rename = "dashboardUid")]
    pub dashboard_uid: String,
    #[serde(rename = "currentVersion")]
    pub current_version: i64,
    #[serde(rename = "currentTitle")]
    pub current_title: String,
    #[serde(rename = "versionCount")]
    pub version_count: usize,
    pub versions: Vec<DashboardHistoryExportVersion>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct DashboardHistoryInventoryItem {
    #[serde(rename = "dashboardUid")]
    pub dashboard_uid: String,
    #[serde(rename = "currentTitle")]
    pub current_title: String,
    #[serde(rename = "currentVersion")]
    pub current_version: i64,
    #[serde(rename = "versionCount")]
    pub version_count: usize,
    pub path: String,
    #[serde(rename = "scope", skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct DashboardHistoryInventoryDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    #[serde(rename = "artifactCount")]
    pub artifact_count: usize,
    pub artifacts: Vec<DashboardHistoryInventoryItem>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DashboardHistoryDiffDocument {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    pub summary: SharedDiffSummary,
    pub rows: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DashboardRestorePreview {
    pub current_version: i64,
    pub current_title: String,
    pub restored_title: String,
    pub target_folder_uid: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LocalHistoryArtifact {
    pub path: PathBuf,
    pub scope: Option<String>,
    pub document: DashboardHistoryExportDocument,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HistoryDiffSource {
    Live {
        dashboard_uid: String,
    },
    Artifact {
        path: PathBuf,
    },
    ImportDir {
        input_dir: PathBuf,
        dashboard_uid: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ResolvedHistoryDiffSide {
    pub source_label: String,
    pub dashboard_uid: String,
    pub version: i64,
    pub title: String,
    pub dashboard: Value,
    pub compare_document: Value,
}
