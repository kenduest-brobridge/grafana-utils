//! Local/document-only sync CLI wrapper.
//!
//! Purpose:
//! - Expose staged Rust sync contracts through a minimal CLI namespace.
//! - Keep the Rust `sync` surface local-file-first with optional live fetch.

use crate::alert_sync::assess_alert_sync_specs;
use crate::dashboard::{build_http_client, build_http_client_for_org, CommonCliArgs};
use crate::dashboard::{fetch_dashboard, list_dashboard_summaries, list_datasources};
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Method;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, string_field, value_as_object, Result};
use crate::sync_bundle_preflight::{
    build_sync_bundle_preflight_document, render_sync_bundle_preflight_text,
    SYNC_BUNDLE_PREFLIGHT_KIND,
};
use crate::sync_contracts::{
    build_sync_apply_intent_document, build_sync_plan_document, build_sync_summary_document,
};
use crate::sync_preflight::{
    build_sync_preflight_document, render_sync_preflight_text, SYNC_PREFLIGHT_KIND,
};

pub const DEFAULT_REVIEW_TOKEN: &str = "reviewed-sync-plan";

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq, Default)]
pub enum SyncOutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util sync",
    about = "Local/document-only sync workflows."
)]
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
        help_heading = "Output Options",
        help = "Render the summary document as text or json."
    )]
    pub output: SyncOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct SyncPlanArgs {
    #[arg(long, help = "JSON file containing the desired sync resource list.")]
    pub desired_file: PathBuf,
    #[arg(long, help = "JSON file containing the live sync resource list.")]
    pub live_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Read the live state directly from Grafana instead of --live-file."
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = 500,
        help = "Dashboard search page size when --fetch-live is active."
    )]
    pub page_size: usize,
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
        help_heading = "Output Options",
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
        help_heading = "Output Options",
        help = "Render the reviewed plan document as text or json."
    )]
    pub output: SyncOutputFormat,
    #[arg(
        long,
        help = "Optional reviewer identity to record in the reviewed plan."
    )]
    pub reviewed_by: Option<String>,
    #[arg(
        long,
        help = "Optional staged reviewed-at value to record in the reviewed plan."
    )]
    pub reviewed_at: Option<String>,
    #[arg(long, help = "Optional review note to record in the reviewed plan.")]
    pub review_note: Option<String>,
}

#[derive(Debug, Clone, Args, Default)]
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
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --execute-live is active."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        help = "Apply supported sync operations to Grafana after review and approval checks pass."
    )]
    pub execute_live: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Continue applying later operations if a prior live apply operation fails."
    )]
    pub continue_on_error: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Allow live deletion of folders when a reviewed plan includes would-delete folder operations."
    )]
    pub allow_folder_delete: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help_heading = "Output Options",
        help = "Render the apply intent document as text or json."
    )]
    pub output: SyncOutputFormat,
    #[arg(
        long,
        help = "Optional apply actor identity to record in the apply intent."
    )]
    pub applied_by: Option<String>,
    #[arg(
        long,
        help = "Optional staged applied-at value to record in the apply intent."
    )]
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
    #[arg(
        long,
        help = "Optional JSON object file containing staged availability hints."
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Fetch live availability hints from Grafana instead of relying only on --availability-file."
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help = "Optional stable trace id to carry through staged preflight files."
    )]
    pub trace_id: Option<String>,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help_heading = "Output Options",
        help = "Render the preflight document as text or json."
    )]
    pub output: SyncOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct SyncAssessAlertsArgs {
    #[arg(long, help = "JSON file containing alert sync resource list.")]
    pub alerts_file: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help_heading = "Output Options",
        help = "Render the alert assessment document as text or json."
    )]
    pub output: SyncOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct SyncBundlePreflightArgs {
    #[arg(
        long,
        help = "JSON file containing the staged multi-resource source bundle."
    )]
    pub source_bundle: PathBuf,
    #[arg(
        long,
        help = "JSON file containing the staged target inventory snapshot."
    )]
    pub target_inventory: PathBuf,
    #[arg(
        long,
        help = "Optional JSON object file containing staged availability hints."
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Fetch live availability hints from Grafana instead of relying only on --availability-file."
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help = "Optional stable trace id to carry through staged bundle-preflight files."
    )]
    pub trace_id: Option<String>,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help_heading = "Output Options",
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
    #[command(about = "Assess alert sync specs and mark candidate/plan-only/blocked states.")]
    AssessAlerts(SyncAssessAlertsArgs),
    #[command(about = "Build a staged bundle-level sync preflight document from local JSON.")]
    BundlePreflight(SyncBundlePreflightArgs),
}

fn load_json_value(path: &Path, label: &str) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(|error| {
        message(format!(
            "Invalid JSON in {} {}: {error}",
            label,
            path.display()
        ))
    })
}

fn load_json_array_file(path: &Path, label: &str) -> Result<Vec<Value>> {
    let value = load_json_value(path, label)?;
    value.as_array().cloned().ok_or_else(|| {
        message(format!(
            "{label} file must contain a JSON array: {}",
            path.display()
        ))
    })
}

fn fetch_live_client(
    common: &CommonCliArgs,
    org_id: Option<i64>,
) -> Result<crate::http::JsonHttpClient> {
    match org_id {
        Some(org_id) => build_http_client_for_org(common, org_id),
        None => build_http_client(common),
    }
}

fn merge_availability_field(target: &mut Vec<String>, values: &[String]) {
    let mut existing = target
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<String>>();
    for value in values {
        if value.is_empty() {
            continue;
        }
        if existing.insert(value.clone()) {
            target.push(value.clone());
        }
    }
}

fn resolve_availability(
    availability_file: Option<&PathBuf>,
    common: &CommonCliArgs,
    org_id: Option<i64>,
    fetch_live: bool,
) -> Result<Option<Value>> {
    let mut availability = match availability_file {
        None => build_empty_availability_object(),
        Some(path) => {
            let document = load_json_value(path, "Sync availability input")?;
            if !document.is_object() {
                return Err(message(format!(
                    "Sync availability input file must contain a JSON object: {}",
                    path.display()
                )));
            }
            document
        }
    };
    if fetch_live {
        availability = merge_availability(availability, fetch_live_availability(common, org_id)?)?;
    }
    Ok(Some(availability))
}

fn resolve_live_specs(args: &SyncPlanArgs) -> Result<Vec<Value>> {
    if args.fetch_live {
        return fetch_live_resource_specs(&args.common, args.org_id, args.page_size);
    }
    match &args.live_file {
        None => Err(message(
            "Sync plan requires --live-file unless --fetch-live is used.",
        )),
        Some(path) => load_json_array_file(path, "Sync live input"),
    }
}

fn merge_availability(base: Value, extra: Value) -> Result<Value> {
    let mut base_map = value_as_object(&base, "availability")?
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Map<String, Value>>();
    for key in ["datasourceUids", "pluginIds", "contactPoints"] {
        let mut existing = base_map
            .get(key)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();
        let extra_values = extra
            .get(key)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();
        merge_availability_field(&mut existing, &extra_values);
        base_map.insert(
            key.to_string(),
            Value::Array(existing.into_iter().map(Value::String).collect()),
        );
    }
    for (key, value) in extra
        .as_object()
        .ok_or_else(|| message("Live availability must be a JSON object."))?
    {
        if matches!(
            key.as_str(),
            "datasourceUids" | "pluginIds" | "contactPoints"
        ) {
            continue;
        }
        base_map.insert(key.clone(), value.clone());
    }
    Ok(Value::Object(base_map))
}

fn build_empty_availability_object() -> Value {
    Value::Object(Map::from_iter(vec![
        (
            "datasourceUids".to_string(),
            Value::Array(Vec::<Value>::new()),
        ),
        ("pluginIds".to_string(), Value::Array(Vec::<Value>::new())),
        (
            "contactPoints".to_string(),
            Value::Array(Vec::<Value>::new()),
        ),
    ]))
}

fn fetch_live_availability(common: &CommonCliArgs, org_id: Option<i64>) -> Result<Value> {
    let client = fetch_live_client(common, org_id)?;
    let mut availability = build_empty_availability_object();
    let datasources = list_datasources(&client)?;
    let mut datasource_uids = Vec::new();
    for datasource in datasources {
        let uid = string_field(&datasource, "uid", "");
        if !uid.is_empty() {
            datasource_uids.push(Value::String(uid));
        }
    }
    let plugins_response = client.request_json(Method::GET, "/api/plugins", &[], None)?;
    let mut plugin_ids = Vec::new();
    let plugin_list = plugins_response
        .as_ref()
        .and_then(Value::as_array)
        .ok_or_else(|| message("Unexpected plugin list response from Grafana."))?;
    for plugin in plugin_list {
        if let Some(object) = plugin.as_object() {
            let plugin_id = string_field(object, "id", "");
            if !plugin_id.is_empty() {
                plugin_ids.push(Value::String(plugin_id));
            }
        }
    }
    let contact_points_response = client.request_json(
        Method::GET,
        "/api/v1/provisioning/contact-points",
        &[],
        None,
    )?;
    let mut contact_points = Vec::new();
    let contact_point_list = contact_points_response
        .as_ref()
        .and_then(Value::as_array)
        .ok_or_else(|| message("Unexpected contact point list response from Grafana."))?;
    for contact_point in contact_point_list {
        if let Some(object) = contact_point.as_object() {
            let name = string_field(object, "name", "");
            let uid = string_field(object, "uid", "");
            let contact_point = if name.is_empty() { uid } else { name };
            if !contact_point.is_empty() {
                contact_points.push(Value::String(contact_point));
            }
        }
    }
    availability["datasourceUids"] = Value::Array(datasource_uids);
    availability["pluginIds"] = Value::Array(plugin_ids);
    availability["contactPoints"] = Value::Array(contact_points);
    Ok(availability)
}

fn fetch_live_resource_specs(
    common: &CommonCliArgs,
    org_id: Option<i64>,
    page_size: usize,
) -> Result<Vec<Value>> {
    let client = fetch_live_client(common, org_id)?;
    let mut specs = Vec::new();
    let folder_response = client.request_json(Method::GET, "/api/folders", &[], None)?;
    let folders = folder_response
        .as_ref()
        .and_then(Value::as_array)
        .ok_or_else(|| message("Unexpected folder list response from Grafana."))?;
    for folder in folders {
        if let Some(folder) = folder.as_object() {
            let uid = string_field(folder, "uid", "");
            if uid.is_empty() {
                continue;
            }
            let title = string_field(folder, "title", &uid);
            let mut body = Map::new();
            body.insert("title".to_string(), Value::String(title.clone()));
            let parent_uid = string_field(folder, "parentUid", "");
            if !parent_uid.is_empty() {
                body.insert("parentUid".to_string(), Value::String(parent_uid));
            }
            specs.push(Value::Object(Map::from_iter(vec![
                ("kind".to_string(), Value::String("folder".to_string())),
                ("uid".to_string(), Value::String(uid)),
                ("title".to_string(), Value::String(title)),
                ("body".to_string(), Value::Object(body)),
            ])));
        }
    }

    let dashboards = list_dashboard_summaries(&client, page_size)?;
    for dashboard in dashboards {
        let uid = string_field(&dashboard, "uid", "");
        if uid.is_empty() {
            continue;
        }
        let dashboard_wrapper = fetch_dashboard(&client, &uid)?;
        let object = value_as_object(
            &dashboard_wrapper,
            "Unexpected dashboard payload from Grafana.",
        )?;
        let dashboard_body = object
            .get("dashboard")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                message(format!(
                    "Grafana dashboard payload for uid {uid} does not include dashboard field."
                ))
            })?
            .clone();
        let mut body = dashboard_body.clone();
        body.remove("id");
        let title = string_field(&body, "title", &uid);
        specs.push(Value::Object(Map::from_iter(vec![
            ("kind".to_string(), Value::String("dashboard".to_string())),
            ("uid".to_string(), Value::String(uid)),
            ("title".to_string(), Value::String(title)),
            ("body".to_string(), Value::Object(body)),
        ])));
    }

    for datasource in list_datasources(&client)? {
        let uid = string_field(&datasource, "uid", "");
        let name = string_field(&datasource, "name", "");
        let identity = if !uid.is_empty() {
            uid.clone()
        } else if !name.is_empty() {
            name.clone()
        } else {
            continue;
        };
        let mut body = Map::new();
        let body_type = string_field(&datasource, "type", "");
        let access = string_field(&datasource, "access", "");
        let url = string_field(&datasource, "url", "");
        body.insert("name".to_string(), Value::String(name.clone()));
        if !body_type.is_empty() {
            body.insert("type".to_string(), Value::String(body_type));
        }
        if !access.is_empty() {
            body.insert("access".to_string(), Value::String(access));
        }
        if !url.is_empty() {
            body.insert("url".to_string(), Value::String(url));
        }
        body.insert(
            "isDefault".to_string(),
            Value::Bool(
                datasource
                    .get("isDefault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        );
        if let Some(json_data) = datasource.get("jsonData").and_then(Value::as_object) {
            if !json_data.is_empty() {
                body.insert("jsonData".to_string(), Value::Object(json_data.clone()));
            }
        }
        specs.push(Value::Object(Map::from_iter(vec![
            ("kind".to_string(), Value::String("datasource".to_string())),
            ("uid".to_string(), Value::String(identity.clone())),
            ("name".to_string(), Value::String(name.clone())),
            (
                "title".to_string(),
                Value::String(if name.is_empty() {
                    identity.clone()
                } else {
                    name.clone()
                }),
            ),
            ("body".to_string(), Value::Object(body)),
        ])));
    }
    Ok(specs)
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
        normalize_optional_text(object.get("parentTraceId").and_then(Value::as_str)),
        normalize_optional_text(expected_parent_trace_id),
    ) {
        (Some(actual), Some(expected)) if actual != expected => Err(message(format!(
            "{label} has unexpected lineage parentTraceId {actual:?}; expected {expected:?}."
        ))),
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
    if let Some(parent_trace_id) =
        normalize_optional_text(object.get("parentTraceId").and_then(Value::as_str))
    {
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
            summary
                .get("resourceCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("dashboardCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("datasourceCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("folderCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("alertCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
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
            document
                .get("stage")
                .and_then(Value::as_str)
                .unwrap_or("missing"),
            document
                .get("stepIndex")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            document
                .get("parentTraceId")
                .and_then(Value::as_str)
                .unwrap_or("none")
        ),
        format!(
            "Summary: create={} update={} delete={} noop={} unmanaged={}",
            summary
                .get("would_create")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("would_update")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("would_delete")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary.get("noop").and_then(Value::as_i64).unwrap_or(0),
            summary
                .get("unmanaged")
                .and_then(Value::as_i64)
                .unwrap_or(0),
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
            document
                .get("reviewed")
                .and_then(Value::as_bool)
                .unwrap_or(false),
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
            document
                .get("stage")
                .and_then(Value::as_str)
                .unwrap_or("missing"),
            document
                .get("stepIndex")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            document
                .get("parentTraceId")
                .and_then(Value::as_str)
                .unwrap_or("none")
        ),
        format!(
            "Summary: create={} update={} delete={} executable={}",
            summary
                .get("would_create")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("would_update")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("would_delete")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            operations.len(),
        ),
        format!(
            "Review: required={} reviewed={} approved={}",
            document
                .get("reviewRequired")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            document
                .get("reviewed")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            document
                .get("approved")
                .and_then(Value::as_bool)
                .unwrap_or(false),
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

fn render_sync_assess_alerts_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some("grafana-utils-alert-sync-plan") {
        return Err(message("Alert assessment document kind is not supported."));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Alert assessment document is missing summary."))?;
    let mut lines = vec![
        "Alert sync assessment".to_string(),
        format!(
            "Alerts: total={} candidate={} plan-only={} blocked={}",
            summary
                .get("alertCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("candidateCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("planOnlyCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            summary
                .get("blockedCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
        ),
    ];
    let alerts = document
        .get("alerts")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Alert assessment document is missing alerts list."))?;
    if alerts.is_empty() {
        lines.push("No alerts".to_string());
        return Ok(lines);
    }
    for alert in alerts {
        let alert_object = value_as_object(alert, "Alert assessment item")?;
        let identity = alert_object
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or("missing");
        let status = alert_object
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let managed_fields = alert_object
            .get("managedFields")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<&str>>()
                    .join(", ")
            })
            .unwrap_or_default();
        lines.push(format!(
            "{identity}: {status} [{}]",
            if managed_fields.is_empty() {
                "none"
            } else {
                managed_fields.as_str()
            }
        ));
        if let Some(detail) = alert_object.get("detail").and_then(Value::as_str) {
            lines.push(format!("  detail: {detail}"));
        }
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
        return Err(message(
            "Sync bundle preflight document kind is not supported.",
        ));
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
        .ok_or_else(|| {
            message("Sync bundle preflight summary is missing providerBlockingCount.")
        })?;
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

fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'.'
                    | b'_'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b':'
                    | b'@'
            )
        {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn response_or_null(response: Option<Value>) -> Value {
    response.unwrap_or(Value::Null)
}

fn parse_operation_field(operation: &Map<String, Value>, field: &str) -> Result<String> {
    operation
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| message(format!("Sync apply operation is missing {field}.")))
}

fn parse_optional_operation_field(
    operation: &Map<String, Value>,
    field: &str,
    fallback: &str,
) -> String {
    operation
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn parse_operation_body(operation: &Map<String, Value>, label: &str) -> Result<Map<String, Value>> {
    operation
        .get("desired")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message(format!("{label} must be a JSON object.")))
}

struct SyncApplyOperation {
    kind: String,
    identity: String,
    title: String,
    action: String,
    body: Map<String, Value>,
}

fn parse_sync_apply_operation(operation: &Value) -> Result<SyncApplyOperation> {
    let operation = value_as_object(operation, "Sync apply operation")?;
    let kind = parse_operation_field(operation, "kind")?.to_lowercase();
    let identity = parse_operation_field(operation, "identity")?;
    let action = parse_operation_field(operation, "action")?;
    let title = parse_optional_operation_field(operation, "title", &identity);
    let body = parse_operation_body(operation, "Sync apply desired body")?;
    Ok(SyncApplyOperation {
        kind,
        identity,
        title,
        action,
        body,
    })
}

fn extract_datasource_target_id(datasource: &Map<String, Value>) -> Option<String> {
    datasource
        .get("id")
        .and_then(Value::as_i64)
        .map(|value| value.to_string())
        .or_else(|| {
            datasource
                .get("id")
                .and_then(Value::as_u64)
                .map(|value| value.to_string())
        })
        .or_else(|| {
            datasource
                .get("id")
                .and_then(Value::as_f64)
                .map(|value| value as i64)
                .map(|value| value.to_string())
        })
        .or_else(|| {
            datasource
                .get("id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
}

fn resolve_datasource_target<'a>(
    live_datasources: &'a [Map<String, Value>],
    identity: &str,
) -> Result<&'a Map<String, Value>> {
    let mut uid_count = 0usize;
    let mut uid_match: Option<&Map<String, Value>> = None;
    let mut name_count = 0usize;
    let mut name_match: Option<&Map<String, Value>> = None;
    for datasource in live_datasources {
        if string_field(datasource, "uid", "") == identity {
            uid_count += 1;
            uid_match = Some(datasource);
        }
        if string_field(datasource, "name", "") == identity {
            name_count += 1;
            name_match = Some(datasource);
        }
    }
    if uid_count > 1 {
        return Err(message(format!(
            "Could not resolve live datasource target {identity}: ambiguous uid."
        )));
    }
    if let Some(target) = uid_match {
        return Ok(target);
    }
    if name_count > 1 {
        return Err(message(format!(
            "Could not resolve live datasource target {identity}: ambiguous name."
        )));
    }
    name_match.ok_or_else(|| {
        message(format!(
            "Could not resolve live datasource target {identity}."
        ))
    })
}

fn build_datasource_add_payload(operation: &SyncApplyOperation) -> Result<Map<String, Value>> {
    let name = parse_optional_operation_field(&operation.body, "name", &operation.identity);
    let datasource_type = operation
        .body
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if name.trim().is_empty() {
        return Err(message(format!(
            "Datasource sync add requires desired name for {}.",
            operation.identity
        )));
    }
    if datasource_type.trim().is_empty() {
        return Err(message(format!(
            "Datasource sync add requires desired type for {}.",
            operation.identity
        )));
    }
    let mut payload = Map::from_iter(vec![
        ("name".to_string(), Value::String(name)),
        ("type".to_string(), Value::String(datasource_type)),
    ]);
    for field in [
        "uid",
        "access",
        "url",
        "isDefault",
        "jsonData",
        "secureJsonData",
    ] {
        if let Some(value) = operation.body.get(field) {
            payload.insert(field.to_string(), value.clone());
        }
    }
    Ok(payload)
}

fn build_datasource_modify_payload(
    target: &Map<String, Value>,
    desired: &Map<String, Value>,
) -> Map<String, Value> {
    let mut payload = Map::from_iter(vec![
        (
            "id".to_string(),
            target.get("id").cloned().unwrap_or(Value::Null),
        ),
        (
            "uid".to_string(),
            Value::String(string_field(target, "uid", "")),
        ),
        (
            "name".to_string(),
            Value::String(string_field(target, "name", "")),
        ),
        (
            "type".to_string(),
            Value::String(string_field(target, "type", "")),
        ),
        (
            "access".to_string(),
            Value::String(string_field(target, "access", "")),
        ),
        (
            "url".to_string(),
            Value::String(string_field(target, "url", "")),
        ),
        (
            "isDefault".to_string(),
            Value::Bool(
                target
                    .get("isDefault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        ),
    ]);
    for field in [
        "orgId",
        "basicAuth",
        "basicAuthUser",
        "user",
        "database",
        "withCredentials",
    ] {
        if let Some(value) = target.get(field) {
            payload.insert((*field).to_string(), value.clone());
        }
    }
    if let Some(value) = target.get("jsonData").or_else(|| desired.get("jsonData")) {
        payload.insert("jsonData".to_string(), value.clone());
    }
    for field in [
        "url",
        "access",
        "isDefault",
        "basicAuth",
        "basicAuthUser",
        "user",
        "withCredentials",
    ] {
        if let Some(value) = desired.get(field) {
            payload.insert(field.to_string(), value.clone());
        }
    }
    if let Some(value) = desired.get("jsonData") {
        if let Some(desired_json) = value.as_object() {
            let merged_json_data = payload
                .get("jsonData")
                .and_then(Value::as_object)
                .unwrap_or(&Map::new())
                .iter()
                .chain(desired_json.iter())
                .fold(Map::new(), |mut merged, (key, value)| {
                    merged.insert(key.clone(), value.clone());
                    merged
                });
            payload.insert("jsonData".to_string(), Value::Object(merged_json_data));
        } else {
            payload.insert("jsonData".to_string(), value.clone());
        }
    }
    if let Some(value) = desired.get("secureJsonData") {
        payload.insert("secureJsonData".to_string(), value.clone());
    }
    payload
}

fn apply_folder_operation<F>(
    request_json: &mut F,
    operation: &SyncApplyOperation,
    allow_folder_delete: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let title = parse_optional_operation_field(&operation.body, "title", &operation.title);
    let parent_uid = parse_optional_operation_field(&operation.body, "parentUid", "");
    let response = match operation.action.as_str() {
        "would-create" => {
            let mut payload = Map::from_iter(vec![
                ("uid".to_string(), Value::String(operation.identity.clone())),
                ("title".to_string(), Value::String(title)),
            ]);
            if !parent_uid.is_empty() {
                payload.insert("parentUid".to_string(), Value::String(parent_uid));
            }
            request_json(
                Method::POST,
                "/api/folders",
                &[],
                Some(&Value::Object(payload)),
            )?
        }
        "would-update" => {
            let mut payload = Map::from_iter(vec![
                ("uid".to_string(), Value::String(operation.identity.clone())),
                ("title".to_string(), Value::String(title)),
            ]);
            if !parent_uid.is_empty() {
                payload.insert("parentUid".to_string(), Value::String(parent_uid));
            }
            request_json(
                Method::PUT,
                &format!("/api/folders/{}", encode_path_segment(&operation.identity)),
                &[],
                Some(&Value::Object(payload)),
            )?
        }
        "would-delete" => {
            if !allow_folder_delete {
                return Err(message(format!(
                    "Refusing live folder delete for {} without --allow-folder-delete.",
                    operation.identity
                )));
            }
            request_json(
                Method::DELETE,
                &format!("/api/folders/{}", encode_path_segment(&operation.identity)),
                &vec![("forceDeleteRules".to_string(), "false".to_string())],
                None,
            )?
        }
        _ => {
            return Err(message(format!(
                "Unsupported folder sync action {}.",
                operation.action
            )));
        }
    };
    Ok(response_or_null(response))
}

fn apply_dashboard_operation<F>(
    request_json: &mut F,
    operation: &SyncApplyOperation,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match operation.action.as_str() {
        "would-delete" => {
            let response = request_json(
                Method::DELETE,
                &format!(
                    "/api/dashboards/uid/{}",
                    encode_path_segment(&operation.identity)
                ),
                &[],
                None,
            )?;
            Ok(response_or_null(response))
        }
        "would-create" | "would-update" => {
            let mut body = operation.body.clone();
            body.insert("uid".to_string(), Value::String(operation.identity.clone()));
            body.insert(
                "title".to_string(),
                Value::String(parse_optional_operation_field(
                    &body,
                    "title",
                    &operation.title,
                )),
            );
            body.remove("id");
            let payload = serde_json::json!({
                "dashboard": Value::Object(body),
                "overwrite": operation.action == "would-update",
            });
            let response = request_json(Method::POST, "/api/dashboards/db", &[], Some(&payload))?;
            Ok(response_or_null(response))
        }
        _ => Err(message(format!(
            "Unsupported dashboard sync action {}.",
            operation.action
        ))),
    }
}

fn apply_datasource_operation<F>(
    request_json: &mut F,
    operation: &SyncApplyOperation,
    live_datasources: &[Map<String, Value>],
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match operation.action.as_str() {
        "would-create" => {
            let payload = build_datasource_add_payload(operation)?;
            let response = request_json(
                Method::POST,
                "/api/datasources",
                &[],
                Some(&Value::Object(payload)),
            )?;
            Ok(response_or_null(response))
        }
        "would-update" => {
            let target = resolve_datasource_target(live_datasources, &operation.identity)?;
            let datasource_id = extract_datasource_target_id(target).ok_or_else(|| {
                message(format!(
                    "Datasource sync update requires a live datasource id for {}.",
                    operation.identity
                ))
            })?;
            let payload = build_datasource_modify_payload(target, &operation.body);
            let response = request_json(
                Method::PUT,
                &format!("/api/datasources/{datasource_id}"),
                &[],
                Some(&Value::Object(payload)),
            )?;
            Ok(response_or_null(response))
        }
        "would-delete" => {
            let target = resolve_datasource_target(live_datasources, &operation.identity)?;
            let datasource_id = extract_datasource_target_id(target).ok_or_else(|| {
                message(format!(
                    "Datasource sync delete requires a live datasource id for {}.",
                    operation.identity
                ))
            })?;
            let response = request_json(
                Method::DELETE,
                &format!("/api/datasources/{datasource_id}"),
                &[],
                None,
            )?;
            Ok(response_or_null(response))
        }
        _ => Err(message(format!(
            "Unsupported datasource sync action {}.",
            operation.action
        ))),
    }
}

fn run_sync_apply_operations<F>(
    operations: &[Value],
    allow_folder_delete: bool,
    continue_on_error: bool,
    request_json: &mut F,
    live_datasources: &[Map<String, Value>],
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut results = Vec::new();
    let mut applied_count = 0;
    let mut failed_count = 0;

    for operation in operations {
        let operation = match parse_sync_apply_operation(operation) {
            Ok(operation) => operation,
            Err(error) => {
                if !continue_on_error {
                    return Err(error);
                }
                failed_count += 1;
                results.push(serde_json::json!({
                    "status": "error",
                    "kind": "",
                    "identity": "",
                    "action": "",
                    "error": error.to_string(),
                }));
                continue;
            }
        };

        let response = match operation.kind.as_str() {
            "folder" => match apply_folder_operation(request_json, &operation, allow_folder_delete)
            {
                Ok(response) => Some(response),
                Err(error) => {
                    if continue_on_error {
                        failed_count += 1;
                        results.push(serde_json::json!({
                            "status": "error",
                            "kind": operation.kind,
                            "identity": operation.identity,
                            "action": operation.action,
                            "error": error.to_string(),
                        }));
                        None
                    } else {
                        return Err(error);
                    }
                }
            },
            "dashboard" => match apply_dashboard_operation(request_json, &operation) {
                Ok(response) => Some(response),
                Err(error) => {
                    if continue_on_error {
                        failed_count += 1;
                        results.push(serde_json::json!({
                            "status": "error",
                            "kind": operation.kind,
                            "identity": operation.identity,
                            "action": operation.action,
                            "error": error.to_string(),
                        }));
                        None
                    } else {
                        return Err(error);
                    }
                }
            },
            "datasource" => {
                match apply_datasource_operation(request_json, &operation, live_datasources) {
                    Ok(response) => Some(response),
                    Err(error) => {
                        if continue_on_error {
                            failed_count += 1;
                            results.push(serde_json::json!({
                                "status": "error",
                                "kind": operation.kind,
                                "identity": operation.identity,
                                "action": operation.action,
                                "error": error.to_string(),
                            }));
                            None
                        } else {
                            return Err(error);
                        }
                    }
                }
            }
            "alert" => {
                if continue_on_error {
                    failed_count += 1;
                    results.push(serde_json::json!({
                        "status": "error",
                        "kind": operation.kind,
                        "identity": operation.identity,
                        "action": operation.action,
                        "error": "Live sync apply does not support alert operations yet; keep alerts in plan-only mode.",
                    }));
                    continue;
                }
                return Err(message(
                    "Live sync apply does not support alert operations yet; keep alerts in plan-only mode.",
                ));
            }
            _ => {
                if continue_on_error {
                    failed_count += 1;
                    results.push(serde_json::json!({
                        "status": "error",
                        "kind": operation.kind,
                        "identity": operation.identity,
                        "action": operation.action,
                        "error": format!("Unsupported sync resource kind {}.", operation.kind),
                    }));
                    continue;
                }
                return Err(message(format!(
                    "Unsupported sync resource kind {}.",
                    operation.kind
                )));
            }
        };

        if let Some(response) = response {
            applied_count += 1;
            results.push(serde_json::json!({
                "status": "ok",
                "kind": operation.kind,
                "identity": operation.identity,
                "action": operation.action,
                "response": response,
            }));
        }
    }

    Ok(serde_json::json!({
        "mode": "live-apply",
        "appliedCount": applied_count,
        "failedCount": failed_count,
        "results": results,
    }))
}

fn build_sync_apply_live_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some("grafana-utils-sync-apply-intent") {
        return Err(message("Sync apply intent document kind is not supported."));
    }
    let operations = document
        .get("operations")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| message("Sync apply intent document is missing operations."))?;
    let mut lines = Vec::new();
    lines.push("Sync apply intent".to_string());
    lines.push(format!("Mode: local apply"));
    lines.push(format!("Operations: {}", operations.len()));
    if !operations.is_empty() {
        for operation in operations {
            let object = value_as_object(&operation, "Sync apply operation")?;
            let kind = object
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let identity = object
                .get("identity")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let action = object
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            lines.push(format!("{kind} {identity} {action}"));
        }
    }
    Ok(lines)
}

fn build_sync_live_apply_text(document: &Value) -> Result<Vec<String>> {
    if document.get("mode").and_then(Value::as_str) != Some("live-apply") {
        return Err(message("Sync live apply document kind is not supported."));
    }
    let results = document
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Sync live apply document is missing results."))?;
    let applied_count = document
        .get("appliedCount")
        .and_then(Value::as_u64)
        .unwrap_or(results.len() as u64);
    let failed_count = document
        .get("failedCount")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut lines = vec![
        "Sync live apply".to_string(),
        format!("AppliedCount: {}", applied_count),
        format!("FailedCount: {}", failed_count),
    ];
    for item in results {
        let item = value_as_object(item, "Sync live apply result item")?;
        let kind = item
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let identity = item
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let action = item
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let status = item.get("status").and_then(Value::as_str).unwrap_or("ok");
        if status.eq_ignore_ascii_case("error") {
            let error = item
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            lines.push(format!("{kind} {identity} {action} [{status}] {error}"));
        } else {
            lines.push(format!("{kind} {identity} {action} [{status}]"));
        }
    }
    Ok(lines)
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
            let live = resolve_live_specs(&args)?;
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
                    require_optional_stage(
                        &preflight,
                        "Sync preflight document",
                        "preflight",
                        2,
                        Some(&trace_id),
                    )?;
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
                    require_optional_stage(
                        &bundle_preflight,
                        "Sync bundle preflight document",
                        "bundle-preflight",
                        2,
                        Some(&trace_id),
                    )?;
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
            if args.execute_live {
                let client = fetch_live_client(&args.common, args.org_id)?;
                let operations = document
                    .get("operations")
                    .and_then(Value::as_array)
                    .ok_or_else(|| message("Sync apply intent document is missing operations."))?
                    .clone();
                let live_datasources = list_datasources(&client)?;
                let results = run_sync_apply_operations(
                    &operations,
                    args.allow_folder_delete,
                    args.continue_on_error,
                    &mut |method, path, params, payload| {
                        client.request_json(method, path, params, payload)
                    },
                    &live_datasources,
                )?;
                emit_text_or_json(&results, build_sync_live_apply_text(&results)?, args.output)?
            } else {
                emit_text_or_json(
                    &document,
                    build_sync_apply_live_text(&document)?,
                    args.output,
                )?
            }
            Ok(())
        }
        SyncGroupCommand::Summary(args) => {
            let desired = load_json_array_file(&args.desired_file, "Sync desired input")?;
            let document = build_sync_summary_document(&desired)?;
            emit_text_or_json(&document, render_sync_summary_text(&document)?, args.output)
        }
        SyncGroupCommand::Preflight(args) => {
            let desired = load_json_array_file(&args.desired_file, "Sync desired input")?;
            let availability = resolve_availability(
                args.availability_file.as_ref(),
                &args.common,
                args.org_id,
                args.fetch_live,
            )?;
            let document = attach_trace_id(
                &build_sync_preflight_document(&desired, availability.as_ref())?,
                args.trace_id.as_deref(),
            )?;
            let trace_id = require_trace_id(&document, "Sync preflight document")?;
            let document = attach_lineage(&document, "preflight", 2, Some(&trace_id))?;
            emit_text_or_json(
                &document,
                render_sync_preflight_text(&document)?,
                args.output,
            )
        }
        SyncGroupCommand::AssessAlerts(args) => {
            let alert_specs = load_json_array_file(&args.alerts_file, "Alert sync input")?;
            let document = assess_alert_sync_specs(&alert_specs)?;
            emit_text_or_json(
                &document,
                render_sync_assess_alerts_text(&document)?,
                args.output,
            )
        }
        SyncGroupCommand::BundlePreflight(args) => {
            let source_bundle = load_json_value(&args.source_bundle, "Sync source bundle input")?;
            let target_inventory =
                load_json_value(&args.target_inventory, "Sync target inventory input")?;
            let availability = resolve_availability(
                args.availability_file.as_ref(),
                &args.common,
                args.org_id,
                args.fetch_live,
            )?;
            let document = attach_trace_id(
                &build_sync_bundle_preflight_document(
                    &source_bundle,
                    &target_inventory,
                    availability.as_ref(),
                )?,
                args.trace_id.as_deref(),
            )?;
            let trace_id = require_trace_id(&document, "Sync bundle preflight document")?;
            let document = attach_lineage(&document, "bundle-preflight", 2, Some(&trace_id))?;
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
