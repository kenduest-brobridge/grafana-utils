use super::{
    assert_exact_object_keys, build_alert_delete_preview_from_files, build_alert_diff_document,
    build_alert_import_dry_run_document, build_compare_diff_text, build_route_preview,
    expect_object_list, load_shared_diff_golden_fixture, normalize_compare_payload,
    parse_template_list_response, render_alert_action_text, serialize_compare_document,
    CONTACT_POINT_KIND, POLICIES_KIND, RULE_KIND, TEMPLATE_KIND, TOOL_VERSION,
};
use serde_json::json;
use std::path::Path;

#[test]
fn build_alert_delete_preview_from_files_blocks_policy_reset_without_guard() {
    let temp = tempfile::tempdir().unwrap();
    let policy_path = temp.path().join("notification-policies.yaml");
    std::fs::write(&policy_path, "receiver: grafana-default-email\n").unwrap();

    let preview = build_alert_delete_preview_from_files(&[policy_path], false).unwrap();
    assert_eq!(preview["reviewRequired"], json!(true));
    assert_eq!(preview["reviewed"], json!(false));
    assert_eq!(preview["summary"]["delete"], json!(0));
    assert_eq!(preview["summary"]["blocked"], json!(1));
    assert_eq!(
        preview["rows"][0]["reason"],
        json!("policy-reset-requires-allow-policy-reset")
    );
}

#[test]
fn render_alert_action_text_surfaces_review_contract() {
    let document = json!({
        "reviewRequired": true,
        "reviewed": false,
        "summary": {
            "delete": 1,
            "blocked": 0
        },
        "rows": [{
            "kind": "grafana-notification-policies",
            "identity": "root",
            "action": "blocked",
            "reason": "policy-reset-requires-allow-policy-reset"
        }]
    });

    let lines = render_alert_action_text("Alert delete preview", &document);
    assert_eq!(lines[0], "Alert delete preview");
    assert_eq!(lines[1], "Review: required=true reviewed=false");
    assert!(
        lines
            .iter()
            .any(|line| line == "Summary: delete=1 blocked=0"
                || line == "Summary: blocked=0 delete=1")
    );
    assert!(lines.iter().any(|line| line == "Rows:"));
    assert!(lines.iter().any(|line| {
        line == "- grafana-notification-policies root action=blocked reason=policy-reset-requires-allow-policy-reset"
    }));
}

#[test]
fn build_route_preview_sorts_group_by_and_matchers_stably() {
    let route = json!({
        "receiver": "team-webhook",
        "group_by": ["grafana_folder", "alertname", "alertname"],
        "object_matchers": [
            ["team", "=", "platform"],
            ["severity", "=", "critical"],
            ["team", "=", "platform"]
        ],
        "routes": [{"receiver": "team-slack"}]
    });

    let preview = build_route_preview(route.as_object().unwrap());
    assert_eq!(preview["groupBy"], json!(["alertname", "grafana_folder"]));
    assert_eq!(
        preview["matchers"],
        json!([["severity", "=", "critical"], ["team", "=", "platform"]])
    );
}

#[test]
fn normalize_compare_payload_dedupes_policy_group_by_and_matchers() {
    let payload = json!({
        "receiver": "team-webhook",
        "group_by": ["grafana_folder", "alertname", "alertname"],
        "routes": [{
            "receiver": "team-slack",
            "group_by": ["grafana_folder", "alertname", "grafana_folder"],
            "object_matchers": [
                ["team", "=", "platform"],
                ["severity", "=", "critical"],
                ["team", "=", "platform"]
            ]
        }]
    });

    let normalized = normalize_compare_payload(POLICIES_KIND, payload.as_object().unwrap());
    assert_eq!(
        normalized["group_by"],
        json!(["alertname", "grafana_folder"])
    );
    assert_eq!(
        normalized["routes"][0]["group_by"],
        json!(["alertname", "grafana_folder"])
    );
    assert_eq!(
        normalized["routes"][0]["object_matchers"],
        json!([["severity", "=", "critical"], ["team", "=", "platform"]])
    );
}

#[test]
fn normalize_compare_payload_recursively_normalizes_nested_policy_routes() {
    let desired = json!({
        "receiver": "team-webhook",
        "routes": [{
            "receiver": "team-slack",
            "continue": false,
            "group_by": ["grafana_folder", "alertname"],
            "object_matchers": [
                ["team", "=", "platform"],
                ["severity", "=", "critical"]
            ],
            "routes": [{
                "receiver": "team-pager",
                "continue": false,
                "group_by": ["grafana_folder", "alertname"],
                "object_matchers": [
                    ["team", "=", "platform"],
                    ["severity", "=", "critical"]
                ]
            }]
        }]
    });
    let live = json!({
        "receiver": "team-webhook",
        "routes": [{
            "receiver": "team-slack",
            "group_by": ["alertname", "grafana_folder"],
            "object_matchers": [
                ["severity", "=", "critical"],
                ["team", "=", "platform"]
            ],
            "routes": [{
                "receiver": "team-pager",
                "group_by": ["alertname", "grafana_folder"],
                "object_matchers": [
                    ["severity", "=", "critical"],
                    ["team", "=", "platform"]
                ]
            }]
        }]
    });

    assert_eq!(
        normalize_compare_payload(POLICIES_KIND, desired.as_object().unwrap()),
        normalize_compare_payload(POLICIES_KIND, live.as_object().unwrap())
    );
}

#[test]
fn normalize_compare_payload_normalizes_template_line_endings_and_trailing_newline() {
    let desired = json!({
        "name": "slack.default",
        "template": "{{ define \"slack.default\" }}ok{{ end }}"
    });
    let live = json!({
        "name": "slack.default",
        "template": "{{ define \"slack.default\" }}ok{{ end }}\r\n"
    });

    assert_eq!(
        normalize_compare_payload(TEMPLATE_KIND, desired.as_object().unwrap()),
        normalize_compare_payload(TEMPLATE_KIND, live.as_object().unwrap())
    );
}

#[test]
fn compare_diff_output_includes_headers_and_local_payload() {
    let remote = json!({
        "kind": RULE_KIND,
        "spec": {
            "uid": "rule-uid",
            "title": "CPU High",
            "folderUID": "infra-folder",
            "ruleGroup": "cpu-alerts",
            "condition": "C",
            "data": [],
        }
    });
    let local = json!({
        "kind": RULE_KIND,
        "spec": {
            "uid": "rule-uid",
            "title": "CPU Critical",
            "folderUID": "infra-folder",
            "ruleGroup": "cpu-alerts",
            "condition": "C",
            "data": [],
        }
    });
    let diff = build_compare_diff_text(
        &remote,
        &local,
        "rule-uid",
        Path::new("alerts/raw/rules/infra-folder/rule-uid.json"),
    )
    .unwrap();
    assert!(diff.contains("--- remote:rule-uid"));
    assert!(diff.contains("+++ alerts/raw/rules/infra-folder/rule-uid.json"));
    assert!(diff.contains("+    \"title\": \"CPU Critical\""));
    assert!(diff.contains("-    \"title\": \"CPU High\""));
}

#[test]
fn serialize_compare_document_sorts_object_keys_stably() {
    let first = json!({
        "spec": {
            "ruleGroup": "cpu-alerts",
            "title": "CPU High",
        },
        "kind": RULE_KIND,
    });
    let second = json!({
        "kind": RULE_KIND,
        "spec": {
            "title": "CPU High",
            "ruleGroup": "cpu-alerts",
        },
    });
    assert_eq!(
        serialize_compare_document(&first).unwrap(),
        serialize_compare_document(&second).unwrap()
    );
}

#[test]
fn expect_object_list_rejects_json_null() {
    let error = expect_object_list(
        Some(serde_json::Value::Null),
        "Unexpected template list response from Grafana.",
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("Unexpected template list response from Grafana."));
}

#[test]
fn template_list_null_is_treated_as_empty_in_live_client_path() {
    let templates = parse_template_list_response(Some(serde_json::Value::Null)).unwrap();
    assert!(templates.is_empty());
}

#[test]
fn build_alert_diff_document_reports_summary_and_rows() {
    let document = build_alert_diff_document(&[
        json!({
            "path": "alerts/raw/contact-points/smoke.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "smoke-webhook",
            "action": "same",
        }),
        json!({
            "path": "alerts/raw/policies/notification-policies.json",
            "kind": "grafana-notification-policies",
            "identity": "grafana-default-email",
            "action": "different",
        }),
        json!({
            "path": "alerts/raw/contact-points/missing.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "smoke-webhook",
            "action": "missing-remote",
        }),
    ]);

    assert_eq!(document["kind"], json!("grafana-util-alert-diff"));
    assert_eq!(document["schemaVersion"], json!(1));
    assert_eq!(document["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(document["summary"]["checked"], json!(3));
    assert_eq!(document["summary"]["same"], json!(1));
    assert_eq!(document["summary"]["different"], json!(1));
    assert_eq!(document["summary"]["missingRemote"], json!(1));
    assert_eq!(document["reviewRequired"], json!(true));
    assert_eq!(document["reviewed"], json!(false));
    assert_eq!(document["rows"].as_array().map(Vec::len), Some(3));
    assert_exact_object_keys(
        &document,
        &[
            "kind",
            "reviewRequired",
            "reviewed",
            "rows",
            "schemaVersion",
            "summary",
            "toolVersion",
        ],
    );
    for row in document["rows"].as_array().unwrap() {
        assert_exact_object_keys(row, &["action", "identity", "kind", "path"]);
    }
    assert_eq!(document["rows"][0]["action"], json!("same"));
    assert_eq!(document["rows"][1]["action"], json!("different"));
    assert_eq!(document["rows"][2]["action"], json!("missing-remote"));
}

#[test]
fn build_alert_diff_document_matches_shared_contract_fixture() {
    let fixture = load_shared_diff_golden_fixture("alert");
    let document = build_alert_diff_document(&[json!({
        "domain": "alert",
        "resourceKind": "contact-point",
        "identity": "smoke-webhook",
        "status": "different",
        "path": "alerts/raw/contact-points/smoke.json",
        "changedFields": ["spec"],
    })]);
    assert_eq!(document, fixture["document"]);
}

#[test]
fn alert_diff_and_import_documents_align_for_update_and_create_actions() {
    let diff_document = build_alert_diff_document(&[
        json!({
            "path": "alerts/raw/contact-points/smoke.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "smoke-webhook",
            "action": "different",
        }),
        json!({
            "path": "alerts/raw/contact-points/missing.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "new-webhook",
            "action": "missing-remote",
        }),
    ]);
    let import_document = build_alert_import_dry_run_document(&[
        json!({
            "path": "alerts/raw/contact-points/smoke.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "smoke-webhook",
            "action": "would-update",
        }),
        json!({
            "path": "alerts/raw/contact-points/missing.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "new-webhook",
            "action": "would-create",
        }),
    ]);

    assert!(diff_document["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["identity"] == "smoke-webhook" && row["action"] == "different"));
    assert!(import_document["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["identity"] == "smoke-webhook" && row["action"] == "would-update"));
    assert!(diff_document["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["identity"] == "new-webhook" && row["action"] == "missing-remote"));
    assert!(import_document["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["identity"] == "new-webhook" && row["action"] == "would-create"));
    assert_eq!(
        import_document["kind"],
        json!(super::alert_runtime_support::ALERT_IMPORT_DRY_RUN_KIND)
    );
    assert_eq!(import_document["schemaVersion"], json!(1));
    assert_eq!(import_document["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(import_document["reviewRequired"], json!(true));
    assert_eq!(import_document["reviewed"], json!(false));
}
