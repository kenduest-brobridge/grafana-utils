use super::{render_import_table, resolve_match, DatasourceCliArgs, DatasourceImportRecord};
use clap::{CommandFactory, Parser};
use serde_json::{json, Value};
use std::path::Path;

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
    assert!(help.contains("--progress"));
    assert!(help.contains("--verbose"));
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

    let lines = render_import_table(&rows, false);

    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("prom-main"));
    assert!(!lines[0].contains("UID"));
}

#[test]
fn parse_datasource_import_preserves_requested_path() {
    let args = DatasourceCliArgs::parse_from([
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
