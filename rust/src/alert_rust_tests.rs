//! Alert domain test suite.
//! Validates export/import/diff document shaping, kind detection, and CLI parser/help
//! behavior.
use super::{alert_list, alert_runtime_support, alert_support};
use super::{
    build_alert_delete_preview_from_files, build_alert_diff_document,
    build_alert_import_dry_run_document, build_alert_live_project_status_domain,
    build_alert_plan_document, build_alert_plan_with_request, build_alert_project_status_domain,
    build_compare_diff_text, build_compare_document, build_contact_point_export_document,
    build_contact_point_output_path, build_empty_root_index, build_import_operation,
    build_managed_policy_route_preview, build_mute_timing_export_document,
    build_new_rule_scaffold_document_with_route, build_route_preview, build_rule_export_document,
    build_rule_output_path, build_stable_route_label_value, build_template_export_document,
    detect_document_kind, determine_import_action_with_request, execute_alert_plan_with_request,
    expect_object_list, fetch_live_compare_document_with_request, get_rule_linkage,
    import_resource_document_with_request, init_alert_runtime_layout, load_alert_resource_file,
    load_panel_id_map, load_string_map, normalize_compare_payload, parse_cli_from,
    parse_template_list_response, render_alert_action_text, resource_subdir_by_kind, root_command,
    run_alert_cli, serialize_compare_document, serialize_rule_list_rows,
    write_contact_point_scaffold, write_new_contact_point_scaffold, write_new_rule_scaffold,
    write_new_template_scaffold, AlertCliArgs, AlertListKind, AlertLiveProjectStatusInputs,
    CONTACT_POINT_KIND, MUTE_TIMING_KIND, POLICIES_KIND, ROOT_INDEX_KIND, RULE_KIND, TEMPLATE_KIND,
    TOOL_API_VERSION, TOOL_SCHEMA_VERSION,
};
use crate::common::{message, Result, TOOL_VERSION};
use alert_list::serialize_contact_point_list_rows;
use reqwest::Method;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[path = "alert_rust_tests_authoring.rs"]
mod alert_rust_tests_authoring;
#[path = "alert_rust_tests_compare_diff.rs"]
mod alert_rust_tests_compare_diff;
#[path = "alert_rust_tests_contract.rs"]
mod alert_rust_tests_contract;
#[path = "alert_rust_tests_parser_help.rs"]
mod alert_rust_tests_parser_help;
#[path = "alert_rust_tests_runtime.rs"]
mod alert_rust_tests_runtime;

fn render_alert_help() -> String {
    let mut command = root_command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn render_alert_subcommand_help(path: &[&str]) -> String {
    let mut command = root_command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing alert subcommand help for {segment}"));
    }
    let mut output = Vec::new();
    current.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn load_alert_export_contract_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../fixtures/alert_export_contract_cases.json"
    ))
    .unwrap()
}

fn load_alert_recreate_contract_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../fixtures/alert_recreate_contract_cases.json"
    ))
    .unwrap()
}

fn load_shared_diff_golden_fixture(domain: &str) -> Value {
    serde_json::from_str::<Vec<Value>>(include_str!("../../fixtures/shared_diff_golden_cases.json"))
        .unwrap()
        .into_iter()
        .find(|value| value.get("domain").and_then(Value::as_str) == Some(domain))
        .map(resolve_tool_version_placeholder)
        .expect("shared diff golden fixture")
}

fn resolve_tool_version_placeholder(mut value: Value) -> Value {
    match &mut value {
        Value::String(text) if text == "__TOOL_VERSION__" => {
            *text = TOOL_VERSION.to_string();
        }
        Value::Array(items) => {
            for item in items {
                *item = resolve_tool_version_placeholder(item.clone());
            }
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                *item = resolve_tool_version_placeholder(item.clone());
            }
        }
        _ => {}
    }
    value
}

fn write_pretty_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(value).unwrap()),
    )
    .unwrap();
}

fn assert_exact_object_keys(value: &Value, expected_keys: &[&str]) {
    let object = value.as_object().expect("expected JSON object");
    let actual_keys = object.keys().cloned().collect::<BTreeSet<_>>();
    let expected_keys = expected_keys
        .iter()
        .map(|key| (*key).to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_keys, expected_keys);
}

fn run_alert_recreate_case(
    kind: &str,
    payload: serde_json::Map<String, Value>,
    identity: &str,
    expected_dry_run_action: &str,
    expected_replay_action: &str,
    request_contract: serde_json::Map<String, Value>,
) {
    let remote_resources = RefCell::new(Vec::<Value>::new());
    let request_log = RefCell::new(Vec::<String>::new());

    let initial_compare = fetch_live_compare_document_with_request(
        |method, path, _params, payload_json| -> Result<Option<Value>> {
            handle_alert_runtime_request(
                &request_log,
                &remote_resources,
                kind,
                method,
                path,
                payload_json,
            )
        },
        kind,
        &payload,
    )
    .unwrap();
    assert!(
        initial_compare.is_none(),
        "expected missing remote for {kind}"
    );

    let action = determine_import_action_with_request(
        |method, path, _params, payload_json| -> Result<Option<Value>> {
            handle_alert_runtime_request(
                &request_log,
                &remote_resources,
                kind,
                method,
                path,
                payload_json,
            )
        },
        kind,
        &payload,
        true,
    )
    .unwrap();
    assert_eq!(
        action, expected_dry_run_action,
        "expected {expected_dry_run_action} for {kind}"
    );

    let (replay_action, replay_identity) = import_resource_document_with_request(
        |method, path, _params, payload_json| -> Result<Option<Value>> {
            handle_alert_runtime_request(
                &request_log,
                &remote_resources,
                kind,
                method,
                path,
                payload_json,
            )
        },
        kind,
        &payload,
        true,
    )
    .unwrap();
    assert_eq!(
        replay_action, expected_replay_action,
        "expected {expected_replay_action} for {kind}"
    );
    assert_eq!(
        replay_identity, identity,
        "expected identity parity for {kind}"
    );

    let live_compare = fetch_live_compare_document_with_request(
        |method, path, _params, payload_json| -> Result<Option<Value>> {
            handle_alert_runtime_request(
                &request_log,
                &remote_resources,
                kind,
                method,
                path,
                payload_json,
            )
        },
        kind,
        &payload,
    )
    .unwrap()
    .unwrap();

    assert_eq!(
        serialize_compare_document(&live_compare).unwrap(),
        serialize_compare_document(&build_compare_document(
            kind,
            &normalize_compare_payload(kind, &payload),
        ))
        .unwrap(),
        "expected same-state after recreate for {kind}"
    );

    let create_request_prefix = request_contract
        .get("createRequestPrefix")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let create_request_count = request_contract
        .get("createRequestCount")
        .and_then(Value::as_u64)
        .unwrap_or(1) as usize;
    let disallow_update_prefix = request_contract
        .get("disallowUpdatePrefix")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let create_like_count = request_log
        .borrow()
        .iter()
        .filter(|entry| entry.starts_with(create_request_prefix))
        .count();
    assert!(
        create_like_count == create_request_count,
        "expected {create_request_count} create replay request(s) for {kind}, got {create_like_count}"
    );
    if !disallow_update_prefix.is_empty() {
        assert!(
            !request_log
                .borrow()
                .iter()
                .any(|entry| entry.starts_with(disallow_update_prefix)),
            "unexpected update path during recreate for {kind}"
        );
    }
}

fn handle_alert_runtime_request(
    request_log: &RefCell<Vec<String>>,
    remote_resources: &RefCell<Vec<Value>>,
    kind: &str,
    method: Method,
    path: &str,
    payload: Option<&Value>,
) -> Result<Option<Value>> {
    let method_name = method.as_str().to_string();
    request_log
        .borrow_mut()
        .push(format!("{} {}", method_name, path));
    match kind {
        RULE_KIND => match (method, path) {
            (Method::GET, path) if path.starts_with("/api/v1/provisioning/alert-rules/") => {
                Ok(remote_resources
                    .borrow()
                    .iter()
                    .find(|item| item.get("uid").and_then(Value::as_str) == path.rsplit('/').next())
                    .cloned())
            }
            (Method::POST, "/api/v1/provisioning/alert-rules") => {
                let created = payload.cloned().ok_or_else(|| {
                    message("alert-rule create payload must be present".to_string())
                })?;
                remote_resources.borrow_mut().push(created.clone());
                Ok(Some(created))
            }
            (Method::PUT, path) if path.starts_with("/api/v1/provisioning/alert-rules/") => {
                Err(message(format!(
                    "unexpected alert-rule update during recreate path {}",
                    path
                )))
            }
            _ => Err(message(format!(
                "unexpected alert runtime request {} {}",
                method_name, path
            ))),
        },
        CONTACT_POINT_KIND => match (method, path) {
            (Method::GET, "/api/v1/provisioning/contact-points") => {
                Ok(Some(Value::Array(remote_resources.borrow().clone())))
            }
            (Method::POST, "/api/v1/provisioning/contact-points") => {
                let created = payload.cloned().ok_or_else(|| {
                    message("contact-point create payload must be present".to_string())
                })?;
                remote_resources.borrow_mut().push(created.clone());
                Ok(Some(created))
            }
            (Method::PUT, path) if path.starts_with("/api/v1/provisioning/contact-points/") => {
                Err(message(format!(
                    "unexpected contact-point update during recreate path {}",
                    path
                )))
            }
            _ => Err(message(format!(
                "unexpected alert runtime request {} {}",
                method_name, path
            ))),
        },
        MUTE_TIMING_KIND => match (method, path) {
            (Method::GET, "/api/v1/provisioning/mute-timings") => {
                Ok(Some(Value::Array(remote_resources.borrow().clone())))
            }
            (Method::POST, "/api/v1/provisioning/mute-timings") => {
                let created = payload.cloned().ok_or_else(|| {
                    message("mute-timing create payload must be present".to_string())
                })?;
                remote_resources.borrow_mut().push(created.clone());
                Ok(Some(created))
            }
            (Method::PUT, path) if path.starts_with("/api/v1/provisioning/mute-timings/") => {
                Err(message(format!(
                    "unexpected mute-timing update during recreate path {}",
                    path
                )))
            }
            _ => Err(message(format!(
                "unexpected alert runtime request {} {}",
                method_name, path
            ))),
        },
        TEMPLATE_KIND => match (method, path) {
            (Method::GET, path) if path.starts_with("/api/v1/provisioning/templates/") => {
                Ok(remote_resources
                    .borrow()
                    .iter()
                    .find(|item| {
                        item.get("name").and_then(Value::as_str) == path.rsplit('/').next()
                    })
                    .cloned())
            }
            (Method::PUT, path) if path.starts_with("/api/v1/provisioning/templates/") => {
                let name = path.rsplit('/').next().unwrap_or_default().to_string();
                let mut created = payload.cloned().ok_or_else(|| {
                    message("template update payload must be present".to_string())
                })?;
                if let Some(object) = created.as_object_mut() {
                    object.insert("name".to_string(), Value::String(name.clone()));
                }
                let existing_index = remote_resources.borrow().iter().position(|item| {
                    item.get("name").and_then(Value::as_str) == Some(name.as_str())
                });
                if let Some(index) = existing_index {
                    remote_resources.borrow_mut()[index] = created.clone();
                } else {
                    remote_resources.borrow_mut().push(created.clone());
                }
                Ok(Some(created))
            }
            _ => Err(message(format!(
                "unexpected alert runtime request {} {}",
                method_name, path
            ))),
        },
        _ => Err(message(format!("unexpected alert kind {}", kind))),
    }
}
