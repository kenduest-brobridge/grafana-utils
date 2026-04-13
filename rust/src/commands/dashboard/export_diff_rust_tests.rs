//! Dashboard domain test suite.
//! Covers parser surfaces, formatter/output contracts, and export/import/inspect/list/diff
//! behavior with in-memory/mocked request fixtures.
#![allow(unused_imports)]

use super::test_support;
use super::test_support::{
    attach_dashboard_folder_paths_with_request, build_export_metadata, build_export_variant_dirs,
    build_external_export_document, build_folder_inventory_status, build_folder_path,
    build_governance_gate_tui_groups, build_governance_gate_tui_items, build_impact_browser_items,
    build_impact_document, build_impact_tui_groups, build_import_auth_context,
    build_import_payload, build_output_path, build_preserved_web_import_document,
    build_root_export_index, build_topology_document, build_topology_tui_groups,
    diff_dashboards_with_request, discover_dashboard_files, export_dashboards_with_request,
    extract_dashboard_variables, filter_impact_tui_items, filter_topology_tui_items,
    format_dashboard_summary_line, format_export_progress_line, format_export_verbose_line,
    format_folder_inventory_status_line, format_import_progress_line, format_import_verbose_line,
    import_dashboards_with_org_clients, import_dashboards_with_request,
    list_dashboards_with_request, parse_cli_from, render_dashboard_governance_gate_result,
    render_dashboard_summary_csv, render_dashboard_summary_json, render_dashboard_summary_table,
    render_impact_text, render_import_dry_run_json, render_import_dry_run_table,
    render_topology_dot, render_topology_mermaid, CommonCliArgs, DashboardCliArgs,
    DashboardCommand, DashboardGovernanceGateFinding, DashboardGovernanceGateResult,
    DashboardGovernanceGateSummary, DiffArgs, ExportArgs, FolderInventoryStatusKind,
    GovernanceGateArgs, GovernanceGateOutputFormat, ImpactAlertResource, ImpactDashboard,
    ImpactDocument, ImpactOutputFormat, ImpactSummary, ImportArgs, InspectExportArgs,
    InspectExportReportFormat, InspectLiveArgs, InspectOutputFormat, ListArgs, SimpleOutputFormat,
    TopologyDocument, TopologyOutputFormat, ValidationOutputFormat,
    DASHBOARD_PERMISSION_BUNDLE_FILENAME, DATASOURCE_INVENTORY_FILENAME, EXPORT_METADATA_FILENAME,
    FOLDER_INVENTORY_FILENAME, TOOL_SCHEMA_VERSION,
};
use super::{
    assert_all_orgs_export_live_documents_match, assert_governance_documents_match,
    export_query_row, load_inspection_analyzer_cases, load_prompt_export_cases,
    make_basic_common_args, make_common_args, make_import_args, sample_topology_tui_document,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata,
};
use crate::common::{
    api_response, build_shared_diff_document, DiffOutputFormat, SharedDiffSummary, TOOL_VERSION,
};
use crate::dashboard::inspect::{
    dispatch_query_analysis, extract_query_field_and_text, resolve_query_analyzer_family,
    QueryAnalysis, QueryExtractionContext,
};
use crate::dashboard::inspect_governance::governance_risk_spec;
use clap::{CommandFactory, Parser};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn collect_folder_inventory_statuses_with_request_reports_match_mismatch_and_missing() {
    let folders = vec![
        test_support::FolderInventoryItem {
            uid: "platform".to_string(),
            title: "Platform".to_string(),
            path: "Platform".to_string(),
            parent_uid: None,
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        },
        test_support::FolderInventoryItem {
            uid: "child".to_string(),
            title: "Child".to_string(),
            path: "Platform / Child".to_string(),
            parent_uid: Some("platform".to_string()),
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        },
        test_support::FolderInventoryItem {
            uid: "missing".to_string(),
            title: "Missing".to_string(),
            path: "Missing".to_string(),
            parent_uid: None,
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        },
    ];

    let statuses = test_support::collect_folder_inventory_statuses_with_request(
        &mut |method, path, _params, _payload| match (method, path) {
            (reqwest::Method::GET, "/api/folders/platform") => Ok(Some(json!({
                "uid": "platform",
                "title": "Platform",
                "parents": []
            }))),
            (reqwest::Method::GET, "/api/folders/child") => Ok(Some(json!({
                "uid": "child",
                "title": "Legacy Child",
                "parents": [{"uid": "platform", "title": "Platform"}]
            }))),
            (reqwest::Method::GET, "/api/folders/missing") => Ok(None),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &folders,
    )
    .unwrap();

    assert_eq!(statuses[0].kind, FolderInventoryStatusKind::Matches);
    assert_eq!(statuses[1].kind, FolderInventoryStatusKind::Mismatch);
    assert_eq!(statuses[2].kind, FolderInventoryStatusKind::Missing);
}

#[test]
fn diff_dashboards_with_client_returns_zero_for_matching_dashboard() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
            "meta": {"folderUid": "old-folder"}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = DiffArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        input_dir: raw_dir,
        input_format: test_support::DashboardImportInputFormat::Raw,
        import_folder_uid: Some("old-folder".to_string()),
        context_lines: 3,
        output_format: DiffOutputFormat::Text,
    };

    let count = diff_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/dashboards/uid/abc" => Ok(Some(json!({
                "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
                "meta": {"folderUid": "old-folder"}
            }))),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 0);
}

#[test]
fn diff_dashboards_with_client_detects_dashboard_difference() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU"}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = DiffArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        input_dir: raw_dir,
        input_format: test_support::DashboardImportInputFormat::Raw,
        import_folder_uid: None,
        context_lines: 3,
        output_format: DiffOutputFormat::Text,
    };

    let count = diff_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/dashboards/uid/abc" => Ok(Some(json!({
                "dashboard": {"id": 7, "uid": "abc", "title": "Memory"}
            }))),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn diff_dashboards_with_client_supports_provisioning_root() {
    let temp = tempdir().unwrap();
    let provisioning_root = temp.path().join("provisioning");
    let dashboards_dir = provisioning_root.join("dashboards");
    fs::create_dir_all(&dashboards_dir).unwrap();
    fs::write(
        dashboards_dir.join("cpu.json"),
        serde_json::to_string_pretty(&json!({
            "uid": "abc",
            "title": "CPU"
        }))
        .unwrap(),
    )
    .unwrap();
    let args = DiffArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        input_dir: provisioning_root,
        input_format: test_support::DashboardImportInputFormat::Provisioning,
        import_folder_uid: None,
        context_lines: 3,
        output_format: DiffOutputFormat::Text,
    };

    let count = diff_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/dashboards/uid/abc" => Ok(Some(json!({
                "dashboard": {"uid": "abc", "title": "CPU"}
            }))),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 0);
}

#[test]
fn diff_dashboards_with_client_supports_provisioning_dashboards_dir() {
    let temp = tempdir().unwrap();
    let provisioning_root = temp.path().join("provisioning");
    let dashboards_dir = provisioning_root.join("dashboards");
    fs::create_dir_all(dashboards_dir.join("nested")).unwrap();
    fs::write(
        dashboards_dir.join("nested/cpu.json"),
        serde_json::to_string_pretty(&json!({
            "uid": "abc",
            "title": "CPU"
        }))
        .unwrap(),
    )
    .unwrap();
    let args = DiffArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        input_dir: dashboards_dir,
        input_format: test_support::DashboardImportInputFormat::Provisioning,
        import_folder_uid: None,
        context_lines: 3,
        output_format: DiffOutputFormat::Text,
    };

    let count = diff_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/dashboards/uid/abc" => Ok(Some(json!({
                "dashboard": {"uid": "abc", "title": "CPU"}
            }))),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 0);
}

fn load_shared_diff_golden_fixture(domain: &str) -> Value {
    serde_json::from_str::<Vec<Value>>(include_str!(
        "../../../../fixtures/shared_diff_golden_cases.json"
    ))
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

#[test]
fn dashboard_diff_json_contract_preserves_envelope_and_rows() {
    let fixture = load_shared_diff_golden_fixture("dashboard");
    let document = build_shared_diff_document(
        "grafana-util-dashboard-diff",
        1,
        SharedDiffSummary {
            checked: 2,
            same: 1,
            different: 1,
            missing_remote: 0,
            extra_remote: 0,
            ambiguous: 0,
        },
        &[json!({
            "domain": "dashboard",
            "resourceKind": "dashboard",
            "identity": "cpu-main",
            "status": "different",
            "path": "./dashboards/raw/cpu-main.json",
            "changedFields": ["dashboard"],
            "diffText": "--- grafana:cpu-main\n+++ ./dashboards/raw/cpu-main.json\n",
            "contextLines": 3,
        })],
    );

    assert_eq!(document["kind"], json!("grafana-util-dashboard-diff"));
    assert_eq!(document["schemaVersion"], json!(1));
    assert_eq!(document["summary"]["checked"], json!(2));
    assert_eq!(document["summary"]["same"], json!(1));
    assert_eq!(document["rows"].as_array().map(Vec::len), Some(1));
    assert_eq!(document, fixture["document"]);
}
