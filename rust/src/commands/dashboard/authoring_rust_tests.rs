//! Regression coverage for dashboard authoring and review helper behavior.
//!
//! Focus:
//! - Cover live fetch + file clone paths for local draft generation.
//! - Verify review output rendering for text/csv/table/json/yaml contract stability.
//! - Keep behavior checks close to command-facing rendering without invoking live API.
use super::authoring::{
    clone_live_dashboard_to_file_with_request, get_live_dashboard_to_file_with_request,
    render_dashboard_review_csv, render_dashboard_review_json, render_dashboard_review_table,
    render_dashboard_review_text, render_dashboard_review_yaml, review_dashboard_file,
};
use crate::common::GrafanaCliError;
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;

type TestRequestResult = crate::common::Result<Option<Value>>;

fn read_json_output_file(path: &std::path::Path) -> Value {
    let raw = fs::read_to_string(path).unwrap();
    assert!(
        raw.ends_with('\n'),
        "expected output file {} to end with a newline",
        path.display()
    );
    serde_json::from_str(&raw).unwrap()
}

fn live_dashboard_payload() -> Value {
    json!({
        "dashboard": {
            "id": 42,
            "uid": "cpu-main",
            "title": "CPU Main",
            "schemaVersion": 39,
            "tags": ["ops"]
        },
        "meta": {
            "folderUid": "infra",
            "folderTitle": "Infra",
            "slug": "cpu-main"
        }
    })
}

#[allow(clippy::type_complexity)]
fn dashboard_request_fixture(
    payload: Value,
) -> impl FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> TestRequestResult {
    move |method, path, _params, _payload| match (method, path) {
        (reqwest::Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(payload.clone())),
        _ => Err(crate::common::message(format!("unexpected request {path}"))),
    }
}

fn write_json_file(path: &std::path::Path, value: &Value) {
    fs::write(path, serde_json::to_string_pretty(value).unwrap() + "\n").unwrap();
}

#[test]
fn get_live_dashboard_to_file_with_request_writes_id_null_and_preserves_meta() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("cpu-main.json");

    let document = get_live_dashboard_to_file_with_request(
        dashboard_request_fixture(live_dashboard_payload()),
        "cpu-main",
        &output,
    )
    .unwrap();

    assert_eq!(document["dashboard"]["id"], Value::Null);
    assert_eq!(document["dashboard"]["uid"], "cpu-main");
    assert_eq!(document["dashboard"]["title"], "CPU Main");
    assert_eq!(document["meta"]["folderUid"], "infra");
    assert_eq!(read_json_output_file(&output), document);
}

#[test]
fn clone_live_dashboard_to_file_with_request_applies_overrides() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("cpu-main-clone.json");

    let document = clone_live_dashboard_to_file_with_request(
        dashboard_request_fixture(live_dashboard_payload()),
        "cpu-main",
        &output,
        Some("CPU Clone"),
        Some("cpu-main-clone"),
        Some("ops"),
    )
    .unwrap();

    assert_eq!(document["dashboard"]["id"], Value::Null);
    assert_eq!(document["dashboard"]["uid"], "cpu-main-clone");
    assert_eq!(document["dashboard"]["title"], "CPU Clone");
    assert_eq!(document["meta"]["folderUid"], "ops");
    assert_eq!(document["meta"]["folderTitle"], "Infra");
    assert_eq!(read_json_output_file(&output), document);
}

#[test]
fn get_live_dashboard_to_file_with_request_errors_on_missing_dashboard() {
    let temp = tempdir().unwrap();
    let output = temp.path().join("missing.json");

    let error = get_live_dashboard_to_file_with_request(
        |_method, _path, _params, _payload| Ok(None),
        "cpu-main",
        &output,
    )
    .unwrap_err();

    assert!(matches!(error, GrafanaCliError::Message(_)));
    assert!(!output.exists());
}

#[test]
fn review_dashboard_file_reports_wrapped_metadata_and_patch_file_next_action() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("wrapped.json");
    write_json_file(
        &input,
        &json!({
            "dashboard": {
                "id": 17,
                "uid": "cpu-main",
                "title": "CPU Main",
                "schemaVersion": 39,
                "tags": ["ops", "sre"]
            },
            "meta": {
                "folderUid": "infra",
                "message": "Promote CPU dashboard"
            }
        }),
    );

    let review = review_dashboard_file(&input).unwrap();
    assert_eq!(review.document_kind, "wrapped");
    assert_eq!(review.title, "CPU Main");
    assert_eq!(review.uid, "cpu-main");
    assert_eq!(review.folder_uid.as_deref(), Some("infra"));
    assert_eq!(review.tags, vec!["ops".to_string(), "sre".to_string()]);
    assert!(!review.dashboard_id_is_null);
    assert!(review.meta_message_present);
    assert!(review.blocking_issues.is_empty());
    assert_eq!(review.suggested_next_action, "patch");

    let text = render_dashboard_review_text(&review);
    assert!(text.iter().any(|line| line == "Kind: wrapped"));
    assert!(text.iter().any(|line| line == "dashboard.id: non-null"));
    let table = render_dashboard_review_table(&review);
    assert!(table.iter().any(|line| line.contains("patch")));
    let csv = render_dashboard_review_csv(&review);
    assert!(csv.iter().any(|line| line.contains("blocking_issues")));
    let json_output = render_dashboard_review_json(&review).unwrap();
    assert!(json_output.contains("\"kind\": \"grafana-utils-dashboard-authoring-review\""));
    assert!(json_output.contains("\"suggestedNextAction\": \"patch\""));
    let yaml_output = render_dashboard_review_yaml(&review).unwrap();
    assert!(yaml_output.contains("suggestedNextAction: patch"));
}

#[test]
fn review_dashboard_file_reports_bare_metadata_and_publish_dry_run_next_action() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("bare.json");
    write_json_file(
        &input,
        &json!({
            "id": null,
            "uid": "cpu-main",
            "title": "CPU Main",
            "schemaVersion": 39,
            "tags": ["ops"]
        }),
    );

    let review = review_dashboard_file(&input).unwrap();
    assert_eq!(review.document_kind, "bare");
    assert_eq!(review.folder_uid, None);
    assert!(review.dashboard_id_is_null);
    assert!(!review.meta_message_present);
    assert_eq!(review.suggested_next_action, "publish --dry-run");

    let text = render_dashboard_review_text(&review);
    assert!(text.iter().any(|line| line == "Kind: bare"));
    assert!(text.iter().any(|line| line == "dashboard.id: null"));
}

#[test]
fn review_dashboard_file_surfaces_blocking_validation_and_adjusts_next_action() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("blocked.json");
    write_json_file(
        &input,
        &json!({
            "__inputs": [
                {
                    "name": "DS_PROMETHEUS",
                    "label": "Prometheus"
                }
            ],
            "dashboard": {
                "id": null,
                "uid": "cpu-main",
                "title": "CPU Main",
                "schemaVersion": 39,
                "tags": ["ops"]
            }
        }),
    );

    let review = review_dashboard_file(&input).unwrap();
    assert_eq!(review.document_kind, "wrapped");
    assert_eq!(
        review.suggested_next_action,
        "fix blocking issues, then publish --dry-run"
    );
    assert!(!review.blocking_issues.is_empty());
    assert!(review
        .blocking_issues
        .iter()
        .any(|issue| issue.contains("__inputs")));
}
