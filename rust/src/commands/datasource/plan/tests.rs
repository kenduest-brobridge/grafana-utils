use serde_json::{json, Map, Value};
use std::path::PathBuf;

use super::super::DatasourceImportRecord;
use super::builder::{build_datasource_plan, build_datasource_plan_json};
use super::model::{
    DatasourcePlanInput, DatasourcePlanOrgInput, DatasourcePlanReport,
    PLAN_ACTION_BLOCKED_READ_ONLY, PLAN_ACTION_WOULD_DELETE, PLAN_ACTION_WOULD_UPDATE,
    PLAN_REASON_TARGET_READ_ONLY, PLAN_STATUS_READY,
};

fn record(uid: &str, name: &str, datasource_type: &str) -> DatasourceImportRecord {
    DatasourceImportRecord {
        uid: uid.to_string(),
        name: name.to_string(),
        datasource_type: datasource_type.to_string(),
        access: "proxy".to_string(),
        url: "http://example:9090".to_string(),
        is_default: false,
        org_name: String::new(),
        org_id: "1".to_string(),
        basic_auth: None,
        basic_auth_user: String::new(),
        database: String::new(),
        json_data: None,
        read_only: None,
        version: None,
        api_version: None,
        secure_json_data_placeholders: None,
        user: String::new(),
        with_credentials: None,
    }
}

fn live(uid: &str, name: &str, datasource_type: &str) -> Map<String, Value> {
    json!({
        "uid": uid,
        "name": name,
        "type": datasource_type,
        "access": "proxy",
        "url": "http://example:9090",
        "isDefault": false,
        "orgId": 1,
        "version": 7
    })
    .as_object()
    .unwrap()
    .clone()
}

fn report(
    records: Vec<DatasourceImportRecord>,
    live: Vec<Map<String, Value>>,
    prune: bool,
) -> DatasourcePlanReport {
    build_datasource_plan(DatasourcePlanInput {
        scope: "current-org".to_string(),
        input_format: "inventory".to_string(),
        prune,
        orgs: vec![DatasourcePlanOrgInput {
            source_org_id: "1".to_string(),
            source_org_name: "Main".to_string(),
            target_org_id: Some("1".to_string()),
            target_org_name: "Main".to_string(),
            org_action: "exists".to_string(),
            input_dir: PathBuf::from("/tmp/datasources"),
            records,
            live,
        }],
    })
}

#[test]
fn datasource_plan_marks_create_update_same_and_extra_remote() {
    let mut changed = record("loki", "Loki", "loki");
    changed.url = "http://loki:3100".to_string();
    let report = report(
        vec![
            record("prom", "Prometheus", "prometheus"),
            changed,
            record("tempo", "Tempo", "tempo"),
        ],
        vec![
            live("prom", "Prometheus", "prometheus"),
            live("loki", "Loki", "loki"),
            live("remote", "Remote Only", "prometheus"),
        ],
        false,
    );

    assert_eq!(report.summary.same, 1);
    assert_eq!(report.summary.create, 1);
    assert_eq!(report.summary.update, 1);
    assert_eq!(report.summary.extra, 1);
    assert_eq!(report.summary.delete, 0);
    assert!(report
        .actions
        .iter()
        .any(|item| item.action == PLAN_ACTION_WOULD_UPDATE && item.changed_fields == vec!["url"]));
}

#[test]
fn datasource_plan_prune_turns_extra_remote_into_delete_candidate() {
    let report = report(
        Vec::new(),
        vec![live("remote", "Remote Only", "loki")],
        true,
    );

    assert_eq!(report.summary.extra, 0);
    assert_eq!(report.summary.delete, 1);
    assert_eq!(report.actions[0].action, PLAN_ACTION_WOULD_DELETE);
    assert_eq!(report.actions[0].status, PLAN_STATUS_READY);
}

#[test]
fn datasource_plan_blocks_read_only_update_and_delete() {
    let mut changed = record("prom", "Prometheus", "prometheus");
    changed.url = "http://new-prometheus:9090".to_string();
    let mut live_record = live("prom", "Prometheus", "prometheus");
    live_record.insert("readOnly".to_string(), Value::Bool(true));
    let report = report(vec![changed], vec![live_record], true);

    assert_eq!(report.summary.blocked, 1);
    assert_eq!(report.actions[0].action, PLAN_ACTION_BLOCKED_READ_ONLY);
    assert_eq!(
        report.actions[0].blocked_reason.as_deref(),
        Some(PLAN_REASON_TARGET_READ_ONLY)
    );
}

#[test]
fn datasource_plan_json_keeps_tui_stable_action_id() {
    let report = report(
        vec![record("prom", "Prometheus", "prometheus")],
        vec![live("prom", "Prometheus", "prometheus")],
        false,
    );
    let value = build_datasource_plan_json(&report).unwrap();

    assert_eq!(value["kind"], json!("grafana-util-datasource-plan"));
    assert_eq!(value["schemaVersion"], json!(1));
    assert_eq!(
        value["actions"][0]["actionId"],
        json!("org:1/datasource:uid:prom")
    );
}
