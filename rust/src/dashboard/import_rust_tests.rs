//! Import-focused dashboard regression tests.
#![allow(unused_imports)]

use super::dashboard_rust_tests::{
    make_basic_common_args, make_common_args, make_import_args,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata,
};
use super::test_support;
use super::test_support::{
    build_import_auth_context, import_dashboards_with_org_clients, import_dashboards_with_request,
    ImportArgs, EXPORT_METADATA_FILENAME, TOOL_SCHEMA_VERSION,
};
use crate::common::GrafanaCliError;
use crate::dashboard::import_lookup::resolve_source_dashboard_folder_path;
use crate::dashboard::import_validation::discover_export_org_import_scopes;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[cfg(test)]
#[path = "import_routed_scope_rust_tests.rs"]
mod import_routed_scope_rust_tests;

#[cfg(test)]
#[path = "import_routed_reporting_rust_tests.rs"]
mod import_routed_reporting_rust_tests;

#[test]
fn discover_export_org_import_scopes_reports_json_error_for_invalid_index_file() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("imports");
    let raw_dir = input_dir.join("org_1").join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(raw_dir.join("index.json"), "[").unwrap();

    let mut args = super::dashboard_rust_tests::make_import_args(input_dir);
    args.use_export_org = true;

    let error = discover_export_org_import_scopes(&args).unwrap_err();
    assert!(matches!(error, GrafanaCliError::Json(_)));
}

#[test]
fn resolve_source_dashboard_folder_path_reports_validation_error_for_unrelated_path() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("imports");
    fs::create_dir_all(&input_dir).unwrap();
    let dashboard_file = temp.path().join("other").join("dash.json");
    let folders_by_uid = std::collections::BTreeMap::new();

    let error = resolve_source_dashboard_folder_path(
        &json!({}),
        &dashboard_file,
        &input_dir,
        &folders_by_uid,
    )
    .unwrap_err();
    assert!(matches!(error, GrafanaCliError::Validation(_)));
    assert!(error
        .to_string()
        .contains("Failed to resolve import-relative dashboard path"));
}
