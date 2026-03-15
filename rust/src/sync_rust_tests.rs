use crate::sync_contracts::{
    build_sync_apply_intent_document, build_sync_plan_document, build_sync_summary_document,
    normalize_resource_spec, summarize_resource_specs, SYNC_APPLY_INTENT_KIND,
};
use crate::sync_preflight::{
    build_sync_preflight_document, render_sync_preflight_text, SYNC_PREFLIGHT_KIND,
};
use serde_json::{json, Value};

fn load_contract_cases() -> Value {
    serde_json::from_str(include_str!(
        "../../tests/fixtures/rust_sync_contract_cases.json"
    ))
    .unwrap()
}

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
    let cases = load_contract_cases();
    let summary_case = cases.get("summaryCase").and_then(Value::as_object).unwrap();
    let raw_specs = summary_case
        .get("rawSpecs")
        .and_then(Value::as_array)
        .unwrap()
        .clone();
    let expected = summary_case
        .get("expectedSummary")
        .and_then(Value::as_object)
        .unwrap();

    let document = build_sync_summary_document(&raw_specs).unwrap();

    assert_eq!(document["kind"], expected.get("kind").cloned().unwrap());
    assert_eq!(
        document["schemaVersion"],
        expected.get("schemaVersion").cloned().unwrap()
    );
    assert_eq!(
        document["summary"]["resourceCount"],
        expected.get("resourceCount").cloned().unwrap()
    );
    assert_eq!(
        document["summary"]["dashboardCount"],
        expected.get("dashboardCount").cloned().unwrap()
    );
    assert_eq!(
        document["summary"]["datasourceCount"],
        expected.get("datasourceCount").cloned().unwrap()
    );
    assert_eq!(
        document["summary"]["folderCount"],
        expected.get("folderCount").cloned().unwrap()
    );
    assert_eq!(
        document["summary"]["alertCount"],
        expected.get("alertCount").cloned().unwrap()
    );
    assert_eq!(
        document["resources"][3]["managedFields"],
        expected.get("alertManagedFields").cloned().unwrap()
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
            "body": {"datasourceUids": ["loki-main", "prom-main"]}
        }),
        json!({
            "kind": "alert",
            "uid": "cpu-high",
            "title": "CPU High",
            "managedFields": ["condition", "contactPoints"],
            "body": {"condition": "A > 90", "contactPoints": ["pagerduty-primary"]}
        }),
    ];
    let availability = json!({
        "pluginIds": ["prometheus"],
        "datasourceUids": ["prom-main"],
        "contactPoints": []
    });

    let document = build_sync_preflight_document(&desired_specs, Some(&availability)).unwrap();

    assert_eq!(document["kind"], json!(SYNC_PREFLIGHT_KIND));
    assert_eq!(document["summary"]["checkCount"], json!(6));
    assert_eq!(document["summary"]["blockingCount"], json!(4));
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
        .any(|item| item["kind"] == "alert-live-apply" && item["status"] == "blocked"));
    assert!(document["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["kind"] == "alert-contact-point" && item["status"] == "missing"));
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
    assert_eq!(intent["mode"], json!("apply"));
    assert_eq!(intent["approved"], json!(true));
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
fn build_sync_plan_document_exposes_scope_and_prune_contract() {
    let cases = load_contract_cases();
    let plan_case = cases.get("planCase").and_then(Value::as_object).unwrap();
    let desired_specs = plan_case
        .get("desiredSpecs")
        .and_then(Value::as_array)
        .unwrap()
        .clone();
    let live_specs = plan_case
        .get("liveSpecs")
        .and_then(Value::as_array)
        .unwrap()
        .clone();
    let allow_prune = plan_case
        .get("allowPrune")
        .and_then(Value::as_bool)
        .unwrap();
    let expected = plan_case
        .get("expectedPlan")
        .and_then(Value::as_object)
        .unwrap();

    let document = build_sync_plan_document(&desired_specs, &live_specs, allow_prune).unwrap();

    assert_eq!(
        document["scope"]["managedResourceKinds"],
        expected["scope"]["managedResourceKinds"].clone()
    );
    assert_eq!(
        document["scope"]["alertOwnership"]["mode"],
        expected["scope"]["alertOwnershipMode"].clone()
    );
    assert_eq!(document["kind"], expected.get("kind").cloned().unwrap());
    assert_eq!(
        document["schemaVersion"],
        expected.get("schemaVersion").cloned().unwrap()
    );
    assert_eq!(
        document["allowPrune"],
        expected.get("allowPrune").cloned().unwrap()
    );
    assert_eq!(
        document["scope"]["prune"]["enabled"],
        expected["scope"]["pruneEnabled"].clone()
    );
    assert_eq!(
        document["scope"]["prune"]["liveOnlyAction"],
        expected["scope"]["liveOnlyAction"].clone()
    );
    assert_eq!(
        document["scope"]["prune"]["whenDisabledAction"],
        expected["scope"]["whenDisabledAction"].clone()
    );
    assert_eq!(
        document["scope"]["liveApplyContract"]["nonMutatingActions"],
        expected["scope"]["nonMutatingActions"].clone()
    );
    assert_eq!(
        document["summary"]["unmanaged"],
        expected["summary"]["unmanaged"].clone()
    );
    assert!(document["operations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["identity"] == expected["unmanagedIdentity"]
            && item["action"] == expected["scope"]["liveOnlyAction"]));
}

#[test]
fn build_sync_apply_intent_document_preserves_scope_and_prune_contract() {
    let cases = load_contract_cases();
    let apply_case = cases.get("applyCase").and_then(Value::as_object).unwrap();
    let plan = apply_case.get("reviewedPlan").cloned().unwrap();
    let approve = apply_case
        .get("approve")
        .and_then(Value::as_bool)
        .unwrap();
    let expected = apply_case
        .get("expectedIntent")
        .and_then(Value::as_object)
        .unwrap();

    let intent = build_sync_apply_intent_document(&plan, approve).unwrap();

    assert_eq!(intent["kind"], expected.get("kind").cloned().unwrap());
    assert_eq!(
        intent["schemaVersion"],
        expected.get("schemaVersion").cloned().unwrap()
    );
    assert_eq!(intent["allowPrune"], expected.get("allowPrune").cloned().unwrap());
    assert_eq!(
        intent["scope"]["prune"]["enabled"],
        expected.get("pruneEnabled").cloned().unwrap()
    );
    assert_eq!(
        intent["scope"]["prune"]["liveOnlyAction"],
        expected.get("liveOnlyAction").cloned().unwrap()
    );
    assert_eq!(
        intent["scope"]["liveApplyContract"]["nonMutatingActions"],
        expected.get("nonMutatingActions").cloned().unwrap()
    );
    assert_eq!(
        intent["operations"].as_array().unwrap().len(),
        expected
            .get("operationCount")
            .and_then(Value::as_u64)
            .unwrap() as usize
    );
    assert_eq!(
        intent["operations"][0]["identity"],
        expected.get("firstIdentity").cloned().unwrap()
    );
}
