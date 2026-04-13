//! Alerting domain entry and orchestration module.
//!
//! Purpose:
//! - Own the alerting command surface (`list`, `export`, `import`, `diff`).
//! - Bridge parsed CLI args to `GrafanaAlertClient` and alerting handlers.
//! - Keep response parsing and payload shaping close to alert domain types.
//!
//! Flow:
//! - Parse CLI args via `alert_cli_defs`.
//! - Normalize legacy/namespaced invocation forms before dispatch.
//! - Build client only in the concrete runtime entrypoint; keep pure routing paths testable.
//!
//! Caveats:
//! - Avoid adding transport policy here; retry/pagination behavior should stay in shared HTTP
//!   layers and alert handlers.
//! - Keep diff/import/export payload transforms next to their handlers, not in dispatcher code.

use crate::common::{message, sanitize_path_component, string_field, write_json_file, Result};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

#[path = "cli/mod.rs"]
mod alert_cli_defs;
#[path = "cli/runtime.rs"]
mod alert_cli_runtime;
#[path = "client.rs"]
mod alert_client;
#[path = "diff.rs"]
mod alert_compare_support;
#[path = "export.rs"]
mod alert_export;
#[path = "import_diff.rs"]
mod alert_import_diff;
#[path = "linkage.rs"]
mod alert_linkage_support;
#[path = "list.rs"]
mod alert_list;
#[path = "project_status/live.rs"]
mod alert_live_project_status;
#[path = "output.rs"]
mod alert_output;
#[path = "project_status/staged.rs"]
mod alert_project_status;
#[path = "runtime_support.rs"]
mod alert_runtime_support;
#[path = "support/mod.rs"]
mod alert_support;

#[cfg(test)]
pub(crate) use crate::grafana_api::alert_live::{
    determine_import_action_with_request, fetch_live_compare_document_with_request,
    import_resource_document_with_request,
};
#[cfg(test)]
pub(crate) use crate::grafana_api::{expect_object_list, parse_template_list_response};
pub use alert_cli_defs::{
    build_auth_context, cli_args_from_common, normalize_alert_group_command,
    normalize_alert_namespace_args, parse_cli_from, root_command, AlertAddContactPointArgs,
    AlertAddRuleArgs, AlertApplyArgs, AlertAuthContext, AlertAuthoringCommandKind, AlertCliArgs,
    AlertCloneRuleArgs, AlertCommandKind, AlertCommandOutputFormat, AlertCommonArgs,
    AlertDeleteArgs, AlertDiffArgs, AlertExportArgs, AlertGroupCommand, AlertImportArgs,
    AlertInitArgs, AlertLegacyArgs, AlertListArgs, AlertListKind, AlertNamespaceArgs,
    AlertNewResourceArgs, AlertPlanArgs, AlertPreviewRouteArgs, AlertResourceKind,
    AlertSetRouteArgs,
};
pub(crate) use alert_client::GrafanaAlertClient;
#[allow(unused_imports)]
pub(crate) use alert_compare_support::{
    append_root_index_item, build_compare_diff_text, build_compare_document,
    build_resource_identity, format_export_summary, serialize_compare_document,
    write_resource_indexes,
};
#[cfg(test)]
pub(crate) use alert_linkage_support::get_rule_linkage;
#[cfg(test)]
pub(crate) use alert_list::serialize_rule_list_rows;
pub use alert_live_project_status::{
    build_alert_live_project_status_domain, AlertLiveProjectStatusInputs,
};
#[cfg(test)]
pub(crate) use alert_output::render_alert_action_text;
pub(crate) use alert_project_status::build_alert_project_status_domain;
pub use alert_runtime_support::{
    apply_managed_policy_subtree_edit_document, build_alert_delete_preview_document,
    build_alert_delete_preview_from_dir, build_alert_delete_preview_from_files,
    build_alert_plan_document, build_alert_plan_with_request,
    build_managed_policy_edit_preview_document, execute_alert_plan_with_request,
    init_alert_runtime_layout, write_contact_point_scaffold, write_new_contact_point_scaffold,
    write_new_rule_scaffold, write_new_template_scaffold, ALERT_DELETE_PREVIEW_KIND,
    ALERT_PLAN_KIND,
};
pub use alert_runtime_support::{build_alert_diff_document, build_alert_import_dry_run_document};
pub use alert_support::{
    build_contact_point_export_document, build_contact_point_import_payload,
    build_contact_point_output_path, build_empty_root_index, build_import_operation,
    build_managed_policy_route_preview, build_mute_timing_export_document,
    build_mute_timing_import_payload, build_mute_timing_output_path,
    build_new_contact_point_scaffold_document, build_new_rule_scaffold_document,
    build_new_rule_scaffold_document_with_route, build_new_template_scaffold_document,
    build_policies_export_document, build_policies_import_payload, build_policies_output_path,
    build_resource_dirs, build_route_preview, build_rule_export_document,
    build_rule_import_payload, build_rule_output_path, build_simple_rule_body,
    build_stable_route_label_value, build_template_export_document, build_template_import_payload,
    build_template_output_path, derive_dashboard_slug, detect_document_kind,
    discover_alert_resource_files, init_alert_managed_dir, load_alert_resource_file,
    load_panel_id_map, load_string_map, normalize_compare_payload, reject_provisioning_export,
    resource_subdir_by_kind, strip_server_managed_fields, write_alert_resource_file,
};
pub(crate) use alert_support::{value_to_string, AlertLinkageMappings};

/// Constant for default url.
pub const DEFAULT_URL: &str = "http://127.0.0.1:3000";
/// Constant for default timeout.
pub const DEFAULT_TIMEOUT: u64 = 30;
/// Constant for default output dir.
pub const DEFAULT_OUTPUT_DIR: &str = "alerts";
/// Constant for raw export subdir.
pub const RAW_EXPORT_SUBDIR: &str = "raw";
/// Constant for rules subdir.
pub const RULES_SUBDIR: &str = "rules";
/// Constant for contact points subdir.
pub const CONTACT_POINTS_SUBDIR: &str = "contact-points";
/// Constant for mute timings subdir.
pub const MUTE_TIMINGS_SUBDIR: &str = "mute-timings";
/// Constant for policies subdir.
pub const POLICIES_SUBDIR: &str = "policies";
/// Constant for templates subdir.
pub const TEMPLATES_SUBDIR: &str = "templates";
/// Constant for rule kind.
pub const RULE_KIND: &str = "grafana-alert-rule";
/// Constant for contact point kind.
pub const CONTACT_POINT_KIND: &str = "grafana-contact-point";
/// Constant for mute timing kind.
pub const MUTE_TIMING_KIND: &str = "grafana-mute-timing";
/// Constant for policies kind.
pub const POLICIES_KIND: &str = "grafana-notification-policies";
/// Constant for template kind.
pub const TEMPLATE_KIND: &str = "grafana-notification-template";
/// Constant for tool api version.
pub const TOOL_API_VERSION: i64 = 1;
/// Constant for tool schema version.
pub const TOOL_SCHEMA_VERSION: i64 = 1;
/// Constant for root index kind.
pub const ROOT_INDEX_KIND: &str = "grafana-util-alert-export-index";

/// Constant for alert help text.
pub const ALERT_HELP_TEXT: &str = "Examples:\n\n  Inventory alert resources without learning the internal tree:\n    grafana-util alert list-rules --url https://grafana.example.com --token \"$GRAFANA_API_TOKEN\" --json\n\n  Back up alerting resources to local files:\n    grafana-util alert export --url https://grafana.example.com --output-dir ./alerts --overwrite\n\n  Import alerting resources back into Grafana:\n    grafana-util alert import --url https://grafana.example.com --input-dir ./alerts/raw --replace-existing\n\n  Compare a local alert export against Grafana:\n    grafana-util alert diff --url https://grafana.example.com --diff-dir ./alerts/raw --output-format json\n\n  Author a staged alert rule:\n    grafana-util alert add-rule --desired-dir ./alerts/desired --name cpu-high --folder platform-alerts --rule-group cpu --receiver pagerduty-primary --severity critical --expr 'A' --threshold 80 --above --for 5m";

fn resource_kind_to_document_kind(kind: AlertResourceKind) -> &'static str {
    match kind {
        AlertResourceKind::Rule => RULE_KIND,
        AlertResourceKind::ContactPoint => CONTACT_POINT_KIND,
        AlertResourceKind::MuteTiming => MUTE_TIMING_KIND,
        AlertResourceKind::PolicyTree => POLICIES_KIND,
        AlertResourceKind::Template => TEMPLATE_KIND,
    }
}

fn scaffold_output_path(
    desired_dir: &Path,
    subdir: &str,
    name: &str,
    file_suffix: &str,
) -> PathBuf {
    desired_dir
        .join(subdir)
        .join(format!("{}{file_suffix}", sanitize_path_component(name)))
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn require_desired_dir<'a>(args: &'a AlertCliArgs, label: &str) -> Result<&'a Path> {
    args.desired_dir
        .as_deref()
        .ok_or_else(|| message(format!("{label} requires --desired-dir.")))
}

fn require_scaffold_name<'a>(args: &'a AlertCliArgs, label: &str) -> Result<&'a str> {
    args.scaffold_name
        .as_deref()
        .ok_or_else(|| message(format!("{label} requires --name.")))
}

fn require_source_name<'a>(args: &'a AlertCliArgs, label: &str) -> Result<&'a str> {
    args.source_name
        .as_deref()
        .ok_or_else(|| message(format!("{label} requires --source.")))
}

fn parse_string_pairs(values: &[String], label: &str) -> Result<Map<String, Value>> {
    let mut pairs = Map::new();
    for entry in values {
        let Some((key, value)) = entry.split_once('=') else {
            return Err(message(format!(
                "{label} entries must use key=value form: {entry}"
            )));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(message(format!("{label} key cannot be empty: {entry}")));
        }
        pairs.insert(key.to_string(), Value::String(value.trim().to_string()));
    }
    Ok(pairs)
}

fn rule_output_path_for_name(desired_dir: &Path, name: &str) -> PathBuf {
    scaffold_output_path(desired_dir, RULES_SUBDIR, name, ".yaml")
}

fn contact_point_output_path_for_name(desired_dir: &Path, name: &str) -> PathBuf {
    scaffold_output_path(desired_dir, CONTACT_POINTS_SUBDIR, name, ".yaml")
}

fn preferred_policy_path(desired_dir: &Path) -> PathBuf {
    desired_dir
        .join(POLICIES_SUBDIR)
        .join("notification-policies.yaml")
}

fn resolve_policy_path(desired_dir: &Path) -> PathBuf {
    let candidates = [
        desired_dir
            .join(POLICIES_SUBDIR)
            .join("notification-policies.yaml"),
        desired_dir
            .join(POLICIES_SUBDIR)
            .join("notification-policies.yml"),
        desired_dir
            .join(POLICIES_SUBDIR)
            .join("notification-policies.json"),
    ];
    candidates
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| preferred_policy_path(desired_dir))
}

fn build_default_policy_document(receiver: &str) -> Value {
    let policy = json!({
        "receiver": if receiver.trim().is_empty() { "grafana-default-email" } else { receiver.trim() },
        "group_by": ["grafana_folder", "alertname"],
        "routes": [],
    });
    let mut document = build_policies_export_document(
        policy
            .as_object()
            .expect("default notification policies must be an object"),
    );
    if let Some(metadata) = document.get_mut("metadata").and_then(Value::as_object_mut) {
        metadata.insert(
            "managedBy".to_string(),
            Value::String("grafana-utils".to_string()),
        );
    }
    document
}

fn load_or_init_policy_document(desired_dir: &Path, receiver: &str) -> Result<(PathBuf, Value)> {
    let path = resolve_policy_path(desired_dir);
    if path.exists() {
        Ok((
            path.clone(),
            load_alert_resource_file(&path, "Notification policies")?,
        ))
    } else {
        Ok((path, build_default_policy_document(receiver)))
    }
}

fn extract_policy_spec(document: &Value) -> Result<Value> {
    if let Some(spec) = document.get("spec") {
        if spec.is_object() {
            return Ok(spec.clone());
        }
    }
    if document.is_object() {
        return Ok(document.clone());
    }
    Err(message(
        "Notification policies document must be a tool document or object.",
    ))
}

fn build_desired_route_document(args: &AlertCliArgs) -> Result<Option<Value>> {
    if args.no_route {
        return Ok(None);
    }
    let Some(receiver) = args.receiver.as_deref() else {
        return Ok(None);
    };
    let labels = parse_string_pairs(&args.labels, "Alert route label")?;
    let mut matchers = labels
        .into_iter()
        .map(|(key, value)| json!([key, "=", value_to_string(&value)]))
        .collect::<Vec<Value>>();
    if let Some(severity) = args.severity.as_deref() {
        matchers.push(json!(["severity", "=", severity]));
    }
    Ok(Some(json!({
        "receiver": receiver,
        "group_by": ["grafana_folder", "alertname"],
        "continue": false,
        "object_matchers": matchers,
    })))
}

fn find_existing_rule_document(desired_dir: &Path, source: &str) -> Result<(PathBuf, Value)> {
    let normalized_source = sanitize_path_component(source);
    for path in discover_alert_resource_files(desired_dir)? {
        let document = load_alert_resource_file(&path, "Alert source rule")?;
        let kind = document
            .as_object()
            .and_then(|item| detect_document_kind(item).ok());
        if kind != Some(RULE_KIND) {
            continue;
        }
        let metadata = document.get("metadata").and_then(Value::as_object);
        let spec = document.get("spec").and_then(Value::as_object);
        let candidates = [
            metadata
                .and_then(|item| item.get("uid"))
                .and_then(Value::as_str)
                .unwrap_or_default(),
            metadata
                .and_then(|item| item.get("title"))
                .and_then(Value::as_str)
                .unwrap_or_default(),
            spec.and_then(|item| item.get("uid"))
                .and_then(Value::as_str)
                .unwrap_or_default(),
            spec.and_then(|item| item.get("title"))
                .and_then(Value::as_str)
                .unwrap_or_default(),
        ];
        if candidates.iter().any(|candidate| {
            !candidate.is_empty()
                && (candidate == &source || sanitize_path_component(candidate) == normalized_source)
        }) {
            return Ok((path, document));
        }
    }
    Err(message(format!(
        "Could not find staged alert rule source {:?} under {}.",
        source,
        desired_dir.display()
    )))
}

/// Alert domain execution boundary.
///
/// Each parsed command path (authoring, plan/apply, export/import/diff/list) is reduced to
/// exactly one handler; this keeps all alert orchestration decisions in one switch.
pub fn run_alert_cli(args: AlertCliArgs) -> Result<()> {
    match args.authoring_command_kind {
        Some(AlertAuthoringCommandKind::AddRule) => {
            return alert_cli_runtime::run_alert_add_rule_cli(&args)
        }
        Some(AlertAuthoringCommandKind::CloneRule) => {
            return alert_cli_runtime::run_alert_clone_rule_cli(&args)
        }
        Some(AlertAuthoringCommandKind::AddContactPoint) => {
            return alert_cli_runtime::run_alert_add_contact_point_cli(&args)
        }
        Some(AlertAuthoringCommandKind::SetRoute) => {
            return alert_cli_runtime::run_alert_set_route_cli(&args)
        }
        Some(AlertAuthoringCommandKind::PreviewRoute) => {
            return alert_cli_runtime::run_alert_preview_route_cli(&args)
        }
        None => {}
    }

    match args.command_kind {
        Some(AlertCommandKind::Plan) => return alert_cli_runtime::run_alert_plan_cli(&args),
        Some(AlertCommandKind::Apply) => return alert_cli_runtime::run_alert_apply_cli(&args),
        Some(AlertCommandKind::Delete) => return alert_cli_runtime::run_alert_delete_cli(&args),
        Some(AlertCommandKind::Init) => return alert_cli_runtime::run_alert_init_cli(&args),
        Some(AlertCommandKind::NewRule) => return alert_cli_runtime::run_alert_new_rule_cli(&args),
        Some(AlertCommandKind::NewContactPoint) => {
            return alert_cli_runtime::run_alert_new_contact_point_cli(&args)
        }
        Some(AlertCommandKind::NewTemplate) => {
            return alert_cli_runtime::run_alert_new_template_cli(&args)
        }
        _ => {}
    }

    if args.list_kind.is_some() {
        return alert_list::list_alert_resources(&args);
    }
    if args.input_dir.is_some() {
        return alert_import_diff::import_alerting_resources(&args);
    }
    if args.diff_dir.is_some() {
        return alert_import_diff::diff_alerting_resources(&args);
    }
    alert_export::export_alerting_resources(&args)
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod alert_rust_tests;
