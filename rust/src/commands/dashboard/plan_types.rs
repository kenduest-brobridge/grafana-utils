//! Shared data types for dashboard reconcile plans.

use serde::Serialize;
use serde_json::{Map, Value};
use std::path::PathBuf;

use super::FolderInventoryItem;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DashboardPlanChange {
    pub(super) field: String,
    pub(super) before: Value,
    pub(super) after: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DashboardPlanAction {
    pub(super) action_id: String,
    pub(super) domain: String,
    pub(super) resource_kind: String,
    pub(super) dashboard_uid: String,
    pub(super) title: String,
    pub(super) folder_uid: String,
    pub(super) folder_path: String,
    pub(super) source_org_id: Option<String>,
    pub(super) source_org_name: String,
    pub(super) target_org_id: Option<String>,
    pub(super) target_org_name: String,
    pub(super) match_basis: String,
    pub(super) action: String,
    pub(super) status: String,
    pub(super) changed_fields: Vec<String>,
    pub(super) changes: Vec<DashboardPlanChange>,
    pub(super) source_file: Option<String>,
    pub(super) target_uid: Option<String>,
    pub(super) target_version: Option<i64>,
    pub(super) target_evidence: Vec<String>,
    pub(super) dependency_hints: Vec<String>,
    pub(super) blocked_reason: Option<String>,
    pub(super) review_hints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DashboardPlanOrgSummary {
    pub(super) source_org_id: Option<String>,
    pub(super) source_org_name: String,
    pub(super) target_org_id: Option<String>,
    pub(super) target_org_name: String,
    pub(super) org_action: String,
    pub(super) input_dir: String,
    pub(super) checked: usize,
    pub(super) same: usize,
    pub(super) create: usize,
    pub(super) update: usize,
    pub(super) extra: usize,
    pub(super) delete: usize,
    pub(super) blocked: usize,
    pub(super) warning: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DashboardPlanSummary {
    pub(super) checked: usize,
    pub(super) same: usize,
    pub(super) create: usize,
    pub(super) update: usize,
    pub(super) extra: usize,
    pub(super) delete: usize,
    pub(super) blocked: usize,
    pub(super) warning: usize,
    pub(super) org_count: usize,
    pub(super) would_create_org_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DashboardPlanReport {
    pub(super) kind: String,
    #[serde(rename = "schemaVersion")]
    pub(super) schema_version: i64,
    pub(super) tool_version: String,
    pub(super) mode: String,
    pub(super) scope: String,
    pub(super) input_type: String,
    pub(super) prune: bool,
    pub(super) summary: DashboardPlanSummary,
    pub(super) orgs: Vec<DashboardPlanOrgSummary>,
    pub(super) actions: Vec<DashboardPlanAction>,
}

#[derive(Debug, Clone)]
pub(super) struct LocalDashboard {
    pub(super) file_path: String,
    pub(super) dashboard: Value,
    pub(super) dashboard_uid: String,
    pub(super) title: String,
    pub(super) folder_uid: String,
    pub(super) folder_path: String,
}

#[derive(Debug, Clone)]
pub(super) struct LiveDashboard {
    pub(super) uid: String,
    pub(super) title: String,
    pub(super) folder_uid: String,
    pub(super) folder_path: String,
    pub(super) version: Option<i64>,
    pub(super) evidence: Vec<String>,
    pub(super) payload: Value,
}

pub(super) type PlanLiveState = (Vec<Map<String, Value>>, Vec<LiveDashboard>);

#[derive(Debug, Clone)]
pub(super) struct OrgPlanInput {
    pub(super) source_org_id: Option<String>,
    pub(super) source_org_name: String,
    pub(super) target_org_id: Option<String>,
    pub(super) target_org_name: String,
    pub(super) org_action: String,
    pub(super) input_dir: PathBuf,
    pub(super) local_dashboards: Vec<LocalDashboard>,
    pub(super) live_dashboards: Vec<LiveDashboard>,
    pub(super) live_datasources: Vec<Map<String, Value>>,
    pub(super) folder_inventory: Vec<FolderInventoryItem>,
}

#[derive(Debug, Clone)]
pub(super) struct DashboardPlanInput {
    pub(super) scope: String,
    pub(super) input_type: String,
    pub(super) prune: bool,
    pub(super) orgs: Vec<OrgPlanInput>,
}
