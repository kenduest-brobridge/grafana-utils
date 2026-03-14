use super::{
    build_import_payload, diff_datasources_with_live, load_import_records, render_import_table,
    resolve_match, run_datasource_cli, DatasourceCliArgs, DatasourceImportRecord,
};
use clap::{CommandFactory, Parser};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;

fn live_datasource(
    id: i64,
    uid: &str,
    name: &str,
    datasource_type: &str,
) -> serde_json::Map<String, Value> {
    json!({
        "id": id,
        "uid": uid,
        "name": name,
        "type": datasource_type
    })
    .as_object()
    .unwrap()
    .clone()
}

fn load_contract_cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../tests/fixtures/datasource_contract_cases.json"
    ))
    .unwrap()
}

#[test]
fn import_help_explains_common_operator_flags() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("import")
        .unwrap_or_else(|| panic!("missing datasource import help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--import-dir"));
    assert!(help.contains("--org-id"));
    assert!(help.contains("--require-matching-export-org"));
    assert!(help.contains("--replace-existing"));
    assert!(help.contains("--update-existing-only"));
    assert!(help.contains("--dry-run"));
    assert!(help.contains("--table"));
    assert!(help.contains("--json"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("--output-columns"));
    assert!(help.contains("--progress"));
    assert!(help.contains("--verbose"));
}

#[test]
fn parse_datasource_list_supports_output_format_json() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-utils",
        "list",
        "--output-format",
        "json",
    ]);

    match args.command {
        super::DatasourceGroupCommand::List(inner) => {
            assert!(inner.json);
            assert!(!inner.table);
            assert!(!inner.csv);
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn resolve_match_marks_multiple_name_matches_as_ambiguous() {
    let record = DatasourceImportRecord {
        uid: String::new(),
        name: "Prometheus Main".to_string(),
        datasource_type: "prometheus".to_string(),
        access: "proxy".to_string(),
        url: "http://prometheus:9090".to_string(),
        is_default: true,
        org_id: "1".to_string(),
    };
    let live = vec![
        live_datasource(1, "prom-a", "Prometheus Main", "prometheus"),
        live_datasource(2, "prom-b", "Prometheus Main", "prometheus"),
    ];

    let matching = resolve_match(&record, &live, false, false);

    assert_eq!(matching.destination, "ambiguous");
    assert_eq!(matching.action, "would-fail-ambiguous");
    assert_eq!(matching.target_name, "Prometheus Main");
    assert_eq!(matching.target_id, None);
}

#[test]
fn resolve_match_allows_update_when_uid_exists_and_replace_existing_is_enabled() {
    let record = DatasourceImportRecord {
        uid: "prom-main".to_string(),
        name: "Prometheus Main".to_string(),
        datasource_type: "prometheus".to_string(),
        access: "proxy".to_string(),
        url: "http://prometheus:9090".to_string(),
        is_default: true,
        org_id: "1".to_string(),
    };
    let live = vec![live_datasource(
        9,
        "prom-main",
        "Prometheus Main",
        "prometheus",
    )];

    let matching = resolve_match(&record, &live, true, false);

    assert_eq!(matching.destination, "exists-uid");
    assert_eq!(matching.action, "would-update");
    assert_eq!(matching.target_uid, "prom-main");
    assert_eq!(matching.target_id, Some(9));
}

#[test]
fn render_import_table_can_omit_header() {
    let rows = vec![vec![
        "prom-main".to_string(),
        "Prometheus Main".to_string(),
        "prometheus".to_string(),
        "exists-uid".to_string(),
        "would-update".to_string(),
        "7".to_string(),
        "datasources.json#0".to_string(),
    ]];

    let lines = render_import_table(&rows, false, None);

    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("prom-main"));
    assert!(!lines[0].contains("UID"));
}

#[test]
fn render_import_table_honors_selected_columns() {
    let rows = vec![vec![
        "prom-main".to_string(),
        "Prometheus Main".to_string(),
        "prometheus".to_string(),
        "exists-uid".to_string(),
        "would-update".to_string(),
        "7".to_string(),
        "datasources.json#0".to_string(),
    ]];

    let lines = render_import_table(
        &rows,
        true,
        Some(&[
            "uid".to_string(),
            "action".to_string(),
            "org_id".to_string(),
        ]),
    );

    assert!(lines[0].contains("UID"));
    assert!(lines[0].contains("ACTION"));
    assert!(lines[0].contains("ORG_ID"));
    assert!(!lines[0].contains("NAME"));
    assert!(lines[2].contains("prom-main"));
    assert!(lines[2].contains("would-update"));
    assert!(lines[2].contains("7"));
}

#[test]
fn parse_datasource_import_preserves_requested_path() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-utils",
        "import",
        "--import-dir",
        "./datasources",
        "--org-id",
        "7",
        "--dry-run",
        "--table",
    ]);

    match args.command {
        super::DatasourceGroupCommand::Import(inner) => {
            assert_eq!(inner.import_dir, Path::new("./datasources"));
            assert_eq!(inner.org_id, Some(7));
            assert!(inner.dry_run);
            assert!(inner.table);
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_import_supports_output_format_table() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-utils",
        "import",
        "--import-dir",
        "./datasources",
        "--dry-run",
        "--output-format",
        "table",
    ]);

    match args.command {
        super::DatasourceGroupCommand::Import(inner) => {
            assert!(inner.dry_run);
            assert!(inner.table);
            assert!(!inner.json);
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_import_supports_output_columns() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-utils",
        "import",
        "--import-dir",
        "./datasources",
        "--dry-run",
        "--output-format",
        "table",
        "--output-columns",
        "uid,action,orgId,file",
    ]);

    match args.command {
        super::DatasourceGroupCommand::Import(inner) => {
            assert!(inner.table);
            assert_eq!(
                inner.output_columns,
                vec!["uid", "action", "org_id", "file"]
            );
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn build_import_payload_matches_shared_contract_fixtures() {
    for case in load_contract_cases() {
        let object = case.as_object().unwrap();
        let normalized = object
            .get("expectedNormalizedRecord")
            .and_then(Value::as_object)
            .unwrap();
        let expected_payload = object.get("expectedImportPayload").cloned().unwrap();
        let record = DatasourceImportRecord {
            uid: normalized
                .get("uid")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
            name: normalized
                .get("name")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
            datasource_type: normalized
                .get("type")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
            access: normalized
                .get("access")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
            url: normalized
                .get("url")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
            is_default: normalized.get("isDefault").and_then(Value::as_str).unwrap() == "true",
            org_id: normalized
                .get("orgId")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
        };

        assert_eq!(build_import_payload(&record), expected_payload);
    }
}

#[test]
fn datasource_import_rejects_output_columns_without_table_output() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("datasources");
    fs::create_dir_all(&import_dir).unwrap();
    fs::write(
        import_dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([])).unwrap(),
    )
    .unwrap();

    let error = run_datasource_cli(
        DatasourceCliArgs::parse_normalized_from([
            "grafana-utils",
            "import",
            "--import-dir",
            import_dir.to_str().unwrap(),
            "--dry-run",
            "--output-columns",
            "uid",
        ])
        .command,
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("--output-columns is only supported with --dry-run --table"));
}

#[test]
fn datasource_import_rejects_extra_secret_or_server_managed_fields() {
    let temp = tempdir().unwrap();
    let import_dir = temp.path().join("datasources");
    fs::create_dir_all(&import_dir).unwrap();
    fs::write(
        import_dir.join("export-metadata.json"),
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "root",
            "resource": "datasource",
            "datasourcesFile": "datasources.json",
            "indexFile": "index.json",
            "datasourceCount": 1,
            "format": "grafana-datasource-inventory-v1"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        import_dir.join("datasources.json"),
        serde_json::to_string_pretty(&json!([{
            "uid": "prom-main",
            "name": "Prometheus Main",
            "type": "prometheus",
            "access": "proxy",
            "url": "http://prometheus:9090",
            "isDefault": true,
            "org": "Main Org.",
            "orgId": "1",
            "id": 7,
            "secureJsonData": {"httpHeaderValue1": "secret"}
        }]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        import_dir.join("index.json"),
        serde_json::to_string_pretty(&json!({"items": []})).unwrap(),
    )
    .unwrap();

    let error = load_import_records(&import_dir).unwrap_err();

    assert!(error
        .to_string()
        .contains("unsupported datasource field(s): id, secureJsonData"));
}

#[test]
fn diff_help_explains_diff_dir_flag() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("diff")
        .unwrap_or_else(|| panic!("missing datasource diff help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--diff-dir"));
    assert!(help.contains("Compare datasource inventory"));
}

#[test]
fn parse_datasource_diff_preserves_requested_path() {
    let args =
        DatasourceCliArgs::parse_from(["grafana-utils", "diff", "--diff-dir", "./datasources"]);

    match args.command {
        super::DatasourceGroupCommand::Diff(inner) => {
            assert_eq!(inner.diff_dir, Path::new("./datasources"));
        }
        _ => panic!("expected datasource diff"),
    }
}

#[test]
fn diff_datasources_with_live_returns_zero_for_matching_inventory() {
    let diff_dir = write_diff_fixture(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": "1"
    })]);
    let live = vec![json!({
        "id": 7,
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": "1"
    })
    .as_object()
    .unwrap()
    .clone()];

    let (compared_count, differences) = diff_datasources_with_live(&diff_dir, &live).unwrap();

    assert_eq!(compared_count, 1);
    assert_eq!(differences, 0);
    fs::remove_dir_all(diff_dir).unwrap();
}

#[test]
fn diff_datasources_with_live_detects_changed_inventory() {
    let diff_dir = write_diff_fixture(&[json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": "1"
    })]);
    let live = vec![json!({
        "id": 7,
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "direct",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "orgId": "1"
    })
    .as_object()
    .unwrap()
    .clone()];

    let (compared_count, differences) = diff_datasources_with_live(&diff_dir, &live).unwrap();

    assert_eq!(compared_count, 1);
    assert_eq!(differences, 1);
    fs::remove_dir_all(diff_dir).unwrap();
}

fn write_diff_fixture(records: &[Value]) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("grafana-utils-datasource-diff-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("export-metadata.json"),
        serde_json::to_vec_pretty(&json!({
            "schemaVersion": 1,
            "kind": "grafana-utils-datasource-export-index",
            "variant": "root",
            "resource": "datasource",
            "datasourcesFile": "datasources.json",
            "indexFile": "index.json",
            "datasourceCount": records.len(),
            "format": "grafana-datasource-inventory-v1"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("datasources.json"),
        serde_json::to_vec_pretty(&Value::Array(records.to_vec())).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("index.json"),
        serde_json::to_vec_pretty(&json!({"items": []})).unwrap(),
    )
    .unwrap();
    dir
}
