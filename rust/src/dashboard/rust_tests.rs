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
    list_dashboards_with_request, load_dashboard_export_root_manifest, parse_cli_from,
    render_dashboard_governance_gate_result, render_dashboard_summary_csv,
    render_dashboard_summary_json, render_dashboard_summary_table, render_impact_text,
    render_import_dry_run_json, render_import_dry_run_table, render_topology_dot,
    render_topology_mermaid, resolve_dashboard_export_root, BrowseArgs, CommonCliArgs,
    DashboardCliArgs, DashboardCommand, DashboardExportRootManifest, DashboardExportRootScopeKind,
    DashboardGovernanceGateFinding, DashboardGovernanceGateResult, DashboardGovernanceGateSummary,
    DiffArgs, ExportArgs, ExportOrgSummary, FolderInventoryStatusKind, GovernanceGateArgs,
    GovernanceGateOutputFormat, GovernancePolicySource, ImpactAlertResource, ImpactDashboard,
    ImpactDocument, ImpactOutputFormat, ImpactSummary, ImportArgs, InspectExportArgs,
    InspectExportReportFormat, InspectLiveArgs, InspectOutputFormat, ListArgs, SimpleOutputFormat,
    TopologyDocument, TopologyOutputFormat, ValidationOutputFormat,
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
use crate::dashboard::{resolve_dashboard_import_source, DashboardImportInputFormat};
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
        color: crate::common::CliColorChoice::Auto,
        profile: None,
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
        color: crate::common::CliColorChoice::Auto,
        profile: None,
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
        input_format: DashboardImportInputFormat::Raw,
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

#[test]
fn dashboard_export_root_manifest_classifies_root_scopes() {
    let org_root = DashboardExportRootManifest::from_metadata(build_export_metadata(
        "root",
        1,
        None,
        None,
        None,
        None,
        Some("Main Org."),
        Some("1"),
        None,
    ));
    assert_eq!(org_root.scope_kind, DashboardExportRootScopeKind::OrgRoot);

    let all_orgs_root = DashboardExportRootManifest::from_metadata(build_export_metadata(
        "root",
        2,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(vec![ExportOrgSummary {
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
            dashboard_count: 2,
            datasource_count: None,
            used_datasource_count: None,
            used_datasources: None,
            export_dir: None,
        }]),
    ));
    assert_eq!(
        all_orgs_root.scope_kind,
        DashboardExportRootScopeKind::AllOrgsRoot
    );

    let workspace_root = all_orgs_root
        .clone()
        .with_scope_kind(DashboardExportRootScopeKind::WorkspaceRoot);
    assert_eq!(
        workspace_root.scope_kind,
        DashboardExportRootScopeKind::WorkspaceRoot
    );
    assert!(workspace_root.scope_kind.is_aggregate());
}

#[test]
fn load_dashboard_export_root_manifest_honors_explicit_workspace_scope_kind() {
    let temp = tempdir().unwrap();
    let metadata_path = temp.path().join(EXPORT_METADATA_FILENAME);
    fs::write(
        &metadata_path,
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "root",
            "scopeKind": "workspace-root",
            "dashboardCount": 2,
            "indexFile": "index.json",
            "orgCount": 2,
            "orgs": [
                {"org": "Main Org.", "orgId": "1", "dashboardCount": 1},
                {"org": "Ops Org.", "orgId": "2", "dashboardCount": 1}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let manifest = load_dashboard_export_root_manifest(&metadata_path).unwrap();
    assert_eq!(
        manifest.scope_kind,
        DashboardExportRootScopeKind::WorkspaceRoot
    );
}

#[test]
fn resolve_dashboard_export_root_detects_workspace_wrapper_root() {
    let temp = tempdir().unwrap();
    let workspace_root = temp.path().join("workspace");
    let dashboard_root = workspace_root.join("dashboards");
    fs::create_dir_all(workspace_root.join("datasources")).unwrap();
    fs::create_dir_all(dashboard_root.join("org_1_Main_Org").join("raw")).unwrap();
    fs::write(
        dashboard_root.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&build_export_metadata(
            "root",
            1,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(vec![ExportOrgSummary {
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
                dashboard_count: 1,
                datasource_count: None,
                used_datasource_count: None,
                used_datasources: None,
                export_dir: None,
            }]),
        ))
        .unwrap(),
    )
    .unwrap();

    let resolved = resolve_dashboard_export_root(&workspace_root)
        .unwrap()
        .expect("workspace root should resolve");
    assert_eq!(resolved.metadata_dir, dashboard_root);
    assert_eq!(
        resolved.manifest.scope_kind,
        DashboardExportRootScopeKind::WorkspaceRoot
    );
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn write_basic_provisioning_export(
    provisioning_dir: &Path,
    org_id: &str,
    org_name: &str,
    dashboard_uid: &str,
    dashboard_title: &str,
    datasource_uid: &str,
    datasource_type: &str,
    panel_type: &str,
    dashboard_rel_path: &str,
    query_field: &str,
    query_text: &str,
) {
    let dashboards_dir = provisioning_dir.join("dashboards");
    let dashboard_path = dashboards_dir.join(dashboard_rel_path);
    fs::create_dir_all(dashboard_path.parent().unwrap()).unwrap();
    fs::write(
        provisioning_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "provisioning",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-file-provisioning-dashboard",
            "org": org_name,
            "orgId": org_id
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        provisioning_dir.join("index.json"),
        serde_json::to_string_pretty(&json!([
            {
                "uid": dashboard_uid,
                "title": dashboard_title,
                "path": format!("dashboards/{dashboard_rel_path}"),
                "format": "grafana-file-provisioning-dashboard",
                "org": org_name,
                "orgId": org_id
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dashboard_path,
        serde_json::to_string_pretty(&json!({
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
#[path = "dashboard_authoring_rust_tests.rs"]
mod dashboard_authoring_rust_tests;
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
#[path = "inspect_live_tui_rust_tests.rs"]
mod inspect_live_tui_rust_tests;
#[cfg(test)]
#[path = "inspect_query_rust_tests.rs"]
mod inspect_query_rust_tests;

#[cfg(test)]
#[path = "dashboard_export_import_topology_import_format_rust_tests.rs"]
mod dashboard_export_import_topology_import_format_rust_tests;

#[test]
fn list_dashboards_with_request_all_orgs_aggregates_results() {
    let args = ListArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        page_size: 500,
        org_id: None,
        all_orgs: true,
        with_sources: false,
        output_columns: Vec::new(),
        text: false,
        table: false,
        csv: false,
        json: true,
        yaml: false,
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
