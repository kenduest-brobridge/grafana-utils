use crate::common::{message, sanitize_path_component, Result};
use crate::grafana_api::{GrafanaApiClient, GrafanaConnection};
use crate::http::JsonHttpClient;
use serde_json::{json, Map, Value};
use std::path::Path;

use super::alert_output::print_alert_action_document;
use super::{
    apply_managed_policy_subtree_edit_document, build_alert_delete_preview_document,
    build_auth_context, build_managed_policy_edit_preview_document,
    build_new_contact_point_scaffold_document, build_new_rule_scaffold_document_with_route,
    build_policies_export_document, build_route_preview, build_stable_route_label_value,
    contact_point_output_path_for_name, execute_alert_plan_with_request, extract_policy_spec,
    find_existing_rule_document, init_alert_runtime_layout, load_alert_resource_file,
    load_or_init_policy_document, path_string, require_desired_dir, require_scaffold_name,
    require_source_name, resource_kind_to_document_kind, rule_output_path_for_name,
    scaffold_output_path, value_to_string, write_alert_resource_file, write_contact_point_scaffold,
    write_new_contact_point_scaffold, write_new_rule_scaffold, write_new_template_scaffold,
    AlertCliArgs, AlertCommandOutputFormat, AlertResourceKind, CONTACT_POINTS_SUBDIR, RULES_SUBDIR,
    TEMPLATES_SUBDIR,
};

pub(super) fn build_alert_http_client(args: &AlertCliArgs) -> Result<JsonHttpClient> {
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

pub(super) fn build_explicit_delete_preview(args: &AlertCliArgs) -> Result<Value> {
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

pub(super) fn run_alert_plan_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = args
        .desired_dir
        .as_deref()
        .ok_or_else(|| message("Alert plan requires --desired-dir."))?;
    let client = build_alert_http_client(args)?;
    let document = super::build_alert_plan_with_request(
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

pub(super) fn run_alert_apply_cli(args: &AlertCliArgs) -> Result<()> {
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

pub(super) fn run_alert_delete_cli(args: &AlertCliArgs) -> Result<()> {
    let preview = build_explicit_delete_preview(args)?;
    print_alert_action_document(
        "Alert delete preview",
        &preview,
        args.command_output
            .unwrap_or(AlertCommandOutputFormat::Text),
    )
}

pub(super) fn run_alert_init_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = args
        .desired_dir
        .as_deref()
        .ok_or_else(|| message("Alert init requires --desired-dir."))?;
    let document = init_alert_runtime_layout(desired_dir)?;
    print_alert_action_document("Alert init", &document, AlertCommandOutputFormat::Text)
}

pub(super) fn run_alert_new_rule_cli(args: &AlertCliArgs) -> Result<()> {
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

pub(super) fn run_alert_new_contact_point_cli(args: &AlertCliArgs) -> Result<()> {
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

pub(super) fn run_alert_new_template_cli(args: &AlertCliArgs) -> Result<()> {
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
        } else if let Some(body) = Value::Object(super::build_simple_rule_body(
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

pub(super) fn run_alert_add_rule_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = require_desired_dir(args, "Alert add-rule")?;
    let name = require_scaffold_name(args, "Alert add-rule")?;
    let path = rule_output_path_for_name(desired_dir, name);
    let document = build_rule_authoring_document(args)?;
    let desired_route = super::build_desired_route_document(args)?;
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

pub(super) fn run_alert_clone_rule_cli(args: &AlertCliArgs) -> Result<()> {
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
    let desired_route = super::build_desired_route_document(args)?;
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

pub(super) fn run_alert_add_contact_point_cli(args: &AlertCliArgs) -> Result<()> {
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

pub(super) fn run_alert_set_route_cli(args: &AlertCliArgs) -> Result<()> {
    let desired_dir = require_desired_dir(args, "Alert set-route")?;
    let receiver = args
        .receiver
        .as_deref()
        .ok_or_else(|| message("Alert set-route requires --receiver."))?;
    let desired_route = super::build_desired_route_document(args)?
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

pub(super) fn run_alert_preview_route_cli(args: &AlertCliArgs) -> Result<()> {
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
