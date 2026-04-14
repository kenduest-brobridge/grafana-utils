//! Datasource inspect-export, local source help, and export-root loader tests.

use crate::datasource::{
    classify_datasource_export_root_scope_kind, load_datasource_export_root_manifest,
    load_datasource_inspect_export_source, load_datasource_inventory_records_from_export_root,
    prompt_datasource_inspect_export_input_format, render_datasource_inspect_export_output,
    resolve_datasource_inspect_export_input_format, DatasourceCliArgs,
    DatasourceExportRootScopeKind, DatasourceImportInputFormat,
    DatasourceInspectExportRenderFormat,
};
use std::fs;
use tempfile::tempdir;

use super::tail_fixtures::{
    write_diff_fixture, write_multi_org_import_fixture, write_provisioning_diff_fixture,
};
use super::*;

#[test]
fn datasource_inspect_export_renders_inventory_root_in_multiple_output_modes() {
    let root = write_diff_fixture(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "org": "Main Org",
        "orgId": "1"
    })]);

    let source =
        load_datasource_inspect_export_source(&root, DatasourceImportInputFormat::Inventory)
            .unwrap();
    let table = render_datasource_inspect_export_output(
        &source,
        DatasourceInspectExportRenderFormat::Table,
        None,
    )
    .unwrap();
    let text = render_datasource_inspect_export_output(
        &source,
        DatasourceInspectExportRenderFormat::Text,
        None,
    )
    .unwrap();
    let json_output = render_datasource_inspect_export_output(
        &source,
        DatasourceInspectExportRenderFormat::Json,
        None,
    )
    .unwrap();
    let yaml_output = render_datasource_inspect_export_output(
        &source,
        DatasourceInspectExportRenderFormat::Yaml,
        None,
    )
    .unwrap();

    assert!(table.contains("UID"));
    assert!(table.contains("Layer: operator-summary"));
    assert!(table.contains("Mode: inventory"));
    assert!(table.contains("Prometheus Main"));
    assert!(text.contains("Layer: operator-summary"));
    assert!(text.contains("Mode: inventory"));
    assert!(text.contains("Bundle: recovery-capable masked export"));
    assert!(text.contains("Datasource count: 1"));
    assert!(text.contains("Prometheus Main"));
    assert!(json_output.contains("\"inputMode\": \"inventory\""));
    assert!(json_output.contains("\"bundleKind\": \"masked-recovery\""));
    assert!(json_output.contains("\"masked\": true"));
    assert!(json_output.contains("\"recoveryCapable\": true"));
    assert!(json_output.contains("\"datasourceCount\": 1"));
    assert!(yaml_output.contains("inputMode: inventory"));
    assert!(yaml_output.contains("bundleKind: masked-recovery"));
    assert!(yaml_output.contains("masked: true"));
    assert!(yaml_output.contains("recoveryCapable: true"));
    assert!(yaml_output.contains("datasourceCount: 1"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn datasource_inspect_export_renders_provisioning_yaml_file_as_csv_and_yaml() {
    let root = write_provisioning_diff_fixture();
    let provisioning_file = root.join("provisioning/datasources.yaml");

    let source = load_datasource_inspect_export_source(
        &provisioning_file,
        DatasourceImportInputFormat::Provisioning,
    )
    .unwrap();
    let csv_output = render_datasource_inspect_export_output(
        &source,
        DatasourceInspectExportRenderFormat::Csv,
        None,
    )
    .unwrap();
    let yaml_output = render_datasource_inspect_export_output(
        &source,
        DatasourceInspectExportRenderFormat::Yaml,
        None,
    )
    .unwrap();

    assert!(csv_output.contains("uid,name,type,url,isDefault"));
    assert!(csv_output.contains("Prometheus Main"));
    assert!(yaml_output.contains("bundleKind: masked-recovery"));
    assert!(yaml_output.contains("masked: true"));
    assert!(yaml_output.contains("recoveryCapable: true"));
    assert!(yaml_output.contains("inputMode: provisioning"));
    assert!(yaml_output.contains("Prometheus Main"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn datasource_list_help_mentions_local_inventory_source_flags() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("list")
        .unwrap_or_else(|| panic!("missing datasource list help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--input-dir"));
    assert!(help.contains("--input-format"));
    assert!(help.contains("local"));
    assert!(help.contains("inventory"));
}

#[test]
fn datasource_export_root_manifest_classifies_org_and_workspace_roots() {
    let temp = tempdir().unwrap();
    let org_root = temp.path().join("org-root");
    fs::create_dir_all(&org_root).unwrap();
    fs::write(
        org_root.join("export-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "root",
            "scopeKind": "org-root",
            "resource": "datasource",
            "datasourcesFile": "datasources.json",
            "datasourceCount": 0,
            "format": "grafana-datasource-masked-recovery-v1",
            "exportMode": "masked-recovery",
            "masked": true,
            "recoveryCapable": true,
            "secretMaterial": "placeholders-only",
            "provisioningProjection": "derived-projection"
        }))
        .unwrap(),
    )
    .unwrap();
    let workspace_root = temp.path().join("workspace-root");
    fs::create_dir_all(&workspace_root).unwrap();
    fs::write(
        workspace_root.join("export-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "all-orgs-root",
            "scopeKind": "workspace-root",
            "resource": "datasource",
            "indexFile": "index.json",
            "datasourceCount": 0,
            "orgCount": 0,
            "format": "grafana-datasource-masked-recovery-v1",
            "exportMode": "masked-recovery",
            "masked": true,
            "recoveryCapable": true,
            "secretMaterial": "placeholders-only",
            "provisioningProjection": "derived-projection"
        }))
        .unwrap(),
    )
    .unwrap();

    let org_manifest =
        load_datasource_export_root_manifest(&org_root.join("export-metadata.json")).unwrap();
    let workspace_manifest =
        load_datasource_export_root_manifest(&workspace_root.join("export-metadata.json")).unwrap();

    assert_eq!(
        classify_datasource_export_root_scope_kind(&org_manifest.metadata),
        DatasourceExportRootScopeKind::OrgRoot
    );
    assert_eq!(
        org_manifest.scope_kind,
        DatasourceExportRootScopeKind::OrgRoot
    );
    assert_eq!(
        workspace_manifest.scope_kind,
        DatasourceExportRootScopeKind::WorkspaceRoot
    );
}

#[test]
fn datasource_inventory_root_loader_combines_all_orgs_children() {
    let temp = tempdir().unwrap();
    let root = write_multi_org_import_fixture(
        temp.path(),
        &[
            (
                1,
                "Main Org",
                vec![json!({
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": "true",
                    "org": "Main Org",
                    "orgId": "1"
                })],
            ),
            (
                2,
                "Ops Org",
                vec![json!({
                    "uid": "loki-ops",
                    "name": "Loki Ops",
                    "type": "loki",
                    "access": "proxy",
                    "url": "http://loki:3100",
                    "isDefault": "false",
                    "org": "Ops Org",
                    "orgId": "2"
                })],
            ),
        ],
    );

    let (manifest, records) = load_datasource_inventory_records_from_export_root(&root).unwrap();

    assert_eq!(
        manifest.scope_kind,
        DatasourceExportRootScopeKind::AllOrgsRoot
    );
    assert_eq!(records.len(), 2);
    assert_eq!(
        records
            .iter()
            .map(|record| record.org_id.as_str())
            .collect::<Vec<_>>(),
        vec!["1", "2"]
    );
}

#[test]
fn datasource_diff_help_mentions_operator_summary_report() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("diff")
        .unwrap_or_else(|| panic!("missing datasource diff help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--diff-dir"));
    assert!(help.contains("--input-format"));
    assert!(help.contains("provisioning"));
    assert!(help.contains("operator-summary diff report"));
    assert!(help.contains("datasource list --input-dir"));
}

#[test]
fn datasource_inspect_export_accepts_all_orgs_root_inventory() {
    let temp = tempdir().unwrap();
    let root = write_multi_org_import_fixture(
        temp.path(),
        &[
            (
                1,
                "Main Org",
                vec![json!({
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": "true",
                    "org": "Main Org",
                    "orgId": "1"
                })],
            ),
            (
                2,
                "Ops Org",
                vec![json!({
                    "uid": "loki-ops",
                    "name": "Loki Ops",
                    "type": "loki",
                    "access": "proxy",
                    "url": "http://loki:3100",
                    "isDefault": "false",
                    "org": "Ops Org",
                    "orgId": "2"
                })],
            ),
        ],
    );
    fs::write(
        root.join("export-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "all-orgs-root",
            "scopeKind": "all-orgs-root",
            "resource": "datasource",
            "indexFile": "index.json",
            "datasourceCount": 2,
            "orgCount": 2,
            "format": "grafana-datasource-masked-recovery-v1",
            "exportMode": "masked-recovery",
            "masked": true,
            "recoveryCapable": true,
            "secretMaterial": "placeholders-only",
            "provisioningProjection": "derived-projection"
        }))
        .unwrap(),
    )
    .unwrap();

    let source =
        load_datasource_inspect_export_source(&root, DatasourceImportInputFormat::Inventory)
            .unwrap();
    let text = render_datasource_inspect_export_output(
        &source,
        DatasourceInspectExportRenderFormat::Text,
        None,
    )
    .unwrap();

    assert!(text.contains("Datasource count: 2"));
    assert!(text.contains("Bundle: recovery-capable masked export"));
    assert!(text.contains("Prometheus Main"));
    assert!(text.contains("Loki Ops"));
}

#[test]
fn datasource_inspect_export_resolves_workspace_root_inventory() {
    let temp = tempdir().unwrap();
    let workspace_root = temp.path().join("snapshot");
    let datasource_export_root = write_multi_org_import_fixture(
        &workspace_root,
        &[
            (
                1,
                "Main Org",
                vec![json!({
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "access": "proxy",
                    "url": "http://prometheus:9090",
                    "isDefault": "true",
                    "org": "Main Org",
                    "orgId": "1"
                })],
            ),
            (
                3,
                "Ops Org",
                vec![json!({
                    "uid": "loki-ops",
                    "name": "Loki Ops",
                    "type": "loki",
                    "access": "proxy",
                    "url": "http://loki:3100",
                    "isDefault": "false",
                    "org": "Ops Org",
                    "orgId": "3"
                })],
            ),
        ],
    );
    let datasource_root = workspace_root.join("datasources");
    fs::rename(&datasource_export_root, &datasource_root).unwrap();
    fs::create_dir_all(workspace_root.join("dashboards")).unwrap();
    fs::write(
        datasource_root.join("export-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "all-orgs-root",
            "scopeKind": "workspace-root",
            "resource": "datasource",
            "indexFile": "index.json",
            "datasourceCount": 2,
            "orgCount": 2,
            "format": "grafana-datasource-masked-recovery-v1",
            "exportMode": "masked-recovery",
            "masked": true,
            "recoveryCapable": true,
            "secretMaterial": "placeholders-only",
            "provisioningProjection": "derived-projection"
        }))
        .unwrap(),
    )
    .unwrap();

    let input_format = resolve_datasource_inspect_export_input_format(&workspace_root, None)
        .unwrap()
        .unwrap();
    assert_eq!(input_format, DatasourceImportInputFormat::Inventory);

    let source = load_datasource_inspect_export_source(
        &workspace_root,
        DatasourceImportInputFormat::Inventory,
    )
    .unwrap();
    let text = render_datasource_inspect_export_output(
        &source,
        DatasourceInspectExportRenderFormat::Text,
        None,
    )
    .unwrap();

    assert!(text.contains("Variant: all-orgs-root"));
    assert!(text.contains("Datasource count: 2"));
    assert!(text.contains("Prometheus Main"));
    assert!(text.contains("Loki Ops"));
}

#[test]
fn datasource_inspect_export_prefers_inventory_for_noninteractive_ambiguous_root() {
    let root = write_diff_fixture(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "org": "Main Org",
        "orgId": "1"
    })]);
    fs::create_dir_all(root.join("provisioning")).unwrap();
    fs::write(
        root.join("provisioning/datasources.yaml"),
        r#"apiVersion: 1
datasources:
  - name: Provisioned Loki
    uid: loki-prov
    type: loki
    access: proxy
    url: http://loki:3100
"#,
    )
    .unwrap();

    let mode = resolve_datasource_inspect_export_input_format(
        &root,
        Some(DatasourceImportInputFormat::Inventory),
    )
    .unwrap()
    .unwrap();
    assert_eq!(mode, DatasourceImportInputFormat::Inventory);

    let source = load_datasource_inspect_export_source(&root, mode).unwrap();
    let text = render_datasource_inspect_export_output(
        &source,
        DatasourceInspectExportRenderFormat::Text,
        None,
    )
    .unwrap();
    assert!(text.contains("Prometheus Main"));
    assert!(!text.contains("Provisioned Loki"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn datasource_inspect_export_requires_explicit_input_type_without_tty_for_ambiguous_root() {
    let root = write_diff_fixture(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "org": "Main Org",
        "orgId": "1"
    })]);
    fs::create_dir_all(root.join("provisioning")).unwrap();
    fs::write(
        root.join("provisioning/datasources.yaml"),
        "apiVersion: 1\ndatasources: []\n",
    )
    .unwrap();

    let error = prompt_datasource_inspect_export_input_format(&root).unwrap_err();
    assert!(error
        .to_string()
        .contains("--input-format inventory or --input-format provisioning"));

    fs::remove_dir_all(root).unwrap();
}
