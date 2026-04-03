//! Sync workbench façade.
//!
//! Maintainer note:
//! - Keep the public sync contract types and entrypoints here.
//! - Push builder-heavy implementation details into sibling modules so this
//!   file stays focused on orchestration and stable re-exports.

use serde::Serialize;
use serde_json::{Map, Value};

use super::{apply_builder, bundle_builder, plan_builder, summary_builder};
use crate::common::Result;

pub const SYNC_SUMMARY_KIND: &str = "grafana-utils-sync-summary";
pub const SYNC_SUMMARY_SCHEMA_VERSION: i64 = 1;
pub const SYNC_SOURCE_BUNDLE_KIND: &str = "grafana-utils-sync-source-bundle";
pub const SYNC_SOURCE_BUNDLE_SCHEMA_VERSION: i64 = 1;
pub const SYNC_PLAN_KIND: &str = "grafana-utils-sync-plan";
pub const SYNC_PLAN_SCHEMA_VERSION: i64 = 1;
pub const SYNC_APPLY_INTENT_KIND: &str = "grafana-utils-sync-apply-intent";
pub const SYNC_APPLY_INTENT_SCHEMA_VERSION: i64 = 1;
pub const RESOURCE_KINDS: &[&str] = &[
    "dashboard",
    "datasource",
    "folder",
    "alert",
    "alert-contact-point",
    "alert-mute-timing",
    "alert-policy",
    "alert-template",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncResourceSpec {
    pub kind: String,
    pub identity: String,
    pub title: String,
    pub body: Map<String, Value>,
    pub managed_fields: Vec<String>,
    pub source_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncSummary {
    pub resource_count: usize,
    pub dashboard_count: usize,
    pub datasource_count: usize,
    pub folder_count: usize,
    pub alert_count: usize,
}

pub fn normalize_resource_spec(raw_spec: &Value) -> Result<SyncResourceSpec> {
    summary_builder::normalize_resource_spec(raw_spec)
}

pub fn normalize_resource_specs(raw_specs: &[Value]) -> Result<Vec<SyncResourceSpec>> {
    summary_builder::normalize_resource_specs(raw_specs)
}

pub fn summarize_resource_specs(specs: &[SyncResourceSpec]) -> SyncSummary {
    summary_builder::summarize_resource_specs(specs)
}

pub fn build_sync_summary_document(raw_specs: &[Value]) -> Result<Value> {
    summary_builder::build_sync_summary_document(raw_specs)
}

pub fn build_sync_source_bundle_document(
    dashboards: &[Value],
    datasources: &[Value],
    folders: &[Value],
    alerts: &[Value],
    alerting: Option<&Value>,
    metadata: Option<&Value>,
) -> Result<Value> {
    bundle_builder::build_sync_source_bundle_document(
        dashboards,
        datasources,
        folders,
        alerts,
        alerting,
        metadata,
    )
}

pub fn render_sync_source_bundle_text(document: &Value) -> Result<Vec<String>> {
    bundle_builder::render_sync_source_bundle_text(document)
}

pub fn build_sync_plan_document(
    desired_specs: &[Value],
    live_specs: &[Value],
    allow_prune: bool,
) -> Result<Value> {
    plan_builder::build_sync_plan_document(desired_specs, live_specs, allow_prune)
}

pub fn build_sync_apply_intent_document(plan_document: &Value, approve: bool) -> Result<Value> {
    apply_builder::build_sync_apply_intent_document(plan_document, approve)
}
