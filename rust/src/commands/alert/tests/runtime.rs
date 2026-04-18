use super::{
    build_alert_plan_document, build_alert_plan_with_request, determine_import_action_with_request,
    execute_alert_plan_with_request, fetch_live_compare_document_with_request,
    import_resource_document_with_request, load_alert_recreate_contract_fixture,
    normalize_compare_payload, run_alert_recreate_case, write_new_rule_scaffold,
    write_new_template_scaffold, CONTACT_POINT_KIND, MUTE_TIMING_KIND, POLICIES_KIND, RULE_KIND,
    TEMPLATE_KIND, TOOL_SCHEMA_VERSION,
};
use crate::common::{api_response, message, Result};
use crate::grafana_api::alert_live::request_optional_object_with_request;
use reqwest::Method;
use serde_json::{json, Value};
use std::cell::RefCell;
use tempfile::tempdir;

#[test]
fn request_optional_object_with_request_treats_http_404_as_missing() {
    let result = request_optional_object_with_request(
        |_method, path, _params, _payload| {
            Err(api_response(
                404,
                format!("http://127.0.0.1:3000{path}"),
                "",
            ))
        },
        Method::GET,
        "/api/v1/provisioning/alert-rules/missing-rule",
        None,
    )
    .unwrap();

    assert!(result.is_none());
}

#[test]
fn build_alert_plan_with_request_generates_create_update_noop_and_blocked_rows() {
    let temp = tempdir().unwrap();
    write_new_rule_scaffold(
        &temp.path().join("rules/create-rule.json"),
        "create-rule",
        true,
    )
    .unwrap();
    super::write_pretty_json(
        &temp.path().join("contact-points/update-contact-point.yaml"),
        &json!({
            "kind": CONTACT_POINT_KIND,
            "apiVersion": super::TOOL_API_VERSION,
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "spec": {
                "uid": "cp-update",
                "name": "Update Me",
                "type": "webhook",
                "settings": {"url": "http://127.0.0.1/new"}
            }
        }),
    );
    write_new_template_scaffold(
        &temp.path().join("templates/example-template.json"),
        "example-template",
        true,
    )
    .unwrap();

    let plan = build_alert_plan_with_request(
        |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/v1/provisioning/alert-rules/create-rule") => Ok(None),
            (Method::GET, "/api/v1/provisioning/contact-points") => Ok(Some(json!([
                {
                    "uid": "cp-update",
                    "name": "Update Me",
                    "type": "webhook",
                    "settings": {"url": "http://127.0.0.1/old"}
                }
            ]))),
            (Method::GET, "/api/v1/provisioning/templates/example-template") => Ok(Some(json!({
                "name": "example-template",
                "template": "{{ define \"example-template\" }}replace me{{ end }}"
            }))),
            (Method::GET, "/api/v1/provisioning/alert-rules") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/mute-timings") => Ok(Some(json!([
                {
                    "name": "off-hours",
                    "time_intervals": []
                }
            ]))),
            (Method::GET, "/api/v1/provisioning/templates") => Ok(Some(json!([
                {
                    "name": "example-template",
                    "template": "{{ define \"example-template\" }}replace me{{ end }}"
                }
            ]))),
            (Method::GET, "/api/v1/provisioning/policies") => Ok(Some(json!({
                "receiver": "grafana-default-email"
            }))),
            _ => panic!("unexpected request {method:?} {path}"),
        },
        temp.path(),
        false,
    )
    .unwrap();

    assert_eq!(plan["summary"]["create"], json!(1));
    assert_eq!(plan["summary"]["update"], json!(1));
    assert_eq!(plan["summary"]["noop"], json!(1));
    assert_eq!(plan["summary"]["blocked"], json!(2));
    assert_eq!(plan["summary"]["warning"], json!(0));

    let rows = plan["rows"].as_array().unwrap();
    assert!(rows.iter().any(|row| {
        row["kind"] == json!(RULE_KIND)
            && row["identity"] == json!("create-rule")
            && row["action"] == json!("create")
            && row["status"] == json!("ready")
            && row["actionId"].as_str().unwrap_or("").ends_with("::create")
            && row["changedFields"].is_array()
            && row["changes"].is_array()
    }));
    assert!(rows.iter().any(|row| {
        row["kind"] == json!(CONTACT_POINT_KIND)
            && row["identity"] == json!("cp-update")
            && row["action"] == json!("update")
            && row["status"] == json!("ready")
            && row["changedFields"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field == "settings")
            && row["changes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|change| change["field"] == json!("settings"))
    }));
    assert!(rows.iter().any(|row| {
        row["kind"] == json!(TEMPLATE_KIND)
            && row["identity"] == json!("example-template")
            && row["action"] == json!("noop")
            && row["status"] == json!("same")
            && row["reviewHints"] == json!([])
    }));
    assert!(rows.iter().any(|row| {
        row["kind"] == json!(MUTE_TIMING_KIND)
            && row["identity"] == json!("off-hours")
            && row["action"] == json!("blocked")
            && row["status"] == json!("blocked")
            && row["reason"] == json!("prune-required")
            && row["blockedReason"] == json!("prune-required")
    }));
    assert!(rows.iter().any(|row| {
        row["kind"] == json!(POLICIES_KIND)
            && row["identity"] == json!("grafana-default-email")
            && row["action"] == json!("blocked")
            && row["status"] == json!("blocked")
    }));
}

#[test]
fn build_alert_plan_with_request_surfaces_linked_rule_review_hints() {
    let temp = tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("rules")).unwrap();
    std::fs::write(
        temp.path().join("rules/linked-rule.json"),
        serde_json::to_string_pretty(&json!({
            "uid": "linked-rule",
            "title": "Linked Rule",
            "folderUID": "alerts",
            "ruleGroup": "linked",
            "condition": "A",
            "data": [],
            "annotations": {
                "__dashboardUid__": "src-dash",
                "__panelId__": "7"
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let plan = build_alert_plan_with_request(
        |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/v1/provisioning/alert-rules/linked-rule") => Ok(Some(json!({
                "uid": "linked-rule",
                "title": "Linked Rule",
                "folderUID": "alerts",
                "ruleGroup": "linked",
                "condition": "A",
                "data": [],
                "annotations": {
                    "__dashboardUid__": "src-dash",
                    "__panelId__": "7"
                }
            }))),
            (Method::GET, "/api/v1/provisioning/alert-rules") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/contact-points") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/mute-timings") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/templates") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/policies") => Ok(Some(json!({"receiver": "root"}))),
            _ => panic!("unexpected request {method:?} {path}"),
        },
        temp.path(),
        false,
    )
    .unwrap();

    assert_eq!(plan["summary"]["warning"], json!(1));
    let row = plan["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["identity"] == json!("linked-rule"))
        .unwrap();
    assert_eq!(row["action"], json!("noop"));
    assert_eq!(row["status"], json!("warning"));
    assert!(row["actionId"]
        .as_str()
        .unwrap_or("")
        .contains("linked-rule"));
    let hints = row["reviewHints"].as_array().unwrap();
    assert!(hints
        .iter()
        .any(|hint| hint["code"] == json!("linked-dashboard-reference")));
    assert!(hints
        .iter()
        .any(|hint| hint["code"] == json!("linked-panel-reference")));
    assert!(row["changedFields"].as_array().unwrap().is_empty());
}

#[test]
fn build_alert_plan_with_request_marks_live_only_resources_delete_when_prune_enabled() {
    let temp = tempdir().unwrap();
    write_new_rule_scaffold(
        &temp.path().join("rules/create-rule.json"),
        "create-rule",
        true,
    )
    .unwrap();

    let plan = build_alert_plan_with_request(
        |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/v1/provisioning/alert-rules/create-rule") => Ok(None),
            (Method::GET, "/api/v1/provisioning/alert-rules") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/contact-points") => Ok(Some(json!([
                {
                    "uid": "cp-delete",
                    "name": "Delete Me",
                    "type": "webhook",
                    "settings": {"url": "http://127.0.0.1/delete"}
                }
            ]))),
            (Method::GET, "/api/v1/provisioning/mute-timings") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/templates") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/policies") => Ok(None),
            _ => panic!("unexpected request {method:?} {path}"),
        },
        temp.path(),
        true,
    )
    .unwrap();

    assert!(plan["rows"].as_array().unwrap().iter().any(|row| {
        row["kind"] == json!(CONTACT_POINT_KIND)
            && row["identity"] == json!("cp-delete")
            && row["action"] == json!("delete")
    }));
}

#[test]
fn normalize_compare_payload_erases_authoring_round_trip_drift_defaults() {
    let contact_point = json!({
        "uid": "cp-authoring",
        "name": "Authoring Webhook",
        "type": "webhook",
        "settings": {"url": "http://127.0.0.1/notify"},
    });
    let contact_point_live = json!({
        "uid": "cp-authoring",
        "name": "Authoring Webhook",
        "type": "webhook",
        "settings": {"url": "http://127.0.0.1/notify"},
        "disableResolveMessage": false,
    });
    assert_eq!(
        normalize_compare_payload(CONTACT_POINT_KIND, contact_point.as_object().unwrap()),
        normalize_compare_payload(CONTACT_POINT_KIND, contact_point_live.as_object().unwrap())
    );

    let policies_desired = json!({
        "receiver": "pagerduty-primary",
        "group_by": ["alertname", "grafana_folder"],
        "routes": [{
            "receiver": "pagerduty-primary",
            "continue": false,
            "group_by": ["alertname", "grafana_folder"],
            "object_matchers": [
                ["team", "=", "platform"],
                ["severity", "=", "critical"],
                ["grafana_utils_route", "=", "pagerduty-primary"]
            ]
        }]
    });
    let policies_live = json!({
        "receiver": "pagerduty-primary",
        "group_by": ["grafana_folder", "alertname"],
        "routes": [{
            "receiver": "pagerduty-primary",
            "group_by": ["grafana_folder", "alertname"],
            "object_matchers": [
                ["grafana_utils_route", "=", "pagerduty-primary"],
                ["severity", "=", "critical"],
                ["team", "=", "platform"]
            ]
        }]
    });
    assert_eq!(
        normalize_compare_payload(POLICIES_KIND, policies_desired.as_object().unwrap()),
        normalize_compare_payload(POLICIES_KIND, policies_live.as_object().unwrap())
    );

    let rule_desired = json!({
        "uid": "cpu-high",
        "title": "cpu-high",
        "folderUID": "platform-alerts",
        "ruleGroup": "cpu",
        "condition": "A",
        "for": "5m",
        "noDataState": "NoData",
        "execErrState": "Alerting",
        "labels": {
            "grafana_utils_route": "pagerduty-primary",
            "severity": "critical",
            "team": "platform"
        },
        "annotations": {},
        "data": [{
            "refId": "A",
            "datasourceUid": "__expr__",
            "relativeTimeRange": {"from": 0, "to": 0},
            "model": {
                "refId": "A",
                "type": "classic_conditions",
                "datasource": {"type": "__expr__", "uid": "__expr__"},
                "expression": "A",
                "conditions": [{
                    "type": "query",
                    "query": {"params": ["A"]},
                    "reducer": {"type": "last", "params": []},
                    "evaluator": {"type": "gt", "params": [80.0]},
                    "operator": {"type": "and"}
                }],
                "intervalMs": 1000,
                "maxDataPoints": 43200
            }
        }]
    });
    let rule_live = json!({
        "uid": "cpu-high",
        "title": "cpu-high",
        "folderUID": "platform-alerts",
        "ruleGroup": "cpu",
        "condition": "A",
        "for": "5m",
        "noDataState": "NoData",
        "execErrState": "Alerting",
        "isPaused": false,
        "keep_firing_for": "0s",
        "notification_settings": null,
        "record": null,
        "orgID": 1,
        "labels": {
            "grafana_utils_route": "pagerduty-primary",
            "severity": "critical",
            "team": "platform"
        },
        "annotations": {},
        "data": [{
            "refId": "A",
            "queryType": "",
            "datasourceUid": "__expr__",
            "relativeTimeRange": {"from": 0, "to": 0},
            "model": {
                "refId": "A",
                "type": "classic_conditions",
                "datasource": {"type": "__expr__", "uid": "__expr__"},
                "expression": "A",
                "conditions": [{
                    "type": "query",
                    "query": {"params": ["A"]},
                    "reducer": {"type": "last", "params": []},
                    "evaluator": {"type": "gt", "params": [80.0]},
                    "operator": {"type": "and"}
                }],
                "intervalMs": 1000,
                "maxDataPoints": 43200
            }
        }]
    });
    assert_eq!(
        normalize_compare_payload(RULE_KIND, rule_desired.as_object().unwrap()),
        normalize_compare_payload(RULE_KIND, rule_live.as_object().unwrap())
    );

    let rule_desired_alias = json!({
        "uid": "cpu-high",
        "title": "cpu-high",
        "folderUID": "platform-alerts",
        "ruleGroup": "cpu",
        "condition": "A",
        "for": "5m",
        "keep_firing_for": "0s",
        "noDataState": "NoData",
        "execErrState": "Alerting",
        "labels": {
            "grafana_utils_route": "pagerduty-primary",
            "severity": "critical",
            "team": "platform"
        },
        "annotations": {},
        "data": [{
            "refId": "A",
            "datasourceUid": "__expr__",
            "relativeTimeRange": {"from": 0, "to": 0},
            "model": {
                "refId": "A",
                "type": "classic_conditions",
                "datasource": {"type": "__expr__", "uid": "__expr__"},
                "expression": "A",
                "conditions": [{
                    "type": "query",
                    "query": {"params": ["A"]},
                    "reducer": {"type": "last", "params": []},
                    "evaluator": {"type": "gt", "params": [80.0]},
                    "operator": {"type": "and"}
                }],
                "intervalMs": 1000,
                "maxDataPoints": 43200
            }
        }]
    });
    let rule_live_alias = json!({
        "uid": "cpu-high",
        "title": "cpu-high",
        "folderUID": "platform-alerts",
        "ruleGroup": "cpu",
        "condition": "A",
        "for": "300s",
        "keep_firing_for": "0m",
        "noDataState": "NoData",
        "execErrState": "Alerting",
        "labels": {
            "grafana_utils_route": "pagerduty-primary",
            "severity": "critical",
            "team": "platform"
        },
        "annotations": {},
        "data": [{
            "refId": "A",
            "queryType": "",
            "datasourceUid": "__expr__",
            "relativeTimeRange": {"from": 0, "to": 0},
            "model": {
                "refId": "A",
                "type": "classic_conditions",
                "datasource": {"type": "__expr__", "uid": "__expr__"},
                "expression": "A",
                "conditions": [{
                    "type": "query",
                    "query": {"params": ["A"]},
                    "reducer": {"type": "last", "params": []},
                    "evaluator": {"type": "gt", "params": [80.0]},
                    "operator": {"type": "and"}
                }],
                "intervalMs": 1000,
                "maxDataPoints": 43200
            }
        }]
    });
    assert_eq!(
        normalize_compare_payload(RULE_KIND, rule_desired_alias.as_object().unwrap()),
        normalize_compare_payload(RULE_KIND, rule_live_alias.as_object().unwrap())
    );
}

#[test]
fn build_alert_plan_with_request_treats_authoring_round_trip_defaults_as_noop() {
    let temp = tempdir().unwrap();
    super::write_pretty_json(
        &temp
            .path()
            .join("contact-points/authoring-contact-point.json"),
        &json!({
            "kind": CONTACT_POINT_KIND,
            "apiVersion": super::TOOL_API_VERSION,
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "spec": {
                "uid": "cp-authoring",
                "name": "Authoring Webhook",
                "type": "webhook",
                "settings": {"url": "http://127.0.0.1/notify"}
            }
        }),
    );
    super::write_pretty_json(
        &temp.path().join("policies/notification-policies.json"),
        &json!({
            "kind": POLICIES_KIND,
            "apiVersion": super::TOOL_API_VERSION,
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "spec": {
                "receiver": "pagerduty-primary",
                "group_by": ["grafana_folder", "alertname"],
                "routes": [{
                    "receiver": "pagerduty-primary",
                    "continue": false,
                    "group_by": ["grafana_folder", "alertname"],
                    "object_matchers": [
                        ["team", "=", "platform"],
                        ["severity", "=", "critical"],
                        ["grafana_utils_route", "=", "pagerduty-primary"]
                    ]
                }]
            }
        }),
    );
    super::write_pretty_json(
        &temp.path().join("rules/cpu-high.json"),
        &json!({
            "kind": RULE_KIND,
            "apiVersion": super::TOOL_API_VERSION,
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "spec": {
                "uid": "cpu-high",
                "title": "cpu-high",
                "folderUID": "platform-alerts",
                "ruleGroup": "cpu",
                "condition": "A",
                "for": "5m",
                "noDataState": "NoData",
                "execErrState": "Alerting",
                "labels": {
                    "grafana_utils_route": "pagerduty-primary",
                    "severity": "critical",
                    "team": "platform"
                },
                "annotations": {},
                "data": [{
                    "refId": "A",
                    "datasourceUid": "__expr__",
                    "relativeTimeRange": {"from": 0, "to": 0},
                    "model": {
                        "refId": "A",
                        "type": "classic_conditions",
                        "datasource": {"type": "__expr__", "uid": "__expr__"},
                        "expression": "A",
                        "conditions": [{
                            "type": "query",
                            "query": {"params": ["A"]},
                            "reducer": {"type": "last", "params": []},
                            "evaluator": {"type": "gt", "params": [80.0]},
                            "operator": {"type": "and"}
                        }],
                        "intervalMs": 1000,
                        "maxDataPoints": 43200
                    }
                }]
            }
        }),
    );

    let plan = build_alert_plan_with_request(
        |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/v1/provisioning/contact-points") => Ok(Some(json!([
                {
                    "uid": "cp-authoring",
                    "name": "Authoring Webhook",
                    "type": "webhook",
                    "settings": {"url": "http://127.0.0.1/notify"},
                    "disableResolveMessage": false
                }
            ]))),
            (Method::GET, "/api/v1/provisioning/policies") => Ok(Some(json!({
                "receiver": "pagerduty-primary",
                "group_by": ["grafana_folder", "alertname"],
                "routes": [{
                    "receiver": "pagerduty-primary",
                    "group_by": ["grafana_folder", "alertname"],
                    "object_matchers": [
                        ["grafana_utils_route", "=", "pagerduty-primary"],
                        ["severity", "=", "critical"],
                        ["team", "=", "platform"]
                    ]
                }]
            }))),
            (Method::GET, "/api/v1/provisioning/alert-rules/cpu-high") => Ok(Some(json!({
                "uid": "cpu-high",
                "title": "cpu-high",
                "folderUID": "platform-alerts",
                "ruleGroup": "cpu",
                "condition": "A",
                "for": "5m",
                "noDataState": "NoData",
                "execErrState": "Alerting",
                "isPaused": false,
                "keep_firing_for": "0s",
                "notification_settings": null,
                "record": null,
                "orgID": 1,
                "data": [{
                    "refId": "A",
                    "queryType": "",
                    "datasourceUid": "__expr__",
                    "relativeTimeRange": {"from": 0, "to": 0},
                    "model": {
                        "refId": "A",
                        "type": "classic_conditions",
                        "datasource": {"type": "__expr__", "uid": "__expr__"},
                        "expression": "A",
                        "conditions": [{
                            "type": "query",
                            "query": {"params": ["A"]},
                            "reducer": {"type": "last", "params": []},
                            "evaluator": {"type": "gt", "params": [80.0]},
                            "operator": {"type": "and"}
                        }],
                        "intervalMs": 1000,
                        "maxDataPoints": 43200
                    }
                }],
                "labels": {
                    "grafana_utils_route": "pagerduty-primary",
                    "severity": "critical",
                    "team": "platform"
                },
                "annotations": {}
            }))),
            (Method::GET, "/api/v1/provisioning/alert-rules") => Ok(Some(json!([
                {"uid": "cpu-high"}
            ]))),
            (Method::GET, "/api/v1/provisioning/mute-timings") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/templates") => Ok(Some(json!([]))),
            _ => panic!("unexpected request {method:?} {path}"),
        },
        temp.path(),
        true,
    )
    .unwrap();

    assert_eq!(plan["summary"]["create"], json!(0));
    assert_eq!(plan["summary"]["update"], json!(0));
    assert_eq!(plan["summary"]["noop"], json!(3));
    assert_eq!(plan["summary"]["delete"], json!(0));
}

#[test]
fn execute_alert_plan_with_request_applies_create_update_and_delete_rows() {
    let plan = build_alert_plan_document(
        &[
            json!({
                "kind": RULE_KIND,
                "identity": "rule-create",
                "action": "create",
                "actionId": "grafana-alert-rule::rule-create::create",
                "status": "ready",
                "blockedReason": null,
                "reviewHints": [],
                "changedFields": ["title"],
                "changes": [{
                    "field": "title",
                    "before": null,
                    "after": "Create Me"
                }],
                "desired": {
                    "uid": "rule-create",
                    "title": "Create Me",
                    "folderUID": "general",
                    "ruleGroup": "default",
                    "condition": "A",
                    "data": []
                },
                "live": null
            }),
            json!({
                "kind": CONTACT_POINT_KIND,
                "identity": "cp-update",
                "action": "update",
                "actionId": "grafana-alert-contact-point::cp-update::update",
                "status": "ready",
                "blockedReason": null,
                "reviewHints": [],
                "changedFields": ["settings"],
                "changes": [{
                    "field": "settings",
                    "before": {"url": "http://127.0.0.1/old"},
                    "after": {"url": "http://127.0.0.1/new"}
                }],
                "desired": {
                    "uid": "cp-update",
                    "name": "Update Me",
                    "type": "webhook",
                    "settings": {"url": "http://127.0.0.1/new"}
                },
                "live": {
                    "uid": "cp-update",
                    "name": "Update Me",
                    "type": "webhook",
                    "settings": {"url": "http://127.0.0.1/old"}
                }
            }),
            json!({
                "kind": TEMPLATE_KIND,
                "identity": "template-delete",
                "action": "delete",
                "actionId": "grafana-message-template::template-delete::delete",
                "status": "ready",
                "blockedReason": null,
                "reviewHints": [],
                "changedFields": ["name"],
                "changes": [{
                    "field": "name",
                    "before": "template-delete",
                    "after": null
                }],
                "desired": null,
                "live": {
                    "name": "template-delete"
                }
            }),
            json!({
                "kind": RULE_KIND,
                "identity": "rule-noop",
                "action": "noop",
                "actionId": "grafana-alert-rule::rule-noop::noop",
                "status": "same",
                "blockedReason": null,
                "reviewHints": [],
                "changedFields": [],
                "changes": [],
                "desired": null,
                "live": null
            }),
        ],
        true,
    );
    let calls = RefCell::new(Vec::new());

    let result = execute_alert_plan_with_request(
        |method, path, _params, payload| {
            calls
                .borrow_mut()
                .push((method.clone(), path.to_string(), payload.cloned()));
            match (method.clone(), path) {
                (Method::POST, "/api/v1/provisioning/alert-rules") => {
                    Ok(Some(json!({"uid": "rule-create"})))
                }
                (Method::PUT, "/api/v1/provisioning/contact-points/cp-update") => {
                    Ok(Some(json!({"uid": "cp-update"})))
                }
                (Method::DELETE, "/api/v1/provisioning/templates/template-delete") => Ok(None),
                _ => panic!("unexpected request {method:?} {path}"),
            }
        },
        &plan,
        false,
    )
    .unwrap();

    assert_eq!(result["appliedCount"], json!(3));
    let calls = calls.borrow();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].0, Method::POST);
    assert_eq!(calls[0].1, "/api/v1/provisioning/alert-rules");
    assert_eq!(calls[1].0, Method::PUT);
    assert_eq!(calls[1].1, "/api/v1/provisioning/contact-points/cp-update");
    assert_eq!(calls[2].0, Method::DELETE);
    assert_eq!(calls[2].1, "/api/v1/provisioning/templates/template-delete");
}

#[test]
fn execute_alert_plan_with_request_rejects_policy_delete_without_guard() {
    let plan = build_alert_plan_document(
        &[json!({
            "kind": POLICIES_KIND,
            "identity": "grafana-default-email",
            "action": "delete",
            "desired": null
        })],
        true,
    );

    let error =
        execute_alert_plan_with_request(|_method, _path, _params, _payload| Ok(None), &plan, false)
            .unwrap_err()
            .to_string();

    assert!(error.contains("--allow-policy-reset"));
}

#[test]
fn alert_recreate_matrix_with_request_covers_rule_contact_point_mute_timing_and_template() {
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
        run_alert_recreate_case(
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

    let initial_compare = fetch_live_compare_document_with_request(
        |method, path, _params, payload| -> Result<Option<Value>> {
            let method_name = method.as_str().to_string();
            request_log
                .borrow_mut()
                .push(format!("{} {}", method_name, path));
            match (method, path) {
                (Method::GET, "/api/v1/provisioning/policies") => {
                    Ok(Some(remote_policy.borrow().clone()))
                }
                (Method::PUT, "/api/v1/provisioning/policies") => {
                    let next = payload.cloned().ok_or_else(|| {
                        message("policies update payload must be present".to_string())
                    })?;
                    *remote_policy.borrow_mut() = next.clone();
                    Ok(Some(next))
                }
                _ => Err(message(format!(
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
        determine_import_action_with_request(
            |method, path, _params, _payload| -> Result<Option<Value>> {
                let method_name = method.as_str().to_string();
                request_log
                    .borrow_mut()
                    .push(format!("{} {}", method_name, path));
                match (method, path) {
                    (Method::GET, "/api/v1/provisioning/policies") => {
                        Ok(Some(remote_policy.borrow().clone()))
                    }
                    _ => Err(message(format!(
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

    let (action, identity) = import_resource_document_with_request(
        |method, path, _params, payload| -> Result<Option<Value>> {
            let method_name = method.as_str().to_string();
            request_log
                .borrow_mut()
                .push(format!("{} {}", method_name, path));
            match (method, path) {
                (Method::PUT, "/api/v1/provisioning/policies") => {
                    let next = payload.cloned().ok_or_else(|| {
                        message("policies update payload must be present".to_string())
                    })?;
                    *remote_policy.borrow_mut() = next.clone();
                    Ok(Some(next))
                }
                _ => Err(message(format!(
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

    let live_compare = fetch_live_compare_document_with_request(
        |method, path, _params, _payload| -> Result<Option<Value>> {
            let method_name = method.as_str().to_string();
            request_log
                .borrow_mut()
                .push(format!("{} {}", method_name, path));
            match (method, path) {
                (Method::GET, "/api/v1/provisioning/policies") => {
                    Ok(Some(remote_policy.borrow().clone()))
                }
                _ => Err(message(format!(
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
