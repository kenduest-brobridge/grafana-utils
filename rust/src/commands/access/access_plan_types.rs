//! Access plan output contract types shared by CLI renderers and future TUI callers.

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPlanChange {
    pub field: String,
    pub before: Value,
    pub after: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPlanAction {
    pub action_id: String,
    pub domain: String,
    pub resource_kind: String,
    pub identity: String,
    pub scope: Option<String>,
    pub action: String,
    pub status: String,
    pub changed_fields: Vec<String>,
    pub changes: Vec<AccessPlanChange>,
    pub target: Option<Map<String, Value>>,
    pub blocked_reason: Option<String>,
    pub review_hints: Vec<String>,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPlanResourceReport {
    pub resource_kind: String,
    pub source_path: String,
    pub bundle_present: bool,
    pub source_count: usize,
    pub live_count: usize,
    pub checked: usize,
    pub same: usize,
    pub create: usize,
    pub update: usize,
    pub extra_remote: usize,
    pub delete: usize,
    pub blocked: usize,
    pub warning: usize,
    pub scope: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPlanSummary {
    pub resource_count: usize,
    pub checked: usize,
    pub same: usize,
    pub create: usize,
    pub update: usize,
    pub extra_remote: usize,
    pub delete: usize,
    pub blocked: usize,
    pub warning: usize,
    pub prune: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessPlanDocument {
    pub kind: String,
    pub schema_version: i64,
    pub tool_version: String,
    pub summary: AccessPlanSummary,
    pub resources: Vec<AccessPlanResourceReport>,
    pub actions: Vec<AccessPlanAction>,
}
