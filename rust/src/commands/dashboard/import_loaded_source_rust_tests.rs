//! Focused regressions for dashboard import source wrappers.
#![allow(unused_imports)]

use super::super::dashboard_rust_tests::{
    make_common_args, make_import_args, with_dashboard_import_live_preflight,
    write_basic_provisioning_export, write_basic_raw_export, write_combined_export_root_metadata,
};
use super::{
    collect_import_dry_run_report_with_request, resolve_diff_source, resolve_import_source,
    LoadedImportSource,
};
use crate::common::DiffOutputFormat;
use crate::dashboard::{DashboardImportInputFormat, DiffArgs};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn write_combined_raw_export_root(export_root: &Path) -> PathBuf {
    write_combined_export_root_metadata(export_root, &[("1", "Main Org", "org_1_Main_Org")]);
    let raw_root = export_root.join("org_1_Main_Org/raw");
    write_basic_raw_export(
        &raw_root,
        "1",
        "Main Org",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "up",
    );
    raw_root
}

fn write_combined_provisioning_export_root(export_root: &Path) -> PathBuf {
    write_combined_export_root_metadata(export_root, &[("1", "Main Org", "org_1_Main_Org")]);
    let provisioning_root = export_root.join("org_1_Main_Org/provisioning");
    write_basic_provisioning_export(
        &provisioning_root,
        "1",
        "Main Org",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "dashboards/cpu-main.json",
        "expr",
        "up",
    );
    provisioning_root
}

fn make_diff_args(input_dir: PathBuf, input_format: DashboardImportInputFormat) -> DiffArgs {
    DiffArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        input_dir,
        input_format,
        import_folder_uid: None,
        context_lines: 3,
        output_format: DiffOutputFormat::Text,
    }
}

fn assert_temp_backed_source_is_owned_until_drop(resolved: LoadedImportSource, variant: &str) {
    let dashboard_dir = resolved.dashboard_dir().to_path_buf();
    let metadata_dir = resolved.metadata_dir().to_path_buf();

    assert_eq!(
        dashboard_dir.file_name().and_then(|name| name.to_str()),
        Some(variant)
    );
    assert_eq!(metadata_dir, dashboard_dir);
    assert!(dashboard_dir.exists());

    drop(resolved);

    assert!(!dashboard_dir.exists());
}

#[test]
fn resolve_import_source_keeps_temp_backed_raw_export_root_alive_until_drop() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("export");
    let _raw_root = write_combined_raw_export_root(&export_root);

    let args = make_import_args(export_root);
    let resolved = resolve_import_source(&args).unwrap();

    assert_temp_backed_source_is_owned_until_drop(resolved, "raw");
}

#[test]
fn resolve_diff_source_keeps_temp_backed_provisioning_export_root_alive_until_drop() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("export");
    let _provisioning_root = write_combined_provisioning_export_root(&export_root);

    let args = make_diff_args(export_root, DashboardImportInputFormat::Provisioning);
    let resolved = resolve_diff_source(&args).unwrap();

    assert_temp_backed_source_is_owned_until_drop(resolved, "provisioning");
}

#[test]
fn resolve_import_source_resolves_git_sync_raw_root_from_repo_root() {
    let temp = tempdir().unwrap();
    let repo_root = temp.path();
    fs::create_dir_all(repo_root.join(".git")).unwrap();
    let raw_root = repo_root.join("dashboards/git-sync/raw");
    fs::create_dir_all(&raw_root).unwrap();
    write_basic_raw_export(
        &raw_root,
        "1",
        "Main Org",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "up",
    );

    let args = make_import_args(repo_root.to_path_buf());
    let resolved = resolve_import_source(&args).unwrap();

    assert_eq!(resolved.dashboard_dir(), raw_root);
    assert_eq!(resolved.metadata_dir(), raw_root);
}

#[test]
fn collect_import_dry_run_report_accepts_provisioning_root_variant_metadata() {
    let temp = tempdir().unwrap();
    let provisioning_root = temp.path().join("provisioning");
    write_basic_provisioning_export(
        &provisioning_root,
        "1",
        "Main Org",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "team/cpu-main.json",
        "expr",
        "up",
    );
    let mut args = make_import_args(provisioning_root);
    args.input_format = DashboardImportInputFormat::Provisioning;
    args.dry_run = true;

    let report = collect_import_dry_run_report_with_request(
        with_dashboard_import_live_preflight(
            json!([]),
            json!([]),
            |_method, path, _params, _payload| match path {
                "/api/dashboards/uid/cpu-main" => Ok(None),
                _ => Err(crate::common::message(format!("unexpected path {path}"))),
            },
        ),
        &args,
    )
    .unwrap();

    assert_eq!(report.dashboard_records.len(), 1);
}

#[cfg(feature = "tui")]
#[test]
fn interactive_import_items_keep_provisioning_folder_path_without_dashboards_wrapper() {
    let temp = tempdir().unwrap();
    let provisioning_root = temp.path().join("provisioning");
    write_basic_provisioning_export(
        &provisioning_root,
        "1",
        "Main Org",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "team/cpu-main.json",
        "expr",
        "up",
    );
    let mut args = make_import_args(provisioning_root);
    args.input_format = DashboardImportInputFormat::Provisioning;

    let items = super::super::import_interactive::load_interactive_import_items(&args).unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].folder_path, "team");
    assert_eq!(items[0].file_label, "team/cpu-main.json");
}
