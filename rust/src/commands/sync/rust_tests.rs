//! Sync preflight planning test suite.
//! Validates plan/summary contracts and resource normalization behavior.
use super::preflight::{
    build_sync_preflight_document, render_sync_preflight_text, SyncPreflightSummary,
    SYNC_PREFLIGHT_KIND,
};
use super::render_sync_plan_text;
use super::workbench::{
    build_sync_apply_intent_document, build_sync_plan_document, build_sync_summary_document,
    normalize_resource_spec, summarize_resource_specs, SYNC_APPLY_INTENT_KIND,
    SYNC_APPLY_INTENT_SCHEMA_VERSION, SYNC_SUMMARY_KIND,
};
use crate::common::TOOL_VERSION;
use serde_json::json;

#[test]
fn normalize_resource_spec_requires_alert_managed_fields() {
    let error = normalize_resource_spec(&json!({
        "kind": "alert",
        "uid": "cpu-high",
        "title": "CPU High",
        "body": {
            "condition": "A > 90"
        }
    }))
    .unwrap_err()
    .to_string();

    assert!(error.contains("managedFields"));
}

#[test]
fn build_sync_summary_document_counts_normalized_resource_kinds() {
    let raw_specs = vec![
        json!({
            "kind": "folder",
            "uid": "ops",
            "title": "Operations",
            "body": {"title": "Operations"},
            "sourcePath": "folders/ops.json"
        }),
        json!({
            "kind": "datasource",
            "uid": "prom-main",
            "name": "Prometheus Main",
            "body": {"type": "prometheus"},
            "sourcePath": "datasources/prom-main.json"
        }),
        json!({
            "kind": "dashboard",
            "uid": "cpu-main",
            "title": "CPU Main",
            "body": {"datasourceUids": ["prom-main"]},
            "sourcePath": "dashboards/cpu-main.json"
        }),
        json!({
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["condition", "contactPoints"],
            "body": {"condition": "A > 90", "contactPoints": ["pagerduty-primary"]},
            "sourcePath": "alerts/cpu-high.json"
        }),
    ];

    let document = build_sync_summary_document(&raw_specs).unwrap();

    assert_eq!(document["kind"], json!(SYNC_SUMMARY_KIND));
    assert_eq!(document["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(document["summary"]["resourceCount"], json!(4));
    assert_eq!(document["summary"]["dashboardCount"], json!(1));
    assert_eq!(document["summary"]["datasourceCount"], json!(1));
    assert_eq!(document["summary"]["folderCount"], json!(1));
    assert_eq!(document["summary"]["alertCount"], json!(1));
    assert_eq!(
        document["resources"][3]["managedFields"],
        json!(["condition", "contactPoints"])
    );
}

#[test]
fn summarize_resource_specs_reports_counts() {
    let specs = vec![
        normalize_resource_spec(&json!({"kind":"folder","uid":"ops","title":"Operations"}))
            .unwrap(),
        normalize_resource_spec(&json!({
            "kind":"alert",
            "uid":"cpu-high",
            "title":"CPU High",
            "managedFields":["condition"],
            "body":{"condition":"A > 90"}
        }))
        .unwrap(),
    ];

    let summary = summarize_resource_specs(&specs);

    assert_eq!(summary.resource_count, 2);
    assert_eq!(summary.folder_count, 1);
    assert_eq!(summary.alert_count, 1);
}

#[test]
fn build_sync_preflight_document_reports_plugin_dependency_and_alert_blocks() {
    let desired_specs = vec![
        json!({
            "kind": "datasource",
            "uid": "loki-main",
            "name": "Loki Main",
            "body": {"type": "loki"}
        }),
        json!({
            "kind": "dashboard",
            "uid": "cpu-main",
            "title": "CPU Main",
            "body": {
                "datasourceUids": ["loki-main", "prom-main"],
                "datasourceNames": ["Prometheus Main"],
                "pluginIds": ["timeseries", "geomap"]
            }
        }),
        json!({
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["condition", "contactPoints"],
            "body": {
                "condition": "A > 90",
                "datasourceUid": "loki-main",
                "datasourceName": "Prometheus Main",
                "pluginIds": ["grafana-oncall-app"],
                "contactPoints": ["pagerduty-primary"],
                "notificationSettings": {"receiver": "slack-primary"}
            }
        }),
    ];
    let availability = json!({
        "pluginIds": ["prometheus", "timeseries"],
        "datasourceUids": ["prom-main"],
        "datasourceNames": [],
        "contactPoints": []
    });

    let document = build_sync_preflight_document(&desired_specs, Some(&availability)).unwrap();

    assert_eq!(document["kind"], json!(SYNC_PREFLIGHT_KIND));
    assert_eq!(document["summary"]["checkCount"], json!(13));
    assert_eq!(document["summary"]["blockingCount"], json!(10));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "plugin" && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "dashboard-datasource"
            && item["identity"] == "cpu-main->loki-main"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "dashboard-datasource-name"
            && item["identity"] == "cpu-main->Prometheus Main"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "dashboard-plugin"
            && item["identity"] == "cpu-main->timeseries"
            && item["status"] == "ok"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "dashboard-plugin"
            && item["identity"] == "cpu-main->geomap"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-live-apply" && item["status"] == "blocked"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-datasource"
            && item["identity"] == "cpu-high->loki-main"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-datasource-name"
            && item["identity"] == "cpu-high->Prometheus Main"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-plugin"
            && item["identity"] == "cpu-high->grafana-oncall-app"
            && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-contact-point" && item["status"] == "missing"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-contact-point"
            && item["identity"] == "cpu-high->slack-primary"
            && item["status"] == "missing"));
}

#[test]
fn sync_preflight_summary_reads_counts_from_document() {
    let document = build_sync_preflight_document(
        &[json!({
            "kind": "folder",
            "uid": "ops",
            "title": "Operations"
        })],
        None,
    )
    .unwrap();

    let summary = SyncPreflightSummary::from_document(&document).unwrap();

    assert_eq!(summary.check_count, 1);
    assert_eq!(summary.ok_count, 1);
    assert_eq!(summary.blocking_count, 0);
}

#[test]
fn build_sync_preflight_document_accepts_non_rule_alert_resources_for_live_apply() {
    let desired_specs = vec![json!({
        "kind": "alert-contact-point",
        "uid": "cp-main",
        "title": "PagerDuty Primary",
        "managedFields": ["uid", "name", "type", "settings"],
        "body": {
            "uid": "cp-main",
            "name": "PagerDuty Primary",
            "type": "webhook",
            "settings": {"url": "http://127.0.0.1/notify"}
        }
    })];

    let document = build_sync_preflight_document(&desired_specs, None).unwrap();

    assert_eq!(document["summary"]["blockingCount"], json!(0));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-live-apply"
            && item["identity"] == "cp-main"
            && item["status"] == "ok"));
}

#[test]
fn render_sync_preflight_text_renders_deterministic_summary() {
    let document = build_sync_preflight_document(
        &[json!({
            "kind": "folder",
            "uid": "ops",
            "title": "Operations"
        })],
        None,
    )
    .unwrap();

    let lines = render_sync_preflight_text(&document).unwrap();

    assert_eq!(lines[0], "Sync preflight summary");
    assert!(lines[1].contains("1 total"));
    assert!(lines
        .iter()
        .any(|line| line.contains("folder identity=ops status=ok")));
}

#[test]
fn render_sync_preflight_text_rejects_wrong_kind() {
    let error = render_sync_preflight_text(&json!({"kind": "wrong"}))
        .unwrap_err()
        .to_string();

    assert!(error.contains("kind is not supported"));
}

#[test]
fn build_sync_apply_intent_document_requires_review_and_approval() {
    let plan = build_sync_plan_document(
        &[json!({
            "kind": "folder",
            "uid": "ops",
            "title": "Operations",
            "body": {"title": "Operations"}
        })],
        &[],
        false,
    )
    .unwrap();

    let not_reviewed = build_sync_apply_intent_document(&plan, true)
        .unwrap_err()
        .to_string();
    assert!(not_reviewed.contains("marked reviewed"));

    let mut reviewed = plan.as_object().cloned().unwrap();
    reviewed.insert("reviewed".to_string(), json!(true));
    let not_approved = build_sync_apply_intent_document(&json!(reviewed), false)
        .unwrap_err()
        .to_string();
    assert!(not_approved.contains("explicit approval"));
}

#[test]
fn build_sync_apply_intent_document_filters_non_mutating_operations() {
    let plan = json!({
        "kind": "grafana-utils-sync-plan",
        "reviewRequired": true,
        "reviewed": true,
        "allowPrune": false,
        "summary": {
            "would_create": 1,
            "would_update": 1,
            "would_delete": 0,
            "noop": 1,
            "unmanaged": 1,
            "alert_candidate": 0,
            "alert_plan_only": 0,
            "alert_blocked": 0
        },
        "alertAssessment": {
            "summary": {
                "candidateCount": 0,
                "planOnlyCount": 0,
                "blockedCount": 0
            }
        },
        "operations": [
            {"kind":"folder","identity":"ops","action":"would-create"},
            {"kind":"dashboard","identity":"cpu-main","action":"would-update"},
            {"kind":"datasource","identity":"prom-main","action":"noop"},
            {"kind":"folder","identity":"legacy","action":"unmanaged"}
        ]
    });

    let intent = build_sync_apply_intent_document(&plan, true).unwrap();

    assert_eq!(intent["kind"], json!(SYNC_APPLY_INTENT_KIND));
    assert_eq!(
        intent["schemaVersion"],
        json!(SYNC_APPLY_INTENT_SCHEMA_VERSION)
    );
    assert_eq!(intent["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(intent["mode"], json!("apply"));
    assert_eq!(intent["approved"], json!(true));
    assert_eq!(intent["reviewRequired"], json!(true));
    assert_eq!(intent["allowPrune"], json!(false));
    assert_eq!(intent["operations"].as_array().unwrap().len(), 2);
    assert!(intent["operations"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| matches!(
            item["action"].as_str(),
            Some("would-create" | "would-update" | "would-delete")
        )));
}

#[test]
fn build_sync_plan_document_prunes_alert_policy_when_requested() {
    let plan = build_sync_plan_document(
        &[],
        &[json!({
            "kind": "alert-policy",
            "title": "grafana-default-email",
            "managedFields": ["receiver"],
            "body": {
                "receiver": "grafana-default-email"
            }
        })],
        true,
    )
    .unwrap();

    assert_eq!(plan["summary"]["would_delete"], json!(1));
    assert_eq!(plan["summary"]["unmanaged"], json!(0));
    assert_eq!(plan["operations"][0]["action"], json!("would-delete"));
    assert_eq!(
        plan["operations"][0]["reason"],
        json!("missing-from-desired-state")
    );
}

#[test]
fn build_sync_plan_document_skips_default_empty_alert_policy_baseline() {
    let plan = build_sync_plan_document(
        &[],
        &[json!({
            "kind": "alert-policy",
            "title": "empty",
            "managedFields": ["receiver", "group_by"],
            "body": {
                "group_by": ["grafana_folder", "alertname"],
                "receiver": "empty"
            }
        })],
        false,
    )
    .unwrap();

    assert_eq!(plan["summary"]["unmanaged"], json!(0));
    assert_eq!(plan["summary"]["would_delete"], json!(0));
    assert_eq!(plan["summary"]["blocked_reasons"], json!([]));
    assert_eq!(plan["alertAssessment"]["summary"]["alertCount"], json!(0));
    assert_eq!(plan["operations"].as_array().map(Vec::len), Some(0));
}

#[test]
fn build_sync_plan_document_prunes_non_rule_alert_delete_when_supported() {
    let plan = build_sync_plan_document(
        &[],
        &[json!({
            "kind": "alert-template",
            "name": "slack.default",
            "title": "slack.default",
            "managedFields": ["name", "template"],
            "body": {
                "name": "slack.default",
                "template": "{{ define \"slack.default\" }}ok{{ end }}"
            }
        })],
        true,
    )
    .unwrap();

    assert_eq!(plan["summary"]["would_delete"], json!(1));
    assert_eq!(plan["summary"]["unmanaged"], json!(0));
    assert_eq!(plan["operations"][0]["action"], json!("would-delete"));
    assert_eq!(
        plan["operations"][0]["reason"],
        json!("missing-from-desired-state")
    );
}

#[test]
fn build_sync_plan_document_adds_dependency_ordering_metadata() {
    let plan = build_sync_plan_document(
        &[
            json!({
                "kind": "dashboard",
                "uid": "cpu-main",
                "title": "CPU Main",
                "body": {"title": "CPU Main"},
            }),
            json!({
                "kind": "folder",
                "uid": "infra",
                "title": "Infra",
                "body": {"title": "Infra"},
            }),
            json!({
                "kind": "datasource",
                "uid": "prom-main",
                "name": "prom-main",
                "title": "prom-main",
                "body": {"type": "prometheus"},
            }),
        ],
        &[],
        false,
    )
    .unwrap();

    assert_eq!(plan["ordering"]["mode"], json!("dependency-aware"));
    assert_eq!(
        plan["ordering"]["createUpdateKindOrder"],
        json!(["folder", "datasource", "dashboard", "alert", "access"])
    );
    assert_eq!(
        plan["ordering"]["deleteKindOrder"],
        json!(["alert", "dashboard", "datasource", "folder", "access"])
    );
    let operations = plan["operations"].as_array().unwrap();
    assert_eq!(operations[0]["kind"], json!("folder"));
    assert_eq!(operations[0]["orderIndex"], json!(1));
    assert_eq!(operations[0]["kindOrder"], json!(0));
    assert_eq!(operations[1]["kind"], json!("datasource"));
    assert_eq!(operations[1]["kindOrder"], json!(1));
    assert_eq!(operations[2]["kind"], json!("dashboard"));
    assert_eq!(operations[2]["orderGroup"], json!("create-update"));
    assert_eq!(operations[2]["kindOrder"], json!(2));
    assert_eq!(plan["summary"]["blocked_reasons"], json!([]));
}

#[test]
fn enrich_workspace_preview_document_normalizes_actions_and_domain_summary() {
    let document = json!({
        "kind": "grafana-utils-sync-plan",
        "traceId": "sync-trace-demo",
        "summary": {
            "would_create": 1,
            "would_update": 0,
            "would_delete": 0,
            "noop": 0,
            "unmanaged": 0,
            "alert_candidate": 0,
            "alert_plan_only": 0,
            "alert_blocked": 1
        },
        "operations": [
            {
                "action": "would-create",
                "resourceKind": "alert",
                "identity": "alert-main"
            },
            {
                "action": "would-create",
                "resourceKind": "folder",
                "identity": "folder-main"
            },
            {
                "action": "would-create",
                "resourceKind": "dashboard",
                "identity": "dashboard-main"
            },
            {
                "action": "would-create",
                "resourceKind": "datasource",
                "identity": "datasource-main"
            },
            {
                "action": "would-create",
                "resourceKind": "access",
                "identity": "team-main"
            },
            {
                "action": "blocked-read-only",
                "resourceKind": "dashboard",
                "identity": "blocked-main",
                "status": "blocked",
                "reason": "target-read-only"
            }
        ]
    });

    let enriched =
        crate::sync::workspace_preview_contract::enrich_workspace_preview_document(&document)
            .unwrap();

    assert_eq!(enriched["actions"].as_array().map(Vec::len), Some(6));
    assert_eq!(enriched["operations"].as_array().map(Vec::len), Some(6));
    assert_eq!(enriched["blockedReasons"], json!(["target-read-only"]));
    assert_eq!(enriched["summary"]["actionCount"], json!(6));
    assert_eq!(enriched["summary"]["domainCount"], json!(5));
    assert_eq!(enriched["summary"]["blockedCount"], json!(1));
    assert_eq!(enriched["summary"]["warningCount"], json!(0));

    let actions = enriched["actions"].as_array().unwrap();
    assert_eq!(actions[0]["resourceKind"], json!("folder"));
    assert_eq!(actions[1]["resourceKind"], json!("datasource"));
    assert_eq!(actions[2]["resourceKind"], json!("dashboard"));
    assert_eq!(actions[3]["resourceKind"], json!("alert"));
    assert_eq!(actions[4]["resourceKind"], json!("access"));
    assert_eq!(actions[5]["action"], json!("blocked-read-only"));
    assert_eq!(actions[5]["domain"], json!("dashboard"));
    assert_eq!(actions[5]["status"], json!("blocked"));

    let domains = enriched["domains"].as_array().unwrap();
    assert!(domains
        .iter()
        .any(|domain| domain["id"] == json!("dashboard")));
    assert!(domains
        .iter()
        .any(|domain| domain["id"] == json!("datasource")));
    assert!(domains.iter().any(|domain| domain["id"] == json!("alert")));
    assert!(domains.iter().any(|domain| domain["id"] == json!("access")));
}

#[test]
fn render_sync_plan_text_shows_ordering_and_blocked_reasons() {
    let plan = build_sync_plan_document(
        &[json!({
            "kind": "folder",
            "uid": "infra",
            "title": "Infra",
            "body": {"title": "Infra"},
        })],
        &[json!({
            "kind": "datasource",
            "uid": "prom-main",
            "name": "prom-main",
            "title": "prom-main",
            "body": {"type": "prometheus"},
        })],
        false,
    )
    .unwrap();

    let lines = render_sync_plan_text(&plan).unwrap();

    assert!(lines
        .iter()
        .any(|line| line == "Ordering: dependency-aware"));
    assert!(lines
        .iter()
        .any(|line| line
            == "Blocked reason: kind=datasource identity=prom-main reason=prune-disabled"));
}
