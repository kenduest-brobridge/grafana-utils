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

use crate::common::{
    message, render_json_value, sanitize_path_component, string_field, write_json_file, Result,
};
use crate::grafana_api::{GrafanaApiClient, GrafanaConnection};
use crate::http::JsonHttpClient;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

#[path = "alert_cli_defs.rs"]
mod alert_cli_defs;
#[path = "alert_client.rs"]
mod alert_client;
#[path = "alert_compare_support.rs"]
mod alert_compare_support;
#[path = "alert_export.rs"]
mod alert_export;
#[path = "alert_import_diff.rs"]
mod alert_import_diff;
#[path = "alert_linkage_support.rs"]
mod alert_linkage_support;
#[path = "alert_list.rs"]
mod alert_list;
#[path = "alert_live_project_status.rs"]
mod alert_live_project_status;
#[path = "alert_project_status.rs"]
mod alert_project_status;
#[path = "alert_runtime_support.rs"]
mod alert_runtime_support;
#[path = "alert_support.rs"]
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

fn build_alert_http_client(args: &AlertCliArgs) -> Result<JsonHttpClient> {
    let context = build_auth_context(args)?;
    Ok(GrafanaApiClient::from_connection(GrafanaConnection::new(
        context.url,
        context.headers,
        context.timeout,
        context.verify_ssl,
        None,
        "unknown".to_string(),
    ))?
    .into_http_client())
}

pub(crate) fn render_alert_action_text(title: &str, document: &Value) -> Vec<String> {
    let mut lines = vec![title.to_string()];
    if document
        .get("reviewRequired")
        .and_then(Value::as_bool)
        .is_some()
        || document.get("reviewed").and_then(Value::as_bool).is_some()
    {
        let review_required = document
            .get("reviewRequired")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let reviewed = document
            .get("reviewed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        lines.push(format!(
            "Review: required={review_required} reviewed={reviewed}"
        ));
    }
    if let Some(summary) = document.get("summary").and_then(Value::as_object) {
        let summary_line = summary
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(" ");
        if !summary_line.is_empty() {
            lines.push(format!("Summary: {summary_line}"));
        }
    }
    if let Some(rows) = document.get("rows").and_then(Value::as_array) {
        lines.push("Rows:".to_string());
        for row in rows.iter().take(20) {
            let kind = row.get("kind").and_then(Value::as_str).unwrap_or("unknown");
            let identity = row
                .get("identity")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let action = row
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let reason = row.get("reason").and_then(Value::as_str).unwrap_or("");
            if reason.is_empty() {
                lines.push(format!("- {kind} {identity} action={action}"));
            } else {
                lines.push(format!(
                    "- {kind} {identity} action={action} reason={reason}"
                ));
            }
        }
        if rows.len() > 20 {
            lines.push(format!("- ... {} more rows", rows.len() - 20));
        }
    }
    if let Some(results) = document.get("results").and_then(Value::as_array) {
        lines.push("Results:".to_string());
        for result in results.iter().take(20) {
            let kind = result
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let identity = result
                .get("identity")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let action = result
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            lines.push(format!("- {kind} {identity} action={action}"));
        }
        if results.len() > 20 {
            lines.push(format!("- ... {} more results", results.len() - 20));
        }
    }
    lines
}

fn print_alert_action_document(
    title: &str,
    document: &Value,
    output: AlertCommandOutputFormat,
) -> Result<()> {
    match output {
        AlertCommandOutputFormat::Json => {
            println!("{}", render_json_value(document)?);
            Ok(())
        }
        AlertCommandOutputFormat::Text => {
            for line in render_alert_action_text(title, document) {
                println!("{line}");
            }
            Ok(())
        }
    }
}

fn resource_kind_to_document_kind(kind: AlertResourceKind) -> &'static str {
    match kind {
        AlertResourceKind::Rule => RULE_KIND,
        AlertResourceKind::ContactPoint => CONTACT_POINT_KIND,
        AlertResourceKind::MuteTiming => MUTE_TIMING_KIND,
        AlertResourceKind::PolicyTree => POLICIES_KIND,
        AlertResourceKind::Template => TEMPLATE_KIND,
    }
}

fn build_explicit_delete_preview(args: &AlertCliArgs) -> Result<Value> {
    let kind = args
        .resource_kind
        .ok_or_else(|| message("Alert delete requires a resource kind."))?;
    let identity = args
        .resource_identity
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| message("Alert delete requires a non-empty identity."))?;
    let document_kind = resource_kind_to_document_kind(kind);
    let blocked = matches!(kind, AlertResourceKind::PolicyTree) && !args.allow_policy_reset;
    Ok(build_alert_delete_preview_document(
        &[json!({
            "path": Value::Null,
            "kind": document_kind,
            "identity": identity,
            "action": if blocked { "blocked" } else { "delete" },
            "reason": if blocked {
                "policy-reset-requires-allow-policy-reset"
            } else {
                "explicit-delete-request"
            },
            "desired": Value::Null,
        })],
        args.allow_policy_reset,
    ))
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

fn build_simple_expression_data(
    expr: Option<&str>,
    threshold: Option<f64>,
    _above: bool,
    below: bool,
) -> Vec<Value> {
    let evaluator_type = if below { "lt" } else { "gt" };
    let evaluator_value = threshold.unwrap_or(0.0);
    let expression = expr.unwrap_or("A");
    vec![json!({
        "refId": "A",
        "relativeTimeRange": { "from": 0, "to": 0 },
        "datasourceUid": "__expr__",
        "model": {
            "refId": "A",
            "type": "classic_conditions",
            "datasource": { "type": "__expr__", "uid": "__expr__" },
            "expression": expression,
            "conditions": [{
                "type": "query",
                "query": { "params": ["A"] },
                "reducer": { "type": "last", "params": [] },
                "evaluator": { "type": evaluator_type, "params": [evaluator_value] },
                "operator": { "type": "and" }
            }],
            "intervalMs": 1000,
            "maxDataPoints": 43200
        }
    })]
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

fn build_rule_authoring_document(args: &AlertCliArgs) -> Result<Value> {
    let name = require_scaffold_name(args, "Alert add-rule")?;
    let folder = args
        .folder
        .as_deref()
        .ok_or_else(|| message("Alert add-rule requires --folder."))?;
    let rule_group = args
        .rule_group
        .as_deref()
        .ok_or_else(|| message("Alert add-rule requires --rule-group."))?;
    let route_name = args.receiver.as_deref().unwrap_or(name);
    let folder_uid = sanitize_path_component(folder);
    if folder_uid.is_empty() {
        return Err(message(
            "Alert add-rule requires a non-empty --folder value.",
        ));
    }
    let mut document =
        build_new_rule_scaffold_document_with_route(name, &folder_uid, rule_group, route_name);
    let labels = parse_string_pairs(&args.labels, "Alert rule label")?;
    let annotations = parse_string_pairs(&args.annotations, "Alert annotation")?;
    if let Some(spec) = document.get_mut("spec").and_then(Value::as_object_mut) {
        if let Some(rule_labels) = spec.get_mut("labels").and_then(Value::as_object_mut) {
            for (key, value) in labels {
                rule_labels.insert(key, value);
            }
            if let Some(severity) = args.severity.as_deref() {
                rule_labels.insert("severity".to_string(), Value::String(severity.to_string()));
            }
        }
        if let Some(rule_annotations) = spec.get_mut("annotations").and_then(Value::as_object_mut) {
            for (key, value) in annotations {
                rule_annotations.insert(key, value);
            }
        }
        if let Some(for_duration) = args.for_duration.as_deref() {
            spec.insert("for".to_string(), Value::String(for_duration.to_string()));
        }
        if args.expr.is_some() || args.threshold.is_some() || args.above || args.below {
            spec.insert(
                "data".to_string(),
                Value::Array(build_simple_expression_data(
                    args.expr.as_deref(),
                    args.threshold,
                    args.above,
                    args.below,
                )),
            );
            spec.insert("condition".to_string(), Value::String("A".to_string()));
        } else if let Some(body) = Value::Object(build_simple_rule_body(
            name,
            &folder_uid,
            rule_group,
            route_name,
        ))
        .get("data")
        {
            spec.insert("data".to_string(), body.clone());
        }
    }
    Ok(document)
}

fn build_route_preview_matches(current_policy: &Map<String, Value>, args: &AlertCliArgs) -> Value {
    let mut labels = parse_string_pairs(&args.labels, "Alert preview label").unwrap_or_default();
    if let Some(severity) = args.severity.as_deref() {
        labels.insert("severity".to_string(), Value::String(severity.to_string()));
    }
    let routes = current_policy
        .get("routes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let matches = routes
        .into_iter()
        .filter_map(|route| route.as_object().cloned())
        .filter(|route| {
            route
                .get("object_matchers")
                .and_then(Value::as_array)
                .map(|matchers| {
                    matchers.iter().all(|matcher| {
                        let parts = matcher.as_array().cloned().unwrap_or_default();
                        if parts.len() != 3 {
                            return false;
                        }
                        let key = parts[0].as_str().unwrap_or_default();
                        let op = parts[1].as_str().unwrap_or_default();
                        let value = parts[2].as_str().unwrap_or_default();
                        if key == "grafana_utils_route" {
                            return true;
                        }
                        op == "="
                            && labels
                                .get(key)
                                .map(|item| value_to_string(item) == value)
                                .unwrap_or(false)
                    })
                })
                .unwrap_or(false)
        })
        .map(|route| build_route_preview(&route))
        .collect::<Vec<Value>>();
    json!(matches)
}

// Alert handlers are split by semantic family (plan/apply, authoring, export/diff/list)
// and each returns a structured action document via a common printer.
// Plan-only run paths request from live and render the resulting action plan.
fn run_alert_plan_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = args
        .desired_dir
        .as_deref()
        .ok_or_else(|| message("Alert plan requires --desired-dir."))?;
    let client = build_alert_http_client(args)?;
    let document = build_alert_plan_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        desired_dir,
        args.prune,
    )?;
    print_alert_action_document(
        "Alert plan",
        &document,
        args.command_output
            .unwrap_or(AlertCommandOutputFormat::Text),
    )
}

// Apply path is explicit-gate only; execution requires --approve and emits apply plan results.
fn run_alert_apply_cli(args: &AlertCliArgs) -> Result<()> {
    let plan_file = args
        .plan_file
        .as_deref()
        .ok_or_else(|| message("Alert apply requires --plan-file."))?;
    if !args.approve {
        return Err(message(
            "Alert apply requires --approve before live execution is allowed.",
        ));
    }
    let document = load_alert_resource_file(plan_file, "Alert plan")?;
    let client = build_alert_http_client(args)?;
    let result = execute_alert_plan_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        &document,
        args.allow_policy_reset,
    )?;
    print_alert_action_document(
        "Alert apply",
        &result,
        args.command_output
            .unwrap_or(AlertCommandOutputFormat::Text),
    )
}

// Delete handler builds a pure preview for local review and prints it as alert action output.
fn run_alert_delete_cli(args: &AlertCliArgs) -> Result<()> {
    let preview = build_explicit_delete_preview(args)?;
    print_alert_action_document(
        "Alert delete preview",
        &preview,
        args.command_output
            .unwrap_or(AlertCommandOutputFormat::Text),
    )
}

// Init path creates the local alert runtime directory layout and emits the bootstrap plan.
fn run_alert_init_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = args
        .desired_dir
        .as_deref()
        .ok_or_else(|| message("Alert init requires --desired-dir."))?;
    let document = init_alert_runtime_layout(desired_dir)?;
    print_alert_action_document("Alert init", &document, AlertCommandOutputFormat::Text)
}

// new-rule builds a scaffold document at the requested export location.
fn run_alert_new_rule_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = args
        .desired_dir
        .as_deref()
        .ok_or_else(|| message("Alert new-rule requires --desired-dir."))?;
    let name = args
        .scaffold_name
        .as_deref()
        .ok_or_else(|| message("Alert new-rule requires --name."))?;
    let path = scaffold_output_path(desired_dir, RULES_SUBDIR, name, ".yaml");
    let document = write_new_rule_scaffold(&path, name, false)?;
    print_alert_action_document(
        "Alert new-rule",
        &json!({"path": path, "document": document}),
        AlertCommandOutputFormat::Text,
    )
}

// new-contact-point writes a webhook contact-point scaffold into the alert bundle.
fn run_alert_new_contact_point_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = args
        .desired_dir
        .as_deref()
        .ok_or_else(|| message("Alert new-contact-point requires --desired-dir."))?;
    let name = args
        .scaffold_name
        .as_deref()
        .ok_or_else(|| message("Alert new-contact-point requires --name."))?;
    let path = scaffold_output_path(desired_dir, CONTACT_POINTS_SUBDIR, name, ".yaml");
    let document = write_new_contact_point_scaffold(&path, name, false)?;
    print_alert_action_document(
        "Alert new-contact-point",
        &json!({"path": path, "document": document}),
        AlertCommandOutputFormat::Text,
    )
}

// new-template writes a scaffold template document into the alert bundle.
fn run_alert_new_template_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = args
        .desired_dir
        .as_deref()
        .ok_or_else(|| message("Alert new-template requires --desired-dir."))?;
    let name = args
        .scaffold_name
        .as_deref()
        .ok_or_else(|| message("Alert new-template requires --name."))?;
    let path = scaffold_output_path(desired_dir, TEMPLATES_SUBDIR, name, ".yaml");
    let document = write_new_template_scaffold(&path, name, false)?;
    print_alert_action_document(
        "Alert new-template",
        &json!({"path": path, "document": document}),
        AlertCommandOutputFormat::Text,
    )
}

fn maybe_update_managed_route(
    desired_dir: &Path,
    args: &AlertCliArgs,
    route_name: &str,
    desired_route: Option<&Value>,
    dry_run: bool,
) -> Result<Option<Value>> {
    let (policy_path, current_policy_document) =
        load_or_init_policy_document(desired_dir, args.receiver.as_deref().unwrap_or(route_name))?;
    let current_policy_spec = extract_policy_spec(&current_policy_document)?;
    let preview = build_managed_policy_edit_preview_document(
        &current_policy_spec,
        route_name,
        desired_route,
    )?;
    let next_policy = if dry_run {
        None
    } else {
        let next_policy_document = apply_managed_policy_subtree_edit_document(
            &current_policy_spec,
            route_name,
            desired_route,
        )?;
        let spec = next_policy_document
            .get("spec")
            .and_then(Value::as_object)
            .ok_or_else(|| message("Managed policy edit did not return a policy spec."))?;
        let mut document = build_policies_export_document(spec);
        if let Some(metadata) = document.get_mut("metadata").and_then(Value::as_object_mut) {
            metadata.insert(
                "managedBy".to_string(),
                Value::String("grafana-utils".to_string()),
            );
        }
        write_alert_resource_file(&policy_path, &document, true)?;
        Some(json!({
            "path": path_string(&policy_path),
            "document": document,
        }))
    };
    Ok(Some(json!({
        "path": path_string(&policy_path),
        "preview": preview,
        "result": next_policy,
    })))
}

// add-rule creates a complete rule document, optionally updates managed routing policy,
// and persists output unless dry-run is requested.
fn run_alert_add_rule_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = require_desired_dir(args, "Alert add-rule")?;
    let name = require_scaffold_name(args, "Alert add-rule")?;
    let path = rule_output_path_for_name(desired_dir, name);
    let document = build_rule_authoring_document(args)?;
    let desired_route = build_desired_route_document(args)?;
    let route_effect = if let Some(route) = desired_route.as_ref() {
        maybe_update_managed_route(
            desired_dir,
            args,
            args.receiver.as_deref().unwrap_or(name),
            Some(route),
            args.dry_run,
        )?
    } else {
        None
    };
    if !args.dry_run {
        write_alert_resource_file(&path, &document, false)?;
    }
    print_alert_action_document(
        "Alert add-rule",
        &json!({
            "path": path_string(&path),
            "dryRun": args.dry_run,
            "document": document,
            "route": route_effect,
        }),
        AlertCommandOutputFormat::Json,
    )
}

// clone-rule duplicates a source rule, rewrites identifiers, and re-bases managed route as needed.
fn run_alert_clone_rule_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = require_desired_dir(args, "Alert clone-rule")?;
    let source = require_source_name(args, "Alert clone-rule")?;
    let name = require_scaffold_name(args, "Alert clone-rule")?;
    let (_, mut document) = find_existing_rule_document(desired_dir, source)?;
    let route_name = args.receiver.as_deref().unwrap_or(name);
    if let Some(spec) = document.get_mut("spec").and_then(Value::as_object_mut) {
        spec.insert(
            "uid".to_string(),
            Value::String(sanitize_path_component(name)),
        );
        spec.insert("title".to_string(), Value::String(name.to_string()));
        if let Some(folder) = args.folder.as_deref() {
            spec.insert(
                "folderUID".to_string(),
                Value::String(sanitize_path_component(folder)),
            );
        }
        if let Some(rule_group) = args.rule_group.as_deref() {
            spec.insert(
                "ruleGroup".to_string(),
                Value::String(rule_group.to_string()),
            );
        }
        if let Some(labels) = spec.get_mut("labels").and_then(Value::as_object_mut) {
            labels.insert(
                "grafana_utils_route".to_string(),
                Value::String(build_stable_route_label_value(route_name)),
            );
            if let Some(severity) = args.severity.as_deref() {
                labels.insert("severity".to_string(), Value::String(severity.to_string()));
            }
        }
    }
    if let Some(metadata) = document.get_mut("metadata").and_then(Value::as_object_mut) {
        metadata.insert(
            "uid".to_string(),
            Value::String(sanitize_path_component(name)),
        );
        metadata.insert("title".to_string(), Value::String(name.to_string()));
        if let Some(folder) = args.folder.as_deref() {
            metadata.insert(
                "folder".to_string(),
                json!({
                    "folderUid": sanitize_path_component(folder),
                    "folderTitle": folder,
                    "resolution": "uid-or-title",
                }),
            );
        }
        metadata.insert(
            "route".to_string(),
            json!({
                "labelKey": "grafana_utils_route",
                "labelValue": build_stable_route_label_value(route_name),
            }),
        );
    }
    let desired_route = build_desired_route_document(args)?;
    let route_effect = if let Some(route) = desired_route.as_ref() {
        maybe_update_managed_route(desired_dir, args, route_name, Some(route), args.dry_run)?
    } else {
        None
    };
    let path = rule_output_path_for_name(desired_dir, name);
    if !args.dry_run {
        write_alert_resource_file(&path, &document, false)?;
    }
    print_alert_action_document(
        "Alert clone-rule",
        &json!({
            "path": path_string(&path),
            "dryRun": args.dry_run,
            "document": document,
            "route": route_effect,
        }),
        AlertCommandOutputFormat::Json,
    )
}

// add-contact-point builds a scaffold or updates it persistently for webhook contacts.
fn run_alert_add_contact_point_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = require_desired_dir(args, "Alert add-contact-point")?;
    let name = require_scaffold_name(args, "Alert add-contact-point")?;
    let path = contact_point_output_path_for_name(desired_dir, name);
    if args.dry_run {
        let document = build_new_contact_point_scaffold_document(name);
        return print_alert_action_document(
            "Alert add-contact-point",
            &json!({
                "path": path_string(&path),
                "dryRun": true,
                "document": document,
            }),
            AlertCommandOutputFormat::Json,
        );
    }
    let document = write_contact_point_scaffold(&path, name, "webhook", false)?;
    print_alert_action_document(
        "Alert add-contact-point",
        &json!({
            "path": path_string(&path),
            "dryRun": false,
            "document": document,
        }),
        AlertCommandOutputFormat::Json,
    )
}

// set-route updates managed receiver routing for the selected receiver in-policy state.
fn run_alert_set_route_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = require_desired_dir(args, "Alert set-route")?;
    let receiver = args
        .receiver
        .as_deref()
        .ok_or_else(|| message("Alert set-route requires --receiver."))?;
    let desired_route = build_desired_route_document(args)?
        .ok_or_else(|| message("Alert set-route requires route content."))?;
    let route_effect = maybe_update_managed_route(
        desired_dir,
        args,
        receiver,
        Some(&desired_route),
        args.dry_run,
    )?;
    print_alert_action_document(
        "Alert set-route",
        &json!({
            "receiver": receiver,
            "dryRun": args.dry_run,
            "routeDocument": desired_route,
            "route": route_effect,
        }),
        AlertCommandOutputFormat::Json,
    )
}

// preview-route simulates route matching against current policies and prints matcher results.
fn run_alert_preview_route_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = require_desired_dir(args, "Alert preview-route")?;
    let (_, current_policy_document) = load_or_init_policy_document(desired_dir, "")?;
    let current_policy_spec = extract_policy_spec(&current_policy_document)?;
    let current_policy = current_policy_spec
        .as_object()
        .ok_or_else(|| message("Notification policies spec must be an object."))?;
    print_alert_action_document(
        "Alert preview-route",
        &json!({
            "input": {
                "labels": parse_string_pairs(&args.labels, "Alert preview label")?,
                "severity": args.severity,
            },
            "matches": build_route_preview_matches(current_policy, args),
        }),
        AlertCommandOutputFormat::Json,
    )
}

/// Alert domain execution boundary.
///
/// Each parsed command path (authoring, plan/apply, export/import/diff/list) is reduced to
/// exactly one handler; this keeps all alert orchestration decisions in one switch.
pub fn run_alert_cli(args: AlertCliArgs) -> Result<()> {
    match args.authoring_command_kind {
        Some(AlertAuthoringCommandKind::AddRule) => return run_alert_add_rule_cli(&args),
        Some(AlertAuthoringCommandKind::CloneRule) => return run_alert_clone_rule_cli(&args),
        Some(AlertAuthoringCommandKind::AddContactPoint) => {
            return run_alert_add_contact_point_cli(&args)
        }
        Some(AlertAuthoringCommandKind::SetRoute) => return run_alert_set_route_cli(&args),
        Some(AlertAuthoringCommandKind::PreviewRoute) => return run_alert_preview_route_cli(&args),
        None => {}
    }

    match args.command_kind {
        Some(AlertCommandKind::Plan) => return run_alert_plan_cli(&args),
        Some(AlertCommandKind::Apply) => return run_alert_apply_cli(&args),
        Some(AlertCommandKind::Delete) => return run_alert_delete_cli(&args),
        Some(AlertCommandKind::Init) => return run_alert_init_cli(&args),
        Some(AlertCommandKind::NewRule) => return run_alert_new_rule_cli(&args),
        Some(AlertCommandKind::NewContactPoint) => return run_alert_new_contact_point_cli(&args),
        Some(AlertCommandKind::NewTemplate) => return run_alert_new_template_cli(&args),
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
#[path = "alert_rust_tests.rs"]
mod alert_rust_tests;
