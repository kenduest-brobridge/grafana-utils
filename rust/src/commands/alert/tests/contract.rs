use super::{
    build_alert_live_project_status_domain, build_alert_project_status_domain,
    build_contact_point_export_document, build_contact_point_output_path, build_empty_root_index,
    build_import_operation, build_rule_export_document, build_rule_output_path,
    build_template_export_document, detect_document_kind, get_rule_linkage,
    load_alert_export_contract_fixture, load_alert_recreate_contract_fixture,
    load_alert_resource_file, load_panel_id_map, load_string_map, resource_subdir_by_kind,
    serialize_contact_point_list_rows, serialize_rule_list_rows, AlertLiveProjectStatusInputs,
    CONTACT_POINT_KIND, POLICIES_KIND, ROOT_INDEX_KIND, RULE_KIND, TOOL_API_VERSION,
    TOOL_SCHEMA_VERSION, TOOL_VERSION,
};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn build_rule_output_path_keeps_folder_structure() {
    let rule = json!({
        "folderUID": "infra folder",
        "ruleGroup": "CPU Alerts",
        "title": "DB CPU > 90%",
        "uid": "rule-1",
    });
    let path = build_rule_output_path(
        Path::new("alerts/raw/rules"),
        rule.as_object().unwrap(),
        false,
    );
    assert_eq!(
        path,
        Path::new("alerts/raw/rules/infra_folder/CPU_Alerts/DB_CPU_90__rule-1.json")
    );
}

#[test]
fn build_alert_project_status_domain_is_partial_without_core_counts() {
    let summary_document = json!({
        "summary": {
            "ruleCount": 0,
            "contactPointCount": 0,
            "policyCount": 0,
            "muteTimingCount": 2,
            "templateCount": 1
        }
    });
    let domain = build_alert_project_status_domain(Some(&summary_document)).unwrap();
    let value = serde_json::to_value(domain).unwrap();

    assert_eq!(value["id"], json!("alert"));
    assert_eq!(value["scope"], json!("staged"));
    assert_eq!(value["mode"], json!("artifact-summary"));
    assert_eq!(value["status"], json!("partial"));
    assert_eq!(value["reasonCode"], json!("partial-no-data"));
    assert_eq!(value["primaryCount"], json!(0));
    assert_eq!(value["blockerCount"], json!(0));
    assert_eq!(value["warningCount"], json!(0));
    assert_eq!(value["sourceKinds"], json!(["alert-export"]));
    assert_eq!(
        value["signalKeys"],
        json!([
            "summary.ruleCount",
            "summary.contactPointCount",
            "summary.policyCount",
            "summary.muteTimingCount",
            "summary.templateCount",
        ])
    );
    assert_eq!(value["blockers"], json!([]));
    assert_eq!(value["warnings"], json!([]));
    assert_eq!(
        value["nextActions"],
        json!(["export at least one alert rule, contact point, or policy"])
    );
}

#[test]
fn build_alert_project_status_domain_is_ready_from_core_counts() {
    let summary_document = json!({
        "summary": {
            "ruleCount": 4,
            "contactPointCount": 2,
            "policyCount": 3,
            "muteTimingCount": 1,
            "templateCount": 5
        }
    });
    let domain = build_alert_project_status_domain(Some(&summary_document)).unwrap();
    let value = serde_json::to_value(domain).unwrap();

    assert_eq!(value["status"], json!("ready"));
    assert_eq!(value["reasonCode"], json!("ready"));
    assert_eq!(value["primaryCount"], json!(4));
    assert_eq!(
        value["nextActions"],
        json!(["re-run alert export after alerting changes"])
    );
}

#[test]
fn build_alert_live_project_status_domain_is_ready_from_live_counts() {
    let rules = json!([{"uid": "cpu-high"}]);
    let contact_points = json!([{"uid": "cp-main"}]);
    let mute_timings = json!([{"name": "off-hours"}]);
    let policies = json!({"receiver": "grafana-default-email"});
    let templates = json!([{"name": "slack.default"}]);

    let domain = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
        rules_document: Some(&rules),
        contact_points_document: Some(&contact_points),
        mute_timings_document: Some(&mute_timings),
        policies_document: Some(&policies),
        templates_document: Some(&templates),
    })
    .unwrap();
    let value = serde_json::to_value(domain).unwrap();

    assert_eq!(value["scope"], json!("live"));
    assert_eq!(value["mode"], json!("live-alert-surfaces"));
    assert_eq!(value["primaryCount"], json!(5));
    assert_eq!(
        value["sourceKinds"],
        json!([
            "alert",
            "alert-contact-point",
            "alert-mute-timing",
            "alert-policy",
            "alert-template"
        ])
    );
}

#[test]
fn build_contact_point_output_path_uses_name_and_uid() {
    let contact_point = json!({
        "name": "Webhook Main",
        "uid": "cp-uid",
    });
    let path = build_contact_point_output_path(
        Path::new("alerts/raw/contact-points"),
        contact_point.as_object().unwrap(),
        false,
    );
    assert_eq!(
        path,
        Path::new("alerts/raw/contact-points/Webhook_Main/Webhook_Main__cp-uid.json")
    );
}

#[test]
fn build_rule_export_document_strips_server_managed_fields() {
    let document = build_rule_export_document(
        json!({
            "uid": "rule-uid",
            "title": "CPU High",
            "folderUID": "infra-folder",
            "ruleGroup": "cpu-alerts",
            "condition": "C",
            "data": [],
            "updated": "2026-03-10T10:00:00Z",
            "provenance": "api"
        })
        .as_object()
        .unwrap(),
    );
    assert_eq!(document["kind"], json!(RULE_KIND));
    assert_eq!(document["toolVersion"], json!(TOOL_VERSION));
    assert!(document["spec"].get("updated").is_none());
    assert!(document["spec"].get("provenance").is_none());
}

#[test]
fn detect_document_kind_accepts_plain_contact_point_shape() {
    let kind = detect_document_kind(
        json!({
            "name": "Webhook Main",
            "type": "webhook",
            "settings": {"url": "http://127.0.0.1/notify"}
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();
    assert_eq!(kind, CONTACT_POINT_KIND);
}

#[test]
fn build_import_operation_accepts_plain_rule_document() {
    let (kind, payload) = build_import_operation(&json!({
        "uid": "rule-uid",
        "title": "CPU High",
        "folderUID": "infra-folder",
        "ruleGroup": "cpu-alerts",
        "condition": "C",
        "data": [],
    }))
    .unwrap();
    assert_eq!(kind, RULE_KIND);
    assert_eq!(payload["title"], "CPU High");
}

#[test]
fn build_contact_point_export_document_wraps_tool_document() {
    let document = build_contact_point_export_document(
        json!({
            "uid": "cp-uid",
            "name": "Webhook Main",
            "type": "webhook",
            "settings": {"url": "http://127.0.0.1/notify"},
            "provenance": "api"
        })
        .as_object()
        .unwrap(),
    );
    assert_eq!(document["kind"], CONTACT_POINT_KIND);
    assert!(document["spec"].get("provenance").is_none());
}

#[test]
fn get_rule_linkage_returns_typed_dashboard_and_panel_ids() {
    let linkage = get_rule_linkage(
        json!({
            "annotations": {
                "__dashboardUid__": "dash-uid",
                "__panelId__": 7
            }
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();
    assert_eq!(linkage.dashboard_uid, "dash-uid");
    assert_eq!(linkage.panel_id.as_deref(), Some("7"));
}

#[test]
fn load_string_map_returns_empty_map_without_input_file() {
    let mapping = load_string_map(None, "Dashboard UID map").unwrap();
    assert!(mapping.is_empty());
}

#[test]
fn load_panel_id_map_parses_nested_dashboard_panel_mapping() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("panel-map.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "source-dashboard": {
                "7": "17",
                "8": 18
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let mapping = load_panel_id_map(Some(&path)).unwrap();

    assert_eq!(
        mapping
            .get("source-dashboard")
            .and_then(|items| items.get("7"))
            .map(String::as_str),
        Some("17")
    );
    assert_eq!(
        mapping
            .get("source-dashboard")
            .and_then(|items| items.get("8"))
            .map(String::as_str),
        Some("18")
    );
}

#[test]
fn build_import_operation_accepts_legacy_tool_document_without_schema_version() {
    let (kind, payload) = build_import_operation(&json!({
        "apiVersion": TOOL_API_VERSION,
        "kind": RULE_KIND,
        "metadata": {"uid": "rule-uid"},
        "spec": {
            "uid": "rule-uid",
            "title": "CPU High",
            "folderUID": "infra-folder",
            "ruleGroup": "cpu-alerts",
            "condition": "C",
            "data": [],
        }
    }))
    .unwrap();
    assert_eq!(kind, RULE_KIND);
    assert_eq!(payload["uid"], "rule-uid");
}

#[test]
fn build_import_operation_rejects_unsupported_schema_version() {
    let error = build_import_operation(&json!({
        "apiVersion": TOOL_API_VERSION,
        "schemaVersion": TOOL_SCHEMA_VERSION + 1,
        "kind": RULE_KIND,
        "spec": {
            "uid": "rule-uid",
            "title": "CPU High",
            "folderUID": "infra-folder",
            "ruleGroup": "cpu-alerts",
            "condition": "C",
            "data": [],
        }
    }))
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("Unsupported grafana-alert-rule schema version"));
}

#[test]
fn build_empty_root_index_contains_version_markers() {
    let index = build_empty_root_index();
    assert_eq!(index["schemaVersion"], json!(TOOL_SCHEMA_VERSION));
    assert_eq!(index["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(index["apiVersion"], json!(TOOL_API_VERSION));
    assert_eq!(index["kind"], json!(ROOT_INDEX_KIND));
    assert_eq!(index["rules"], json!([]));
    assert_eq!(index["contact-points"], json!([]));
    assert_eq!(index["mute-timings"], json!([]));
    assert_eq!(index["policies"], json!([]));
    assert_eq!(index["templates"], json!([]));
}

#[test]
fn alert_export_contract_fixture_matches_root_index_and_resource_subdirs() {
    let fixture = load_alert_export_contract_fixture();
    let root_index = build_empty_root_index();

    assert_eq!(fixture["rootIndex"]["kind"], json!(ROOT_INDEX_KIND));
    assert_eq!(
        fixture["rootIndex"]["schemaVersion"],
        json!(TOOL_SCHEMA_VERSION)
    );
    assert_eq!(fixture["rootIndex"]["apiVersion"], json!(TOOL_API_VERSION));

    for section in fixture["rootIndex"]["requiredSections"]
        .as_array()
        .unwrap_or(&Vec::new())
    {
        let key = section.as_str().unwrap_or("");
        assert_eq!(root_index.get(key), Some(&json!([])));
    }

    let subdirs = resource_subdir_by_kind();
    for case in fixture["cases"].as_array().unwrap_or(&Vec::new()) {
        let kind = case["kind"].as_str().unwrap_or("");
        let subdir = case["subdir"].as_str().unwrap_or("");
        assert_eq!(subdirs.get(kind).copied(), Some(subdir));
    }
}

#[test]
fn build_alert_import_dry_run_document_reports_summary_and_rows() {
    let document = super::build_alert_import_dry_run_document(&[
        json!({
            "path": "alerts/raw/contact-points/smoke.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "smoke-webhook",
            "action": "would-update",
        }),
        json!({
            "path": "alerts/raw/policies/notification-policies.json",
            "kind": "grafana-notification-policies",
            "identity": "grafana-default-email",
            "action": "would-create",
        }),
        json!({
            "path": "alerts/raw/templates/template.json",
            "kind": "grafana-message-template",
            "identity": "slack",
            "action": "would-fail-existing",
        }),
    ]);

    assert_eq!(
        document["kind"],
        json!(super::alert_runtime_support::ALERT_IMPORT_DRY_RUN_KIND)
    );
    assert_eq!(document["schemaVersion"], json!(1));
    assert_eq!(document["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(document["reviewRequired"], json!(true));
    assert_eq!(document["reviewed"], json!(false));
    assert_eq!(document["summary"]["processed"], json!(3));
    assert_eq!(document["summary"]["wouldCreate"], json!(1));
    assert_eq!(document["summary"]["wouldUpdate"], json!(1));
    assert_eq!(document["summary"]["wouldFailExisting"], json!(1));
    assert_eq!(document["rows"].as_array().map(Vec::len), Some(3));
    assert_eq!(document["rows"][0]["identity"], json!("smoke-webhook"));
}

#[test]
fn contact_point_list_and_export_document_share_identity_fields() {
    let contact_point = json!({
        "uid": "cp-uid",
        "name": "Webhook Main",
        "type": "webhook",
        "settings": {"url": "http://127.0.0.1/notify"},
    });

    let rows = serialize_contact_point_list_rows(&[contact_point.as_object().unwrap().clone()]);
    let document = build_contact_point_export_document(contact_point.as_object().unwrap());

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("uid").map(String::as_str), Some("cp-uid"));
    assert_eq!(
        rows[0].get("name").map(String::as_str),
        Some("Webhook Main")
    );
    assert_eq!(document["spec"]["uid"], json!("cp-uid"));
    assert_eq!(document["spec"]["name"], json!("Webhook Main"));
}

#[test]
fn mute_timing_list_and_export_document_share_identity_fields() {
    let mute_timing = json!({
        "name": "Off Hours",
        "time_intervals": [{"times": [{"start_time": "00:00", "end_time": "06:00"}]}]
    });

    let rows = super::alert_list::serialize_mute_timing_list_rows(&[mute_timing
        .as_object()
        .unwrap()
        .clone()]);
    let document = super::build_mute_timing_export_document(mute_timing.as_object().unwrap());

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("name").map(String::as_str), Some("Off Hours"));
    assert_eq!(rows[0].get("intervals").map(String::as_str), Some("1"));
    assert_eq!(document["spec"]["name"], json!("Off Hours"));
}

#[test]
fn template_list_and_export_document_share_identity_fields() {
    let template = json!({
        "name": "slack.default",
        "template": "{{ define \"slack.default\" }}ok{{ end }}",
        "version": "template-version-1"
    });

    let rows =
        super::alert_list::serialize_template_list_rows(&[template.as_object().unwrap().clone()]);
    let document = build_template_export_document(template.as_object().unwrap());

    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].get("name").map(String::as_str),
        Some("slack.default")
    );
    assert_eq!(document["spec"]["name"], json!("slack.default"));
    assert!(document["spec"].get("version").is_none());
}

#[test]
fn rule_list_and_export_document_share_identity_fields() {
    let rule = json!({
        "uid": "cpu-high",
        "title": "CPU High",
        "folderUID": "infra",
        "ruleGroup": "cpu-alerts",
        "condition": "A",
        "data": []
    });

    let rows = serialize_rule_list_rows(&[rule.as_object().unwrap().clone()]);
    let document = build_rule_export_document(rule.as_object().unwrap());

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("uid").map(String::as_str), Some("cpu-high"));
    assert_eq!(rows[0].get("title").map(String::as_str), Some("CPU High"));
    assert_eq!(document["spec"]["uid"], json!("cpu-high"));
    assert_eq!(document["spec"]["title"], json!("CPU High"));
}

#[test]
fn load_alert_resource_file_accepts_json_and_yaml_desired_documents() {
    let temp = tempdir().unwrap();
    let json_path = temp.path().join("rule.json");
    let yaml_path = temp.path().join("contact-point.yaml");

    super::write_pretty_json(
        &json_path,
        &json!({
            "uid": "rule-json",
            "title": "JSON Rule",
            "folderUID": "general",
            "ruleGroup": "default",
            "condition": "A",
            "data": [],
        }),
    );
    fs::write(
        &yaml_path,
        r#"name: yaml-contact-point
type: webhook
settings:
  url: http://127.0.0.1:9000/notify
"#,
    )
    .unwrap();

    let (json_kind, json_payload) =
        build_import_operation(&load_alert_resource_file(&json_path, "Alert resource").unwrap())
            .unwrap();
    let (yaml_kind, yaml_payload) =
        build_import_operation(&load_alert_resource_file(&yaml_path, "Alert resource").unwrap())
            .unwrap();

    assert_eq!(json_kind, RULE_KIND);
    assert_eq!(json_payload["uid"], json!("rule-json"));
    assert_eq!(yaml_kind, CONTACT_POINT_KIND);
    assert_eq!(yaml_payload["name"], json!("yaml-contact-point"));
}

#[test]
fn build_alert_recreate_matrix_with_request_covers_rule_contact_point_mute_timing_and_template() {
    let fixture = load_alert_recreate_contract_fixture();
    for case in fixture["recreateCases"].as_array().unwrap_or(&Vec::new()) {
        let kind = case["kind"].as_str().unwrap_or("");
        let identity = case["identity"].as_str().unwrap_or("");
        let expected_dry_run_action = case["expectedDryRunAction"]
            .as_str()
            .unwrap_or("would-create");
        let expected_replay_action = case["expectedReplayAction"].as_str().unwrap_or("created");
        let request_contract = case["requestContract"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        let payload = case["payload"].as_object().cloned().unwrap_or_default();
        super::run_alert_recreate_case(
            kind,
            payload,
            identity,
            expected_dry_run_action,
            expected_replay_action,
            request_contract,
        );
    }
}

#[test]
fn policies_with_request_stay_update_only_and_return_to_same_state() {
    let fixture = load_alert_recreate_contract_fixture();
    let expected_identity = fixture["policiesCase"]["identity"].as_str().unwrap_or("");
    let expected_dry_run_action = fixture["policiesCase"]["expectedDryRunAction"]
        .as_str()
        .unwrap_or("would-update");
    let expected_replay_action = fixture["policiesCase"]["expectedReplayAction"]
        .as_str()
        .unwrap_or("updated");
    let request_contract = fixture["policiesCase"]["requestContract"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let local_payload = fixture["policiesCase"]["payload"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let remote_policy = RefCell::new(json!({
        "receiver": "legacy-email",
        "routes": [{"receiver": "legacy-email"}]
    }));
    let request_log = RefCell::new(Vec::<String>::new());

    let initial_compare = super::fetch_live_compare_document_with_request(
        |method, path, _params, payload| -> super::Result<Option<Value>> {
            let method_name = method.as_str().to_string();
            request_log
                .borrow_mut()
                .push(format!("{} {}", method_name, path));
            match (method, path) {
                (reqwest::Method::GET, "/api/v1/provisioning/policies") => {
                    Ok(Some(remote_policy.borrow().clone()))
                }
                (reqwest::Method::PUT, "/api/v1/provisioning/policies") => {
                    let next = payload.cloned().ok_or_else(|| {
                        super::message("policies update payload must be present".to_string())
                    })?;
                    *remote_policy.borrow_mut() = next.clone();
                    Ok(Some(next))
                }
                _ => Err(super::message(format!(
                    "unexpected alert runtime request {} {}",
                    method_name, path
                ))),
            }
        },
        POLICIES_KIND,
        &local_payload,
    )
    .unwrap()
    .unwrap();
    assert_ne!(
        super::serialize_compare_document(&initial_compare).unwrap(),
        super::serialize_compare_document(&json!({
            "kind": POLICIES_KIND,
            "spec": local_payload,
        }))
        .unwrap()
    );

    assert_eq!(
        super::determine_import_action_with_request(
            |method, path, _params, _payload| -> super::Result<Option<Value>> {
                let method_name = method.as_str().to_string();
                request_log
                    .borrow_mut()
                    .push(format!("{} {}", method_name, path));
                match (method, path) {
                    (reqwest::Method::GET, "/api/v1/provisioning/policies") => {
                        Ok(Some(remote_policy.borrow().clone()))
                    }
                    _ => Err(super::message(format!(
                        "unexpected alert runtime request {} {}",
                        method_name, path
                    ))),
                }
            },
            POLICIES_KIND,
            &local_payload,
            true,
        )
        .unwrap(),
        expected_dry_run_action
    );

    let (action, identity) = super::import_resource_document_with_request(
        |method, path, _params, payload| -> super::Result<Option<Value>> {
            let method_name = method.as_str().to_string();
            request_log
                .borrow_mut()
                .push(format!("{} {}", method_name, path));
            match (method, path) {
                (reqwest::Method::PUT, "/api/v1/provisioning/policies") => {
                    let next = payload.cloned().ok_or_else(|| {
                        super::message("policies update payload must be present".to_string())
                    })?;
                    *remote_policy.borrow_mut() = next.clone();
                    Ok(Some(next))
                }
                _ => Err(super::message(format!(
                    "unexpected alert runtime request {} {}",
                    method_name, path
                ))),
            }
        },
        POLICIES_KIND,
        &local_payload,
        true,
    )
    .unwrap();
    assert_eq!(action, expected_replay_action);
    assert_eq!(identity, expected_identity);

    let update_request_prefix = request_contract
        .get("updateRequestPrefix")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let update_request_count = request_contract
        .get("updateRequestCount")
        .and_then(Value::as_u64)
        .unwrap_or(1) as usize;
    let actual_update_count = request_log
        .borrow()
        .iter()
        .filter(|entry| entry.starts_with(update_request_prefix))
        .count();
    assert_eq!(
        actual_update_count, update_request_count,
        "expected {update_request_count} update request(s) for policies"
    );
    assert!(
        !update_request_prefix.is_empty(),
        "policies request contract is missing updateRequestPrefix"
    );

    let live_compare = super::fetch_live_compare_document_with_request(
        |method, path, _params, _payload| -> super::Result<Option<Value>> {
            let method_name = method.as_str().to_string();
            request_log
                .borrow_mut()
                .push(format!("{} {}", method_name, path));
            match (method, path) {
                (reqwest::Method::GET, "/api/v1/provisioning/policies") => {
                    Ok(Some(remote_policy.borrow().clone()))
                }
                _ => Err(super::message(format!(
                    "unexpected alert runtime request {} {}",
                    method_name, path
                ))),
            }
        },
        POLICIES_KIND,
        &local_payload,
    )
    .unwrap()
    .unwrap();

    assert_eq!(
        super::serialize_compare_document(&live_compare).unwrap(),
        super::serialize_compare_document(&json!({
            "kind": POLICIES_KIND,
            "spec": local_payload,
        }))
        .unwrap()
    );
}
