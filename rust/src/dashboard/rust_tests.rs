//! Dashboard domain test suite.
//! Covers parser surfaces, formatter/output contracts, and export/import/inspect/list/diff
//! behavior with in-memory/mocked request fixtures.
#![allow(unused_imports)]

use super::browse_support::fetch_dashboard_view_lines_with_request;
use super::delete::delete_dashboards_with_request;
use super::delete_support::{build_delete_plan_with_request, validate_delete_args};
use super::edit::{
    apply_dashboard_edit_with_request, fetch_dashboard_edit_draft_with_request,
    resolve_folder_uid_for_path, DashboardEditDraft, DashboardEditUpdate,
};
use super::edit_external::{
    apply_external_dashboard_edit_with_request, build_external_dashboard_edit_summary,
    review_external_dashboard_edit, validate_external_dashboard_edit_value,
    ExternalDashboardEditDraft,
};
use super::history::{
    list_dashboard_history_versions_with_request, restore_dashboard_history_version_with_request,
};
use super::import_interactive::{
    load_interactive_import_items, InteractiveImportAction, InteractiveImportState,
};
use super::test_support;
use super::test_support::{
    attach_dashboard_folder_paths_with_request, build_dashboard_browse_document,
    build_export_metadata, build_export_variant_dirs, build_external_export_document,
    build_folder_inventory_status, build_folder_path, build_governance_gate_tui_groups,
    build_governance_gate_tui_items, build_impact_browser_items, build_impact_document,
    build_impact_tui_groups, build_import_auth_context, build_import_payload, build_output_path,
    build_preserved_web_import_document, build_root_export_index, build_topology_document,
    build_topology_tui_groups, diff_dashboards_with_request, discover_dashboard_files,
    export_dashboards_with_request, extract_dashboard_variables, filter_impact_tui_items,
    filter_topology_tui_items, format_dashboard_summary_line, format_export_progress_line,
    format_export_verbose_line, format_folder_inventory_status_line, format_import_progress_line,
    format_import_verbose_line, import_dashboards_with_org_clients, import_dashboards_with_request,
    list_dashboards_with_request, parse_cli_from, render_dashboard_governance_gate_result,
    render_dashboard_summary_csv, render_dashboard_summary_json, render_dashboard_summary_table,
    render_impact_text, render_import_dry_run_json, render_import_dry_run_table,
    render_topology_dot, render_topology_mermaid, BrowseArgs, CommonCliArgs, DashboardCliArgs,
    DashboardCommand, DashboardGovernanceGateFinding, DashboardGovernanceGateResult,
    DashboardGovernanceGateSummary, DiffArgs, ExportArgs, FolderInventoryStatusKind,
    GovernanceGateArgs, GovernanceGateOutputFormat, GovernancePolicySource, ImpactAlertResource,
    ImpactDashboard, ImpactDocument, ImpactOutputFormat, ImpactSummary, ImportArgs,
    InspectExportArgs, InspectExportReportFormat, InspectLiveArgs, InspectOutputFormat, ListArgs,
    SimpleOutputFormat, TopologyDocument, TopologyOutputFormat, ValidationOutputFormat,
    DASHBOARD_PERMISSION_BUNDLE_FILENAME, DATASOURCE_INVENTORY_FILENAME, EXPORT_METADATA_FILENAME,
    FOLDER_INVENTORY_FILENAME, TOOL_SCHEMA_VERSION,
};
use crate::common::{api_response, message};
use crate::dashboard::inspect::{
    dispatch_query_analysis, extract_query_field_and_text, resolve_query_analyzer_family,
    QueryAnalysis, QueryExtractionContext,
};
use crate::dashboard::inspect_governance::governance_risk_spec;
use crate::dashboard::DeleteArgs;
use clap::{CommandFactory, Parser};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use reqwest::Method;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

pub(crate) type TestRequestResult = crate::common::Result<Option<Value>>;

pub(crate) fn make_common_args(base_url: String) -> CommonCliArgs {
    CommonCliArgs {
        url: base_url,
        api_token: Some("token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

pub(crate) fn make_basic_common_args(base_url: String) -> CommonCliArgs {
    CommonCliArgs {
        url: base_url,
        api_token: None,
        username: Some("admin".to_string()),
        password: Some("admin".to_string()),
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

#[allow(dead_code)]
fn load_prompt_export_cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/dashboard_prompt_export_cases.json"
    ))
    .unwrap()
}

fn load_inspection_analyzer_cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/dashboard_inspection_analyzer_cases.json"
    ))
    .unwrap()
}

fn sample_topology_tui_document() -> TopologyDocument {
    let governance = json!({
        "dashboardGovernance": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main"
            }
        ],
        "dashboardDatasourceEdges": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "datasourceUid": "prom-main",
                "datasource": "Prometheus Main",
                "panelCount": 1,
                "queryCount": 1,
                "queryFields": ["expr"],
                "queryVariables": ["cluster"],
                "metrics": ["up"],
                "functions": [],
                "measurements": [],
                "buckets": []
            }
        ],
        "dashboardDependencies": [
            {
                "dashboardUid": "cpu-main",
                "panelIds": ["7"],
                "panelVariables": ["cluster"],
                "queryVariables": ["cluster"]
            }
        ]
    });
    let alert_contract = json!({
        "kind": "grafana-utils-sync-alert-contract",
        "resources": [
            {
                "kind": "grafana-alert-rule",
                "identity": "cpu-high",
                "title": "CPU High",
                "references": ["prom-main", "cpu-main"]
            }
        ]
    });

    build_topology_document(&governance, Some(&alert_contract)).unwrap()
}

#[allow(clippy::type_complexity)]
pub(crate) fn with_dashboard_import_live_preflight<F>(
    preflight_datasources: Value,
    preflight_plugins: Value,
    mut handler: F,
) -> impl FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> TestRequestResult
where
    F: FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> TestRequestResult,
{
    move |method, path, params, payload| {
        if method == reqwest::Method::GET && path == "/api/datasources" {
            return Ok(Some(preflight_datasources.clone()));
        }
        if method == reqwest::Method::GET && path == "/api/plugins" {
            return Ok(Some(preflight_plugins.clone()));
        }
        if method == reqwest::Method::GET && path == "/api/search" {
            return Ok(Some(json!([])));
        }
        handler(method, path, params, payload)
    }
}

pub(crate) fn make_import_args(import_dir: PathBuf) -> ImportArgs {
    ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        org_id: None,
        use_export_org: false,
        only_org_id: Vec::new(),
        create_missing_orgs: false,
        import_dir,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: false,
        require_matching_folder_path: false,
        require_matching_export_org: false,
        strict_schema: false,
        target_schema_version: None,
        import_message: "sync dashboards".to_string(),
        interactive: false,
        dry_run: true,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
        output_columns: Vec::new(),
        progress: false,
        verbose: false,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn write_basic_raw_export(
    raw_dir: &Path,
    org_id: &str,
    org_name: &str,
    dashboard_uid: &str,
    dashboard_title: &str,
    datasource_uid: &str,
    datasource_type: &str,
    panel_type: &str,
    folder_uid: &str,
    folder_title: &str,
    query_field: &str,
    query_text: &str,
) {
    fs::create_dir_all(raw_dir).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": FOLDER_INVENTORY_FILENAME,
            "datasourcesFile": DATASOURCE_INVENTORY_FILENAME,
            "org": org_name,
            "orgId": org_id
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join(FOLDER_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&json!([
            {
                "uid": folder_uid,
                "title": folder_title,
                "path": folder_title,
                "org": org_name,
                "orgId": org_id
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join(DATASOURCE_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&json!([
            {
                "uid": datasource_uid,
                "name": datasource_uid,
                "type": datasource_type,
                "access": "proxy",
                "url": "http://grafana.example.internal",
                "isDefault": "true",
                "org": org_name,
                "orgId": org_id
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("index.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": dashboard_uid,
                "title": dashboard_title,
                "path": "dash.json",
                "format": "grafana-web-import-preserve-uid",
                "org": org_name,
                "orgId": org_id
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "id": null,
                "uid": dashboard_uid,
                "title": dashboard_title,
                "schemaVersion": 38,
                "panels": [{
                    "id": 7,
                    "title": dashboard_title,
                    "type": panel_type,
                    "datasource": {"uid": datasource_uid, "type": datasource_type},
                    "targets": [{
                        "refId": "A",
                        query_field: query_text
                    }]
                }]
            },
            "meta": {
                "folderUid": folder_uid,
                "folderTitle": folder_title
            }
        }))
        .unwrap(),
    )
    .unwrap();
}

pub(crate) fn write_combined_export_root_metadata(export_root: &Path, orgs: &[(&str, &str, &str)]) {
    fs::create_dir_all(export_root).unwrap();
    let org_entries: Vec<Value> = orgs
        .iter()
        .map(|(org_id, org_name, export_dir)| {
            json!({
                "org": org_name,
                "orgId": org_id,
                "dashboardCount": 1,
                "exportDir": export_dir
            })
        })
        .collect();
    fs::write(
        export_root.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "root",
            "dashboardCount": orgs.len(),
            "indexFile": "index.json",
            "orgCount": orgs.len(),
            "orgs": org_entries
        }))
        .unwrap(),
    )
    .unwrap();
}

pub(crate) fn read_json_output_file(path: &Path) -> Value {
    let raw = fs::read_to_string(path).unwrap();
    assert!(
        raw.ends_with('\n'),
        "expected output file {} to end with a newline",
        path.display()
    );
    serde_json::from_str(&raw).unwrap()
}

#[allow(clippy::too_many_arguments)]
fn make_core_family_report_row(
    dashboard_uid: &str,
    panel_id: &str,
    ref_id: &str,
    datasource_uid: &str,
    datasource_name: &str,
    datasource_type: &str,
    datasource_family: &str,
    query_text: &str,
    measurements: &[&str],
) -> test_support::ExportInspectionQueryRow {
    test_support::ExportInspectionQueryRow {
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
        dashboard_uid: dashboard_uid.to_string(),
        dashboard_title: format!("{dashboard_uid} Dashboard"),
        dashboard_tags: Vec::new(),
        folder_path: "General".to_string(),
        folder_full_path: "/".to_string(),
        folder_level: "1".to_string(),
        folder_uid: "general".to_string(),
        parent_folder_uid: String::new(),
        panel_id: panel_id.to_string(),
        panel_title: "Query".to_string(),
        panel_type: "table".to_string(),
        panel_target_count: 1,
        panel_query_count: 1,
        panel_datasource_count: 0,
        panel_variables: Vec::new(),
        ref_id: ref_id.to_string(),
        datasource: datasource_name.to_string(),
        datasource_name: datasource_name.to_string(),
        datasource_uid: datasource_uid.to_string(),
        datasource_org: String::new(),
        datasource_org_id: String::new(),
        datasource_database: String::new(),
        datasource_bucket: String::new(),
        datasource_organization: String::new(),
        datasource_index_pattern: String::new(),
        datasource_type: datasource_type.to_string(),
        datasource_family: datasource_family.to_string(),
        query_field: "query".to_string(),
        target_hidden: "false".to_string(),
        target_disabled: "false".to_string(),
        query_text: query_text.to_string(),
        query_variables: Vec::new(),
        metrics: Vec::new(),
        functions: Vec::new(),
        measurements: measurements.iter().map(|value| value.to_string()).collect(),
        buckets: Vec::new(),
        file_path: format!("/tmp/raw/{dashboard_uid}.json"),
    }
}

fn json_query_report_row<'a>(document: &'a Value, ref_id: &str) -> &'a Value {
    document["queries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["refId"] == Value::String(ref_id.to_string()))
        .unwrap()
}

pub(crate) fn assert_json_query_report_row_parity(
    export_document: &Value,
    live_document: &Value,
    ref_id: &str,
) {
    let export_row = json_query_report_row(export_document, ref_id);
    let live_row = json_query_report_row(live_document, ref_id);
    for field in [
        "org",
        "orgId",
        "dashboardUid",
        "dashboardTitle",
        "dashboardTags",
        "folderPath",
        "folderFullPath",
        "folderLevel",
        "folderUid",
        "parentFolderUid",
        "panelId",
        "panelTitle",
        "panelType",
        "panelTargetCount",
        "panelQueryCount",
        "panelDatasourceCount",
        "panelVariables",
        "refId",
        "datasource",
        "datasourceName",
        "datasourceUid",
        "datasourceType",
        "datasourceFamily",
        "queryField",
        "targetHidden",
        "targetDisabled",
        "queryVariables",
        "metrics",
        "functions",
        "measurements",
        "buckets",
        "query",
    ] {
        assert_eq!(
            export_row[field], live_row[field],
            "field={field}, refId={ref_id}"
        );
    }
}

pub(crate) fn normalize_governance_document_for_compare(document: &Value) -> Value {
    let mut normalized = document.clone();
    if let Some(rows) = normalized
        .get_mut("dashboardDependencies")
        .and_then(|value| value.as_array_mut())
    {
        for row in rows {
            if let Some(object) = row.as_object_mut() {
                object.remove("file");
            }
        }
    }
    normalized
}

pub(crate) fn normalize_queries_document_for_compare(document: &Value) -> Value {
    let mut normalized = document.clone();
    if let Some(rows) = normalized
        .get_mut("queries")
        .and_then(|value| value.as_array_mut())
    {
        for row in rows {
            if let Some(object) = row.as_object_mut() {
                object.remove("file");
                object.remove("datasourceOrg");
                object.remove("datasourceOrgId");
                object.remove("datasourceDatabase");
                object.remove("datasourceBucket");
                object.remove("datasourceOrganization");
                object.remove("datasourceIndexPattern");
            }
        }
    }
    normalized
}

pub(crate) fn assert_governance_documents_match(export_document: &Value, live_document: &Value) {
    assert_eq!(
        normalize_governance_document_for_compare(export_document),
        normalize_governance_document_for_compare(live_document)
    );
}

pub(crate) fn assert_all_orgs_export_live_documents_match(
    export_report_document: &Value,
    live_report_document: &Value,
    export_dependency_document: &Value,
    live_dependency_document: &Value,
    export_governance_document: &Value,
    live_governance_document: &Value,
) {
    assert_eq!(
        normalize_queries_document_for_compare(export_report_document),
        normalize_queries_document_for_compare(live_report_document)
    );
    assert_eq!(
        normalize_queries_document_for_compare(export_dependency_document),
        normalize_queries_document_for_compare(live_dependency_document)
    );
    assert_governance_documents_match(export_governance_document, live_governance_document);
}

#[allow(clippy::type_complexity)]
pub(crate) fn core_family_inspect_live_request_fixture(
    datasource_inventory: Value,
    dashboard_payload: Value,
) -> impl FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> TestRequestResult {
    move |method, path, params, _payload| {
        let method_name = method.to_string();
        match (method, path) {
            (reqwest::Method::GET, "/api/org") => Ok(Some(json!({
                "id": 1,
                "name": "Main Org."
            }))),
            (reqwest::Method::GET, "/api/datasources") => Ok(Some(datasource_inventory.clone())),
            (reqwest::Method::GET, "/api/search") => Ok(Some(json!([
                {
                    "uid": "core-main",
                    "title": "Core Main",
                    "type": "dash-db",
                    "folderUid": "general",
                    "folderTitle": "General"
                }
            ]))),
            (reqwest::Method::GET, "/api/folders/general") => Ok(Some(json!({
                "uid": "general",
                "title": "General"
            }))),
            (reqwest::Method::GET, "/api/folders/general/permissions") => Ok(Some(json!([]))),
            (reqwest::Method::GET, "/api/dashboards/uid/core-main") => {
                Ok(Some(dashboard_payload.clone()))
            }
            (reqwest::Method::GET, "/api/dashboards/uid/core-main/permissions") => {
                Ok(Some(json!([])))
            }
            _ => Err(test_support::message(format!(
                "unexpected request {method_name} {path} {params:?}"
            ))),
        }
    }
}

#[allow(clippy::type_complexity)]
fn all_orgs_inspect_live_request_fixture(
) -> impl FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> TestRequestResult {
    move |method, path, params, _payload| {
        let method_name = method.to_string();
        match (method, path) {
            (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                {"id": 1, "name": "Main Org."},
                {"id": 2, "name": "Ops Org"}
            ]))),
            (reqwest::Method::GET, "/api/org") => {
                let scoped_org = params
                    .iter()
                    .find(|(key, _)| key == "orgId")
                    .map(|(_, value)| value.as_str())
                    .unwrap_or("1");
                match scoped_org {
                    "1" => Ok(Some(json!({"id": 1, "name": "Main Org."}))),
                    "2" => Ok(Some(json!({"id": 2, "name": "Ops Org"}))),
                    other => panic!("unexpected org context {other}"),
                }
            }
            (reqwest::Method::GET, "/api/datasources") => {
                let scoped_org = params
                    .iter()
                    .find(|(key, _)| key == "orgId")
                    .map(|(_, value)| value.as_str())
                    .unwrap_or("1");
                match scoped_org {
                    "1" => Ok(Some(json!([
                        {
                            "uid": "prom-main",
                            "name": "Prometheus Main",
                            "type": "prometheus",
                            "access": "proxy",
                            "url": "http://prometheus:9090",
                            "isDefault": true
                        }
                    ]))),
                    "2" => Ok(Some(json!([
                        {
                            "uid": "prom-two",
                            "name": "Prometheus Two",
                            "type": "prometheus",
                            "access": "proxy",
                            "url": "http://prometheus-two:9090",
                            "isDefault": true
                        }
                    ]))),
                    other => panic!("unexpected org context {other}"),
                }
            }
            (reqwest::Method::GET, "/api/search") => {
                let scoped_org = params
                    .iter()
                    .find(|(key, _)| key == "orgId")
                    .map(|(_, value)| value.as_str())
                    .unwrap_or("1");
                match scoped_org {
                    "1" => Ok(Some(json!([
                        {
                            "uid": "cpu-main",
                            "title": "CPU Main",
                            "type": "dash-db",
                            "folderUid": "general",
                            "folderTitle": "General"
                        }
                    ]))),
                    "2" => Ok(Some(json!([
                        {
                            "uid": "latency-main",
                            "title": "Latency Main",
                            "type": "dash-db",
                            "folderUid": "ops",
                            "folderTitle": "Ops"
                        }
                    ]))),
                    other => panic!("unexpected org context {other}"),
                }
            }
            (reqwest::Method::GET, "/api/folders/general") => {
                Ok(Some(json!({"uid": "general", "title": "General"})))
            }
            (reqwest::Method::GET, "/api/folders/general/permissions") => Ok(Some(json!([]))),
            (reqwest::Method::GET, "/api/folders/ops") => {
                Ok(Some(json!({"uid": "ops", "title": "Ops"})))
            }
            (reqwest::Method::GET, "/api/folders/ops/permissions") => Ok(Some(json!([]))),
            (reqwest::Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "id": 11,
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "panels": [{
                        "id": 7,
                        "title": "CPU Query",
                        "type": "timeseries",
                        "datasource": {"uid": "prom-main", "type": "prometheus"},
                        "targets": [{"refId": "A", "expr": "up"}]
                    }]
                },
                "meta": {"folderUid": "general", "folderTitle": "General"}
            }))),
            (reqwest::Method::GET, "/api/dashboards/uid/cpu-main/permissions") => {
                Ok(Some(json!([])))
            }
            (reqwest::Method::GET, "/api/dashboards/uid/latency-main") => Ok(Some(json!({
                "dashboard": {
                    "id": 12,
                    "uid": "latency-main",
                    "title": "Latency Main",
                    "panels": [{
                        "id": 8,
                        "title": "Latency Query",
                        "type": "timeseries",
                        "datasource": {"uid": "prom-two", "type": "prometheus"},
                        "targets": [{"refId": "A", "expr": "rate(http_requests_total[5m])"}]
                    }]
                },
                "meta": {"folderUid": "ops", "folderTitle": "Ops"}
            }))),
            (reqwest::Method::GET, "/api/dashboards/uid/latency-main/permissions") => {
                Ok(Some(json!([])))
            }
            (_method, path) => Err(test_support::message(format!(
                "unexpected request {method_name} {path} {params:?}"
            ))),
        }
    }
}

fn export_query_row<'a>(
    report: &'a test_support::ExportInspectionQueryReport,
    dashboard_uid: &str,
) -> &'a test_support::ExportInspectionQueryRow {
    report
        .queries
        .iter()
        .find(|query| query.dashboard_uid == dashboard_uid)
        .unwrap()
}

#[derive(Clone, Copy, Default)]
struct CoreFamilyQueryRowExpectation<'a> {
    dashboard_uid: &'a str,
    dashboard_title: &'a str,
    panel_id: &'a str,
    panel_title: &'a str,
    panel_type: &'a str,
    ref_id: &'a str,
    datasource: &'a str,
    datasource_name: &'a str,
    datasource_uid: &'a str,
    datasource_type: &'a str,
    datasource_family: &'a str,
    query_field: &'a str,
    query_text: &'a str,
    folder_path: &'a str,
    folder_full_path: &'a str,
    folder_level: &'a str,
    folder_uid: &'a str,
    parent_folder_uid: &'a str,
    datasource_org: &'a str,
    datasource_org_id: &'a str,
    datasource_database: &'a str,
    datasource_bucket: &'a str,
    datasource_organization: &'a str,
    datasource_index_pattern: &'a str,
    metrics: &'a [&'a str],
    functions: &'a [&'a str],
    measurements: &'a [&'a str],
    buckets: &'a [&'a str],
}

fn assert_core_family_query_row(
    report: &test_support::ExportInspectionQueryReport,
    expected: CoreFamilyQueryRowExpectation<'_>,
) {
    let row = export_query_row(report, expected.dashboard_uid);
    if !expected.dashboard_uid.is_empty() {
        assert_eq!(row.dashboard_uid, expected.dashboard_uid);
    }
    if !expected.dashboard_title.is_empty() {
        assert_eq!(row.dashboard_title, expected.dashboard_title);
    }
    if !expected.panel_id.is_empty() {
        assert_eq!(row.panel_id, expected.panel_id);
    }
    if !expected.panel_title.is_empty() {
        assert_eq!(row.panel_title, expected.panel_title);
    }
    if !expected.panel_type.is_empty() {
        assert_eq!(row.panel_type, expected.panel_type);
    }
    if !expected.ref_id.is_empty() {
        assert_eq!(row.ref_id, expected.ref_id);
    }
    if !expected.datasource.is_empty() {
        assert_eq!(row.datasource, expected.datasource);
    }
    if !expected.datasource_name.is_empty() {
        assert_eq!(row.datasource_name, expected.datasource_name);
    }
    if !expected.datasource_uid.is_empty() {
        assert_eq!(row.datasource_uid, expected.datasource_uid);
    }
    if !expected.datasource_type.is_empty() {
        assert_eq!(row.datasource_type, expected.datasource_type);
    }
    if !expected.datasource_family.is_empty() {
        assert_eq!(row.datasource_family, expected.datasource_family);
    }
    if !expected.query_field.is_empty() {
        assert_eq!(row.query_field, expected.query_field);
    }
    if !expected.query_text.is_empty() {
        assert_eq!(row.query_text, expected.query_text);
    }
    if !expected.folder_path.is_empty() {
        assert_eq!(row.folder_path, expected.folder_path);
    }
    if !expected.folder_full_path.is_empty() {
        assert_eq!(row.folder_full_path, expected.folder_full_path);
    }
    if !expected.folder_level.is_empty() {
        assert_eq!(row.folder_level, expected.folder_level);
    }
    if !expected.folder_uid.is_empty() {
        assert_eq!(row.folder_uid, expected.folder_uid);
    }
    if !expected.parent_folder_uid.is_empty() {
        assert_eq!(row.parent_folder_uid, expected.parent_folder_uid);
    }
    if !expected.datasource_org.is_empty() {
        assert_eq!(row.datasource_org, expected.datasource_org);
    }
    if !expected.datasource_org_id.is_empty() {
        assert_eq!(row.datasource_org_id, expected.datasource_org_id);
    }
    if !expected.datasource_database.is_empty() {
        assert_eq!(row.datasource_database, expected.datasource_database);
    }
    if !expected.datasource_bucket.is_empty() {
        assert_eq!(row.datasource_bucket, expected.datasource_bucket);
    }
    if !expected.datasource_organization.is_empty() {
        assert_eq!(
            row.datasource_organization,
            expected.datasource_organization
        );
    }
    if !expected.datasource_index_pattern.is_empty() {
        assert_eq!(
            row.datasource_index_pattern,
            expected.datasource_index_pattern
        );
    }
    assert_eq!(row.dashboard_tags, Vec::<String>::new());
    assert_eq!(row.panel_target_count, 1);
    assert_eq!(row.panel_query_count, 1);
    assert_eq!(row.panel_datasource_count, 1);
    assert_eq!(row.panel_variables, Vec::<String>::new());
    assert_eq!(row.query_variables, Vec::<String>::new());
    assert_eq!(row.target_hidden, "false");
    assert_eq!(row.target_disabled, "false");
    assert_eq!(
        row.metrics,
        expected
            .metrics
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        row.functions,
        expected
            .functions
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        row.measurements,
        expected
            .measurements
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>()
    );
    assert_eq!(
        row.buckets,
        expected
            .buckets
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>()
    );
}

#[cfg(test)]
#[path = "export_diff_rust_tests.rs"]
mod export_diff_rust_tests;
#[cfg(test)]
#[path = "export_diff_tail_rust_tests.rs"]
mod export_diff_tail_rust_tests;
#[cfg(test)]
#[path = "export_focus_report_rust_tests.rs"]
mod export_focus_report_rust_tests;
#[cfg(test)]
#[path = "export_focus_rust_tests.rs"]
mod export_focus_rust_tests;
#[cfg(test)]
#[path = "import_edge_rust_tests.rs"]
mod import_edge_rust_tests;
#[cfg(test)]
#[path = "inspect_live_export_all_orgs_rust_tests.rs"]
mod inspect_live_export_all_orgs_rust_tests;
#[cfg(test)]
#[path = "inspect_live_export_parity_rust_tests.rs"]
mod inspect_live_export_parity_rust_tests;
#[cfg(test)]
#[path = "inspect_query_rust_tests.rs"]
mod inspect_query_rust_tests;

#[test]
fn build_export_metadata_serializes_expected_shape() {
    let value = serde_json::to_value(build_export_metadata(
        "raw",
        2,
        Some("grafana-web-import-preserve-uid"),
        Some(FOLDER_INVENTORY_FILENAME),
        Some(DATASOURCE_INVENTORY_FILENAME),
        Some(DASHBOARD_PERMISSION_BUNDLE_FILENAME),
        Some("Main Org."),
        Some("1"),
        None,
    ))
    .unwrap();

    assert_eq!(
        value,
        json!({
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "kind": "grafana-utils-dashboard-export-index",
            "variant": "raw",
            "dashboardCount": 2,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": "folders.json",
            "datasourcesFile": "datasources.json",
            "permissionsFile": "permissions.json",
            "org": "Main Org.",
            "orgId": "1"
        })
    );
}

#[test]
fn build_root_export_index_serializes_expected_shape() {
    let summary = serde_json::from_value(json!({
        "uid": "cpu-main",
        "title": "CPU Overview",
        "folderTitle": "Infra",
        "orgName": "Main Org.",
        "orgId": 1
    }))
    .unwrap();
    let mut item = test_support::build_dashboard_index_item(&summary, "cpu-main");
    item.raw_path = Some("/tmp/raw/cpu-main.json".to_string());

    let value = serde_json::to_value(build_root_export_index(
        &[item],
        Some(Path::new("/tmp/raw/index.json")),
        None,
        &[],
    ))
    .unwrap();

    assert_eq!(
        value,
        json!({
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "kind": "grafana-utils-dashboard-export-index",
            "items": [
                {
                    "uid": "cpu-main",
                    "title": "CPU Overview",
                    "folderTitle": "Infra",
                    "org": "Main Org.",
                    "orgId": "1",
                    "raw_path": "/tmp/raw/cpu-main.json"
                }
            ],
            "variants": {
                "raw": "/tmp/raw/index.json",
                "prompt": null
            },
            "folders": []
        })
    );
}

#[test]
fn collect_folder_inventory_with_request_records_parent_chain() {
    let summaries = vec![json!({
        "uid": "cpu-main",
        "title": "CPU Overview",
        "folderTitle": "Infra",
        "folderUid": "infra",
        "orgName": "Main Org.",
        "orgId": 1
    })
    .as_object()
    .unwrap()
    .clone()];

    let folders = test_support::collect_folder_inventory_with_request(
        |_method, path, _params, _payload| match path {
            "/api/folders/infra" => Ok(Some(json!({
                "uid": "infra",
                "title": "Infra",
                "parents": [
                    {"uid": "platform", "title": "Platform"},
                    {"uid": "team", "title": "Team"}
                ]
            }))),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &summaries,
    )
    .unwrap();

    assert_eq!(
        serde_json::to_value(folders).unwrap(),
        json!([
            {
                "uid": "platform",
                "title": "Platform",
                "path": "Platform",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "team",
                "title": "Team",
                "path": "Platform / Team",
                "parentUid": "platform",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "infra",
                "title": "Infra",
                "path": "Platform / Team / Infra",
                "parentUid": "team",
                "org": "Main Org.",
                "orgId": "1"
            }
        ])
    );
}

#[test]
fn build_datasource_inventory_record_keeps_datasource_config_fields() {
    let datasource = json!({
        "uid": "influx-main",
        "name": "Influx Main",
        "type": "influxdb",
        "access": "proxy",
        "url": "http://influxdb:8086",
        "jsonData": {
            "dbName": "metrics_v1",
            "defaultBucket": "prod-default",
            "organization": "acme-observability"
        }
    })
    .as_object()
    .unwrap()
    .clone();
    let org = json!({
        "id": 1,
        "name": "Main Org."
    })
    .as_object()
    .unwrap()
    .clone();

    let record = test_support::build_datasource_inventory_record(&datasource, &org);
    assert_eq!(record.database, "metrics_v1");
    assert_eq!(record.default_bucket, "prod-default");
    assert_eq!(record.organization, "acme-observability");

    let elastic = json!({
        "uid": "elastic-main",
        "name": "Elastic Main",
        "type": "elasticsearch",
        "access": "proxy",
        "url": "http://elasticsearch:9200",
        "jsonData": {
            "indexPattern": "[logs-]YYYY.MM.DD"
        }
    })
    .as_object()
    .unwrap()
    .clone();
    let elastic_record = test_support::build_datasource_inventory_record(&elastic, &org);
    assert_eq!(elastic_record.index_pattern, "[logs-]YYYY.MM.DD");
}

fn make_inspect_live_tui_fixture() -> (
    test_support::ExportInspectionSummary,
    test_support::inspect_governance::ExportInspectionGovernanceDocument,
    test_support::ExportInspectionQueryReport,
) {
    let summary = test_support::ExportInspectionSummary {
        import_dir: "/tmp/raw".to_string(),
        export_org: Some("Main Org.".to_string()),
        export_org_id: Some("1".to_string()),
        dashboard_count: 1,
        folder_count: 1,
        panel_count: 1,
        query_count: 1,
        datasource_inventory_count: 1,
        orphaned_datasource_count: 0,
        mixed_dashboard_count: 0,
        folder_paths: Vec::new(),
        datasource_usage: Vec::new(),
        datasource_inventory: Vec::new(),
        orphaned_datasources: Vec::new(),
        mixed_dashboards: Vec::new(),
    };
    let query = make_core_family_report_row(
        "cpu-main",
        "7",
        "A",
        "prom-main",
        "Prometheus Main",
        "prometheus",
        "prometheus",
        "up",
        &[],
    );
    let report = test_support::ExportInspectionQueryReport {
        import_dir: "/tmp/raw".to_string(),
        summary: test_support::QueryReportSummary {
            dashboard_count: 1,
            panel_count: 1,
            query_count: 1,
            report_row_count: 1,
        },
        queries: vec![query],
    };
    let governance = test_support::inspect_governance::ExportInspectionGovernanceDocument {
        summary: test_support::inspect_governance::GovernanceSummary {
            dashboard_count: 1,
            query_record_count: 1,
            datasource_inventory_count: 1,
            datasource_family_count: 1,
            datasource_coverage_count: 1,
            dashboard_datasource_edge_count: 1,
            datasource_risk_coverage_count: 1,
            dashboard_risk_coverage_count: 1,
            mixed_datasource_dashboard_count: 0,
            orphaned_datasource_count: 0,
            risk_record_count: 2,
            query_audit_count: 1,
            dashboard_audit_count: 0,
        },
        datasource_families: Vec::new(),
        dashboard_dependencies: Vec::new(),
        dashboard_governance: vec![test_support::inspect_governance::DashboardGovernanceRow {
            dashboard_uid: "cpu-main".to_string(),
            dashboard_title: "CPU Main".to_string(),
            folder_path: "General".to_string(),
            panel_count: 1,
            query_count: 1,
            datasource_count: 1,
            datasource_family_count: 1,
            datasources: vec!["prom-main".to_string()],
            datasource_families: vec!["prometheus".to_string()],
            mixed_datasource: false,
            risk_count: 1,
            risk_kinds: vec!["prometheus-query-cost-score".to_string()],
        }],
        dashboard_datasource_edges: Vec::new(),
        datasource_governance: Vec::new(),
        datasources: Vec::new(),
        risk_records: vec![test_support::inspect_governance::GovernanceRiskRow {
            kind: "prometheus-query-cost-score".to_string(),
            severity: "high".to_string(),
            category: "cost".to_string(),
            dashboard_uid: "cpu-main".to_string(),
            panel_id: "7".to_string(),
            datasource: "Prometheus Main".to_string(),
            detail: "cost=3".to_string(),
            recommendation: "Reduce expensive Prometheus query shapes before broad rollout."
                .to_string(),
        }],
        query_audits: vec![test_support::inspect_governance::QueryAuditRow {
            dashboard_uid: "cpu-main".to_string(),
            dashboard_title: "CPU Main".to_string(),
            folder_path: "General".to_string(),
            panel_id: "7".to_string(),
            panel_title: "CPU".to_string(),
            ref_id: "A".to_string(),
            datasource: "Prometheus Main".to_string(),
            datasource_uid: "prom-main".to_string(),
            datasource_family: "prometheus".to_string(),
            aggregation_depth: 0,
            regex_matcher_count: 0,
            estimated_series_risk: "low".to_string(),
            query_cost_score: 3,
            score: 2,
            severity: "medium".to_string(),
            reasons: vec![
                "broad-prometheus-selector".to_string(),
                "prometheus-query-cost-score".to_string(),
            ],
            recommendations: vec![
                "Add label filters to the Prometheus selector.".to_string(),
                "Trim costly aggregation and range windows.".to_string(),
            ],
        }],
        dashboard_audits: Vec::new(),
    };

    (summary, governance, report)
}

#[test]
fn build_inspect_live_tui_groups_summarizes_dashboard_query_and_risk_sections() {
    let (summary, governance, report) = make_inspect_live_tui_fixture();
    let groups = test_support::build_inspect_live_tui_groups(&summary, &governance, &report);

    assert_eq!(groups.len(), 4);
    assert_eq!(groups[0].label, "Overview");
    assert_eq!(groups[0].count, 1);
    assert_eq!(groups[1].label, "Findings");
    assert_eq!(groups[1].count, 2);
    assert_eq!(groups[2].label, "Queries");
    assert_eq!(groups[2].count, 1);
    assert_eq!(groups[3].label, "Dependencies");
    assert_eq!(groups[3].count, 0);
}

#[test]
fn inspect_live_group_order_uses_human_review_modes() {
    let (summary, governance, report) = make_inspect_live_tui_fixture();
    let groups = test_support::build_inspect_live_tui_groups(&summary, &governance, &report);

    let labels = groups
        .iter()
        .map(|group| group.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec!["Overview", "Findings", "Queries", "Dependencies"]
    );
}

#[test]
fn filter_inspect_live_tui_items_limits_items_to_selected_mode() {
    let (summary, governance, report) = make_inspect_live_tui_fixture();
    let overview_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "overview");
    let query_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "queries");
    let finding_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "findings");
    let dependency_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "dependencies");

    assert_eq!(overview_items.len(), 1);
    assert!(overview_items
        .iter()
        .all(|item| item.kind == "dashboard-summary"));
    assert_eq!(query_items.len(), 1);
    assert!(query_items.iter().all(|item| item.kind == "query"));
    assert_eq!(finding_items.len(), 2);
    assert!(finding_items.iter().any(|item| item.kind == "finding"));
    assert!(finding_items.iter().any(|item| item.kind == "query-review"));
    assert!(dependency_items.is_empty());
}

#[test]
fn build_inspect_workbench_document_adds_dependency_coverage_views() {
    let (summary, mut governance, report) = make_inspect_live_tui_fixture();
    governance.datasources = vec![test_support::inspect_governance::DatasourceCoverageRow {
        datasource_uid: "prom-main".to_string(),
        datasource: "Prometheus Main".to_string(),
        family: "prometheus".to_string(),
        query_count: 1,
        dashboard_count: 1,
        panel_count: 1,
        dashboard_uids: vec!["cpu-main".to_string()],
        query_fields: vec!["expr".to_string()],
        orphaned: false,
    }];
    governance.datasource_governance =
        vec![test_support::inspect_governance::DatasourceGovernanceRow {
            datasource_uid: "prom-main".to_string(),
            datasource: "Prometheus Main".to_string(),
            family: "prometheus".to_string(),
            query_count: 1,
            dashboard_count: 1,
            panel_count: 1,
            mixed_dashboard_count: 0,
            risk_count: 1,
            risk_kinds: vec!["prometheus-query-cost-score".to_string()],
            dashboard_uids: vec!["cpu-main".to_string()],
            orphaned: false,
        }];

    let document = test_support::build_inspect_workbench_document(
        "export artifacts",
        &summary,
        &governance,
        &report,
    );

    assert_eq!(document.groups.len(), 4);
    assert_eq!(document.groups[0].label, "Overview");
    assert_eq!(document.groups[1].label, "Findings");
    assert_eq!(document.groups[2].label, "Queries");
    assert_eq!(document.groups[3].label, "Dependencies");
    let dependency_group = document
        .groups
        .iter()
        .find(|group| group.kind == "dependencies")
        .expect("dependency group");
    assert_eq!(dependency_group.views.len(), 2);
    assert_eq!(dependency_group.views[0].label, "Usage Coverage");
    assert_eq!(dependency_group.views[1].label, "Finding Coverage");
    assert_eq!(dependency_group.views[0].items.len(), 1);
    assert_eq!(dependency_group.views[1].items.len(), 1);
    assert!(document.summary_lines[0].contains("Source=export artifacts"));
    assert!(document.summary_lines[2].contains("Overview"));
}

#[test]
fn overview_mode_items_use_human_dashboard_summary_kind() {
    let (summary, governance, report) = make_inspect_live_tui_fixture();
    let overview_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "overview");

    assert_eq!(overview_items[0].kind, "dashboard-summary");
}

#[test]
fn finding_mode_items_use_human_finding_kinds() {
    let (summary, governance, report) = make_inspect_live_tui_fixture();
    let finding_items =
        test_support::filter_inspect_live_tui_items(&summary, &governance, &report, "findings");

    assert!(finding_items
        .iter()
        .all(|item| { item.kind == "finding" || item.kind == "query-review" }));
}

#[test]
fn build_topology_tui_groups_summarize_node_kinds() {
    let document = sample_topology_tui_document();
    let groups = build_topology_tui_groups(&document);

    let counts = groups
        .iter()
        .map(|group| (group.label.as_str(), group.count))
        .collect::<Vec<_>>();
    assert_eq!(
        counts,
        vec![
            ("All", 5),
            ("Datasources", 1),
            ("Dashboards", 1),
            ("Panels", 1),
            ("Variables", 1),
            ("Alert Rules", 1),
            ("Contact Points", 0),
            ("Mute Timings", 0),
            ("Policies", 0),
            ("Templates", 0),
            ("Alert Resources", 0),
        ]
    );
}

#[test]
fn filter_topology_tui_items_limits_items_to_selected_group() {
    let document = sample_topology_tui_document();

    let variables = filter_topology_tui_items(&document, "variable");
    assert_eq!(variables.len(), 1);
    assert_eq!(variables[0].kind, "variable");
    assert_eq!(variables[0].title, "cluster");

    let panels = filter_topology_tui_items(&document, "panel");
    assert_eq!(panels.len(), 1);
    assert_eq!(panels[0].kind, "panel");
    assert_eq!(panels[0].title, "Panel 7");

    let all = filter_topology_tui_items(&document, "all");
    assert_eq!(all.len(), document.nodes.len());
}

#[test]
fn validate_dashboard_export_dir_detects_custom_plugin_legacy_layout_and_schema_migration() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join("legacy.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "legacy-main",
                "title": "Legacy Main",
                "schemaVersion": 30,
                "rows": [],
                "panels": [
                    {"id": 7, "type": "acme-panel", "datasource": {"type": "acme-ds"}}
                ]
            },
            "__inputs": [{"name": "DS_PROM"}]
        }))
        .unwrap(),
    )
    .unwrap();

    let result =
        test_support::validate_dashboard_export_dir(&raw_dir, true, true, Some(39)).unwrap();
    let output = test_support::render_validation_result_json(&result).unwrap();

    assert_eq!(result.dashboard_count, 1);
    assert!(result.error_count >= 4);
    assert!(output.contains("custom-panel-plugin"));
    assert!(output.contains("custom-datasource-plugin"));
    assert!(output.contains("legacy-row-layout"));
    assert!(output.contains("schema-migration-required"));
}

#[test]
fn snapshot_live_dashboard_export_with_fetcher_writes_dashboards_in_parallel() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    let summaries = vec![
        json!({"uid": "cpu-main", "title": "CPU Main", "folderTitle": "Infra"})
            .as_object()
            .unwrap()
            .clone(),
        json!({"uid": "logs-main", "title": "Logs Main", "folderTitle": "Ops"})
            .as_object()
            .unwrap()
            .clone(),
    ];

    let count = test_support::snapshot_live_dashboard_export_with_fetcher(
        &raw_dir,
        &summaries,
        4,
        false,
        |uid| {
            Ok(json!({
                "dashboard": {
                    "uid": uid,
                    "title": uid,
                    "schemaVersion": 39,
                    "panels": []
                },
                "meta": {}
            }))
        },
    )
    .unwrap();

    assert_eq!(count, 2);
    assert!(raw_dir.join("Infra/CPU_Main__cpu-main.json").is_file());
    assert!(raw_dir.join("Ops/Logs_Main__logs-main.json").is_file());
}

#[test]
fn import_dashboards_with_strict_schema_rejects_custom_plugins_before_live_write() {
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
        raw_dir.join("custom.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {
                "uid": "custom-main",
                "title": "Custom Main",
                "schemaVersion": 39,
                "panels": [
                    {"id": 7, "type": "acme-panel", "datasource": {"type": "prometheus"}}
                ]
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let mut args = make_import_args(raw_dir);
    args.strict_schema = true;
    args.dry_run = true;
    let error = test_support::import_dashboards_with_request(
        |_method, _path, _params, _payload| Ok(None),
        &args,
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("custom-panel-plugin"));
    assert!(error.contains("unsupported custom panel plugin type"));
}

#[test]
fn build_output_path_keeps_folder_structure() {
    let summary = json!({
        "folderTitle": "Infra Team",
        "title": "Cluster Health",
        "uid": "abc",
    });
    let path = build_output_path(Path::new("out"), summary.as_object().unwrap(), false);
    assert_eq!(path, Path::new("out/Infra_Team/Cluster_Health__abc.json"));
}

#[test]
fn build_folder_inventory_status_reports_missing_folder() {
    let folder = test_support::FolderInventoryItem {
        uid: "child".to_string(),
        title: "Child".to_string(),
        path: "Platform / Child".to_string(),
        parent_uid: Some("platform".to_string()),
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
    };

    let status = build_folder_inventory_status(&folder, None);

    assert_eq!(status.kind, FolderInventoryStatusKind::Missing);
    assert_eq!(
        format_folder_inventory_status_line(&status),
        "Folder inventory missing uid=child title=Child parentUid=platform path=Platform / Child"
    );
}

#[test]
fn build_folder_inventory_status_reports_matching_folder() {
    let folder = test_support::FolderInventoryItem {
        uid: "child".to_string(),
        title: "Child".to_string(),
        path: "Platform / Child".to_string(),
        parent_uid: Some("platform".to_string()),
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
    };
    let destination_folder = json!({
        "uid": "child",
        "title": "Child",
        "parents": [{"uid": "platform", "title": "Platform"}]
    })
    .as_object()
    .unwrap()
    .clone();

    let status = build_folder_inventory_status(&folder, Some(&destination_folder));

    assert_eq!(status.kind, FolderInventoryStatusKind::Matches);
    assert_eq!(
        format_folder_inventory_status_line(&status),
        "Folder inventory matches uid=child title=Child parentUid=platform path=Platform / Child"
    );
}

#[test]
fn build_folder_inventory_status_reports_mismatch_details() {
    let folder = test_support::FolderInventoryItem {
        uid: "child".to_string(),
        title: "Child".to_string(),
        path: "Platform / Child".to_string(),
        parent_uid: Some("platform".to_string()),
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
    };
    let destination_folder = json!({
        "uid": "child",
        "title": "Ops Child",
        "parents": [{"uid": "ops", "title": "Ops"}]
    })
    .as_object()
    .unwrap()
    .clone();

    let status = build_folder_inventory_status(&folder, Some(&destination_folder));

    assert_eq!(status.kind, FolderInventoryStatusKind::Mismatch);
    assert_eq!(
        format_folder_inventory_status_line(&status),
        "Folder inventory mismatch uid=child expected(title=Child, parentUid=platform, path=Platform / Child) actual(title=Ops Child, parentUid=ops, path=Ops / Ops Child)"
    );
}

#[test]
fn render_folder_inventory_dry_run_table_supports_expected_columns() {
    let rows = vec![[
        "child".to_string(),
        "exists".to_string(),
        "mismatch".to_string(),
        "path".to_string(),
        "Platform / Child".to_string(),
        "Legacy / Child".to_string(),
    ]];

    let with_header = test_support::render_folder_inventory_dry_run_table(&rows, true);

    assert!(with_header[0].contains("EXPECTED_PATH"));
    assert!(with_header[0].contains("ACTUAL_PATH"));
    assert!(with_header[2].contains("Legacy / Child"));
}

#[test]
fn export_progress_line_uses_concise_counter_format() {
    assert_eq!(
        format_export_progress_line(2, 5, "cpu-main", false),
        "Exporting dashboard 2/5: cpu-main"
    );
    assert_eq!(
        format_export_progress_line(2, 5, "cpu-main", true),
        "Would export dashboard 2/5: cpu-main"
    );
}

#[test]
fn export_verbose_line_includes_variant_and_path() {
    assert_eq!(
        format_export_verbose_line("prompt", "cpu-main", Path::new("/tmp/out.json"), false),
        "Exported prompt cpu-main -> /tmp/out.json"
    );
    assert_eq!(
        format_export_verbose_line("raw", "cpu-main", Path::new("/tmp/out.json"), true),
        "Would export raw    cpu-main -> /tmp/out.json"
    );
}

#[test]
fn import_progress_line_uses_concise_counter_format() {
    assert_eq!(
        format_import_progress_line(3, 7, "/tmp/raw/cpu.json", false, None, None),
        "Importing dashboard 3/7: /tmp/raw/cpu.json"
    );
    assert_eq!(
        format_import_progress_line(
            3,
            7,
            "cpu-main",
            true,
            Some("would-update"),
            Some("General")
        ),
        "Dry-run dashboard 3/7: cpu-main dest=exists action=update folderPath=General"
    );
    assert_eq!(
        format_import_progress_line(3, 7, "cpu-main", true, Some("would-skip-missing"), Some("Platform / Infra")),
        "Dry-run dashboard 3/7: cpu-main dest=missing action=skip-missing folderPath=Platform / Infra"
    );
}

#[test]
fn render_import_dry_run_table_supports_optional_header() {
    let rows = vec![
        [
            "abc".to_string(),
            "exists".to_string(),
            "update".to_string(),
            "General".to_string(),
            "General".to_string(),
            "General".to_string(),
            "".to_string(),
            "/tmp/a.json".to_string(),
        ],
        [
            "xyz".to_string(),
            "missing".to_string(),
            "create".to_string(),
            "Platform / Infra".to_string(),
            "Platform / Infra".to_string(),
            "".to_string(),
            "".to_string(),
            "/tmp/b.json".to_string(),
        ],
    ];
    let with_header = test_support::render_import_dry_run_table(&rows, true, None);
    assert!(with_header[0].contains("UID"));
    assert!(with_header[0].contains("DESTINATION"));
    assert!(with_header[0].contains("ACTION"));
    assert!(with_header[0].contains("FOLDER_PATH"));
    assert!(with_header[0].contains("FILE"));
    assert!(with_header[2].contains("abc"));
    assert!(with_header[2].contains("exists"));
    assert!(with_header[2].contains("update"));
    assert!(with_header[2].contains("General"));
    assert!(with_header[2].contains("/tmp/a.json"));
    let without_header = test_support::render_import_dry_run_table(&rows, false, None);
    assert_eq!(without_header.len(), 2);
    assert!(without_header[0].contains("abc"));
    assert!(without_header[0].contains("exists"));
    assert!(without_header[0].contains("update"));
    assert!(without_header[0].contains("General"));
    assert!(without_header[0].contains("/tmp/a.json"));
}

#[test]
fn render_import_dry_run_table_honors_selected_columns() {
    let rows = vec![[
        "abc".to_string(),
        "exists".to_string(),
        "skip-folder-mismatch".to_string(),
        "Platform / Ops".to_string(),
        "Platform / Source".to_string(),
        "Platform / Dest".to_string(),
        "path".to_string(),
        "/tmp/a.json".to_string(),
    ]];

    let lines = test_support::render_import_dry_run_table(
        &rows,
        true,
        Some(&["uid".to_string(), "reason".to_string(), "file".to_string()]),
    );

    assert!(lines[0].contains("UID"));
    assert!(lines[0].contains("REASON"));
    assert!(lines[0].contains("FILE"));
    assert!(!lines[0].contains("DESTINATION"));
    assert!(lines[2].contains("abc"));
    assert!(lines[2].contains("path"));
    assert!(lines[2].contains("/tmp/a.json"));
}

#[test]
fn render_import_dry_run_json_returns_structured_document() {
    let folder_status = test_support::FolderInventoryStatus {
        uid: "infra".to_string(),
        expected_title: "Infra".to_string(),
        expected_parent_uid: Some("platform".to_string()),
        expected_path: "Platform / Infra".to_string(),
        actual_title: Some("Infra".to_string()),
        actual_parent_uid: Some("platform".to_string()),
        actual_path: Some("Platform / Infra".to_string()),
        kind: FolderInventoryStatusKind::Matches,
    };
    let rows = vec![[
        "abc".to_string(),
        "exists".to_string(),
        "update".to_string(),
        "Platform / Infra".to_string(),
        "Platform / Infra".to_string(),
        "Platform / Infra".to_string(),
        "".to_string(),
        "/tmp/a.json".to_string(),
    ]];

    let value: Value = serde_json::from_str(
        &test_support::render_import_dry_run_json(
            "create-or-update",
            &[folder_status],
            &rows,
            Path::new("/tmp/raw"),
            0,
            0,
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(value["mode"], "create-or-update");
    assert_eq!(value["folders"][0]["uid"], "infra");
    assert_eq!(value["dashboards"][0]["folderPath"], "Platform / Infra");
    assert_eq!(
        value["dashboards"][0]["sourceFolderPath"],
        "Platform / Infra"
    );
    assert_eq!(
        value["dashboards"][0]["destinationFolderPath"],
        "Platform / Infra"
    );
    assert_eq!(value["summary"]["dashboardCount"], 1);
}

#[test]
fn render_routed_import_org_table_includes_org_level_columns() {
    let rows = vec![
        [
            "2".to_string(),
            "Org Two".to_string(),
            "exists".to_string(),
            "2".to_string(),
            "3".to_string(),
        ],
        [
            "9".to_string(),
            "Ops Org".to_string(),
            "would-create".to_string(),
            "<new>".to_string(),
            "1".to_string(),
        ],
    ];

    let lines = test_support::import::render_routed_import_org_table(&rows, true);

    assert!(lines[0].contains("SOURCE_ORG_ID"));
    assert!(lines[0].contains("ORG_ACTION"));
    assert!(lines[2].contains("Org Two"));
    assert!(lines[3].contains("would-create"));
}

#[test]
fn routed_import_scope_identity_matches_table_json_and_progress_surfaces() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_two_raw = export_root.join("org_2_Org_Two").join("raw");
    let org_nine_raw = export_root.join("org_9_Ops_Org").join("raw");
    write_combined_export_root_metadata(
        &export_root,
        &[
            ("2", "Org Two", "org_2_Org_Two"),
            ("9", "Ops Org", "org_9_Ops_Org"),
        ],
    );
    write_basic_raw_export(
        &org_two_raw,
        "2",
        "Org Two",
        "cpu-two",
        "CPU Two",
        "prom-two",
        "prometheus",
        "timeseries",
        "general",
        "General",
        "expr",
        "up",
    );
    write_basic_raw_export(
        &org_nine_raw,
        "9",
        "Ops Org",
        "ops-main",
        "Ops Main",
        "loki-nine",
        "loki",
        "logs",
        "ops",
        "Ops",
        "expr",
        "{job=\"grafana\"}",
    );

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.create_missing_orgs = true;
    args.dry_run = true;
    args.json = true;

    let payload: Value = serde_json::from_str(
        &test_support::import::build_routed_import_dry_run_json_with_request(
            |method, path, _params, _payload| match (method, path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 2, "name": "Org Two"}
                ]))),
                _ => Err(test_support::message(format!("unexpected request {path}"))),
            },
            |_target_org_id, scoped_args| {
                Ok(test_support::import::ImportDryRunReport {
                    mode: "create-only".to_string(),
                    import_dir: scoped_args.import_dir.clone(),
                    folder_statuses: Vec::new(),
                    dashboard_records: Vec::new(),
                    skipped_missing_count: 0,
                    skipped_folder_mismatch_count: 0,
                })
            },
            &args,
        )
        .unwrap(),
    )
    .unwrap();

    let org_entries = payload["orgs"].as_array().unwrap();
    let rows: Vec<[String; 5]> = org_entries
        .iter()
        .map(|entry| {
            [
                entry["sourceOrgId"].as_i64().unwrap().to_string(),
                entry["sourceOrgName"].as_str().unwrap().to_string(),
                entry["orgAction"].as_str().unwrap().to_string(),
                test_support::import::format_routed_import_target_org_label(
                    entry["targetOrgId"].as_i64(),
                ),
                entry["dashboardCount"].as_u64().unwrap().to_string(),
            ]
        })
        .collect();
    let table_lines = test_support::import::render_routed_import_org_table(&rows, true);

    let org_two = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap();
    let org_nine = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();

    let existing_summary = test_support::import::format_routed_import_scope_summary_fields(
        2,
        "Org Two",
        "exists",
        Some(2),
        Path::new(org_two["importDir"].as_str().unwrap()),
    );
    let would_create_summary = test_support::import::format_routed_import_scope_summary_fields(
        9,
        "Ops Org",
        "would-create",
        None,
        Path::new(org_nine["importDir"].as_str().unwrap()),
    );

    assert_eq!(org_two["targetOrgId"], json!(2));
    assert_eq!(org_nine["targetOrgId"], Value::Null);
    assert!(table_lines[2].contains("Org Two"));
    assert!(table_lines[2].contains("2"));
    assert!(table_lines[3].contains("Ops Org"));
    assert!(table_lines[3].contains("<new>"));
    assert!(existing_summary.contains("export orgId=2"));
    assert!(existing_summary.contains("name=Org Two"));
    assert!(existing_summary.contains("orgAction=exists"));
    assert!(existing_summary.contains("targetOrgId=2"));
    assert!(existing_summary.contains(org_two["importDir"].as_str().unwrap()));
    assert!(would_create_summary.contains("export orgId=9"));
    assert!(would_create_summary.contains("name=Ops Org"));
    assert!(would_create_summary.contains("orgAction=would-create"));
    assert!(would_create_summary.contains("targetOrgId=<new>"));
    assert!(would_create_summary.contains(org_nine["importDir"].as_str().unwrap()));
}

#[test]
fn routed_import_selected_scope_statuses_match_json_table_and_summary_contract() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    let org_two_raw = export_root.join("org_2_Org_Two").join("raw");
    let org_five_raw = export_root.join("org_5_Org_Five").join("raw");
    let org_nine_raw = export_root.join("org_9_Ops_Org").join("raw");
    write_combined_export_root_metadata(
        &export_root,
        &[
            ("2", "Org Two", "org_2_Org_Two"),
            ("5", "Org Five", "org_5_Org_Five"),
            ("9", "Ops Org", "org_9_Ops_Org"),
        ],
    );
    write_basic_raw_export(
        &org_two_raw,
        "2",
        "Org Two",
        "cpu-two",
        "CPU Two",
        "prom-two",
        "prometheus",
        "timeseries",
        "general",
        "General",
        "expr",
        "up",
    );
    write_basic_raw_export(
        &org_five_raw,
        "5",
        "Org Five",
        "cpu-five",
        "CPU Five",
        "prom-five",
        "prometheus",
        "timeseries",
        "general",
        "General",
        "expr",
        "up",
    );
    write_basic_raw_export(
        &org_nine_raw,
        "9",
        "Ops Org",
        "ops-main",
        "Ops Main",
        "loki-nine",
        "loki",
        "logs",
        "ops",
        "Ops",
        "expr",
        "{job=\"grafana\"}",
    );

    let mut args = make_import_args(export_root);
    args.common = make_basic_common_args("http://127.0.0.1:3000".to_string());
    args.use_export_org = true;
    args.only_org_id = vec![2, 9];
    args.create_missing_orgs = false;
    args.dry_run = true;
    args.json = true;

    let payload: Value = serde_json::from_str(
        &test_support::import::build_routed_import_dry_run_json_with_request(
            |method, path, _params, _payload| match (method, path) {
                (reqwest::Method::GET, "/api/orgs") => Ok(Some(json!([
                    {"id": 2, "name": "Org Two"}
                ]))),
                _ => Err(test_support::message(format!("unexpected request {path}"))),
            },
            |_target_org_id, scoped_args| {
                Ok(test_support::import::ImportDryRunReport {
                    mode: "create-only".to_string(),
                    import_dir: scoped_args.import_dir.clone(),
                    folder_statuses: Vec::new(),
                    dashboard_records: Vec::new(),
                    skipped_missing_count: 0,
                    skipped_folder_mismatch_count: 0,
                })
            },
            &args,
        )
        .unwrap(),
    )
    .unwrap();

    let org_entries = payload["orgs"].as_array().unwrap();
    let import_entries = payload["imports"].as_array().unwrap();
    assert_eq!(org_entries.len(), 2);
    assert_eq!(import_entries.len(), 2);
    assert_eq!(payload["summary"]["orgCount"], json!(2));
    assert_eq!(payload["summary"]["existingOrgCount"], json!(1));
    assert_eq!(payload["summary"]["missingOrgCount"], json!(1));
    assert_eq!(payload["summary"]["wouldCreateOrgCount"], json!(0));

    let org_two = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(2))
        .unwrap();
    let org_nine = org_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();
    assert!(org_entries
        .iter()
        .all(|entry| entry["sourceOrgId"] != json!(5)));

    assert_eq!(org_two["orgAction"], json!("exists"));
    assert_eq!(org_two["targetOrgId"], json!(2));
    assert_eq!(org_nine["orgAction"], json!("missing"));
    assert_eq!(org_nine["targetOrgId"], Value::Null);

    let org_nine_import = import_entries
        .iter()
        .find(|entry| entry["sourceOrgId"] == json!(9))
        .unwrap();
    assert_eq!(org_nine_import["orgAction"], json!("missing"));
    assert_eq!(org_nine_import["dashboards"], json!([]));
    assert_eq!(org_nine_import["summary"]["dashboardCount"], json!(1));

    let rows: Vec<[String; 5]> = org_entries
        .iter()
        .map(|entry| {
            [
                entry["sourceOrgId"].as_i64().unwrap().to_string(),
                entry["sourceOrgName"].as_str().unwrap().to_string(),
                entry["orgAction"].as_str().unwrap().to_string(),
                test_support::import::format_routed_import_target_org_label(
                    entry["targetOrgId"].as_i64(),
                ),
                entry["dashboardCount"].as_u64().unwrap().to_string(),
            ]
        })
        .collect();
    let table_lines = test_support::import::render_routed_import_org_table(&rows, true);
    assert!(table_lines[2].contains("Org Two"));
    assert!(table_lines[2].contains("exists"));
    assert!(table_lines[2].contains("2"));
    assert!(table_lines[3].contains("Ops Org"));
    assert!(table_lines[3].contains("missing"));
    assert!(table_lines[3].contains("<new>"));

    let missing_summary = test_support::import::format_routed_import_scope_summary_fields(
        9,
        "Ops Org",
        "missing",
        None,
        Path::new(org_nine["importDir"].as_str().unwrap()),
    );
    assert!(missing_summary.contains("export orgId=9"));
    assert!(missing_summary.contains("name=Ops Org"));
    assert!(missing_summary.contains("orgAction=missing"));
    assert!(missing_summary.contains("targetOrgId=<new>"));
    assert!(missing_summary.contains(org_nine["importDir"].as_str().unwrap()));
}

#[test]
fn describe_dashboard_import_mode_uses_expected_labels() {
    assert_eq!(
        test_support::describe_dashboard_import_mode(false, false),
        "create-only"
    );
    assert_eq!(
        test_support::describe_dashboard_import_mode(true, false),
        "create-or-update"
    );
    assert_eq!(
        test_support::describe_dashboard_import_mode(false, true),
        "update-or-skip-missing"
    );
}

#[test]
fn import_verbose_line_includes_dry_run_action() {
    assert_eq!(
        format_import_verbose_line(Path::new("/tmp/raw/cpu.json"), false, None, None, None),
        "Imported /tmp/raw/cpu.json"
    );
    assert_eq!(
        format_import_verbose_line(
            Path::new("/tmp/raw/cpu.json"),
            true,
            Some("cpu-main"),
            Some("would-update"),
            Some("General")
        ),
        "Dry-run import uid=cpu-main dest=exists action=update folderPath=General file=/tmp/raw/cpu.json"
    );
    assert_eq!(
        format_import_verbose_line(
            Path::new("/tmp/raw/cpu.json"),
            true,
            Some("cpu-main"),
            Some("would-skip-missing"),
            Some("Platform / Infra")
        ),
        "Dry-run import uid=cpu-main dest=missing action=skip-missing folderPath=Platform / Infra file=/tmp/raw/cpu.json"
    );
}

#[test]
fn build_export_variant_dirs_returns_raw_and_prompt_dirs() {
    let (raw_dir, prompt_dir) = build_export_variant_dirs(Path::new("dashboards"));
    assert_eq!(raw_dir, Path::new("dashboards/raw"));
    assert_eq!(prompt_dir, Path::new("dashboards/prompt"));
}

#[test]
fn discover_dashboard_files_rejects_combined_export_root() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("raw")).unwrap();
    fs::create_dir_all(temp.path().join("prompt")).unwrap();
    let error = discover_dashboard_files(temp.path()).unwrap_err();
    assert!(error.to_string().contains("combined export root"));
}

#[test]
fn discover_dashboard_files_ignores_export_metadata() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("raw/subdir")).unwrap();
    fs::write(
        temp.path().join("raw/subdir/dashboard.json"),
        serde_json::to_string_pretty(&json!({"uid": "abc", "title": "CPU"})).unwrap(),
    )
    .unwrap();
    fs::write(
        temp.path().join("raw").join(EXPORT_METADATA_FILENAME),
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

    let files = discover_dashboard_files(&temp.path().join("raw")).unwrap();
    assert_eq!(files, vec![temp.path().join("raw/subdir/dashboard.json")]);
}

#[test]
fn discover_dashboard_files_ignores_folder_inventory() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("raw/subdir")).unwrap();
    fs::write(
        temp.path().join("raw/subdir/dashboard.json"),
        serde_json::to_string_pretty(&json!({"uid": "abc", "title": "CPU"})).unwrap(),
    )
    .unwrap();
    fs::write(
        temp.path().join("raw").join(FOLDER_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&json!([
            {"uid": "infra", "title": "Infra", "path": "Infra", "org": "Main Org.", "orgId": "1"}
        ]))
        .unwrap(),
    )
    .unwrap();

    let files = discover_dashboard_files(&temp.path().join("raw")).unwrap();
    assert_eq!(files, vec![temp.path().join("raw/subdir/dashboard.json")]);
}

#[test]
fn discover_dashboard_files_ignores_permission_bundle() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join("raw/subdir")).unwrap();
    fs::write(
        temp.path().join("raw/subdir/dashboard.json"),
        serde_json::to_string_pretty(&json!({"uid": "abc", "title": "CPU"})).unwrap(),
    )
    .unwrap();
    fs::write(
        temp.path().join("raw").join(DASHBOARD_PERMISSION_BUNDLE_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-permission-bundle",
            "schemaVersion": 1,
            "summary": {"resourceCount": 0, "dashboardCount": 0, "folderCount": 0, "permissionCount": 0},
            "resources": []
        }))
        .unwrap(),
    )
    .unwrap();

    let files = discover_dashboard_files(&temp.path().join("raw")).unwrap();
    assert_eq!(files, vec![temp.path().join("raw/subdir/dashboard.json")]);
}

#[test]
fn build_import_payload_accepts_wrapped_document() {
    let payload = build_import_payload(
        &json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
            "meta": {"folderUid": "old-folder"}
        }),
        Some("new-folder"),
        true,
        "sync dashboards",
    )
    .unwrap();

    assert_eq!(payload["dashboard"]["id"], Value::Null);
    assert_eq!(payload["folderUid"], "new-folder");
    assert_eq!(payload["overwrite"], true);
    assert_eq!(payload["message"], "sync dashboards");
}

#[test]
fn build_preserved_web_import_document_clears_numeric_id() {
    let document = build_preserved_web_import_document(&json!({
        "dashboard": {"id": 7, "uid": "abc", "title": "CPU"}
    }))
    .unwrap();

    assert_eq!(document["id"], Value::Null);
    assert_eq!(document["uid"], "abc");
}

#[test]
fn format_dashboard_summary_line_uses_uid_name_and_folder_details() {
    let summary = json!({
        "uid": "abc",
        "folderUid": "infra",
        "folderPath": "Platform / Infra",
        "folderTitle": "Infra",
        "orgId": 1,
        "orgName": "Main Org",
        "title": "CPU"
    });

    let line = format_dashboard_summary_line(summary.as_object().unwrap());
    assert_eq!(
        line,
        "uid=abc name=CPU folder=Infra folderUid=infra path=Platform / Infra org=Main Org orgId=1"
    );
}

#[test]
fn format_dashboard_summary_line_appends_sources_when_present() {
    let summary = json!({
        "uid": "abc",
        "folderUid": "infra",
        "folderPath": "Platform / Infra",
        "folderTitle": "Infra",
        "orgId": 1,
        "orgName": "Main Org",
        "title": "CPU",
        "sources": ["Loki Logs", "Prom Main"]
    });

    let line = format_dashboard_summary_line(summary.as_object().unwrap());
    assert_eq!(
        line,
        "uid=abc name=CPU folder=Infra folderUid=infra path=Platform / Infra org=Main Org orgId=1 sources=Loki Logs,Prom Main"
    );
}

#[test]
fn render_dashboard_summary_table_uses_headers_and_defaults() {
    let summaries = vec![
        json!({
            "uid": "abc",
            "folderUid": "infra",
            "folderPath": "Platform / Infra",
            "folderTitle": "Infra",
            "orgId": 1,
            "orgName": "Main Org",
            "title": "CPU"
        })
        .as_object()
        .unwrap()
        .clone(),
        json!({
            "orgId": 1,
            "orgName": "Main Org",
            "uid": "xyz",
            "title": "Overview"
        })
        .as_object()
        .unwrap()
        .clone(),
    ];

    let lines = render_dashboard_summary_table(&summaries, &[], true);
    assert!(lines[0].contains("ORG"));
    assert!(lines[0].contains("ORG_ID"));
    assert!(lines[2].contains("Main Org"));
    assert!(lines[2].contains("  1"));
    assert!(lines[3].contains("Main Org"));
}

#[test]
fn render_dashboard_summary_table_includes_sources_column_when_present() {
    let summaries = vec![json!({
        "uid": "abc",
        "folderUid": "infra",
        "folderPath": "Platform / Infra",
        "folderTitle": "Infra",
        "orgId": 1,
        "orgName": "Main Org",
        "title": "CPU",
        "sources": ["Prom Main", "Loki Logs"]
    })
    .as_object()
    .unwrap()
    .clone()];

    let lines = render_dashboard_summary_table(&summaries, &[], true);
    assert!(lines[0].contains("ORG"));
    assert!(lines[0].contains("SOURCES"));
    assert!(lines[2].starts_with("abc  CPU   Infra   infra"));
    assert!(lines[2].contains("Main Org"));
    assert!(lines[2].ends_with("Prom Main,Loki Logs"));
}

#[test]
fn render_dashboard_summary_table_can_omit_header() {
    let summaries = vec![json!({
        "uid": "abc",
        "folderUid": "infra",
        "folderPath": "Platform / Infra",
        "folderTitle": "Infra",
        "orgId": 1,
        "orgName": "Main Org",
        "title": "CPU"
    })
    .as_object()
    .unwrap()
    .clone()];

    let lines = render_dashboard_summary_table(&summaries, &[], false);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("abc"));
}

#[test]
fn render_dashboard_summary_csv_uses_headers_and_escaping() {
    let summaries = vec![
        json!({
            "uid": "abc",
            "folderUid": "infra",
            "folderPath": "Platform / Infra",
            "folderTitle": "Infra",
            "orgId": 1,
            "orgName": "Main Org",
            "title": "CPU"
        })
        .as_object()
        .unwrap()
        .clone(),
        json!({
            "uid": "xyz",
            "folderUid": "ops",
            "folderPath": "Root / Ops",
            "folderTitle": "Ops",
            "orgId": 1,
            "orgName": "Main Org",
            "title": "CPU, \"critical\""
        })
        .as_object()
        .unwrap()
        .clone(),
    ];

    let lines = render_dashboard_summary_csv(&summaries, &[]);
    assert_eq!(lines[0], "uid,name,folder,folderUid,path,org,orgId");
    assert_eq!(lines[1], "abc,CPU,Infra,infra,Platform / Infra,Main Org,1");
    assert_eq!(
        lines[2],
        "xyz,\"CPU, \"\"critical\"\"\",Ops,ops,Root / Ops,Main Org,1"
    );
}

#[test]
fn render_dashboard_summary_csv_includes_sources_column_when_present() {
    let summaries = vec![json!({
        "uid": "abc",
        "folderUid": "infra",
        "folderPath": "Platform / Infra",
        "folderTitle": "Infra",
        "orgId": 1,
        "orgName": "Main Org",
        "title": "CPU",
        "sources": ["Prom Main", "Loki Logs"],
        "sourceUids": ["loki_uid", "prom_uid"]
    })
    .as_object()
    .unwrap()
    .clone()];

    let lines = render_dashboard_summary_csv(&summaries, &[]);
    assert_eq!(
        lines[0],
        "uid,name,folder,folderUid,path,org,orgId,sources,sourceUids"
    );
    assert_eq!(
        lines[1],
        "abc,CPU,Infra,infra,Platform / Infra,Main Org,1,\"Prom Main,Loki Logs\",\"loki_uid,prom_uid\""
    );
}

#[test]
fn dashboard_delete_validate_args_requires_yes_without_dry_run() {
    let args = DeleteArgs {
        common: CommonCliArgs {
            url: "https://grafana.example.com".to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        },
        page_size: 500,
        org_id: None,
        uid: Some("cpu-main".to_string()),
        path: None,
        delete_folders: false,
        yes: false,
        interactive: false,
        dry_run: false,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
    };

    let error = validate_delete_args(&args).unwrap_err();
    assert!(error.to_string().contains("requires --yes"));
}

#[test]
fn dashboard_delete_build_plan_matches_path_subtree() {
    let args = DeleteArgs {
        common: CommonCliArgs {
            url: "https://grafana.example.com".to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        },
        page_size: 500,
        org_id: None,
        uid: None,
        path: Some("Platform / Infra".to_string()),
        delete_folders: true,
        yes: true,
        interactive: false,
        dry_run: false,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
    };

    let plan = build_delete_plan_with_request(
        |method, path, params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/search") => {
                let page = params
                    .iter()
                    .find(|(key, _)| key == "page")
                    .map(|(_, value)| value.as_str())
                    .unwrap_or("1");
                if page == "1" {
                    Ok(Some(json!([
                        {"uid":"cpu-main","title":"CPU","folderUid":"infra","folderTitle":"Infra"},
                        {"uid":"mem-main","title":"Memory","folderUid":"child","folderTitle":"Child"},
                        {"uid":"ops-main","title":"Ops","folderUid":"ops","folderTitle":"Ops"}
                    ])))
                } else {
                    Ok(Some(json!([])))
                }
            }
            (Method::GET, "/api/folders/infra") => Ok(Some(json!({
                "uid":"infra",
                "title":"Infra",
                "parents":[{"uid":"platform","title":"Platform"}]
            }))),
            (Method::GET, "/api/folders/child") => Ok(Some(json!({
                "uid":"child",
                "title":"Child",
                "parents":[{"uid":"platform","title":"Platform"},{"uid":"infra","title":"Infra"}]
            }))),
            (Method::GET, "/api/folders/ops") => Ok(Some(json!({
                "uid":"ops",
                "title":"Ops"
            }))),
            (Method::GET, "/api/folders/platform") => Ok(Some(json!({
                "uid":"platform",
                "title":"Platform"
            }))),
            _ => Err(message(format!("unexpected request {method} {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(plan.dashboards.len(), 2);
    assert_eq!(plan.folders.len(), 2);
    assert_eq!(plan.dashboards[0].uid, "cpu-main");
    assert_eq!(plan.dashboards[1].uid, "mem-main");
    assert_eq!(plan.folders[0].uid, "child");
    assert_eq!(plan.folders[1].uid, "infra");
}

#[test]
fn dashboard_delete_with_request_deletes_dashboards_then_folders() {
    let args = DeleteArgs {
        common: CommonCliArgs {
            url: "https://grafana.example.com".to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        },
        page_size: 500,
        org_id: None,
        uid: None,
        path: Some("Platform / Infra".to_string()),
        delete_folders: true,
        yes: true,
        interactive: false,
        dry_run: false,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
    };
    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = calls.clone();

    let count = delete_dashboards_with_request(
        move |method, path, params, _payload| {
            recorded
                .lock()
                .unwrap()
                .push((method.clone(), path.to_string(), params.to_vec()));
            match (method.clone(), path) {
                (Method::GET, "/api/search") => {
                    let page = params
                        .iter()
                        .find(|(key, _)| key == "page")
                        .map(|(_, value)| value.as_str())
                        .unwrap_or("1");
                    if page == "1" {
                        Ok(Some(json!([
                            {"uid":"cpu-main","title":"CPU","folderUid":"infra","folderTitle":"Infra"},
                            {"uid":"mem-main","title":"Memory","folderUid":"child","folderTitle":"Child"}
                        ])))
                    } else {
                        Ok(Some(json!([])))
                    }
                }
                (Method::GET, "/api/folders/infra") => Ok(Some(json!({
                    "uid":"infra",
                    "title":"Infra",
                    "parents":[{"uid":"platform","title":"Platform"}]
                }))),
                (Method::GET, "/api/folders/child") => Ok(Some(json!({
                    "uid":"child",
                    "title":"Child",
                    "parents":[{"uid":"platform","title":"Platform"},{"uid":"infra","title":"Infra"}]
                }))),
                (Method::GET, "/api/folders/platform") => Ok(Some(json!({
                    "uid":"platform",
                    "title":"Platform"
                }))),
                (Method::DELETE, "/api/dashboards/uid/cpu-main") => {
                    Ok(Some(json!({"status":"success"})))
                }
                (Method::DELETE, "/api/dashboards/uid/mem-main") => {
                    Ok(Some(json!({"status":"success"})))
                }
                (Method::DELETE, "/api/folders/child") => Ok(Some(json!({"status":"success"}))),
                (Method::DELETE, "/api/folders/infra") => Ok(Some(json!({"status":"success"}))),
                _ => Err(message(format!("unexpected request {method} {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 4);
    let calls = calls.lock().unwrap();
    let delete_paths: Vec<String> = calls
        .iter()
        .filter(|(method, _, _)| *method == Method::DELETE)
        .map(|(_, path, _)| path.clone())
        .collect();
    assert_eq!(
        delete_paths,
        vec![
            "/api/dashboards/uid/cpu-main".to_string(),
            "/api/dashboards/uid/mem-main".to_string(),
            "/api/folders/child".to_string(),
            "/api/folders/infra".to_string(),
        ]
    );
}

#[test]
fn dashboard_browse_document_builds_tree_with_general_and_nested_folders() {
    let summaries = vec![
        serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "folderUid": "infra",
            "folderTitle": "Infra",
            "folderPath": "Platform / Infra",
            "url": "/d/cpu-main/cpu-main"
        }))
        .unwrap(),
        serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "mem-main",
            "title": "Memory Main",
            "folderUid": "",
            "folderTitle": "General",
            "folderPath": "General",
            "url": "/d/mem-main/memory-main"
        }))
        .unwrap(),
    ];
    let folders = vec![crate::dashboard::FolderInventoryItem {
        uid: "infra".to_string(),
        title: "Infra".to_string(),
        path: "Platform / Infra".to_string(),
        parent_uid: Some("platform".to_string()),
        org: "Main Org.".to_string(),
        org_id: "1".to_string(),
    }];

    let document = build_dashboard_browse_document(&summaries, &folders, None).unwrap();

    assert_eq!(document.summary.folder_count, 3);
    assert_eq!(document.summary.dashboard_count, 2);
    assert_eq!(document.nodes[0].title, "General");
    assert_eq!(document.nodes[1].title, "Memory Main");
    assert_eq!(document.nodes[1].depth, 1);
    assert_eq!(document.nodes[2].title, "Platform");
    assert_eq!(document.nodes[3].title, "Infra");
    assert_eq!(document.nodes[4].title, "CPU Main");
    assert_eq!(document.nodes[4].depth, 2);
}

#[test]
fn dashboard_browse_document_filters_to_requested_root_path() {
    let summaries = vec![
        serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "folderUid": "infra",
            "folderTitle": "Infra",
            "folderPath": "Platform / Infra"
        }))
        .unwrap(),
        serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "ops-main",
            "title": "Ops Main",
            "folderUid": "ops",
            "folderTitle": "Ops",
            "folderPath": "Ops"
        }))
        .unwrap(),
    ];
    let folders = vec![
        crate::dashboard::FolderInventoryItem {
            uid: "infra".to_string(),
            title: "Infra".to_string(),
            path: "Platform / Infra".to_string(),
            parent_uid: Some("platform".to_string()),
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        },
        crate::dashboard::FolderInventoryItem {
            uid: "ops".to_string(),
            title: "Ops".to_string(),
            path: "Ops".to_string(),
            parent_uid: None,
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        },
    ];

    let document =
        build_dashboard_browse_document(&summaries, &folders, Some("Platform / Infra")).unwrap();

    assert_eq!(
        document.summary.root_path.as_deref(),
        Some("Platform / Infra")
    );
    assert_eq!(document.summary.folder_count, 1);
    assert_eq!(document.summary.dashboard_count, 1);
    assert_eq!(document.nodes.len(), 2);
    assert_eq!(document.nodes[0].title, "Infra");
    assert_eq!(document.nodes[0].depth, 0);
    assert_eq!(document.nodes[1].title, "CPU Main");
    assert_eq!(document.nodes[1].depth, 1);
}

#[test]
fn dashboard_edit_resolves_destination_folder_uid_from_browser_tree() {
    let document = build_dashboard_browse_document(
        &[serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "folderUid": "infra",
            "folderTitle": "Infra",
            "folderPath": "Platform / Infra"
        }))
        .unwrap()],
        &[crate::dashboard::FolderInventoryItem {
            uid: "infra".to_string(),
            title: "Infra".to_string(),
            path: "Platform / Infra".to_string(),
            parent_uid: Some("platform".to_string()),
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        }],
        None,
    )
    .unwrap();

    let uid = resolve_folder_uid_for_path(&document, "Platform / Infra").unwrap();
    assert_eq!(uid, "infra");
}

#[test]
fn dashboard_edit_fetch_draft_reads_current_live_title_and_tags() {
    let node = crate::dashboard::browse_support::DashboardBrowseNode {
        kind: crate::dashboard::browse_support::DashboardBrowseNodeKind::Dashboard,
        title: "CPU Main".to_string(),
        path: "Platform / Infra".to_string(),
        uid: Some("cpu-main".to_string()),
        depth: 1,
        meta: "uid=cpu-main".to_string(),
        details: Vec::new(),
        url: None,
        org_name: "Main Org.".to_string(),
        org_id: "1".to_string(),
        child_count: 0,
    };

    let draft = fetch_dashboard_edit_draft_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "tags": ["prod", "infra"]
                },
                "meta": {
                    "folderUid": "infra"
                }
            }))),
            _ => Err(message("unexpected request")),
        },
        &node,
    )
    .unwrap();

    assert_eq!(draft.uid, "cpu-main");
    assert_eq!(draft.title, "CPU Main");
    assert_eq!(draft.folder_path, "Platform / Infra");
    assert_eq!(draft.tags, vec!["prod".to_string(), "infra".to_string()]);
}

#[test]
fn dashboard_edit_apply_posts_updated_title_tags_and_folder_uid() {
    let payloads = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let recorded = payloads.clone();
    let draft = DashboardEditDraft {
        uid: "cpu-main".to_string(),
        title: "CPU Main".to_string(),
        folder_path: "Platform / Infra".to_string(),
        tags: vec!["prod".to_string()],
    };
    let update = DashboardEditUpdate {
        title: Some("CPU Overview".to_string()),
        folder_path: Some("Platform / Ops".to_string()),
        tags: Some(vec!["ops".to_string(), "gold".to_string()]),
    };

    apply_dashboard_edit_with_request(
        move |method, path, _params, payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "tags": ["prod"]
                },
                "meta": {
                    "folderUid": "infra"
                }
            }))),
            (Method::POST, "/api/dashboards/db") => {
                recorded
                    .lock()
                    .unwrap()
                    .push(payload.cloned().unwrap_or(Value::Null));
                Ok(Some(json!({"status": "success"})))
            }
            _ => Err(message("unexpected request")),
        },
        &draft,
        &update,
        Some("ops"),
    )
    .unwrap();

    let payloads = payloads.lock().unwrap();
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0]["dashboard"]["title"], "CPU Overview");
    assert_eq!(payloads[0]["dashboard"]["tags"], json!(["ops", "gold"]));
    assert_eq!(payloads[0]["folderUid"], "ops");
    assert_eq!(payloads[0]["overwrite"], true);
}

#[test]
fn dashboard_edit_dialog_folder_picker_selects_existing_folder_path() {
    let document = build_dashboard_browse_document(
        &[serde_json::from_value::<Map<String, Value>>(json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "folderUid": "infra",
            "folderTitle": "Infra",
            "folderPath": "Platform / Infra"
        }))
        .unwrap()],
        &[
            crate::dashboard::FolderInventoryItem {
                uid: "platform".to_string(),
                title: "Platform".to_string(),
                path: "Platform".to_string(),
                parent_uid: None,
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
            },
            crate::dashboard::FolderInventoryItem {
                uid: "infra".to_string(),
                title: "Infra".to_string(),
                path: "Platform / Infra".to_string(),
                parent_uid: Some("platform".to_string()),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
            },
            crate::dashboard::FolderInventoryItem {
                uid: "ops".to_string(),
                title: "Ops".to_string(),
                path: "Platform / Ops".to_string(),
                parent_uid: Some("platform".to_string()),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
            },
        ],
        None,
    )
    .unwrap();
    let draft = DashboardEditDraft {
        uid: "cpu-main".to_string(),
        title: "CPU Main".to_string(),
        folder_path: "Platform / Infra".to_string(),
        tags: vec!["prod".to_string()],
    };
    let mut dialog =
        crate::dashboard::browse_edit_dialog::EditDialogState::from_draft(draft, &document);

    let _ = dialog.handle_key(&KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    let _ = dialog.handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let _ = dialog.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let action = dialog.handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        action,
        crate::dashboard::browse_edit_dialog::EditDialogAction::Continue
    );

    let save = dialog.handle_key(&KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
    match save {
        crate::dashboard::browse_edit_dialog::EditDialogAction::Save { update, .. } => {
            assert_eq!(update.folder_path.as_deref(), Some("Platform / Ops"));
        }
        _ => panic!("expected save action"),
    }
}

#[test]
fn dashboard_edit_dialog_ctrl_x_closes_dialog() {
    let document = build_dashboard_browse_document(
        &[],
        &[crate::dashboard::FolderInventoryItem {
            uid: "infra".to_string(),
            title: "Infra".to_string(),
            path: "Platform / Infra".to_string(),
            parent_uid: Some("platform".to_string()),
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        }],
        None,
    )
    .unwrap();
    let draft = DashboardEditDraft {
        uid: "cpu-main".to_string(),
        title: "CPU Main".to_string(),
        folder_path: "Platform / Infra".to_string(),
        tags: vec!["prod".to_string()],
    };
    let mut dialog =
        crate::dashboard::browse_edit_dialog::EditDialogState::from_draft(draft, &document);

    let action = dialog.handle_key(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
    assert_eq!(
        action,
        crate::dashboard::browse_edit_dialog::EditDialogAction::Cancelled
    );
}

#[test]
fn dashboard_view_lines_include_recent_versions_when_history_exists() {
    let node = crate::dashboard::browse_support::DashboardBrowseNode {
        kind: crate::dashboard::browse_support::DashboardBrowseNodeKind::Dashboard,
        title: "CPU Main".to_string(),
        path: "Platform / Infra".to_string(),
        uid: Some("cpu-main".to_string()),
        depth: 1,
        meta: "uid=cpu-main".to_string(),
        details: vec!["Type: Dashboard".to_string()],
        url: None,
        org_name: "Main Org.".to_string(),
        org_id: "1".to_string(),
        child_count: 0,
    };

    let lines = fetch_dashboard_view_lines_with_request(
        |method, path, params, _payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "id": 42,
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "version": 7,
                    "schemaVersion": 39,
                    "tags": ["prod"],
                    "panels": [],
                    "links": []
                },
                "meta": {
                    "slug": "cpu-main",
                    "canEdit": true
                }
            }))),
            (Method::GET, "/api/dashboards/uid/cpu-main/versions") => {
                assert_eq!(params, &vec![("limit".to_string(), "5".to_string())]);
                Ok(Some(json!([
                    {
                        "version": 7,
                        "created": "2026-03-26T10:00:00Z",
                        "createdBy": "admin",
                        "message": "rename"
                    },
                    {
                        "version": 6,
                        "created": "2026-03-20T08:00:00Z",
                        "createdBy": "ops",
                        "message": ""
                    }
                ])))
            }
            _ => Err(message("unexpected request")),
        },
        &node,
    )
    .unwrap();

    assert!(lines.iter().any(|line| line == "Recent versions:"));
    assert!(lines
        .iter()
        .any(|line| line.contains("v7 | 2026-03-26T10:00:00Z | admin | rename")));
    assert!(lines
        .iter()
        .any(|line| line.contains("v6 | 2026-03-20T08:00:00Z | ops")));
}

#[test]
fn dashboard_view_lines_ignore_missing_versions_endpoint() {
    let node = crate::dashboard::browse_support::DashboardBrowseNode {
        kind: crate::dashboard::browse_support::DashboardBrowseNodeKind::Dashboard,
        title: "CPU Main".to_string(),
        path: "Platform / Infra".to_string(),
        uid: Some("cpu-main".to_string()),
        depth: 1,
        meta: "uid=cpu-main".to_string(),
        details: vec!["Type: Dashboard".to_string()],
        url: None,
        org_name: "Main Org.".to_string(),
        org_id: "1".to_string(),
        child_count: 0,
    };

    let lines = fetch_dashboard_view_lines_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "id": 42,
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "version": 7,
                    "schemaVersion": 39,
                    "tags": ["prod"],
                    "panels": [],
                    "links": []
                },
                "meta": {
                    "slug": "cpu-main",
                    "canEdit": true
                }
            }))),
            (Method::GET, "/api/dashboards/uid/cpu-main/versions") => Err(api_response(
                404,
                "http://localhost:3000/api/dashboards/uid/cpu-main/versions?limit=5",
                "{\"message\":\"Not found\"}",
            )),
            _ => Err(message("unexpected request")),
        },
        &node,
    )
    .unwrap();

    assert!(!lines.iter().any(|line| line == "Recent versions:"));
    assert!(lines.iter().any(|line| line == "Version: 7"));
}

#[test]
fn browser_state_replace_document_preserves_selected_dashboard_uid() {
    let old_document = build_dashboard_browse_document(
        &[
            serde_json::from_value::<Map<String, Value>>(json!({
                "uid": "cpu-main",
                "title": "CPU Main",
                "folderUid": "infra",
                "folderTitle": "Infra",
                "folderPath": "Platform / Infra"
            }))
            .unwrap(),
            serde_json::from_value::<Map<String, Value>>(json!({
                "uid": "mem-main",
                "title": "Memory Main",
                "folderUid": "infra",
                "folderTitle": "Infra",
                "folderPath": "Platform / Infra"
            }))
            .unwrap(),
        ],
        &[crate::dashboard::FolderInventoryItem {
            uid: "infra".to_string(),
            title: "Infra".to_string(),
            path: "Platform / Infra".to_string(),
            parent_uid: Some("platform".to_string()),
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        }],
        None,
    )
    .unwrap();
    let new_document = build_dashboard_browse_document(
        &[
            serde_json::from_value::<Map<String, Value>>(json!({
                "uid": "cpu-main",
                "title": "CPU Main",
                "folderUid": "ops",
                "folderTitle": "Ops",
                "folderPath": "Platform / Ops"
            }))
            .unwrap(),
            serde_json::from_value::<Map<String, Value>>(json!({
                "uid": "mem-main",
                "title": "Memory Main",
                "folderUid": "infra",
                "folderTitle": "Infra",
                "folderPath": "Platform / Infra"
            }))
            .unwrap(),
        ],
        &[
            crate::dashboard::FolderInventoryItem {
                uid: "infra".to_string(),
                title: "Infra".to_string(),
                path: "Platform / Infra".to_string(),
                parent_uid: Some("platform".to_string()),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
            },
            crate::dashboard::FolderInventoryItem {
                uid: "ops".to_string(),
                title: "Ops".to_string(),
                path: "Platform / Ops".to_string(),
                parent_uid: Some("platform".to_string()),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
            },
        ],
        None,
    )
    .unwrap();
    let mut state = crate::dashboard::browse_state::BrowserState::new(old_document);
    let selected_index = state
        .document
        .nodes
        .iter()
        .position(|node| node.uid.as_deref() == Some("cpu-main"))
        .expect("cpu-main index");
    state.list_state.select(Some(selected_index));

    state.replace_document(new_document);

    let selected = state.selected_node().expect("selected node");
    assert_eq!(selected.uid.as_deref(), Some("cpu-main"));
    assert_eq!(selected.path, "Platform / Ops");
}

#[test]
fn dashboard_history_versions_lists_recent_versions_by_uid() {
    let versions = list_dashboard_history_versions_with_request(
        |method, path, params, _payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main/versions") => {
                assert_eq!(params, &vec![("limit".to_string(), "20".to_string())]);
                Ok(Some(json!({
                    "versions": [
                        {
                            "version": 7,
                            "created": "2026-03-26T10:00:00Z",
                            "createdBy": "admin",
                            "message": "rename"
                        },
                        {
                            "version": 6,
                            "created": "2026-03-20T08:00:00Z",
                            "createdBy": "ops",
                            "message": ""
                        }
                    ]
                })))
            }
            _ => Err(message("unexpected request")),
        },
        "cpu-main",
        20,
    )
    .unwrap();

    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, 7);
    assert_eq!(versions[0].created_by, "admin");
    assert_eq!(versions[1].version, 6);
}

#[test]
fn dashboard_history_restore_reimports_selected_version_payload() {
    let payloads = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let recorded = payloads.clone();

    restore_dashboard_history_version_with_request(
        move |method, path, _params, payload| match (method, path) {
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "uid": "cpu-main",
                    "title": "CPU Main"
                },
                "meta": {
                    "folderUid": "infra"
                }
            }))),
            (Method::GET, "/api/dashboards/uid/cpu-main/versions/5") => Ok(Some(json!({
                "version": 5,
                "data": {
                    "id": 42,
                    "version": 5,
                    "uid": "cpu-main",
                    "title": "CPU Old",
                    "tags": ["legacy"]
                }
            }))),
            (Method::POST, "/api/dashboards/db") => {
                recorded
                    .lock()
                    .unwrap()
                    .push(payload.cloned().unwrap_or(Value::Null));
                Ok(Some(json!({"status": "success"})))
            }
            _ => Err(message("unexpected request")),
        },
        "cpu-main",
        5,
    )
    .unwrap();

    let payloads = payloads.lock().unwrap();
    assert_eq!(payloads.len(), 1);
    let payload = payloads[0].as_object().unwrap();
    assert_eq!(payload["overwrite"], json!(true));
    assert_eq!(payload["folderUid"], json!("infra"));
    assert_eq!(payload["dashboard"]["uid"], json!("cpu-main"));
    assert_eq!(payload["dashboard"]["id"], Value::Null);
    assert_eq!(payload["dashboard"]["title"], json!("CPU Old"));
    assert!(payload["dashboard"].get("version").is_none());
}

#[test]
fn dashboard_history_dialog_escape_and_q_close_dialog() {
    let versions = vec![crate::dashboard::history::DashboardHistoryVersion {
        version: 7,
        created: "2026-03-26T10:00:00Z".to_string(),
        created_by: "admin".to_string(),
        message: "rename".to_string(),
    }];
    let mut dialog = crate::dashboard::browse_history_dialog::HistoryDialogState::new(
        "cpu-main".to_string(),
        "CPU Main".to_string(),
        versions.clone(),
    );
    let esc = dialog.handle_key(&KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(
        esc,
        crate::dashboard::browse_history_dialog::HistoryDialogAction::Close
    );

    let mut dialog = crate::dashboard::browse_history_dialog::HistoryDialogState::new(
        "cpu-main".to_string(),
        "CPU Main".to_string(),
        versions,
    );
    let q = dialog.handle_key(&KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
    assert_eq!(
        q,
        crate::dashboard::browse_history_dialog::HistoryDialogAction::Close
    );
}

#[test]
fn interactive_import_loads_dashboard_titles_and_folder_paths() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "rate(cpu[5m])",
    );

    let args = make_import_args(raw_dir);
    let items = load_interactive_import_items(&args).unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].uid, "cpu-main");
    assert_eq!(items[0].title, "CPU Main");
    assert_eq!(items[0].folder_path, "Infra");
}

#[test]
fn interactive_import_state_toggles_and_confirms_selected_files() {
    let items = vec![
        crate::dashboard::import_interactive::InteractiveImportItem {
            path: PathBuf::from("a.json"),
            uid: "a".to_string(),
            title: "CPU".to_string(),
            folder_path: "Infra".to_string(),
            file_label: "a.json".to_string(),
        },
        crate::dashboard::import_interactive::InteractiveImportItem {
            path: PathBuf::from("b.json"),
            uid: "b".to_string(),
            title: "Memory".to_string(),
            folder_path: "Infra".to_string(),
            file_label: "b.json".to_string(),
        },
    ];
    let mut state = InteractiveImportState::new(items);

    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(state.selected_files(), vec![PathBuf::from("a.json")]);
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.selected_files(),
        vec![PathBuf::from("a.json"), PathBuf::from("b.json")]
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        InteractiveImportAction::Confirm(vec![PathBuf::from("a.json"), PathBuf::from("b.json")])
    );
}

#[test]
fn import_with_use_export_org_rejects_interactive_selection() {
    let temp = tempdir().unwrap();
    let mut args = make_import_args(temp.path().join("exports"));
    args.use_export_org = true;
    args.interactive = true;

    let error = import_dashboards_with_org_clients(&args).unwrap_err();

    assert!(error
        .to_string()
        .contains("Dashboard import --interactive does not support --use-export-org yet."));
}

#[test]
fn dashboard_raw_edit_validation_rejects_overwrite_in_user_payload() {
    let error = validate_external_dashboard_edit_value(&json!({
        "dashboard": {
            "uid": "cpu-main",
            "title": "CPU Main"
        },
        "overwrite": true
    }))
    .unwrap_err();

    assert!(error.to_string().contains("must not include overwrite"));
}

#[test]
fn dashboard_raw_edit_review_summarizes_title_tags_and_folder_uid_changes() {
    let draft = ExternalDashboardEditDraft {
        uid: "cpu-main".to_string(),
        title: "CPU Main".to_string(),
        payload: json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main",
                "tags": ["prod"]
            },
            "folderUid": "infra"
        }),
    };

    let review = review_external_dashboard_edit(
        &draft,
        &json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Overview",
                "tags": ["gold", "ops"]
            },
            "folderUid": "ops"
        }),
    )
    .unwrap()
    .unwrap();

    assert!(review.summary_lines[0].contains("uid=cpu-main"));
    assert!(review.summary_lines[1].contains("CPU Main -> CPU Overview"));
    assert!(review.summary_lines[3].contains("infra -> ops"));
    assert!(review.summary_lines[4].contains("prod -> gold, ops"));
}

#[test]
fn dashboard_raw_edit_apply_posts_payload_with_overwrite_and_message() {
    let payloads = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let recorded = payloads.clone();

    apply_external_dashboard_edit_with_request(
        move |method, path, _params, payload| match (method, path) {
            (Method::POST, "/api/dashboards/db") => {
                recorded
                    .lock()
                    .unwrap()
                    .push(payload.cloned().unwrap_or(Value::Null));
                Ok(Some(json!({"status":"success"})))
            }
            _ => Err(message("unexpected request")),
        },
        &json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Overview",
                "tags": ["gold", "ops"]
            },
            "folderUid": "ops"
        }),
    )
    .unwrap();

    let payloads = payloads.lock().unwrap();
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0]["dashboard"]["title"], "CPU Overview");
    assert_eq!(payloads[0]["folderUid"], "ops");
    assert_eq!(payloads[0]["overwrite"], true);
    assert_eq!(
        payloads[0]["message"],
        "Edited by grafana-utils dashboard browse"
    );
}

#[test]
fn render_dashboard_summary_json_returns_objects() {
    let summaries = vec![
        json!({
            "uid": "abc",
            "folderUid": "infra",
            "folderPath": "Platform / Infra",
            "folderTitle": "Infra",
            "orgId": 1,
            "orgName": "Main Org",
            "title": "CPU"
        })
        .as_object()
        .unwrap()
        .clone(),
        json!({
            "orgId": 1,
            "orgName": "Main Org",
            "uid": "xyz",
            "title": "Overview"
        })
        .as_object()
        .unwrap()
        .clone(),
    ];

    let value = render_dashboard_summary_json(&summaries, &[]);
    assert_eq!(
        value,
        json!([
            {
                "uid": "abc",
                "name": "CPU",
                "folder": "Infra",
                "folderUid": "infra",
                "path": "Platform / Infra",
                "org": "Main Org",
                "orgId": "1"
            },
            {
                "uid": "xyz",
                "name": "Overview",
                "folder": "General",
                "folderUid": "general",
                "path": "General",
                "org": "Main Org",
                "orgId": "1"
            }
        ])
    );
}

#[test]
fn render_dashboard_summary_json_includes_sources_when_present() {
    let summaries = vec![json!({
        "uid": "abc",
        "folderUid": "infra",
        "folderPath": "Platform / Infra",
        "folderTitle": "Infra",
        "orgId": 1,
        "orgName": "Main Org",
        "title": "CPU",
        "sources": ["Loki Logs", "Prom Main"],
        "sourceUids": ["loki_uid", "prom_uid"]
    })
    .as_object()
    .unwrap()
    .clone()];

    let value = render_dashboard_summary_json(&summaries, &[]);
    assert_eq!(
        value,
        json!([
            {
                "uid": "abc",
                "name": "CPU",
                "folder": "Infra",
                "folderUid": "infra",
                "path": "Platform / Infra",
                "org": "Main Org",
                "orgId": "1",
                "sources": ["Loki Logs", "Prom Main"],
                "sourceUids": ["loki_uid", "prom_uid"]
            }
        ])
    );
}

#[test]
fn render_dashboard_summary_table_respects_selected_columns() {
    let summaries = vec![json!({
        "uid": "abc",
        "folderUid": "infra",
        "folderPath": "Platform / Infra",
        "folderTitle": "Infra",
        "orgId": 1,
        "orgName": "Main Org",
        "title": "CPU",
        "sources": ["Prom Main"]
    })
    .as_object()
    .unwrap()
    .clone()];

    let lines = render_dashboard_summary_table(
        &summaries,
        &["uid".to_string(), "name".to_string(), "sources".to_string()],
        true,
    );
    assert!(lines[0].contains("UID"));
    assert!(lines[0].contains("NAME"));
    assert!(lines[0].contains("SOURCES"));
    assert!(lines[2].contains("abc"));
    assert!(lines[2].contains("CPU"));
    assert!(lines[2].contains("Prom Main"));
}

#[test]
fn render_dashboard_summary_json_respects_selected_columns() {
    let summaries = vec![json!({
        "uid": "abc",
        "folderUid": "infra",
        "folderPath": "Platform / Infra",
        "folderTitle": "Infra",
        "orgId": 1,
        "orgName": "Main Org",
        "title": "CPU",
        "sources": ["Loki Logs", "Prom Main"],
        "sourceUids": ["loki_uid", "prom_uid"]
    })
    .as_object()
    .unwrap()
    .clone()];

    let value = render_dashboard_summary_json(
        &summaries,
        &[
            "uid".to_string(),
            "org_id".to_string(),
            "source_uids".to_string(),
        ],
    );
    assert_eq!(
        value,
        json!([
            {
                "uid": "abc",
                "orgId": "1",
                "sourceUids": ["loki_uid", "prom_uid"]
            }
        ])
    );
}

#[test]
fn build_folder_path_joins_parents_and_title() {
    let folder = json!({
        "title": "Child",
        "parents": [{"title": "Root"}, {"title": "Team"}]
    });
    let path = build_folder_path(folder.as_object().unwrap(), "Child");
    assert_eq!(path, "Root / Team / Child");
}

#[test]
fn attach_dashboard_folder_paths_with_request_uses_folder_hierarchy() {
    let summaries = vec![
        json!({
            "uid": "abc",
            "folderUid": "child",
            "folderTitle": "Child",
            "title": "CPU"
        })
        .as_object()
        .unwrap()
        .clone(),
        json!({
            "uid": "xyz",
            "title": "Overview"
        })
        .as_object()
        .unwrap()
        .clone(),
    ];

    let enriched = attach_dashboard_folder_paths_with_request(
        |_method, path, _params, _payload| match path {
            "/api/folders/child" => Ok(Some(json!({
                "title": "Child",
                "parents": [{"title": "Root"}]
            }))),
            _ => Err(test_support::message(format!("unexpected path {path}"))),
        },
        &summaries,
    )
    .unwrap();

    assert_eq!(enriched[0]["folderPath"], json!("Root / Child"));
    assert_eq!(enriched[1]["folderPath"], json!("General"));
}

#[test]
fn list_dashboards_with_request_returns_dashboard_count() {
    let args = ListArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        with_sources: false,
        output_columns: Vec::new(),
        table: false,
        csv: false,
        json: false,
        output_format: None,
        no_header: false,
    };

    let mut calls = Vec::new();
    let count = list_dashboards_with_request(
        |method, path, _params, _payload| {
            calls.push((method.to_string(), path.to_string()));
            match path {
                "/api/search" => Ok(Some(json!([
                    {"uid": "abc", "title": "CPU", "folderTitle": "Infra", "folderUid": "infra"},
                    {"uid": "def", "title": "Memory", "folderTitle": "Infra"},
                ]))),
                "/api/org" => Ok(Some(json!({
                    "id": 1,
                    "name": "Main Org"
                }))),
                "/api/folders/infra" => Ok(Some(json!({
                    "title": "Infra",
                    "parents": [{"title": "Platform"}]
                }))),
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 2);
    assert_eq!(
        calls.iter().filter(|(_, path)| path == "/api/org").count(),
        1
    );
    assert!(!calls.iter().any(|(_, path)| path == "/api/datasources"));
    assert!(!calls
        .iter()
        .any(|(_, path)| path.starts_with("/api/dashboards/uid/")));
}

#[test]
fn collect_dashboard_source_names_prefers_datasource_names() {
    let payload = json!({
        "dashboard": {
            "uid": "abc",
            "title": "CPU",
            "panels": [
                {"datasource": {"uid": "prom_uid", "type": "prometheus"}},
                {"datasource": "Loki Logs"},
                {"datasource": "prometheus"},
                {"datasource": "-- Mixed --"}
            ]
        }
    });
    let catalog = test_support::build_datasource_catalog(&[
        json!({
            "uid": "prom_uid",
            "name": "Prom Main",
            "type": "prometheus",
            "pluginVersion": "11.0.0"
        })
        .as_object()
        .unwrap()
        .clone(),
        json!({
            "uid": "loki_uid",
            "name": "Loki Logs",
            "type": "loki",
            "meta": {"info": {"version": "3.1.0"}}
        })
        .as_object()
        .unwrap()
        .clone(),
    ]);

    let (sources, source_uids) =
        test_support::collect_dashboard_source_metadata(&payload, &catalog).unwrap();
    assert_eq!(
        sources,
        vec!["Loki Logs".to_string(), "Prom Main".to_string()]
    );
    assert_eq!(
        source_uids,
        vec!["loki_uid".to_string(), "prom_uid".to_string()]
    );
}

#[test]
fn collect_dashboard_source_names_accepts_preserved_raw_dashboard_documents() {
    let payload = json!({
        "uid": "abc",
        "title": "CPU",
        "panels": [
            {"datasource": {"uid": "prom_uid", "type": "prometheus"}},
            {"datasource": "Loki Logs"}
        ]
    });
    let catalog = test_support::build_datasource_catalog(&[
        json!({
            "uid": "prom_uid",
            "name": "Prom Main",
            "type": "prometheus"
        })
        .as_object()
        .unwrap()
        .clone(),
        json!({
            "uid": "loki_uid",
            "name": "Loki Logs",
            "type": "loki"
        })
        .as_object()
        .unwrap()
        .clone(),
    ]);

    let (sources, source_uids) =
        test_support::collect_dashboard_source_metadata(&payload, &catalog).unwrap();
    assert_eq!(
        sources,
        vec!["Loki Logs".to_string(), "Prom Main".to_string()]
    );
    assert_eq!(
        source_uids,
        vec!["loki_uid".to_string(), "prom_uid".to_string()]
    );
}

#[test]
fn list_dashboards_with_request_json_fetches_dashboards_and_datasources_by_default() {
    let args = ListArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        with_sources: false,
        output_columns: Vec::new(),
        table: false,
        csv: false,
        json: true,
        output_format: None,
        no_header: false,
    };
    let mut calls = Vec::new();

    let count = list_dashboards_with_request(
        |method, path, _params, _payload| {
            calls.push((method.to_string(), path.to_string()));
            match path {
                "/api/search" => Ok(Some(json!([
                    {"uid": "abc", "title": "CPU", "folderTitle": "Infra", "folderUid": "infra"}
                ]))),
                "/api/org" => Ok(Some(json!({
                    "id": 1,
                    "name": "Main Org"
                }))),
                "/api/folders/infra" => Ok(Some(json!({
                    "title": "Infra",
                    "parents": [{"title": "Platform"}]
                }))),
                "/api/datasources" => Ok(Some(json!([
                    {"uid": "prom_uid", "name": "Prom Main", "type": "prometheus"}
                ]))),
                "/api/dashboards/uid/abc" => Ok(Some(json!({
                    "dashboard": {
                        "uid": "abc",
                        "title": "CPU",
                        "panels": [
                            {"datasource": {"uid": "prom_uid", "type": "prometheus"}}
                        ]
                    }
                }))),
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(
        calls.iter().filter(|(_, path)| path == "/api/org").count(),
        1
    );
    assert!(calls.iter().any(|(_, path)| path == "/api/datasources"));
    assert!(calls
        .iter()
        .any(|(_, path)| path == "/api/dashboards/uid/abc"));
}

#[test]
fn list_dashboards_with_request_output_columns_sources_fetches_dashboard_sources() {
    let args = ListArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        with_sources: false,
        output_columns: vec!["uid".to_string(), "sources".to_string()],
        table: true,
        csv: false,
        json: false,
        output_format: None,
        no_header: false,
    };
    let mut calls = Vec::new();

    let count = list_dashboards_with_request(
        |method, path, _params, _payload| {
            calls.push((method.to_string(), path.to_string()));
            match path {
                "/api/search" => Ok(Some(json!([
                    {"uid": "abc", "title": "CPU", "folderTitle": "Infra", "folderUid": "infra"}
                ]))),
                "/api/org" => Ok(Some(json!({
                    "id": 1,
                    "name": "Main Org"
                }))),
                "/api/folders/infra" => Ok(Some(json!({
                    "title": "Infra",
                    "parents": [{"title": "Platform"}]
                }))),
                "/api/datasources" => Ok(Some(json!([
                    {"uid": "prom_uid", "name": "Prom Main", "type": "prometheus"}
                ]))),
                "/api/dashboards/uid/abc" => Ok(Some(json!({
                    "dashboard": {
                        "uid": "abc",
                        "title": "CPU",
                        "panels": [
                            {"datasource": {"uid": "prom_uid", "type": "prometheus"}}
                        ]
                    }
                }))),
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert!(calls.iter().any(|(_, path)| path == "/api/datasources"));
    assert!(calls
        .iter()
        .any(|(_, path)| path == "/api/dashboards/uid/abc"));
}

#[test]
fn list_dashboards_with_request_with_org_id_scopes_requests() {
    let args = ListArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        page_size: 500,
        org_id: Some(7),
        all_orgs: false,
        with_sources: false,
        output_columns: Vec::new(),
        table: false,
        csv: false,
        json: true,
        output_format: None,
        no_header: false,
    };
    let mut calls = Vec::new();

    let count = list_dashboards_with_request(
        |method, path, params, _payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            let scoped_org = params
                .iter()
                .find(|(key, _)| key == "orgId")
                .map(|(_, value)| value.as_str());
            match (path, scoped_org) {
                ("/api/search", Some("7")) => Ok(Some(json!([
                    {"uid": "abc", "title": "CPU", "folderTitle": "Infra", "folderUid": "infra"}
                ]))),
                ("/api/org", Some("7")) => Ok(Some(json!({
                    "id": 7,
                    "name": "Scoped Org"
                }))),
                ("/api/folders/infra", Some("7")) => Ok(Some(json!({
                    "title": "Infra",
                    "parents": [{"title": "Platform"}]
                }))),
                ("/api/datasources", Some("7")) => Ok(Some(json!([
                    {"uid": "prom_uid", "name": "Prom Main", "type": "prometheus"}
                ]))),
                ("/api/dashboards/uid/abc", Some("7")) => Ok(Some(json!({
                    "dashboard": {
                        "uid": "abc",
                        "title": "CPU",
                        "panels": [
                            {"datasource": {"uid": "prom_uid", "type": "prometheus"}}
                        ]
                    }
                }))),
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params)| path == "/api/search"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "7"))
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params)| path == "/api/datasources"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "7"))
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params)| path == "/api/dashboards/uid/abc"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "7"))
            .count(),
        1
    );
}

#[test]
fn list_dashboards_with_request_all_orgs_aggregates_results() {
    let args = ListArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        page_size: 500,
        org_id: None,
        all_orgs: true,
        with_sources: false,
        output_columns: Vec::new(),
        table: false,
        csv: false,
        json: true,
        output_format: None,
        no_header: false,
    };
    let mut calls = Vec::new();

    let count = list_dashboards_with_request(
        |method, path, params, _payload| {
            calls.push((method.to_string(), path.to_string(), params.to_vec()));
            let scoped_org = params
                .iter()
                .find(|(key, _)| key == "orgId")
                .map(|(_, value)| value.as_str());
            match (path, scoped_org) {
                ("/api/orgs", None) => Ok(Some(json!([
                    {"id": 1, "name": "Main Org"},
                    {"id": 2, "name": "Ops Org"}
                ]))),
                ("/api/search", Some("1")) => Ok(Some(json!([
                    {"uid": "abc", "title": "CPU", "folderTitle": "Infra", "folderUid": "infra"}
                ]))),
                ("/api/datasources", Some("1")) => Ok(Some(json!([
                    {"uid": "prom_uid", "name": "Prom Main", "type": "prometheus"}
                ]))),
                ("/api/search", Some("2")) => Ok(Some(json!([
                    {"uid": "xyz", "title": "Logs", "folderTitle": "Ops", "folderUid": "ops"}
                ]))),
                ("/api/datasources", Some("2")) => Ok(Some(json!([
                    {"uid": "loki_uid", "name": "Loki Logs", "type": "loki"}
                ]))),
                ("/api/folders/infra", Some("1")) => Ok(Some(json!({
                    "title": "Infra",
                    "parents": [{"title": "Platform"}]
                }))),
                ("/api/folders/ops", Some("2")) => Ok(Some(json!({
                    "title": "Ops",
                    "parents": [{"title": "Platform"}]
                }))),
                ("/api/dashboards/uid/abc", Some("1")) => Ok(Some(json!({
                    "dashboard": {
                        "uid": "abc",
                        "title": "CPU",
                        "panels": [
                            {"datasource": {"uid": "prom_uid", "type": "prometheus"}}
                        ]
                    }
                }))),
                ("/api/dashboards/uid/xyz", Some("2")) => Ok(Some(json!({
                    "dashboard": {
                        "uid": "xyz",
                        "title": "Logs",
                        "panels": [
                            {"datasource": {"uid": "loki_uid", "type": "loki"}}
                        ]
                    }
                }))),
                _ => Err(test_support::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 2);
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, _)| path == "/api/orgs")
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params)| path == "/api/search"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "1"))
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params)| path == "/api/search"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "2"))
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params)| path == "/api/datasources"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "1"))
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params)| path == "/api/datasources"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "2"))
            .count(),
        1
    );
}
