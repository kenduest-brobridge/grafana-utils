//! Shared types for the raw-to-prompt pipeline.

use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

pub(crate) const RAW_TO_PROMPT_KIND: &str = "grafana-utils-dashboard-raw-to-prompt-summary";
pub(crate) const MAPPING_KIND: &str = "grafana-utils-dashboard-datasource-map";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RawToPromptStatus {
    Ok,
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RawToPromptResolutionKind {
    Exact,
    Inferred,
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RawToPromptItemSummary {
    pub input_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file: Option<String>,
    pub status: RawToPromptStatus,
    pub resolution: RawToPromptResolutionKind,
    pub datasource_slots: usize,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct RawToPromptSummary {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    pub mode: String,
    pub scanned: usize,
    pub converted: usize,
    pub failed: usize,
    pub exact: usize,
    pub inferred: usize,
    pub unresolved: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_file: Option<String>,
    pub items: Vec<RawToPromptItemSummary>,
}

#[derive(Debug, Clone)]
pub(crate) struct RawToPromptPlanItem {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct RawToPromptPlan {
    pub mode: String,
    pub output_root: Option<PathBuf>,
    pub items: Vec<RawToPromptPlanItem>,
    pub metadata_source_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RawToPromptStats {
    pub exact: usize,
    pub inferred: usize,
    pub unresolved: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct RawToPromptOutcome {
    pub prompt_document: Value,
    pub datasource_slots: usize,
    pub resolution: RawToPromptResolutionKind,
    pub warnings: Vec<String>,
}

impl RawToPromptOutcome {
    pub(crate) fn resolution_string(&self) -> &'static str {
        match self.resolution {
            RawToPromptResolutionKind::Exact => "exact",
            RawToPromptResolutionKind::Inferred => "inferred",
            RawToPromptResolutionKind::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DashboardScanContext {
    pub ref_families: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedDatasourceReplacement {
    pub key: String,
    pub uid: String,
    pub name: String,
    pub datasource_type: String,
    pub exact: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct DatasourceMapDocument {
    #[serde(default)]
    pub kind: String,
    #[serde(rename = "schemaVersion", default)]
    pub schema_version: Option<i64>,
    #[serde(default)]
    pub datasources: Vec<DatasourceMapEntry>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct DatasourceMapEntry {
    #[serde(default)]
    pub r#match: DatasourceMatchRule,
    pub replace: DatasourceReplaceRule,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct DatasourceMatchRule {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub uid: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq, serde::Deserialize)]
pub(crate) struct DatasourceReplaceRule {
    #[serde(default)]
    pub uid: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub datasource_type: String,
}
