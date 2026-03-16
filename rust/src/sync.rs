//! Local/document-only sync CLI wrapper.
//!
//! Purpose:
//! - Expose staged Rust sync contracts through a minimal CLI namespace.
//! - Keep the Rust `sync` surface local-file-only until live wiring is deliberate.

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, Result};
use crate::sync_bundle_preflight::{
    build_sync_bundle_preflight_document, render_sync_bundle_preflight_text,
    SYNC_BUNDLE_PREFLIGHT_KIND,
};
use crate::sync_preflight::{
    build_sync_preflight_document, render_sync_preflight_text, SYNC_PREFLIGHT_KIND,
};
use crate::sync_workbench::{
    build_sync_apply_intent_document, build_sync_plan_document, build_sync_summary_document,
};

pub const DEFAULT_REVIEW_TOKEN: &str = "reviewed-sync-plan";

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SyncOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Parser)]
#[command(name = "grafana-util sync", about = "Local/document-only sync workflows.")]
pub struct SyncCliArgs {
    #[command(subcommand)]
    pub command: SyncGroupCommand,
}

#[derive(Debug, Clone, Args)]
pub struct SyncSummaryArgs {
    #[arg(long, help = "JSON file containing the desired sync resource list.")]
    pub desired_file: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the summary document as text or json."
    )]
    pub output: SyncOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct SyncPlanArgs {
    #[arg(long, help = "JSON file containing the desired sync resource list.")]
    pub desired_file: PathBuf,
    #[arg(long, help = "JSON file containing the live sync resource list.")]
    pub live_file: PathBuf,
    #[arg(
        long,
        default_value_t = false,
        help = "Mark live-only resources as would-delete instead of unmanaged."
    )]
    pub allow_prune: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the plan document as text or json."
    )]
    pub output: SyncOutputFormat,
    #[arg(
        long,
        help = "Optional stable trace id to carry through staged plan/review/apply files."
    )]
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct SyncReviewArgs {
    #[arg(long, help = "JSON file containing the staged sync plan document.")]
    pub plan_file: PathBuf,
    #[arg(
        long,
        default_value = DEFAULT_REVIEW_TOKEN,
        help = "Explicit review token required to mark the plan reviewed."
    )]
    pub review_token: String,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the reviewed plan document as text or json."
    )]
    pub output: SyncOutputFormat,
    #[arg(long, help = "Optional reviewer identity to record in the reviewed plan.")]
    pub reviewed_by: Option<String>,
    #[arg(long, help = "Optional staged reviewed-at value to record in the reviewed plan.")]
    pub reviewed_at: Option<String>,
    #[arg(long, help = "Optional review note to record in the reviewed plan.")]
    pub review_note: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct SyncApplyArgs {
    #[arg(long, help = "JSON file containing the reviewed sync plan document.")]
    pub plan_file: PathBuf,
    #[arg(
        long,
        help = "Optional JSON file containing a staged sync preflight document."
    )]
    pub preflight_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON file containing a staged sync bundle-preflight document."
    )]
    pub bundle_preflight_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Explicit acknowledgement required before a local apply intent is emitted."
    )]
    pub approve: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the apply intent document as text or json."
    )]
    pub output: SyncOutputFormat,
    #[arg(long, help = "Optional apply actor identity to record in the apply intent.")]
    pub applied_by: Option<String>,
    #[arg(long, help = "Optional staged applied-at value to record in the apply intent.")]
    pub applied_at: Option<String>,
    #[arg(long, help = "Optional approval reason to record in the apply intent.")]
    pub approval_reason: Option<String>,
    #[arg(long, help = "Optional apply note to record in the apply intent.")]
    pub apply_note: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct SyncPreflightArgs {
    #[arg(long, help = "JSON file containing the desired sync resource list.")]
    pub desired_file: PathBuf,
    #[arg(long, help = "Optional JSON object file containing staged availability hints.")]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the preflight document as text or json."
    )]
    pub output: SyncOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct SyncBundlePreflightArgs {
    #[arg(long, help = "JSON file containing the staged multi-resource source bundle.")]
    pub source_bundle: PathBuf,
    #[arg(long, help = "JSON file containing the staged target inventory snapshot.")]
    pub target_inventory: PathBuf,
    #[arg(long, help = "Optional JSON object file containing staged availability hints.")]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the bundle preflight document as text or json."
    )]
    pub output: SyncOutputFormat,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SyncGroupCommand {
    #[command(about = "Build a staged sync plan from local desired and live JSON files.")]
    Plan(SyncPlanArgs),
    #[command(about = "Mark a staged sync plan JSON document reviewed.")]
    Review(SyncReviewArgs),
    #[command(about = "Build a gated local apply intent from a reviewed sync plan.")]
    Apply(SyncApplyArgs),
    #[command(about = "Summarize local desired sync resources from JSON.")]
    Summary(SyncSummaryArgs),
    #[command(about = "Build a staged sync preflight document from local JSON.")]
    Preflight(SyncPreflightArgs),
    #[command(about = "Build a staged bundle-level sync preflight document from local JSON.")]
    BundlePreflight(SyncBundlePreflightArgs),
}

fn load_json_value(path: &Path, label: &str) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw)
        .map_err(|error| message(format!("Invalid JSON in {} {}: {error}", label, path.display())))
}

fn load_json_array_file(path: &Path, label: &str) -> Result<Vec<Value>> {
    let value = load_json_value(path, label)?;
    value.as_array()
        .cloned()
        .ok_or_else(|| message(format!("{label} file must contain a JSON array: {}", path.display())))
}

fn load_optional_json_object_file(path: Option<&PathBuf>, label: &str) -> Result<Option<Value>> {
    match path {
        None => Ok(None),
        Some(path) => {
            let value = load_json_value(path, label)?;
            if !value.is_object() {
                return Err(message(format!(
                    "{label} file must contain a JSON object: {}",
                    path.display()
                )));
            }
            Ok(Some(value))
        }
    }
}

fn fnv1a64_hex(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn normalize_trace_id(trace_id: Option<&str>) -> Option<String> {
    let normalized = trace_id.unwrap_or("").trim();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

fn derive_trace_id(document: &Value) -> Result<String> {
    let serialized = serde_json::to_string(document)?;
    Ok(format!("sync-trace-{}", fnv1a64_hex(&serialized)))
}

fn attach_trace_id(document: &Value, trace_id: Option<&str>) -> Result<Value> {
    let mut object = document
        .as_object()
        .cloned()
        .ok_or_else(|| message("Sync document must be a JSON object."))?;
    let resolved = match normalize_trace_id(trace_id) {
        Some(value) => value,
        None => derive_trace_id(document)?,
    };
    object.insert("traceId".to_string(), Value::String(resolved));
    Ok(Value::Object(object))
}

fn get_trace_id(document: &Value) -> Option<String> {
    normalize_trace_id(document.get("traceId").and_then(Value::as_str))
}

fn require_trace_id(document: &Value, label: &str) -> Result<String> {
    get_trace_id(document).ok_or_else(|| message(format!("{label} is missing traceId.")))
}

fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    let normalized = value.unwrap_or("").trim();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

fn deterministic_stage_marker(trace_id: &str, stage: &str) -> String {
    format!("staged:{trace_id}:{stage}")
}

fn attach_lineage(
    document: &Value,
    stage: &str,
    step_index: i64,
    parent_trace_id: Option<&str>,
) -> Result<Value> {
    let mut object = document
        .as_object()
        .cloned()
        .ok_or_else(|| message("Sync staged document must be a JSON object."))?;
    object.insert("stage".to_string(), Value::String(stage.to_string()));
    object.insert("stepIndex".to_string(), Value::Number(step_index.into()));
    if let Some(parent) = normalize_optional_text(parent_trace_id) {
        object.insert("parentTraceId".to_string(), Value::String(parent));
    } else {
        object.remove("parentTraceId");
    }
    Ok(Value::Object(object))
}

fn require_json_object<'a>(document: &'a Value, label: &str) -> Result<&'a Map<String, Value>> {
    document
        .as_object()
        .ok_or_else(|| message(format!("{label} must be a JSON object.")))
}

fn has_lineage_metadata(object: &Map<String, Value>) -> bool {
    object.contains_key("stage")
        || object.contains_key("stepIndex")
        || object.contains_key("parentTraceId")
}

fn require_optional_stage(
    document: &Value,
    label: &str,
    expected_stage: &str,
    expected_step_index: i64,
    expected_parent_trace_id: Option<&str>,
) -> Result<()> {
    let object = require_json_object(document, label)?;
    if !has_lineage_metadata(object) {
        return Ok(());
    }
    let stage = object
        .get("stage")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| message(format!("{label} is missing lineage stage metadata.")))?;
    if stage != expected_stage {
        return Err(message(format!(
            "{label} has unexpected lineage stage {stage:?}; expected {expected_stage:?}."
        )));
    }
    let step_index = object
        .get("stepIndex")
        .and_then(Value::as_i64)
        .ok_or_else(|| message(format!("{label} is missing lineage stepIndex metadata.")))?;
    if step_index != expected_step_index {
        return Err(message(format!(
            "{label} has unexpected lineage stepIndex {step_index}; expected {expected_step_index}."
        )));
    }
    match (
        normalize_optional_text(
            object.get("parentTraceId").and_then(Value::as_str),
        ),
        normalize_optional_text(expected_parent_trace_id),
    ) {
        (Some(actual), Some(expected)) if actual != expected => {
            Err(message(format!(
                "{label} has unexpected lineage parentTraceId {actual:?}; expected {expected:?}."
            )))
        }
        (Some(actual), None) => Err(message(format!(
            "{label} has unexpected lineage parentTraceId {actual:?}; expected no parent trace."
        ))),
        _ => Ok(()),
    }
}

fn require_matching_optional_trace_id(
    document: &Value,
    label: &str,
    expected_trace_id: &str,
) -> Result<()> {
    let object = require_json_object(document, label)?;
    if has_lineage_metadata(object) {
        object
            .get("stage")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| message(format!("{label} is missing lineage stage metadata.")))?;
        object
            .get("stepIndex")
            .and_then(Value::as_i64)
            .ok_or_else(|| message(format!("{label} is missing lineage stepIndex metadata.")))?;
    }
    let trace_id = match get_trace_id(document) {
        Some(value) => value,
        None if has_lineage_metadata(object) => {
            return Err(message(format!(
                "{label} is missing traceId for lineage-aware staged validation."
            )))
        }
        None => return Ok(()),
    };
    if trace_id != expected_trace_id {
        return Err(message(format!(
            "{label} traceId {trace_id:?} does not match sync plan traceId {expected_trace_id:?}."
        )));
    }
    if let Some(parent_trace_id) = normalize_optional_text(
        object.get("parentTraceId").and_then(Value::as_str),
    ) {
        if parent_trace_id != expected_trace_id {
            return Err(message(format!(
                "{label} parentTraceId {parent_trace_id:?} does not match sync plan traceId {expected_trace_id:?}."
            )));
        }
    }
    Ok(())
}

pub fn render_sync_summary_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-summary") {
        return Err(message("Sync summary document kind is not supported."));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Sync summary document is missing summary."))?;
    Ok(vec![
        "Sync summary".to_string(),
        format!(
            "Resources: {} total, {} dashboards, {} datasources, {} folders, {} alerts",
            summary.get("resourceCount").and_then(Value::as_i64).unwrap_or(0),
            summary.get("dashboardCount").and_then(Value::as_i64).unwrap_or(0),
            summary.get("datasourceCount").and_then(Value::as_i64).unwrap_or(0),
            summary.get("folderCount").and_then(Value::as_i64).unwrap_or(0),
            summary.get("alertCount").and_then(Value::as_i64).unwrap_or(0),
        ),
    ])
}

pub fn render_sync_plan_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-plan") {
        return Err(message("Sync plan document kind is not supported."));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Sync plan document is missing summary."))?;
    let mut lines = vec![
        "Sync plan".to_string(),
        format!(
            "Trace: {}",
            document
                .get("traceId")
                .and_then(Value::as_str)
                .unwrap_or("missing")
        ),
        format!(
            "Lineage: stage={} step={} parent={}",
            document.get("stage").and_then(Value::as_str).unwrap_or("missing"),
            document.get("stepIndex").and_then(Value::as_i64).unwrap_or(0),
            document
                .get("parentTraceId")
                .and_then(Value::as_str)
                .unwrap_or("none")
        ),
        format!(
            "Summary: create={} update={} delete={} noop={} unmanaged={}",
            summary.get("would_create").and_then(Value::as_i64).unwrap_or(0),
            summary.get("would_update").and_then(Value::as_i64).unwrap_or(0),
            summary.get("would_delete").and_then(Value::as_i64).unwrap_or(0),
            summary.get("noop").and_then(Value::as_i64).unwrap_or(0),
            summary.get("unmanaged").and_then(Value::as_i64).unwrap_or(0),
        ),
        format!(
            "Alerts: candidate={} plan-only={} blocked={}",
            summary
                .get("alert_candidate")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("alert_plan_only")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("alert_blocked")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
        format!(
            "Review: required={} reviewed={}",
            document
                .get("reviewRequired")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            document.get("reviewed").and_then(Value::as_bool).unwrap_or(false),
        ),
    ];
    if let Some(reviewed_by) = document.get("reviewedBy").and_then(Value::as_str) {
        lines.push(format!("Reviewed by: {reviewed_by}"));
    }
    if let Some(reviewed_at) = document.get("reviewedAt").and_then(Value::as_str) {
        lines.push(format!("Reviewed at: {reviewed_at}"));
    }
    if let Some(review_note) = document.get("reviewNote").and_then(Value::as_str) {
        lines.push(format!("Review note: {review_note}"));
    }
    Ok(lines)
}

pub fn render_sync_apply_intent_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-apply-intent") {
        return Err(message("Sync apply intent document kind is not supported."));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Sync apply intent document is missing summary."))?;
    let operations = document
        .get("operations")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| message("Sync apply intent document is missing operations."))?;
    let mut lines = vec![
        "Sync apply intent".to_string(),
        format!(
            "Trace: {}",
            document
                .get("traceId")
                .and_then(Value::as_str)
                .unwrap_or("missing")
        ),
        format!(
            "Lineage: stage={} step={} parent={}",
            document.get("stage").and_then(Value::as_str).unwrap_or("missing"),
            document.get("stepIndex").and_then(Value::as_i64).unwrap_or(0),
            document
                .get("parentTraceId")
                .and_then(Value::as_str)
                .unwrap_or("none")
        ),
        format!(
            "Summary: create={} update={} delete={} executable={}",
            summary.get("would_create").and_then(Value::as_i64).unwrap_or(0),
            summary.get("would_update").and_then(Value::as_i64).unwrap_or(0),
            summary.get("would_delete").and_then(Value::as_i64).unwrap_or(0),
            operations.len(),
        ),
        format!(
            "Review: required={} reviewed={} approved={}",
            document
                .get("reviewRequired")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            document.get("reviewed").and_then(Value::as_bool).unwrap_or(false),
            document.get("approved").and_then(Value::as_bool).unwrap_or(false),
        ),
    ];
    if let Some(preflight_summary) = document.get("preflightSummary").and_then(Value::as_object) {
        lines.push(format!(
            "Preflight: kind={} checks={} ok={} blocking={}",
            preflight_summary
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            preflight_summary
                .get("checkCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            preflight_summary
                .get("okCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            preflight_summary
                .get("blockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ));
    }
    if let Some(bundle_summary) = document
        .get("bundlePreflightSummary")
        .and_then(Value::as_object)
    {
        lines.push(format!(
            "Bundle preflight: resources={} sync-blocking={} provider-blocking={}",
            bundle_summary
                .get("resourceCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            bundle_summary
                .get("syncBlockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            bundle_summary
                .get("providerBlockingCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ));
    }
    if let Some(applied_by) = document.get("appliedBy").and_then(Value::as_str) {
        lines.push(format!("Applied by: {applied_by}"));
    }
    if let Some(applied_at) = document.get("appliedAt").and_then(Value::as_str) {
        lines.push(format!("Applied at: {applied_at}"));
    }
    if let Some(approval_reason) = document.get("approvalReason").and_then(Value::as_str) {
        lines.push(format!("Approval reason: {approval_reason}"));
    }
    if let Some(apply_note) = document.get("applyNote").and_then(Value::as_str) {
        lines.push(format!("Apply note: {apply_note}"));
    }
    Ok(lines)
}

fn mark_plan_reviewed(document: &Value, review_token: &str) -> Result<Value> {
    let mut object = document
        .as_object()
        .cloned()
        .ok_or_else(|| message("Sync plan document must be a JSON object."))?;
    if object.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-plan") {
        return Err(message("Sync plan document kind is not supported."));
    }
    if review_token.trim() != DEFAULT_REVIEW_TOKEN {
        return Err(message("Sync plan review token rejected."));
    }
    let trace_id = require_trace_id(document, "Sync plan document")?;
    object.insert("reviewed".to_string(), Value::Bool(true));
    object.insert("traceId".to_string(), Value::String(trace_id));
    Ok(Value::Object(object))
}

fn validate_apply_preflight(document: &Value) -> Result<Value> {
    require_json_object(document, "Sync preflight document")?;
    let object = document
        .as_object()
        .ok_or_else(|| message("Sync preflight document must be a JSON object."))?;
    let kind = object
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| message("Sync preflight document is missing kind."))?;
    let summary = object
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Sync preflight document is missing summary."))?;
    let mut bridged = Map::new();
    let blocking = match kind {
        SYNC_PREFLIGHT_KIND => {
            let check_count = summary
                .get("checkCount")
                .and_then(Value::as_i64)
                .ok_or_else(|| message("Sync preflight summary is missing checkCount."))?;
            let ok_count = summary
                .get("okCount")
                .and_then(Value::as_i64)
                .ok_or_else(|| message("Sync preflight summary is missing okCount."))?;
            let blocking_count = summary
                .get("blockingCount")
                .and_then(Value::as_i64)
                .ok_or_else(|| message("Sync preflight summary is missing blockingCount."))?;
            bridged.insert("kind".to_string(), Value::String(kind.to_string()));
            bridged.insert("checkCount".to_string(), Value::Number(check_count.into()));
            bridged.insert("okCount".to_string(), Value::Number(ok_count.into()));
            bridged.insert(
                "blockingCount".to_string(),
                Value::Number(blocking_count.into()),
            );
            blocking_count
        }
        SYNC_BUNDLE_PREFLIGHT_KIND => {
            return Err(message(
                "Sync bundle preflight document is not supported via --preflight-file; use --bundle-preflight-file.",
            ))
        }
        _ => return Err(message("Sync preflight document kind is not supported.")),
    };
    if blocking > 0 {
        return Err(message(format!(
            "Refusing local sync apply intent because preflight reports {blocking} blocking checks."
        )));
    }
    Ok(Value::Object(bridged))
}

fn validate_apply_bundle_preflight(document: &Value) -> Result<Value> {
    require_json_object(document, "Sync bundle preflight document")?;
    let object = document
        .as_object()
        .ok_or_else(|| message("Sync bundle preflight document must be a JSON object."))?;
    if object.get("kind").and_then(Value::as_str) != Some(SYNC_BUNDLE_PREFLIGHT_KIND) {
        return Err(message("Sync bundle preflight document kind is not supported."));
    }
    let summary = object
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Sync bundle preflight document is missing summary."))?;
    let resource_count = summary
        .get("resourceCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Sync bundle preflight summary is missing resourceCount."))?;
    let sync_blocking_count = summary
        .get("syncBlockingCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Sync bundle preflight summary is missing syncBlockingCount."))?;
    let provider_blocking_count = summary
        .get("providerBlockingCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Sync bundle preflight summary is missing providerBlockingCount."))?;
    let blocking_count = sync_blocking_count + provider_blocking_count;
    if blocking_count > 0 {
        return Err(message(format!(
            "Refusing local sync apply intent because bundle preflight reports {blocking_count} blocking checks."
        )));
    }
    Ok(serde_json::json!({
        "kind": SYNC_BUNDLE_PREFLIGHT_KIND,
        "resourceCount": resource_count,
        "checkCount": resource_count,
        "okCount": (resource_count - blocking_count).max(0),
        "blockingCount": blocking_count,
        "syncBlockingCount": sync_blocking_count,
        "providerBlockingCount": provider_blocking_count,
    }))
}

fn attach_preflight_summary(intent: &Value, preflight_summary: Option<Value>) -> Result<Value> {
    let mut object = intent
        .as_object()
        .cloned()
        .ok_or_else(|| message("Sync apply intent document must be a JSON object."))?;
    if let Some(summary) = preflight_summary {
        object.insert("preflightSummary".to_string(), summary);
    }
    Ok(Value::Object(object))
}

fn attach_bundle_preflight_summary(
    intent: &Value,
    bundle_preflight_summary: Option<Value>,
) -> Result<Value> {
    let mut object = intent
        .as_object()
        .cloned()
        .ok_or_else(|| message("Sync apply intent document must be a JSON object."))?;
    if let Some(summary) = bundle_preflight_summary {
        object.insert("bundlePreflightSummary".to_string(), summary);
    }
    Ok(Value::Object(object))
}

fn attach_review_audit(
    document: &Value,
    trace_id: &str,
    reviewed_by: Option<&str>,
    reviewed_at: Option<&str>,
    review_note: Option<&str>,
) -> Result<Value> {
    let mut object = document
        .as_object()
        .cloned()
        .ok_or_else(|| message("Sync reviewed plan document must be a JSON object."))?;
    if let Some(actor) = normalize_optional_text(reviewed_by) {
        object.insert("reviewedBy".to_string(), Value::String(actor));
    }
    object.insert(
        "reviewedAt".to_string(),
        Value::String(
            normalize_optional_text(reviewed_at)
                .unwrap_or_else(|| deterministic_stage_marker(trace_id, "reviewed")),
        ),
    );
    if let Some(note) = normalize_optional_text(review_note) {
        object.insert("reviewNote".to_string(), Value::String(note));
    }
    Ok(Value::Object(object))
}

fn attach_apply_audit(
    document: &Value,
    trace_id: &str,
    applied_by: Option<&str>,
    applied_at: Option<&str>,
    approval_reason: Option<&str>,
    apply_note: Option<&str>,
) -> Result<Value> {
    let mut object = document
        .as_object()
        .cloned()
        .ok_or_else(|| message("Sync apply intent document must be a JSON object."))?;
    if let Some(actor) = normalize_optional_text(applied_by) {
        object.insert("appliedBy".to_string(), Value::String(actor));
    }
    object.insert(
        "appliedAt".to_string(),
        Value::String(
            normalize_optional_text(applied_at)
                .unwrap_or_else(|| deterministic_stage_marker(trace_id, "applied")),
        ),
    );
    if let Some(reason) = normalize_optional_text(approval_reason) {
        object.insert("approvalReason".to_string(), Value::String(reason));
    }
    if let Some(note) = normalize_optional_text(apply_note) {
        object.insert("applyNote".to_string(), Value::String(note));
    }
    Ok(Value::Object(object))
}

fn emit_text_or_json(document: &Value, lines: Vec<String>, output: SyncOutputFormat) -> Result<()> {
    match output {
        SyncOutputFormat::Json => println!("{}", serde_json::to_string_pretty(document)?),
        SyncOutputFormat::Text => {
            for line in lines {
                println!("{line}");
            }
        }
    }
    Ok(())
}

pub fn run_sync_cli(command: SyncGroupCommand) -> Result<()> {
    match command {
        SyncGroupCommand::Plan(args) => {
            let desired = load_json_array_file(&args.desired_file, "Sync desired input")?;
            let live = load_json_array_file(&args.live_file, "Sync live input")?;
            let document = attach_lineage(
                &attach_trace_id(
                    &build_sync_plan_document(&desired, &live, args.allow_prune)?,
                    args.trace_id.as_deref(),
                )?,
                "plan",
                1,
                None,
            )?;
            emit_text_or_json(&document, render_sync_plan_text(&document)?, args.output)
        }
        SyncGroupCommand::Review(args) => {
            let plan = load_json_value(&args.plan_file, "Sync plan input")?;
            let trace_id = require_trace_id(&plan, "Sync plan document")?;
            require_optional_stage(&plan, "Sync plan document", "plan", 1, None)?;
            let document = attach_lineage(
                &attach_review_audit(
                &mark_plan_reviewed(&plan, &args.review_token)?,
                &trace_id,
                args.reviewed_by.as_deref(),
                args.reviewed_at.as_deref(),
                args.review_note.as_deref(),
            )?,
                "review",
                2,
                Some(&trace_id),
            )?;
            emit_text_or_json(&document, render_sync_plan_text(&document)?, args.output)
        }
        SyncGroupCommand::Apply(args) => {
            let plan = load_json_value(&args.plan_file, "Sync plan input")?;
            let trace_id = require_trace_id(&plan, "Sync plan document")?;
            require_optional_stage(&plan, "Sync plan document", "review", 2, Some(&trace_id))?;
            let preflight_summary = match args.preflight_file.as_ref() {
                None => None,
                Some(path) => {
                    let preflight = load_json_value(path, "Sync preflight input")?;
                    require_matching_optional_trace_id(
                        &preflight,
                        "Sync preflight document",
                        &trace_id,
                    )?;
                    Some(validate_apply_preflight(&preflight)?)
                }
            };
            let bundle_preflight_summary = match args.bundle_preflight_file.as_ref() {
                None => None,
                Some(path) => {
                    let bundle_preflight = load_json_value(path, "Sync bundle preflight input")?;
                    require_matching_optional_trace_id(
                        &bundle_preflight,
                        "Sync bundle preflight document",
                        &trace_id,
                    )?;
                    Some(validate_apply_bundle_preflight(&bundle_preflight)?)
                }
            };
            let document = attach_lineage(
                &attach_trace_id(
                    &attach_apply_audit(
                        &attach_bundle_preflight_summary(
                            &attach_preflight_summary(
                                &build_sync_apply_intent_document(&plan, args.approve)?,
                                preflight_summary,
                            )?,
                            bundle_preflight_summary,
                        )?,
                        &trace_id,
                        args.applied_by.as_deref(),
                        args.applied_at.as_deref(),
                        args.approval_reason.as_deref(),
                    args.apply_note.as_deref(),
                )?,
                Some(&trace_id),
            )?,
                "apply",
                3,
                Some(&trace_id),
            )?;
            emit_text_or_json(
                &document,
                render_sync_apply_intent_text(&document)?,
                args.output,
            )
        }
        SyncGroupCommand::Summary(args) => {
            let desired = load_json_array_file(&args.desired_file, "Sync desired input")?;
            let document = build_sync_summary_document(&desired)?;
            emit_text_or_json(&document, render_sync_summary_text(&document)?, args.output)
        }
        SyncGroupCommand::Preflight(args) => {
            let desired = load_json_array_file(&args.desired_file, "Sync desired input")?;
            let availability = load_optional_json_object_file(
                args.availability_file.as_ref(),
                "Sync availability input",
            )?;
            let document = build_sync_preflight_document(&desired, availability.as_ref())?;
            emit_text_or_json(
                &document,
                render_sync_preflight_text(&document)?,
                args.output,
            )
        }
        SyncGroupCommand::BundlePreflight(args) => {
            let source_bundle = load_json_value(&args.source_bundle, "Sync source bundle input")?;
            let target_inventory =
                load_json_value(&args.target_inventory, "Sync target inventory input")?;
            let availability = load_optional_json_object_file(
                args.availability_file.as_ref(),
                "Sync availability input",
            )?;
            let document = build_sync_bundle_preflight_document(
                &source_bundle,
                &target_inventory,
                availability.as_ref(),
            )?;
            emit_text_or_json(
                &document,
                render_sync_bundle_preflight_text(&document)?,
                args.output,
            )
        }
    }
}

#[cfg(test)]
#[path = "sync_cli_rust_tests.rs"]
mod sync_cli_rust_tests;
