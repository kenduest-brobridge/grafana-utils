#[path = "datasource_diff.rs"]
mod datasource_diff;

use datasource_diff::{
    build_datasource_diff_report, normalize_export_records, normalize_live_records,
    DatasourceDiffStatus,
};
use serde_json::json;

#[test]
fn normalize_export_records_handles_string_bools_and_org_ids() {
    let records = normalize_export_records(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": "true",
        "orgId": 7
    })]);

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].uid, "prom-main");
    assert!(records[0].is_default);
    assert_eq!(records[0].org_id, "7");
}

#[test]
fn diff_report_marks_matching_records_by_uid() {
    let export_records = normalize_export_records(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": "1"
    })]);
    let live_records = normalize_live_records(&[json!({
        "id": 9,
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": 1
    })]);

    let report = build_datasource_diff_report(&export_records, &live_records);

    assert_eq!(report.summary.compared_count, 1);
    assert_eq!(report.summary.matches_count, 1);
    assert_eq!(report.entries[0].status, DatasourceDiffStatus::Matches);
    assert!(report.entries[0].differences.is_empty());
}

#[test]
fn diff_report_captures_field_level_differences() {
    let export_records = normalize_export_records(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": "1"
    })]);
    let live_records = normalize_live_records(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "loki",
        "access": "direct",
        "url": "http://loki:3100",
        "isDefault": false,
        "orgId": "1"
    })]);

    let report = build_datasource_diff_report(&export_records, &live_records);

    assert_eq!(report.summary.different_count, 1);
    assert_eq!(report.entries[0].status, DatasourceDiffStatus::Different);
    assert_eq!(report.entries[0].differences.len(), 4);
    assert_eq!(report.entries[0].differences[0].field, "type");
}

#[test]
fn diff_report_marks_missing_live_records() {
    let export_records = normalize_export_records(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus"
    })]);
    let live_records = normalize_live_records(&[]);

    let report = build_datasource_diff_report(&export_records, &live_records);

    assert_eq!(report.summary.missing_in_live_count, 1);
    assert_eq!(
        report.entries[0].status,
        DatasourceDiffStatus::MissingInLive
    );
}

#[test]
fn diff_report_marks_unmatched_live_records_as_missing_in_export() {
    let export_records = normalize_export_records(&[]);
    let live_records = normalize_live_records(&[json!({
        "uid": "logs-main",
        "name": "Logs Main",
        "type": "loki"
    })]);

    let report = build_datasource_diff_report(&export_records, &live_records);

    assert_eq!(report.summary.missing_in_export_count, 1);
    assert_eq!(
        report.entries[0].status,
        DatasourceDiffStatus::MissingInExport
    );
}

#[test]
fn diff_report_marks_ambiguous_name_matches_without_uid() {
    let export_records = normalize_export_records(&[json!({
        "name": "Shared Name",
        "type": "prometheus"
    })]);
    let live_records = normalize_live_records(&[
        json!({"uid": "a", "name": "Shared Name", "type": "prometheus"}),
        json!({"uid": "b", "name": "Shared Name", "type": "prometheus"}),
    ]);

    let report = build_datasource_diff_report(&export_records, &live_records);

    assert_eq!(report.summary.ambiguous_live_match_count, 1);
    assert_eq!(
        report.entries[0].status,
        DatasourceDiffStatus::AmbiguousLiveMatch
    );
}
