//! Local/document-only sync CLI wrapper.
//!
//! Purpose:
//! - Expose staged Rust sync contracts through a minimal CLI namespace.
//! - Keep dry-run/reviewable sync artifacts available even when optional live
//!   fetch/apply wiring is enabled.

use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Method;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub mod bundle_alert_contracts;
pub mod bundle_preflight;
pub mod preflight;
pub mod workbench;

use self::bundle_preflight::{
    build_sync_bundle_preflight_document, render_sync_bundle_preflight_text,
    SYNC_BUNDLE_PREFLIGHT_KIND,
};
use self::preflight::{
    build_sync_preflight_document, render_sync_preflight_text, SYNC_PREFLIGHT_KIND,
};
use self::workbench::{
    build_sync_apply_intent_document, build_sync_plan_document, build_sync_source_bundle_document,
    build_sync_summary_document, render_sync_source_bundle_text,
};
use crate::alert::{
    build_contact_point_import_payload, build_mute_timing_import_payload,
    build_policies_import_payload, build_rule_import_payload, build_template_import_payload,
    detect_document_kind, CONTACT_POINT_KIND, MUTE_TIMING_KIND, POLICIES_KIND, RULE_KIND,
    TEMPLATE_KIND,
};
use crate::alert_sync::{assess_alert_sync_specs, ALERT_SYNC_KIND};
use crate::common::{message, Result};
use crate::dashboard::DASHBOARD_PERMISSION_BUNDLE_FILENAME;
use crate::dashboard::{build_http_client, build_http_client_for_org, CommonCliArgs};

/// Constant for default review token.
pub const DEFAULT_REVIEW_TOKEN: &str = "reviewed-sync-plan";
const SYNC_ROOT_HELP_TEXT: &str = "Examples:\n\n  Summarize desired resources:\n    grafana-util sync summary --desired-file ./desired.json\n\n  Package local exports into one source bundle:\n    grafana-util sync bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json\n\n  Compare a source bundle against target inventory before apply:\n    grafana-util sync bundle-preflight --source-bundle ./sync-source-bundle.json --target-inventory ./target-inventory.json --output json\n\n  Build a live-backed sync plan:\n    grafana-util sync plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"\n\n  Apply a reviewed plan back to Grafana:\n    grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"";
const SYNC_SUMMARY_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync summary --desired-file ./desired.json\n  grafana-util sync summary --desired-file ./desired.json --output json";
const SYNC_PLAN_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync plan --desired-file ./desired.json --live-file ./live.json\n  grafana-util sync plan --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --allow-prune --output json";
const SYNC_REVIEW_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync review --plan-file ./sync-plan.json\n  grafana-util sync review --plan-file ./sync-plan.json --review-note 'peer-reviewed' --output json";
const SYNC_APPLY_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve\n  grafana-util sync apply --plan-file ./sync-plan-reviewed.json --approve --execute-live --allow-folder-delete --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\"";
const SYNC_PREFLIGHT_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync preflight --desired-file ./desired.json --availability-file ./availability.json\n  grafana-util sync preflight --desired-file ./desired.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output json";
const SYNC_ASSESS_ALERTS_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync assess-alerts --alerts-file ./alerts.json\n  grafana-util sync assess-alerts --alerts-file ./alerts.json --output json";
const SYNC_BUNDLE_PREFLIGHT_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync bundle-preflight --source-bundle ./bundle.json --target-inventory ./target.json\n  grafana-util sync bundle-preflight --source-bundle ./bundle.json --target-inventory ./target.json --fetch-live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output json";
const SYNC_BUNDLE_HELP_TEXT: &str = "Examples:\n\n  grafana-util sync bundle --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts/raw --output-file ./sync-source-bundle.json\n  grafana-util sync bundle --dashboard-export-dir ./dashboards/raw --datasource-export-file ./datasources/datasources.json --output json";

/// Enum definition for SyncOutputFormat.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SyncOutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util sync",
    about = "Reviewable sync workflows with optional live Grafana fetch/apply paths.",
    after_help = SYNC_ROOT_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
/// Struct definition for SyncCliArgs.
pub struct SyncCliArgs {
    #[command(subcommand)]
    pub command: SyncGroupCommand,
}

/// Struct definition for SyncSummaryArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncSummaryArgs {
    #[arg(
        long,
        help = "JSON file containing the desired sync resource list.",
        help_heading = "Input Options"
    )]
    pub desired_file: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the summary document as text or json.",
        help_heading = "Output Options"
    )]
    pub output: SyncOutputFormat,
}

/// Struct definition for SyncPlanArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncPlanArgs {
    #[arg(
        long,
        help = "JSON file containing the desired sync resource list.",
        help_heading = "Input Options"
    )]
    pub desired_file: PathBuf,
    #[arg(
        long,
        help = "JSON file containing the live sync resource list.",
        help_heading = "Input Options"
    )]
    pub live_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Read the current live state directly from Grafana instead of --live-file.",
        help_heading = "Live Options"
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active.",
        help_heading = "Live Options"
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = 500usize,
        help = "Dashboard search page size when --fetch-live is active.",
        help_heading = "Live Options"
    )]
    pub page_size: usize,
    #[arg(
        long,
        default_value_t = false,
        help = "Mark live-only resources as would-delete instead of unmanaged.",
        help_heading = "Planning Options"
    )]
    pub allow_prune: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the plan document as text or json.",
        help_heading = "Output Options"
    )]
    pub output: SyncOutputFormat,
    #[arg(
        long,
        help = "Optional stable trace id to carry through staged plan/review/apply files."
    )]
    pub trace_id: Option<String>,
}

/// Struct definition for SyncReviewArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncReviewArgs {
    #[arg(
        long,
        help = "JSON file containing the staged sync plan document.",
        help_heading = "Input Options"
    )]
    pub plan_file: PathBuf,
    #[arg(
        long,
        default_value = DEFAULT_REVIEW_TOKEN,
        help = "Explicit review token required to mark the plan reviewed.",
        help_heading = "Review Options"
    )]
    pub review_token: String,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the reviewed plan document as text or json.",
        help_heading = "Output Options"
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

/// Struct definition for SyncApplyArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncApplyArgs {
    #[arg(
        long,
        help = "JSON file containing the reviewed sync plan document.",
        help_heading = "Input Options"
    )]
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
        help = "Explicit acknowledgement required before a local apply intent is emitted.",
        help_heading = "Approval Options"
    )]
    pub approve: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --execute-live is active.",
        help_heading = "Live Options"
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        help = "Apply supported sync operations to Grafana after review and approval checks pass.",
        help_heading = "Live Options"
    )]
    pub execute_live: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Allow live deletion of folders when a reviewed plan includes would-delete folder operations.",
        help_heading = "Approval Options"
    )]
    pub allow_folder_delete: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the apply intent document as text or json.",
        help_heading = "Output Options"
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

/// Struct definition for SyncPreflightArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncPreflightArgs {
    #[arg(
        long,
        help = "JSON file containing the desired sync resource list.",
        help_heading = "Input Options"
    )]
    pub desired_file: PathBuf,
    #[arg(
        long,
        help = "Optional JSON object file containing staged availability hints."
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Fetch availability hints from Grafana instead of relying only on --availability-file.",
        help_heading = "Live Options"
    )]
    pub fetch_live: bool,
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Optional Grafana org id used when --fetch-live is active.",
        help_heading = "Live Options"
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the preflight document as text or json."
    )]
    pub output: SyncOutputFormat,
}

/// Struct definition for SyncAssessAlertsArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncAssessAlertsArgs {
    #[arg(
        long,
        help = "JSON file containing the alert sync resource list.",
        help_heading = "Input Options"
    )]
    pub alerts_file: PathBuf,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the alert assessment document as text or json."
    )]
    pub output: SyncOutputFormat,
}

/// Struct definition for SyncBundlePreflightArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncBundlePreflightArgs {
    #[arg(
        long,
        help = "JSON file containing the staged multi-resource source bundle.",
        help_heading = "Input Options"
    )]
    pub source_bundle: PathBuf,
    #[arg(
        long,
        help = "JSON file containing the staged target inventory snapshot.",
        help_heading = "Input Options"
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
        help = "Fetch availability hints from Grafana instead of relying only on --availability-file.",
        help_heading = "Live Options"
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
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the bundle preflight document as text or json."
    )]
    pub output: SyncOutputFormat,
}

/// Struct definition for SyncBundleArgs.
#[derive(Debug, Clone, Args)]
pub struct SyncBundleArgs {
    #[arg(
        long,
        help = "Path to one existing dashboard raw export directory such as ./dashboards/raw."
    )]
    pub dashboard_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Path to one existing alert raw export directory such as ./alerts/raw."
    )]
    pub alert_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional standalone datasource inventory JSON file to include or prefer over dashboards/raw/datasources.json."
    )]
    pub datasource_export_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON object file containing extra bundle metadata."
    )]
    pub metadata_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional JSON file path to write the source bundle artifact."
    )]
    pub output_file: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        default_value_t = SyncOutputFormat::Text,
        help = "Render the source bundle document as text or json."
    )]
    pub output: SyncOutputFormat,
}

/// Enum definition for SyncGroupCommand.
#[derive(Debug, Clone, Subcommand)]
pub enum SyncGroupCommand {
    #[command(about = "Build a staged sync plan from local desired and live JSON files.", after_help = SYNC_PLAN_HELP_TEXT)]
    Plan(SyncPlanArgs),
    #[command(about = "Mark a staged sync plan JSON document reviewed.", after_help = SYNC_REVIEW_HELP_TEXT)]
    Review(SyncReviewArgs),
    #[command(about = "Build a gated local apply intent from a reviewed sync plan.", after_help = SYNC_APPLY_HELP_TEXT)]
    Apply(SyncApplyArgs),
    #[command(about = "Summarize local desired sync resources from JSON.", after_help = SYNC_SUMMARY_HELP_TEXT)]
    Summary(SyncSummaryArgs),
    #[command(about = "Build a staged sync preflight document from local JSON.", after_help = SYNC_PREFLIGHT_HELP_TEXT)]
    Preflight(SyncPreflightArgs),
    #[command(about = "Assess alert sync specs for candidate, plan-only, and blocked states.", after_help = SYNC_ASSESS_ALERTS_HELP_TEXT)]
    AssessAlerts(SyncAssessAlertsArgs),
    #[command(about = "Build a staged bundle-level sync preflight document from local JSON.", after_help = SYNC_BUNDLE_PREFLIGHT_HELP_TEXT)]
    BundlePreflight(SyncBundlePreflightArgs),
    #[command(
        about = "Package exported dashboards, alerting resources, datasource inventory, and metadata into one local source bundle.",
        after_help = SYNC_BUNDLE_HELP_TEXT
    )]
    Bundle(SyncBundleArgs),
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

fn build_sync_http_client(
    common: &CommonCliArgs,
    org_id: Option<i64>,
) -> Result<crate::http::JsonHttpClient> {
    match org_id {
        Some(org_id) => build_http_client_for_org(common, org_id),
        None => build_http_client(common),
    }
}

fn append_unique_strings(target: &mut Vec<Value>, values: &[String]) {
    let mut seen = target
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<std::collections::BTreeSet<_>>();
    for value in values {
        if !value.trim().is_empty() && seen.insert(value.clone()) {
            target.push(Value::String(value.clone()));
        }
    }
}

fn merge_availability(base: Option<Value>, extra: &Value) -> Result<Value> {
    let mut merged = match base {
        Some(Value::Object(object)) => object,
        Some(_) => {
            return Err(message(
                "Sync availability input file must contain a JSON object.",
            ))
        }
        None => Map::new(),
    };
    let extra_object = require_json_object(extra, "Live availability document")?;
    for (key, value) in extra_object {
        if matches!(
            key.as_str(),
            "datasourceUids" | "datasourceNames" | "pluginIds" | "contactPoints"
        ) {
            let existing = merged
                .remove(key)
                .and_then(|item| item.as_array().cloned())
                .unwrap_or_default();
            let mut combined = existing;
            let extra_items = value
                .as_array()
                .ok_or_else(|| message(format!("Live availability field {key} must be a list.")))?;
            let strings = extra_items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            append_unique_strings(&mut combined, &strings);
            merged.insert(key.clone(), Value::Array(combined));
        } else {
            merged.insert(key.clone(), value.clone());
        }
    }
    Ok(Value::Object(merged))
}

fn fetch_live_resource_specs_with_request<F>(
    mut request_json: F,
    page_size: usize,
) -> Result<Vec<Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    fn build_live_alert_resource_spec(sync_kind: &str, body: Map<String, Value>) -> Result<Value> {
        let (identity, title) = normalize_alert_resource_identity_and_title(sync_kind, &body)?;
        Ok(serde_json::json!({
            "kind": sync_kind,
            "uid": if sync_kind == "alert-contact-point" { identity.clone() } else { String::new() },
            "name": if matches!(sync_kind, "alert-mute-timing" | "alert-template") { identity.clone() } else { String::new() },
            "title": title,
            "managedFields": normalize_alert_managed_fields(&body),
            "body": body,
        }))
    }

    let mut specs = Vec::new();
    match request_json(Method::GET, "/api/folders", &[], None)? {
        Some(Value::Array(folders)) => {
            for folder in folders {
                let object = require_json_object(&folder, "Grafana folder payload")?;
                let uid = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                if uid.is_empty() {
                    continue;
                }
                let title = object
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(uid);
                let mut body = Map::new();
                body.insert("title".to_string(), Value::String(title.to_string()));
                if let Some(parent_uid) = object
                    .get("parentUid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    body.insert(
                        "parentUid".to_string(),
                        Value::String(parent_uid.to_string()),
                    );
                }
                specs.push(serde_json::json!({
                    "kind": "folder",
                    "uid": uid,
                    "title": title,
                    "body": body,
                }));
            }
        }
        Some(_) => return Err(message("Unexpected folder list response from Grafana.")),
        None => {}
    }

    let mut page = 1usize;
    loop {
        let params = vec![
            ("type".to_string(), "dash-db".to_string()),
            ("limit".to_string(), page_size.to_string()),
            ("page".to_string(), page.to_string()),
        ];
        let batch = match request_json(Method::GET, "/api/search", &params, None)? {
            Some(Value::Array(items)) => items,
            Some(_) => return Err(message("Unexpected search response from Grafana.")),
            None => Vec::new(),
        };
        if batch.is_empty() {
            break;
        }
        let batch_len = batch.len();
        for item in batch {
            let summary = require_json_object(&item, "Grafana dashboard summary")?;
            let uid = summary
                .get("uid")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if uid.is_empty() {
                continue;
            }
            let dashboard_wrapper = match request_json(
                Method::GET,
                &format!("/api/dashboards/uid/{uid}"),
                &[],
                None,
            )? {
                Some(value) => value,
                None => continue,
            };
            let wrapper = require_json_object(&dashboard_wrapper, "Grafana dashboard payload")?;
            let dashboard = wrapper
                .get("dashboard")
                .ok_or_else(|| message(format!("Unexpected dashboard payload for UID {uid}.")))?;
            let body = require_json_object(dashboard, "Grafana dashboard body")?;
            let mut normalized = body.clone();
            normalized.remove("id");
            let title = normalized
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(uid);
            specs.push(serde_json::json!({
                "kind": "dashboard",
                "uid": uid,
                "title": title,
                "body": normalized,
            }));
        }
        if batch_len < page_size {
            break;
        }
        page += 1;
    }

    match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(datasources)) => {
            for datasource in datasources {
                let object = require_json_object(&datasource, "Grafana datasource payload")?;
                let uid = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                let name = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                if uid.is_empty() && name.is_empty() {
                    continue;
                }
                let title = if name.is_empty() { uid } else { name };
                let mut body = Map::new();
                body.insert("uid".to_string(), Value::String(uid.to_string()));
                body.insert("name".to_string(), Value::String(title.to_string()));
                body.insert(
                    "type".to_string(),
                    object
                        .get("type")
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                );
                body.insert(
                    "access".to_string(),
                    object
                        .get("access")
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                );
                body.insert(
                    "url".to_string(),
                    object
                        .get("url")
                        .cloned()
                        .unwrap_or(Value::String(String::new())),
                );
                body.insert(
                    "isDefault".to_string(),
                    object
                        .get("isDefault")
                        .cloned()
                        .unwrap_or(Value::Bool(false)),
                );
                if let Some(json_data) = object.get("jsonData").and_then(Value::as_object) {
                    if !json_data.is_empty() {
                        body.insert("jsonData".to_string(), Value::Object(json_data.clone()));
                    }
                }
                specs.push(serde_json::json!({
                    "kind": "datasource",
                    "uid": uid,
                    "name": title,
                    "title": title,
                    "body": body,
                }));
            }
        }
        Some(_) => return Err(message("Unexpected datasource list response from Grafana.")),
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/alert-rules", &[], None)? {
        Some(Value::Array(rules)) => {
            for rule in rules {
                let object = require_json_object(&rule, "Grafana alert-rule payload")?;
                let body = build_rule_import_payload(object)?;
                let uid = body
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| message("Live alert rule payload is missing uid."))?;
                let title = body
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(uid);
                specs.push(serde_json::json!({
                    "kind": "alert",
                    "uid": uid,
                    "title": title,
                    "body": body,
                }));
            }
        }
        Some(_) => return Err(message("Unexpected alert-rule list response from Grafana.")),
        None => {}
    }

    match request_json(
        Method::GET,
        "/api/v1/provisioning/contact-points",
        &[],
        None,
    )? {
        Some(Value::Array(contact_points)) => {
            for contact_point in contact_points {
                let object = require_json_object(&contact_point, "Grafana contact-point payload")?;
                specs.push(build_live_alert_resource_spec(
                    "alert-contact-point",
                    object.clone(),
                )?);
            }
        }
        Some(_) => {
            return Err(message(
                "Unexpected contact-point list response from Grafana.",
            ))
        }
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/mute-timings", &[], None)? {
        Some(Value::Array(mute_timings)) => {
            for mute_timing in mute_timings {
                let object = require_json_object(&mute_timing, "Grafana mute-timing payload")?;
                specs.push(build_live_alert_resource_spec(
                    "alert-mute-timing",
                    object.clone(),
                )?);
            }
        }
        Some(_) => {
            return Err(message(
                "Unexpected mute-timing list response from Grafana.",
            ))
        }
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/policies", &[], None)? {
        Some(Value::Object(policies)) => {
            specs.push(build_live_alert_resource_spec(
                "alert-policy",
                policies.clone(),
            )?);
        }
        Some(_) => {
            return Err(message(
                "Unexpected notification policy response from Grafana.",
            ))
        }
        None => {}
    }

    match request_json(Method::GET, "/api/v1/provisioning/templates", &[], None)? {
        Some(Value::Array(templates)) => {
            for template in templates {
                let object = require_json_object(&template, "Grafana template summary payload")?;
                let name = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| message("Live template payload is missing name."))?;
                let template_payload = match request_json(
                    Method::GET,
                    &format!("/api/v1/provisioning/templates/{name}"),
                    &[],
                    None,
                )? {
                    Some(Value::Object(template_object)) => template_object,
                    Some(_) => return Err(message("Unexpected template payload from Grafana.")),
                    None => continue,
                };
                specs.push(build_live_alert_resource_spec(
                    "alert-template",
                    template_payload,
                )?);
            }
        }
        Some(_) => return Err(message("Unexpected template list response from Grafana.")),
        None => {}
    }

    Ok(specs)
}

fn fetch_live_resource_specs(
    common: &CommonCliArgs,
    org_id: Option<i64>,
    page_size: usize,
) -> Result<Vec<Value>> {
    let client = build_sync_http_client(common, org_id)?;
    fetch_live_resource_specs_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        page_size,
    )
}

fn fetch_live_availability_with_request<F>(mut request_json: F) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut availability = Map::from_iter(vec![
        ("datasourceUids".to_string(), Value::Array(Vec::new())),
        ("datasourceNames".to_string(), Value::Array(Vec::new())),
        ("pluginIds".to_string(), Value::Array(Vec::new())),
        ("contactPoints".to_string(), Value::Array(Vec::new())),
    ]);

    match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(datasources)) => {
            let mut uids = Vec::new();
            let mut names = Vec::new();
            for datasource in datasources {
                let object = require_json_object(&datasource, "Grafana datasource payload")?;
                if let Some(uid) = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    uids.push(uid.to_string());
                }
                if let Some(name) = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    names.push(name.to_string());
                }
            }
            append_unique_strings(
                availability
                    .get_mut("datasourceUids")
                    .and_then(Value::as_array_mut)
                    .expect("datasourceUids should be array"),
                &uids,
            );
            append_unique_strings(
                availability
                    .get_mut("datasourceNames")
                    .and_then(Value::as_array_mut)
                    .expect("datasourceNames should be array"),
                &names,
            );
        }
        Some(_) => return Err(message("Unexpected datasource list response from Grafana.")),
        None => {}
    }

    match request_json(Method::GET, "/api/plugins", &[], None)? {
        Some(Value::Array(plugins)) => {
            let ids = plugins
                .iter()
                .filter_map(Value::as_object)
                .filter_map(|plugin| plugin.get("id").and_then(Value::as_str))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            append_unique_strings(
                availability
                    .get_mut("pluginIds")
                    .and_then(Value::as_array_mut)
                    .expect("pluginIds should be array"),
                &ids,
            );
        }
        Some(_) => return Err(message("Unexpected plugin list response from Grafana.")),
        None => {}
    }

    match request_json(
        Method::GET,
        "/api/v1/provisioning/contact-points",
        &[],
        None,
    )? {
        Some(Value::Array(contact_points)) => {
            let mut names = Vec::new();
            for item in contact_points {
                let object = require_json_object(&item, "Grafana contact-point payload")?;
                if let Some(name) = object
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    names.push(name.to_string());
                }
                if let Some(uid) = object
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    names.push(uid.to_string());
                }
            }
            append_unique_strings(
                availability
                    .get_mut("contactPoints")
                    .and_then(Value::as_array_mut)
                    .expect("contactPoints should be array"),
                &names,
            );
        }
        Some(_) => {
            return Err(message(
                "Unexpected contact-point list response from Grafana.",
            ))
        }
        None => {}
    }

    Ok(Value::Object(availability))
}

fn fetch_live_availability(common: &CommonCliArgs, org_id: Option<i64>) -> Result<Value> {
    let client = build_sync_http_client(common, org_id)?;
    fetch_live_availability_with_request(|method, path, params, payload| {
        client.request_json(method, path, params, payload)
    })
}

fn discover_json_files(root: &Path, ignored_names: &[&str]) -> Result<Vec<PathBuf>> {
    fn visit(current: &Path, ignored_names: &[&str], files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                visit(&path, ignored_names, files)?;
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if ignored_names.contains(&file_name) {
                continue;
            }
            files.push(path);
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, ignored_names, &mut files)?;
    files.sort();
    Ok(files)
}

fn normalize_dashboard_bundle_item(document: &Value, source_path: &str) -> Result<Value> {
    let mut body = if let Some(body) = document.get("dashboard").and_then(Value::as_object) {
        body.clone()
    } else {
        require_json_object(document, "Dashboard export document")?.clone()
    };
    body.remove("id");
    let uid = body
        .get("uid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            message(format!(
                "Dashboard export document is missing dashboard.uid: {source_path}"
            ))
        })?;
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(uid);
    Ok(serde_json::json!({
        "kind": "dashboard",
        "uid": uid,
        "title": title,
        "body": body,
        "sourcePath": source_path,
    }))
}

fn normalize_folder_bundle_item(document: &Value) -> Result<Value> {
    let object = require_json_object(document, "Folder inventory record")?;
    let uid = object
        .get("uid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| message("Folder inventory record is missing uid."))?;
    let title = object
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(uid);
    let mut body = Map::new();
    body.insert("title".to_string(), Value::String(title.to_string()));
    if let Some(parent_uid) = object
        .get("parentUid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        body.insert(
            "parentUid".to_string(),
            Value::String(parent_uid.to_string()),
        );
    }
    if let Some(path) = object
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        body.insert("path".to_string(), Value::String(path.to_string()));
    }
    Ok(serde_json::json!({
        "kind": "folder",
        "uid": uid,
        "title": title,
        "body": body,
        "sourcePath": object.get("sourcePath").cloned().unwrap_or(Value::String(String::new())),
    }))
}

fn normalize_datasource_bundle_item(document: &Value) -> Result<Value> {
    let object = require_json_object(document, "Datasource inventory record")?;
    let uid = object
        .get("uid")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if uid.is_empty() && name.is_empty() {
        return Err(message("Datasource inventory record requires uid or name."));
    }
    let title = if name.is_empty() { uid } else { name };
    Ok(serde_json::json!({
        "kind": "datasource",
        "uid": uid,
        "name": title,
        "title": title,
        "body": {
            "uid": uid,
            "name": title,
            "type": object.get("type").cloned().unwrap_or(Value::String(String::new())),
            "access": object.get("access").cloned().unwrap_or(Value::String(String::new())),
            "url": object.get("url").cloned().unwrap_or(Value::String(String::new())),
            "isDefault": object.get("isDefault").cloned().unwrap_or(Value::Bool(false)),
        },
        "secureJsonDataProviders": object.get("secureJsonDataProviders").cloned().unwrap_or(Value::Object(Map::new())),
        "secureJsonDataPlaceholders": object.get("secureJsonDataPlaceholders").cloned().unwrap_or(Value::Object(Map::new())),
        "sourcePath": object.get("sourcePath").cloned().unwrap_or(Value::String(String::new())),
    }))
}

fn classify_alert_export_path(relative_path: &str) -> Option<&'static str> {
    let first = relative_path.split('/').next().unwrap_or("");
    match first {
        "rules" => Some("rules"),
        "contact-points" => Some("contactPoints"),
        "mute-timings" => Some("muteTimings"),
        "policies" => Some("policies"),
        "templates" => Some("templates"),
        _ => None,
    }
}

type DashboardBundleSections = (Vec<Value>, Vec<Value>, Vec<Value>, Map<String, Value>);

fn load_dashboard_bundle_sections(export_dir: &Path) -> Result<DashboardBundleSections> {
    let mut dashboards = Vec::new();
    for path in discover_json_files(
        export_dir,
        &[
            "index.json",
            "export-metadata.json",
            "folders.json",
            "datasources.json",
            DASHBOARD_PERMISSION_BUNDLE_FILENAME,
        ],
    )? {
        let source_path = path
            .strip_prefix(export_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        dashboards.push(normalize_dashboard_bundle_item(
            &load_json_value(&path, "Dashboard export document")?,
            &source_path,
        )?);
    }
    let folders_path = export_dir.join("folders.json");
    let folders = if folders_path.is_file() {
        load_json_array_file(&folders_path, "Dashboard folder inventory")?
            .into_iter()
            .map(|item| normalize_folder_bundle_item(&item))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let datasources_path = export_dir.join("datasources.json");
    let datasources = if datasources_path.is_file() {
        load_json_array_file(&datasources_path, "Dashboard datasource inventory")?
            .into_iter()
            .map(|item| normalize_datasource_bundle_item(&item))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let mut metadata = Map::new();
    let export_metadata_path = export_dir.join("export-metadata.json");
    if export_metadata_path.is_file() {
        metadata.insert(
            "dashboardExport".to_string(),
            load_json_value(&export_metadata_path, "Dashboard export metadata")?,
        );
    }
    metadata.insert(
        "dashboardExportDir".to_string(),
        Value::String(export_dir.display().to_string()),
    );
    Ok((dashboards, datasources, folders, metadata))
}

fn load_alerting_bundle_section(export_dir: &Path) -> Result<Value> {
    let mut alerting = Map::from_iter(vec![
        ("rules".to_string(), Value::Array(Vec::<Value>::new())),
        (
            "contactPoints".to_string(),
            Value::Array(Vec::<Value>::new()),
        ),
        ("muteTimings".to_string(), Value::Array(Vec::<Value>::new())),
        ("policies".to_string(), Value::Array(Vec::<Value>::new())),
        ("templates".to_string(), Value::Array(Vec::<Value>::new())),
    ]);
    for path in discover_json_files(export_dir, &["index.json", "export-metadata.json"])? {
        let relative_path = path
            .strip_prefix(export_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let Some(section) = classify_alert_export_path(&relative_path) else {
            continue;
        };
        let item = serde_json::json!({
            "sourcePath": relative_path,
            "document": load_json_value(&path, "Alert export document")?,
        });
        alerting
            .entry(section.to_string())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .expect("alerting section array")
            .push(item);
    }
    let summary = serde_json::json!({
        "ruleCount": alerting.get("rules").and_then(Value::as_array).map(|items| items.len()).unwrap_or(0),
        "contactPointCount": alerting.get("contactPoints").and_then(Value::as_array).map(|items| items.len()).unwrap_or(0),
        "muteTimingCount": alerting.get("muteTimings").and_then(Value::as_array).map(|items| items.len()).unwrap_or(0),
        "policyCount": alerting.get("policies").and_then(Value::as_array).map(|items| items.len()).unwrap_or(0),
        "templateCount": alerting.get("templates").and_then(Value::as_array).map(|items| items.len()).unwrap_or(0),
    });
    alerting.insert("summary".to_string(), summary);
    let export_metadata_path = export_dir.join("export-metadata.json");
    if export_metadata_path.is_file() {
        alerting.insert(
            "exportMetadata".to_string(),
            load_json_value(&export_metadata_path, "Alert export metadata")?,
        );
    }
    alerting.insert(
        "exportDir".to_string(),
        Value::String(export_dir.display().to_string()),
    );
    Ok(Value::Object(alerting))
}

fn add_non_empty_text_field(
    body: &mut Map<String, Value>,
    managed_fields: &mut Vec<String>,
    field: &str,
    value: &str,
) {
    let normalized = value.trim();
    if normalized.is_empty() {
        return;
    }
    body.insert(field.to_string(), Value::String(normalized.to_string()));
    managed_fields.push(field.to_string());
}

fn extract_rule_dependency_lists(
    rule: &Map<String, Value>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut datasource_uids = std::collections::BTreeSet::new();
    let mut datasource_names = std::collections::BTreeSet::new();
    let mut plugin_ids = std::collections::BTreeSet::new();
    for item in rule
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(object) = item.as_object() else {
            continue;
        };
        for (key, sink) in [
            ("datasourceUid", &mut datasource_uids),
            ("datasourceName", &mut datasource_names),
            ("datasourceType", &mut plugin_ids),
        ] {
            if let Some(value) = object
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                sink.insert(value.to_string());
            }
        }
        if let Some(datasource) = object
            .get("model")
            .and_then(Value::as_object)
            .and_then(|model| model.get("datasource"))
            .and_then(Value::as_object)
        {
            if let Some(uid) = datasource
                .get("uid")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                datasource_uids.insert(uid.to_string());
            }
            if let Some(name) = datasource
                .get("name")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                datasource_names.insert(name.to_string());
            }
            if let Some(ds_type) = datasource
                .get("type")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                plugin_ids.insert(ds_type.to_string());
            }
        }
    }
    (
        datasource_uids.into_iter().collect(),
        datasource_names.into_iter().collect(),
        plugin_ids.into_iter().collect(),
    )
}

fn extract_rule_contact_points(rule: &Map<String, Value>) -> Vec<String> {
    let mut contact_points = std::collections::BTreeSet::new();
    if let Some(receiver) = rule
        .get("receiver")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        contact_points.insert(receiver.to_string());
    }
    if let Some(receiver) = rule
        .get("notification_settings")
        .or_else(|| rule.get("notificationSettings"))
        .and_then(Value::as_object)
        .and_then(|settings| settings.get("receiver"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        contact_points.insert(receiver.to_string());
    }
    contact_points.into_iter().collect()
}

fn normalize_alert_managed_fields(body: &Map<String, Value>) -> Vec<String> {
    body.keys().cloned().collect()
}

fn normalize_alert_resource_identity_and_title(
    sync_kind: &str,
    payload: &Map<String, Value>,
) -> Result<(String, String)> {
    let identity = match sync_kind {
        "alert" | "alert-contact-point" => payload
            .get("uid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or_else(|| {
                payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or(""),
        "alert-mute-timing" | "alert-template" => payload
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(""),
        "alert-policy" => payload
            .get("receiver")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("root"),
        _ => "",
    };
    if identity.is_empty() {
        return Err(message(format!(
            "Alert provisioning export document is missing a stable identity for {sync_kind}."
        )));
    }
    let title = payload
        .get("name")
        .or_else(|| payload.get("title"))
        .or_else(|| payload.get("receiver"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(identity);
    Ok((identity.to_string(), title.to_string()))
}

fn map_alert_document_kind_to_sync_kind(kind: &str) -> Option<&'static str> {
    match kind {
        RULE_KIND => Some("alert"),
        CONTACT_POINT_KIND => Some("alert-contact-point"),
        MUTE_TIMING_KIND => Some("alert-mute-timing"),
        POLICIES_KIND => Some("alert-policy"),
        TEMPLATE_KIND => Some("alert-template"),
        _ => None,
    }
}

fn normalize_rule_group_rule_document(
    group: &Map<String, Value>,
    rule: &Map<String, Value>,
) -> Map<String, Value> {
    let mut normalized = rule.clone();
    if !normalized.contains_key("folderUID") {
        if let Some(folder_uid) = group
            .get("folderUID")
            .or_else(|| group.get("folderUid"))
            .cloned()
        {
            normalized.insert("folderUID".to_string(), folder_uid);
        }
    }
    if !normalized.contains_key("ruleGroup") {
        if let Some(rule_group) = group.get("name").cloned() {
            normalized.insert("ruleGroup".to_string(), rule_group);
        }
    }
    if !normalized.contains_key("notificationSettings") {
        if let Some(notification_settings) = normalized.remove("notification_settings") {
            normalized.insert("notificationSettings".to_string(), notification_settings);
        }
    }
    normalized
}

fn normalize_alert_rule_sync_spec(
    document: &Map<String, Value>,
    source_path: &str,
) -> Result<Value> {
    let rule = build_rule_import_payload(document)?;
    let uid = rule
        .get("uid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            message(format!(
                "Alert rule export document is missing uid: {source_path}"
            ))
        })?;
    let title = rule
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(uid);

    let mut body = Map::new();
    let mut managed_fields = Vec::new();
    add_non_empty_text_field(
        &mut body,
        &mut managed_fields,
        "condition",
        rule.get("condition").and_then(Value::as_str).unwrap_or(""),
    );
    if let Some(annotations) = rule
        .get("annotations")
        .and_then(Value::as_object)
        .cloned()
        .filter(|value| !value.is_empty())
    {
        body.insert("annotations".to_string(), Value::Object(annotations));
        managed_fields.push("annotations".to_string());
    }
    let contact_points = extract_rule_contact_points(&rule);
    if !contact_points.is_empty() {
        body.insert(
            "contactPoints".to_string(),
            Value::Array(contact_points.into_iter().map(Value::String).collect()),
        );
        managed_fields.push("contactPoints".to_string());
    }
    let (datasource_uids, datasource_names, plugin_ids) = extract_rule_dependency_lists(&rule);
    if !datasource_uids.is_empty() {
        body.insert(
            "datasourceUids".to_string(),
            Value::Array(datasource_uids.into_iter().map(Value::String).collect()),
        );
        managed_fields.push("datasourceUids".to_string());
    }
    if !datasource_names.is_empty() {
        body.insert(
            "datasourceNames".to_string(),
            Value::Array(datasource_names.into_iter().map(Value::String).collect()),
        );
        managed_fields.push("datasourceNames".to_string());
    }
    if !plugin_ids.is_empty() {
        body.insert(
            "pluginIds".to_string(),
            Value::Array(plugin_ids.into_iter().map(Value::String).collect()),
        );
        managed_fields.push("pluginIds".to_string());
    }
    if let Some(data) = rule
        .get("data")
        .and_then(Value::as_array)
        .cloned()
        .filter(|value| !value.is_empty())
    {
        body.insert("data".to_string(), Value::Array(data));
        managed_fields.push("data".to_string());
    }

    Ok(serde_json::json!({
        "kind": "alert",
        "uid": uid,
        "title": title,
        "managedFields": managed_fields,
        "body": body,
        "sourcePath": source_path,
    }))
}

fn normalize_alert_resource_sync_spec(
    document: &Map<String, Value>,
    source_path: &str,
) -> Result<Option<Value>> {
    let document_kind = detect_document_kind(document)?;
    let Some(sync_kind) = map_alert_document_kind_to_sync_kind(document_kind) else {
        return Ok(None);
    };
    if sync_kind == "alert" {
        return Ok(Some(normalize_alert_rule_sync_spec(document, source_path)?));
    }
    let body = match document_kind {
        CONTACT_POINT_KIND => build_contact_point_import_payload(document)?,
        MUTE_TIMING_KIND => build_mute_timing_import_payload(document)?,
        POLICIES_KIND => build_policies_import_payload(document)?,
        TEMPLATE_KIND => build_template_import_payload(document)?,
        _ => return Ok(None),
    };
    let (identity, title) = normalize_alert_resource_identity_and_title(sync_kind, &body)?;
    Ok(Some(serde_json::json!({
        "kind": sync_kind,
        "uid": if sync_kind == "alert-contact-point" { identity.clone() } else { String::new() },
        "name": if matches!(sync_kind, "alert-mute-timing" | "alert-template") { identity.clone() } else { String::new() },
        "title": title,
        "managedFields": normalize_alert_managed_fields(&body),
        "body": body,
        "sourcePath": source_path,
    })))
}

fn build_alert_sync_specs(alerting: &Value) -> Result<Vec<Value>> {
    let mut alerts = Vec::new();
    let Some(alerting_object) = alerting.as_object() else {
        return Ok(alerts);
    };
    for (section, items) in alerting_object {
        let Some(items) = items.as_array() else {
            continue;
        };
        for item in items {
            let Some(object) = item.as_object() else {
                continue;
            };
            let source_path = object
                .get("sourcePath")
                .and_then(Value::as_str)
                .unwrap_or("");
            let Some(document) = object.get("document").and_then(Value::as_object) else {
                continue;
            };
            if section == "rules" {
                if let Some(groups) = document.get("groups").and_then(Value::as_array) {
                    for group in groups {
                        let Some(group_object) = group.as_object() else {
                            continue;
                        };
                        let Some(group_rules) = group_object.get("rules").and_then(Value::as_array)
                        else {
                            continue;
                        };
                        for rule in group_rules {
                            let Some(rule_object) = rule.as_object() else {
                                continue;
                            };
                            let normalized_rule =
                                normalize_rule_group_rule_document(group_object, rule_object);
                            alerts.push(normalize_alert_rule_sync_spec(
                                &normalized_rule,
                                source_path,
                            )?);
                        }
                    }
                    continue;
                }
            }
            if let Some(resource) = normalize_alert_resource_sync_spec(document, source_path)? {
                alerts.push(resource);
            }
        }
    }
    Ok(alerts)
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

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
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

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn render_alert_sync_assessment_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some(ALERT_SYNC_KIND) {
        return Err(message(
            "Alert sync assessment document kind is not supported.",
        ));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| message("Alert sync assessment document is missing summary."))?;
    let mut lines = vec![
        "Alert sync assessment".to_string(),
        format!(
            "Alerts: {} total, {} candidate, {} plan-only, {} blocked",
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
        String::new(),
        "# Alerts".to_string(),
    ];
    if let Some(items) = document.get("alerts").and_then(Value::as_array) {
        for item in items {
            if let Some(object) = item.as_object() {
                lines.push(format!(
                    "- {} status={} liveApplyAllowed={} detail={}",
                    object
                        .get("identity")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    object
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    if object
                        .get("liveApplyAllowed")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        "true"
                    } else {
                        "false"
                    },
                    object.get("detail").and_then(Value::as_str).unwrap_or(""),
                ));
            }
        }
    }
    Ok(lines)
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
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

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
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
            "Bundle preflight: resources={} sync-blocking={} provider-blocking={} alert-artifacts={} plan-only={} blocking={}",
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
            bundle_summary
                .get("alertArtifactCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            bundle_summary
                .get("alertArtifactPlanOnlyCount")
                .and_then(Value::as_i64)
                .unwrap_or(0),
            bundle_summary
                .get("alertArtifactBlockingCount")
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

fn load_operation_object(operation: &Value) -> Result<&Map<String, Value>> {
    require_json_object(operation, "Sync apply operation")
}

fn apply_folder_operation_with_request<F>(
    request_json: &mut F,
    operation: &Map<String, Value>,
    allow_folder_delete: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let action = operation
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("");
    let identity = operation
        .get("identity")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    let desired = operation
        .get("desired")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    match action {
        "would-create" => {
            let title = desired
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(identity);
            let mut payload = Map::new();
            payload.insert("uid".to_string(), Value::String(identity.to_string()));
            payload.insert("title".to_string(), Value::String(title.to_string()));
            if let Some(parent_uid) = desired
                .get("parentUid")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                payload.insert(
                    "parentUid".to_string(),
                    Value::String(parent_uid.to_string()),
                );
            }
            Ok(request_json(
                Method::POST,
                "/api/folders",
                &[],
                Some(&Value::Object(payload)),
            )?
            .unwrap_or(Value::Null))
        }
        "would-update" => Ok(request_json(
            Method::PUT,
            &format!("/api/folders/{identity}"),
            &[],
            Some(&Value::Object(desired)),
        )?
        .unwrap_or(Value::Null)),
        "would-delete" => {
            if !allow_folder_delete {
                return Err(message(format!(
                    "Refusing live folder delete for {identity} without --allow-folder-delete."
                )));
            }
            Ok(request_json(
                Method::DELETE,
                &format!("/api/folders/{identity}"),
                &[("forceDeleteRules".to_string(), "false".to_string())],
                None,
            )?
            .unwrap_or(Value::Null))
        }
        _ => Err(message(format!("Unsupported folder sync action {action}."))),
    }
}

fn apply_dashboard_operation_with_request<F>(
    request_json: &mut F,
    operation: &Map<String, Value>,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let action = operation
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("");
    let identity = operation
        .get("identity")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    if action == "would-delete" {
        return Ok(request_json(
            Method::DELETE,
            &format!("/api/dashboards/uid/{identity}"),
            &[],
            None,
        )?
        .unwrap_or(Value::Null));
    }
    let mut body = operation
        .get("desired")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    body.insert("uid".to_string(), Value::String(identity.to_string()));
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(identity);
    body.insert("title".to_string(), Value::String(title.to_string()));
    body.remove("id");
    let mut payload = Map::new();
    payload.insert("dashboard".to_string(), Value::Object(body.clone()));
    payload.insert(
        "overwrite".to_string(),
        Value::Bool(action == "would-update"),
    );
    if let Some(folder_uid) = body
        .get("folderUid")
        .or_else(|| body.get("folderUID"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        payload.insert(
            "folderUid".to_string(),
            Value::String(folder_uid.to_string()),
        );
    }
    Ok(request_json(
        Method::POST,
        "/api/dashboards/db",
        &[],
        Some(&Value::Object(payload)),
    )?
    .unwrap_or(Value::Null))
}

fn resolve_live_datasource_target_with_request<F>(
    request_json: &mut F,
    identity: &str,
) -> Result<Option<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let datasources = match request_json(Method::GET, "/api/datasources", &[], None)? {
        Some(Value::Array(items)) => items,
        Some(_) => return Err(message("Unexpected datasource list response from Grafana.")),
        None => Vec::new(),
    };
    for datasource in &datasources {
        let object = require_json_object(datasource, "Grafana datasource payload")?;
        if object.get("uid").and_then(Value::as_str).map(str::trim) == Some(identity) {
            return Ok(Some(object.clone()));
        }
    }
    for datasource in &datasources {
        let object = require_json_object(datasource, "Grafana datasource payload")?;
        if object.get("name").and_then(Value::as_str).map(str::trim) == Some(identity) {
            return Ok(Some(object.clone()));
        }
    }
    Ok(None)
}

fn apply_datasource_operation_with_request<F>(
    request_json: &mut F,
    operation: &Map<String, Value>,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let action = operation
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("");
    let identity = operation
        .get("identity")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    let mut body = operation
        .get("desired")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if !identity.is_empty() {
        body.entry("uid".to_string())
            .or_insert_with(|| Value::String(identity.to_string()));
    }
    let title = body
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(identity);
    body.insert("name".to_string(), Value::String(title.to_string()));
    match action {
        "would-create" => Ok(request_json(
            Method::POST,
            "/api/datasources",
            &[],
            Some(&Value::Object(body)),
        )?
        .unwrap_or(Value::Null)),
        "would-update" => {
            let target = resolve_live_datasource_target_with_request(request_json, identity)?
                .ok_or_else(|| {
                    message(format!(
                        "Could not resolve live datasource target {identity} during sync apply."
                    ))
                })?;
            let datasource_id = target
                .get("id")
                .map(|value| match value {
                    Value::String(text) => text.clone(),
                    _ => value.to_string(),
                })
                .filter(|value| !value.is_empty())
                .ok_or_else(|| message("Datasource sync update requires a live datasource id."))?;
            Ok(request_json(
                Method::PUT,
                &format!("/api/datasources/{datasource_id}"),
                &[],
                Some(&Value::Object(body)),
            )?
            .unwrap_or(Value::Null))
        }
        "would-delete" => {
            let target = resolve_live_datasource_target_with_request(request_json, identity)?
                .ok_or_else(|| {
                    message(format!(
                        "Could not resolve live datasource target {identity} during sync apply."
                    ))
                })?;
            let datasource_id = target
                .get("id")
                .map(|value| match value {
                    Value::String(text) => text.clone(),
                    _ => value.to_string(),
                })
                .filter(|value| !value.is_empty())
                .ok_or_else(|| message("Datasource sync delete requires a live datasource id."))?;
            Ok(request_json(
                Method::DELETE,
                &format!("/api/datasources/{datasource_id}"),
                &[],
                None,
            )?
            .unwrap_or(Value::Null))
        }
        _ => Err(message(format!(
            "Unsupported datasource sync action {action}."
        ))),
    }
}

fn apply_alert_operation_with_request<F>(
    request_json: &mut F,
    operation: &Map<String, Value>,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let kind = operation.get("kind").and_then(Value::as_str).unwrap_or("");
    let action = operation
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("");
    let identity = operation
        .get("identity")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    let desired = operation
        .get("desired")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    match action {
        "would-delete" => match kind {
            "alert" => {
                if identity.is_empty() {
                    return Err(message(
                        "Alert sync delete requires a stable uid identity for live apply.",
                    ));
                }
                Ok(request_json(
                    Method::DELETE,
                    &format!("/api/v1/provisioning/alert-rules/{identity}"),
                    &[],
                    None,
                )?
                .unwrap_or(Value::Null))
            }
            "alert-contact-point" => Ok(request_json(
                Method::DELETE,
                &format!("/api/v1/provisioning/contact-points/{identity}"),
                &[],
                None,
            )?
            .unwrap_or(Value::Null)),
            "alert-mute-timing" => Ok(request_json(
                Method::DELETE,
                &format!("/api/v1/provisioning/mute-timings/{identity}"),
                &[("version".to_string(), String::new())],
                None,
            )?
            .unwrap_or(Value::Null)),
            "alert-template" => Ok(request_json(
                Method::DELETE,
                &format!("/api/v1/provisioning/templates/{identity}"),
                &[("version".to_string(), String::new())],
                None,
            )?
            .unwrap_or(Value::Null)),
            "alert-policy" => Err(message(
                "Sync live delete is not wired for alert-policy resources yet.".to_string(),
            )),
            _ => Err(message(format!("Unsupported alert sync kind {kind}."))),
        },
        "would-create" | "would-update" => match kind {
            "alert" => {
                let mut payload = build_rule_import_payload(&desired)?;
                if !identity.is_empty() && !payload.contains_key("uid") {
                    payload.insert("uid".to_string(), Value::String(identity.to_string()));
                }
                let uid = payload
                    .get("uid")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        message("Alert sync live apply requires alert rule payloads with a uid.")
                    })?;
                let method = if action == "would-create" {
                    Method::POST
                } else {
                    Method::PUT
                };
                let path = if action == "would-create" {
                    "/api/v1/provisioning/alert-rules".to_string()
                } else {
                    format!("/api/v1/provisioning/alert-rules/{uid}")
                };
                Ok(
                    request_json(method, &path, &[], Some(&Value::Object(payload)))?
                        .unwrap_or(Value::Null),
                )
            }
            "alert-contact-point" => {
                let mut payload = build_contact_point_import_payload(&desired)?;
                if !identity.is_empty() && !payload.contains_key("uid") {
                    payload.insert("uid".to_string(), Value::String(identity.to_string()));
                }
                let method = if action == "would-create" {
                    Method::POST
                } else {
                    Method::PUT
                };
                let path = if action == "would-create" {
                    "/api/v1/provisioning/contact-points".to_string()
                } else {
                    format!("/api/v1/provisioning/contact-points/{identity}")
                };
                Ok(
                    request_json(method, &path, &[], Some(&Value::Object(payload)))?
                        .unwrap_or(Value::Null),
                )
            }
            "alert-mute-timing" => {
                let payload = build_mute_timing_import_payload(&desired)?;
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(identity);
                let method = if action == "would-create" {
                    Method::POST
                } else {
                    Method::PUT
                };
                let path = if action == "would-create" {
                    "/api/v1/provisioning/mute-timings".to_string()
                } else {
                    format!("/api/v1/provisioning/mute-timings/{name}")
                };
                Ok(
                    request_json(method, &path, &[], Some(&Value::Object(payload)))?
                        .unwrap_or(Value::Null),
                )
            }
            "alert-policy" => {
                let payload = build_policies_import_payload(&desired)?;
                Ok(request_json(
                    Method::PUT,
                    "/api/v1/provisioning/policies",
                    &[],
                    Some(&Value::Object(payload)),
                )?
                .unwrap_or(Value::Null))
            }
            "alert-template" => {
                let mut payload = build_template_import_payload(&desired)?;
                let name = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(identity)
                    .to_string();
                payload.remove("name");
                Ok(request_json(
                    Method::PUT,
                    &format!("/api/v1/provisioning/templates/{name}"),
                    &[],
                    Some(&Value::Object(payload)),
                )?
                .unwrap_or(Value::Null))
            }
            _ => Err(message(format!("Unsupported alert sync kind {kind}."))),
        },
        _ => Err(message(format!("Unsupported alert sync action {action}."))),
    }
}

fn execute_live_apply_with_request<F>(
    mut request_json: F,
    operations: &[Value],
    allow_folder_delete: bool,
) -> Result<Value>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut results = Vec::new();
    for operation in operations {
        let object = load_operation_object(operation)?;
        let kind = object.get("kind").and_then(Value::as_str).unwrap_or("");
        let identity = object
            .get("identity")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let action = object.get("action").and_then(Value::as_str).unwrap_or("");
        let response = match kind {
            "folder" => {
                apply_folder_operation_with_request(&mut request_json, object, allow_folder_delete)?
            }
            "dashboard" => apply_dashboard_operation_with_request(&mut request_json, object)?,
            "datasource" => apply_datasource_operation_with_request(&mut request_json, object)?,
            "alert"
            | "alert-contact-point"
            | "alert-mute-timing"
            | "alert-policy"
            | "alert-template" => apply_alert_operation_with_request(&mut request_json, object)?,
            _ => return Err(message(format!("Unsupported sync resource kind {kind}."))),
        };
        results.push(serde_json::json!({
            "kind": kind,
            "identity": identity,
            "action": action,
            "response": response,
        }));
    }
    Ok(serde_json::json!({
        "mode": "live-apply",
        "appliedCount": results.len(),
        "results": results,
    }))
}

fn execute_live_apply(
    common: &CommonCliArgs,
    org_id: Option<i64>,
    operations: &[Value],
    allow_folder_delete: bool,
) -> Result<Value> {
    let client = build_sync_http_client(common, org_id)?;
    execute_live_apply_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        operations,
        allow_folder_delete,
    )
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
    let alert_artifact_blocking_count = object
        .get("alertArtifactAssessment")
        .and_then(Value::as_object)
        .and_then(|assessment| assessment.get("summary"))
        .and_then(Value::as_object)
        .and_then(|summary| summary.get("blockedCount"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let alert_artifact_plan_only_count = object
        .get("alertArtifactAssessment")
        .and_then(Value::as_object)
        .and_then(|assessment| assessment.get("summary"))
        .and_then(Value::as_object)
        .and_then(|summary| summary.get("planOnlyCount"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let alert_artifact_count = object
        .get("alertArtifactAssessment")
        .and_then(Value::as_object)
        .and_then(|assessment| assessment.get("summary"))
        .and_then(Value::as_object)
        .and_then(|summary| summary.get("resourceCount"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let blocking_count =
        sync_blocking_count + provider_blocking_count + alert_artifact_blocking_count;
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
        "alertArtifactBlockingCount": alert_artifact_blocking_count,
        "alertArtifactPlanOnlyCount": alert_artifact_plan_only_count,
        "alertArtifactCount": alert_artifact_count,
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

fn run_sync_bundle(args: SyncBundleArgs) -> Result<()> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: sync.rs:run_sync_cli
    // Downstream callees: common.rs:message, sync.rs:build_alert_sync_specs, sync.rs:emit_text_or_json, sync.rs:load_alerting_bundle_section, sync.rs:load_dashboard_bundle_sections, sync.rs:load_json_array_file, sync.rs:load_optional_json_object_file, sync.rs:normalize_datasource_bundle_item, sync_workbench.rs:build_sync_source_bundle_document, sync_workbench.rs:render_sync_source_bundle_text

    if args.dashboard_export_dir.is_none()
        && args.alert_export_dir.is_none()
        && args.datasource_export_file.is_none()
        && args.metadata_file.is_none()
    {
        return Err(message(
            "Sync bundle requires at least one export input such as --dashboard-export-dir, --alert-export-dir, --datasource-export-file, or --metadata-file.",
        ));
    }
    let mut dashboards = Vec::new();
    let mut datasources = Vec::new();
    let mut folders = Vec::new();
    let mut metadata = Map::new();
    if let Some(export_dir) = args.dashboard_export_dir.as_ref() {
        let (dashboard_items, dashboard_datasources, folder_items, dashboard_metadata) =
            load_dashboard_bundle_sections(export_dir)?;
        dashboards = dashboard_items;
        datasources.extend(dashboard_datasources);
        folders = folder_items;
        metadata.extend(dashboard_metadata);
    }
    if let Some(datasource_export_file) = args.datasource_export_file.as_ref() {
        datasources = load_json_array_file(datasource_export_file, "Datasource export inventory")?
            .into_iter()
            .map(|item| normalize_datasource_bundle_item(&item))
            .collect::<Result<Vec<_>>>()?;
        metadata.insert(
            "datasourceExportFile".to_string(),
            Value::String(datasource_export_file.display().to_string()),
        );
    }
    let alerting = match args.alert_export_dir.as_ref() {
        Some(export_dir) => {
            metadata.insert(
                "alertExportDir".to_string(),
                Value::String(export_dir.display().to_string()),
            );
            load_alerting_bundle_section(export_dir)?
        }
        None => Value::Object(Map::new()),
    };
    let alerts = build_alert_sync_specs(&alerting)?;
    if let Some(extra_metadata) =
        load_optional_json_object_file(args.metadata_file.as_ref(), "Sync bundle metadata input")?
    {
        if let Some(object) = extra_metadata.as_object() {
            metadata.extend(object.clone());
        }
    }
    let document = build_sync_source_bundle_document(
        &dashboards,
        &datasources,
        &folders,
        &alerts,
        Some(&alerting),
        Some(&Value::Object(metadata)),
    )?;
    if let Some(output_file) = args.output_file.as_ref() {
        fs::write(
            output_file,
            format!("{}\n", serde_json::to_string_pretty(&document)?),
        )?;
    }
    emit_text_or_json(
        &document,
        render_sync_source_bundle_text(&document)?,
        args.output,
    )
}

/// run sync cli.
pub fn run_sync_cli(command: SyncGroupCommand) -> Result<()> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: sync_cli_rust_tests.rs:run_sync_cli_apply_accepts_explicit_audit_metadata, sync_cli_rust_tests.rs:run_sync_cli_apply_accepts_non_blocking_bundle_preflight_file, sync_cli_rust_tests.rs:run_sync_cli_apply_accepts_non_blocking_preflight_file, sync_cli_rust_tests.rs:run_sync_cli_apply_accepts_reviewed_plan_file, sync_cli_rust_tests.rs:run_sync_cli_apply_rejects_blocking_bundle_preflight_file, sync_cli_rust_tests.rs:run_sync_cli_apply_rejects_blocking_preflight_file, sync_cli_rust_tests.rs:run_sync_cli_apply_rejects_bundle_preflight_with_mismatched_trace_id, sync_cli_rust_tests.rs:run_sync_cli_apply_rejects_lineage_aware_bundle_preflight_with_mismatched_parent, sync_cli_rust_tests.rs:run_sync_cli_apply_rejects_lineage_aware_preflight_without_trace_id, sync_cli_rust_tests.rs:run_sync_cli_apply_rejects_missing_trace_id, sync_cli_rust_tests.rs:run_sync_cli_apply_rejects_plan_with_non_review_lineage, sync_cli_rust_tests.rs:run_sync_cli_apply_rejects_preflight_with_mismatched_trace_id ...
    // Downstream callees: alert_sync.rs:assess_alert_sync_specs, common.rs:message, sync.rs:attach_apply_audit, sync.rs:attach_bundle_preflight_summary, sync.rs:attach_lineage, sync.rs:attach_preflight_summary, sync.rs:attach_review_audit, sync.rs:attach_trace_id, sync.rs:emit_text_or_json, sync.rs:execute_live_apply, sync.rs:fetch_live_availability, sync.rs:fetch_live_resource_specs ...

    match command {
        SyncGroupCommand::Plan(args) => {
            let desired = load_json_array_file(&args.desired_file, "Sync desired input")?;
            let live = if args.fetch_live {
                fetch_live_resource_specs(&args.common, args.org_id, args.page_size)?
            } else {
                let live_file = args.live_file.as_ref().ok_or_else(|| {
                    message("Sync plan requires --live-file unless --fetch-live is used.")
                })?;
                load_json_array_file(live_file, "Sync live input")?
            };
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
            if args.execute_live {
                let operations = document
                    .get("operations")
                    .and_then(Value::as_array)
                    .cloned()
                    .ok_or_else(|| message("Sync apply intent document is missing operations."))?;
                let live_result = execute_live_apply(
                    &args.common,
                    args.org_id,
                    &operations,
                    args.allow_folder_delete,
                )?;
                emit_text_or_json(
                    &live_result,
                    vec![
                        "Sync live apply".to_string(),
                        format!(
                            "Applied: {}",
                            live_result
                                .get("appliedCount")
                                .and_then(Value::as_i64)
                                .unwrap_or(0)
                        ),
                    ],
                    args.output,
                )
            } else {
                emit_text_or_json(
                    &document,
                    render_sync_apply_intent_text(&document)?,
                    args.output,
                )
            }
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
            let availability = if args.fetch_live {
                Some(merge_availability(
                    availability,
                    &fetch_live_availability(&args.common, args.org_id)?,
                )?)
            } else {
                availability
            };
            let document = build_sync_preflight_document(&desired, availability.as_ref())?;
            emit_text_or_json(
                &document,
                render_sync_preflight_text(&document)?,
                args.output,
            )
        }
        SyncGroupCommand::AssessAlerts(args) => {
            let alerts = load_json_array_file(&args.alerts_file, "Alert sync input")?;
            let document = assess_alert_sync_specs(&alerts)?;
            emit_text_or_json(
                &document,
                render_alert_sync_assessment_text(&document)?,
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
            let availability = if args.fetch_live {
                Some(merge_availability(
                    availability,
                    &fetch_live_availability(&args.common, args.org_id)?,
                )?)
            } else {
                availability
            };
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
        SyncGroupCommand::Bundle(args) => run_sync_bundle(args),
    }
}

#[cfg(test)]
#[path = "cli_rust_tests.rs"]
mod sync_cli_rust_tests;

#[cfg(test)]
#[path = "rust_tests.rs"]
mod sync_rust_tests;
