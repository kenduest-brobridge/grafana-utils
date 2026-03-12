use super::{
    attach_dashboard_folder_paths_with_request, build_export_metadata, build_export_variant_dirs,
    build_external_export_document, build_folder_inventory_status, build_folder_path,
    build_import_payload, build_output_path, build_preserved_web_import_document,
    build_root_export_index, diff_dashboards_with_request, discover_dashboard_files,
    export_dashboards_with_request, format_dashboard_summary_line, format_data_source_line,
    format_export_progress_line, format_export_verbose_line, format_folder_inventory_status_line,
    format_import_progress_line, format_import_verbose_line, import_dashboards_with_request,
    list_dashboards_with_request, list_data_sources_with_request, parse_cli_from,
    render_dashboard_summary_csv, render_dashboard_summary_json, render_dashboard_summary_table,
    render_data_source_csv, render_data_source_json, render_data_source_table, CommonCliArgs,
    DashboardCliArgs, DashboardCommand, DiffArgs, ExportArgs, FolderInventoryStatusKind,
    ImportArgs, ListArgs, ListDataSourcesArgs, EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME,
    TOOL_SCHEMA_VERSION,
};
use crate::common::api_response;
use clap::{CommandFactory, Parser};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn make_common_args(base_url: String) -> CommonCliArgs {
    CommonCliArgs {
        url: base_url,
        api_token: Some("token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        timeout: 30,
        verify_ssl: false,
    }
}

fn render_dashboard_subcommand_help(name: &str) -> String {
    let mut command = DashboardCliArgs::command();
    let subcommand = command
        .find_subcommand_mut(name)
        .unwrap_or_else(|| panic!("missing subcommand {name}"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn render_dashboard_help() -> String {
    let mut command = DashboardCliArgs::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn build_export_metadata_serializes_expected_shape() {
    let value = serde_json::to_value(build_export_metadata(
        "raw",
        2,
        Some("grafana-web-import-preserve-uid"),
        Some(FOLDER_INVENTORY_FILENAME),
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
            "foldersFile": "folders.json"
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
    let mut item = super::build_dashboard_index_item(&summary, "cpu-main");
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

    let folders = super::collect_folder_inventory_with_request(
        |_method, path, _params, _payload| match path {
            "/api/folders/infra" => Ok(Some(json!({
                "uid": "infra",
                "title": "Infra",
                "parents": [
                    {"uid": "platform", "title": "Platform"},
                    {"uid": "team", "title": "Team"}
                ]
            }))),
            _ => Err(super::message(format!("unexpected path {path}"))),
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
fn parse_cli_supports_list_mode() {
    let args = parse_cli_from([
        "grafana-utils",
        "list",
        "--url",
        "https://grafana.example.com",
        "--page-size",
        "25",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.common.url, "https://grafana.example.com");
            assert_eq!(list_args.page_size, 25);
            assert_eq!(list_args.org_id, None);
            assert!(!list_args.all_orgs);
            assert!(!list_args.with_sources);
            assert!(!list_args.table);
            assert!(!list_args.csv);
            assert!(!list_args.json);
            assert!(!list_args.no_header);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_supports_list_with_sources() {
    let args = parse_cli_from([
        "grafana-utils",
        "list",
        "--url",
        "https://grafana.example.com",
        "--with-sources",
        "--json",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, None);
            assert!(!list_args.all_orgs);
            assert!(list_args.with_sources);
            assert!(list_args.json);
            assert!(!list_args.table);
            assert!(!list_args.csv);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_supports_list_data_sources_mode() {
    let args = parse_cli_from([
        "grafana-utils",
        "list-data-sources",
        "--url",
        "https://grafana.example.com",
        "--table",
    ]);

    match args.command {
        DashboardCommand::ListDataSources(list_args) => {
            assert_eq!(list_args.common.url, "https://grafana.example.com");
            assert!(list_args.table);
            assert!(!list_args.csv);
            assert!(!list_args.json);
            assert!(!list_args.no_header);
        }
        _ => panic!("expected list-data-sources command"),
    }
}

#[test]
fn parse_cli_supports_preferred_auth_aliases() {
    let args = parse_cli_from([
        "grafana-utils",
        "export",
        "--token",
        "abc123",
        "--basic-user",
        "user",
        "--basic-password",
        "pass",
    ]);

    match args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(export_args.common.api_token.as_deref(), Some("abc123"));
            assert_eq!(export_args.common.username.as_deref(), Some("user"));
            assert_eq!(export_args.common.password.as_deref(), Some("pass"));
            assert!(!export_args.common.prompt_password);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_supports_prompt_password() {
    let args = parse_cli_from([
        "grafana-utils",
        "export",
        "--basic-user",
        "user",
        "--prompt-password",
    ]);

    match args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(export_args.common.username.as_deref(), Some("user"));
            assert_eq!(export_args.common.password.as_deref(), None);
            assert!(export_args.common.prompt_password);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_supports_export_org_scope_flags() {
    let org_args = parse_cli_from(["grafana-utils", "export", "--org-id", "7"]);
    let all_orgs_args = parse_cli_from(["grafana-utils", "export", "--all-orgs"]);

    match org_args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(export_args.org_id, Some(7));
            assert!(!export_args.all_orgs);
        }
        _ => panic!("expected export command"),
    }

    match all_orgs_args.command {
        DashboardCommand::Export(export_args) => {
            assert_eq!(export_args.org_id, None);
            assert!(export_args.all_orgs);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_rejects_conflicting_export_org_scope_flags() {
    let error = DashboardCliArgs::try_parse_from([
        "grafana-utils",
        "export",
        "--org-id",
        "7",
        "--all-orgs",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("--org-id"));
    assert!(error.to_string().contains("--all-orgs"));
}

#[test]
fn export_help_explains_flat_layout() {
    let help = render_dashboard_subcommand_help("export");
    assert!(help.contains("Write dashboard files directly into each export variant directory"));
    assert!(help.contains("folder-based subdirectories on disk"));
}

#[test]
fn export_help_describes_progress_and_verbose_modes() {
    let help = render_dashboard_subcommand_help("export");
    assert!(help.contains("--progress"));
    assert!(help.contains("<current>/<total>"));
    assert!(help.contains("-v, --verbose"));
    assert!(help.contains("Overrides --progress output"));
}

#[test]
fn import_help_explains_common_operator_flags() {
    let help = render_dashboard_subcommand_help("import");
    assert!(help.contains("not the combined export root"));
    assert!(help.contains("folder missing/match/mismatch state"));
    assert!(help.contains("skipped/blocked"));
    assert!(help.contains("folder check is also shown in table form"));
}

#[test]
fn top_level_help_includes_examples() {
    let help = render_dashboard_help();
    assert!(help.contains("Export dashboards from local Grafana with Basic auth"));
    assert!(help.contains("Export dashboards with an API token"));
    assert!(help.contains("grafana-utils export"));
    assert!(help.contains("grafana-utils diff"));
}

#[test]
fn parse_cli_supports_list_csv_mode() {
    let args = parse_cli_from([
        "grafana-utils",
        "list",
        "--url",
        "https://grafana.example.com",
        "--csv",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, None);
            assert!(!list_args.all_orgs);
            assert!(!list_args.table);
            assert!(list_args.csv);
            assert!(!list_args.json);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_supports_export_progress_and_verbose_flags() {
    let args = parse_cli_from(["grafana-utils", "export", "--progress", "--verbose"]);

    match args.command {
        DashboardCommand::Export(export_args) => {
            assert!(export_args.progress);
            assert!(export_args.verbose);
        }
        _ => panic!("expected export command"),
    }
}

#[test]
fn parse_cli_supports_import_progress_and_verbose_flags() {
    let args = parse_cli_from([
        "grafana-utils",
        "import",
        "--import-dir",
        "./dashboards/raw",
        "--progress",
        "-v",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.progress);
            assert!(import_args.verbose);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_dry_run_json_flag() {
    let args = parse_cli_from([
        "grafana-utils",
        "import",
        "--import-dir",
        "./dashboards/raw",
        "--dry-run",
        "--json",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.dry_run);
            assert!(import_args.json);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_import_update_existing_only_flag() {
    let args = parse_cli_from([
        "grafana-utils",
        "import",
        "--import-dir",
        "./dashboards/raw",
        "--update-existing-only",
    ]);

    match args.command {
        DashboardCommand::Import(import_args) => {
            assert!(import_args.update_existing_only);
            assert!(!import_args.replace_existing);
        }
        _ => panic!("expected import command"),
    }
}

#[test]
fn parse_cli_supports_list_json_mode() {
    let args = parse_cli_from([
        "grafana-utils",
        "list",
        "--url",
        "https://grafana.example.com",
        "--json",
    ]);

    match args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, None);
            assert!(!list_args.all_orgs);
            assert!(!list_args.table);
            assert!(!list_args.csv);
            assert!(list_args.json);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_rejects_conflicting_list_output_modes() {
    let error = DashboardCliArgs::try_parse_from([
        "grafana-utils",
        "list",
        "--url",
        "https://grafana.example.com",
        "--table",
        "--json",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("--table"));
    assert!(error.to_string().contains("--json"));
}

#[test]
fn parse_cli_supports_list_org_scope_flags() {
    let org_args = parse_cli_from(["grafana-utils", "list", "--org-id", "7"]);
    let all_orgs_args = parse_cli_from(["grafana-utils", "list", "--all-orgs"]);

    match org_args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, Some(7));
            assert!(!list_args.all_orgs);
        }
        _ => panic!("expected list command"),
    }

    match all_orgs_args.command {
        DashboardCommand::List(list_args) => {
            assert_eq!(list_args.org_id, None);
            assert!(list_args.all_orgs);
        }
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_rejects_conflicting_list_org_scope_flags() {
    let error =
        DashboardCliArgs::try_parse_from(["grafana-utils", "list", "--org-id", "7", "--all-orgs"])
            .unwrap_err();

    assert!(error.to_string().contains("--org-id"));
    assert!(error.to_string().contains("--all-orgs"));
}

#[test]
fn parse_cli_supports_legacy_list_alias() {
    let args = parse_cli_from(["grafana-utils", "list-dashboard", "--json"]);

    match args.command {
        DashboardCommand::List(list_args) => assert!(list_args.json),
        _ => panic!("expected list command"),
    }
}

#[test]
fn parse_cli_rejects_conflicting_list_data_sources_output_modes() {
    let error = DashboardCliArgs::try_parse_from([
        "grafana-utils",
        "list-data-sources",
        "--table",
        "--json",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("--table"));
    assert!(error.to_string().contains("--json"));
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
    let folder = super::FolderInventoryItem {
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
    let folder = super::FolderInventoryItem {
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
    let folder = super::FolderInventoryItem {
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

    let with_header = super::render_folder_inventory_dry_run_table(&rows, true);

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
        format_import_progress_line(3, 7, "cpu-main", true, Some("would-update"), Some("General")),
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
            "/tmp/a.json".to_string(),
        ],
        [
            "xyz".to_string(),
            "missing".to_string(),
            "create".to_string(),
            "Platform / Infra".to_string(),
            "/tmp/b.json".to_string(),
        ],
    ];
    let with_header = super::render_import_dry_run_table(&rows, true);
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
    let without_header = super::render_import_dry_run_table(&rows, false);
    assert_eq!(without_header.len(), 2);
    assert!(without_header[0].contains("abc"));
    assert!(without_header[0].contains("exists"));
    assert!(without_header[0].contains("update"));
    assert!(without_header[0].contains("General"));
    assert!(without_header[0].contains("/tmp/a.json"));
}

#[test]
fn render_import_dry_run_json_returns_structured_document() {
    let folder_status = super::FolderInventoryStatus {
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
        "/tmp/a.json".to_string(),
    ]];

    let value: Value = serde_json::from_str(
        &super::render_import_dry_run_json(
            "create-or-update",
            &[folder_status],
            &rows,
            Path::new("/tmp/raw"),
            0,
        )
        .unwrap(),
    )
    .unwrap();

    assert_eq!(value["mode"], "create-or-update");
    assert_eq!(value["folders"][0]["uid"], "infra");
    assert_eq!(value["dashboards"][0]["folderPath"], "Platform / Infra");
    assert_eq!(value["summary"]["dashboardCount"], 1);
}

#[test]
fn describe_dashboard_import_mode_uses_expected_labels() {
    assert_eq!(
        super::describe_dashboard_import_mode(false, false),
        "create-only"
    );
    assert_eq!(
        super::describe_dashboard_import_mode(true, false),
        "create-or-update"
    );
    assert_eq!(
        super::describe_dashboard_import_mode(false, true),
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
fn format_data_source_line_uses_expected_fields() {
    let datasource = json!({
        "uid": "prom_uid",
        "name": "Prometheus Main",
        "type": "prometheus",
        "url": "http://prometheus:9090",
        "isDefault": true
    });

    let line = format_data_source_line(datasource.as_object().unwrap());
    assert_eq!(
        line,
        "uid=prom_uid name=Prometheus Main type=prometheus url=http://prometheus:9090 isDefault=true"
    );
}

#[test]
fn render_data_source_table_uses_headers_and_values() {
    let datasources = vec![
        json!({
            "uid": "prom_uid",
            "name": "Prometheus Main",
            "type": "prometheus",
            "url": "http://prometheus:9090",
            "isDefault": true
        })
        .as_object()
        .unwrap()
        .clone(),
        json!({
            "uid": "loki_uid",
            "name": "Loki Logs",
            "type": "loki",
            "url": "http://loki:3100",
            "isDefault": false
        })
        .as_object()
        .unwrap()
        .clone(),
    ];

    let lines = render_data_source_table(&datasources, true);
    assert_eq!(
        lines[0],
        "UID       NAME             TYPE        URL                     IS_DEFAULT"
    );
    assert_eq!(
        lines[2],
        "prom_uid  Prometheus Main  prometheus  http://prometheus:9090  true      "
    );
    assert_eq!(
        lines[3],
        "loki_uid  Loki Logs        loki        http://loki:3100        false     "
    );
}

#[test]
fn render_data_source_csv_uses_expected_fields() {
    let datasources = vec![json!({
        "uid": "prom_uid",
        "name": "Prometheus Main",
        "type": "prometheus",
        "url": "http://prometheus:9090",
        "isDefault": true
    })
    .as_object()
    .unwrap()
    .clone()];

    let lines = render_data_source_csv(&datasources);
    assert_eq!(lines[0], "uid,name,type,url,isDefault");
    assert_eq!(
        lines[1],
        "prom_uid,Prometheus Main,prometheus,http://prometheus:9090,true"
    );
}

#[test]
fn render_data_source_json_uses_expected_fields() {
    let datasources = vec![json!({
        "uid": "prom_uid",
        "name": "Prometheus Main",
        "type": "prometheus",
        "url": "http://prometheus:9090",
        "isDefault": true
    })
    .as_object()
    .unwrap()
    .clone()];

    let value = render_data_source_json(&datasources);
    assert_eq!(
        value,
        json!([
            {
                "uid": "prom_uid",
                "name": "Prometheus Main",
                "type": "prometheus",
                "url": "http://prometheus:9090",
                "isDefault": "true"
            }
        ])
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

    let lines = render_dashboard_summary_table(&summaries, true);
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

    let lines = render_dashboard_summary_table(&summaries, true);
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

    let lines = render_dashboard_summary_table(&summaries, false);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("abc"));
}

#[test]
fn render_data_source_table_can_omit_header() {
    let datasources = vec![json!({
        "uid": "prom_uid",
        "name": "Prometheus Main",
        "type": "prometheus",
        "url": "http://prometheus:9090",
        "isDefault": true
    })
    .as_object()
    .unwrap()
    .clone()];

    let lines = render_data_source_table(&datasources, false);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("prom_uid"));
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

    let lines = render_dashboard_summary_csv(&summaries);
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

    let lines = render_dashboard_summary_csv(&summaries);
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

    let value = render_dashboard_summary_json(&summaries);
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

    let value = render_dashboard_summary_json(&summaries);
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
            _ => Err(super::message(format!("unexpected path {path}"))),
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
        table: false,
        csv: false,
        json: false,
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
                _ => Err(super::message(format!("unexpected path {path}"))),
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
    let catalog = super::build_datasource_catalog(&vec![
        json!({"uid": "prom_uid", "name": "Prom Main", "type": "prometheus"})
            .as_object()
            .unwrap()
            .clone(),
        json!({"uid": "loki_uid", "name": "Loki Logs", "type": "loki"})
            .as_object()
            .unwrap()
            .clone(),
    ]);

    let (sources, source_uids) =
        super::collect_dashboard_source_metadata(&payload, &catalog).unwrap();
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
fn list_dashboards_with_request_with_sources_fetches_dashboards_and_datasources() {
    let args = ListArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        with_sources: true,
        table: false,
        csv: false,
        json: true,
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
                _ => Err(super::message(format!("unexpected path {path}"))),
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
fn list_dashboards_with_request_with_org_id_scopes_requests() {
    let args = ListArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        page_size: 500,
        org_id: Some(7),
        all_orgs: false,
        with_sources: false,
        table: false,
        csv: false,
        json: true,
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
                _ => Err(super::message(format!("unexpected path {path}"))),
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
}

#[test]
fn list_dashboards_with_request_all_orgs_aggregates_results() {
    let args = ListArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        page_size: 500,
        org_id: None,
        all_orgs: true,
        with_sources: false,
        table: false,
        csv: false,
        json: true,
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
                ("/api/search", Some("2")) => Ok(Some(json!([
                    {"uid": "xyz", "title": "Logs", "folderTitle": "Ops", "folderUid": "ops"}
                ]))),
                ("/api/folders/infra", Some("1")) => Ok(Some(json!({
                    "title": "Infra",
                    "parents": [{"title": "Platform"}]
                }))),
                ("/api/folders/ops", Some("2")) => Ok(Some(json!({
                    "title": "Ops",
                    "parents": [{"title": "Platform"}]
                }))),
                _ => Err(super::message(format!("unexpected path {path}"))),
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
}

#[test]
fn list_data_sources_with_request_returns_count() {
    let args = ListDataSourcesArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        table: false,
        csv: true,
        json: false,
        no_header: false,
    };

    let count = list_data_sources_with_request(
        |_method, path, _params, _payload| match path {
            "/api/datasources" => Ok(Some(json!([
                {
                    "uid": "prom_uid",
                    "name": "Prometheus Main",
                    "type": "prometheus",
                    "url": "http://prometheus:9090",
                    "isDefault": true
                },
                {
                    "uid": "loki_uid",
                    "name": "Loki Logs",
                    "type": "loki",
                    "url": "http://loki:3100",
                    "isDefault": false
                }
            ]))),
            _ => Err(super::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 2);
}

#[test]
fn export_dashboards_with_client_writes_raw_variant_and_indexes() {
    let temp = tempdir().unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        export_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        flat: false,
        overwrite: true,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        dry_run: false,
        progress: false,
        verbose: false,
    };
    let mut calls = Vec::new();
    let count = export_dashboards_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            if path == "/api/org" {
                return Ok(Some(json!({"id": 1, "name": "Main Org."})));
            }
            if path == "/api/search" {
                return Ok(Some(
                    json!([{ "uid": "abc", "title": "CPU", "folderTitle": "Infra" }]),
                ));
            }
            if path == "/api/dashboards/uid/abc" {
                return Ok(Some(
                    json!({"dashboard": {"id": 7, "uid": "abc", "title": "CPU"}}),
                ));
            }
            Err(super::message(format!("unexpected path {path}")))
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert!(args.export_dir.join("raw/Infra/CPU__abc.json").is_file());
    assert!(args.export_dir.join("raw/index.json").is_file());
    assert!(args.export_dir.join("raw/export-metadata.json").is_file());
    assert!(args.export_dir.join("index.json").is_file());
    assert!(args.export_dir.join("export-metadata.json").is_file());
    assert_eq!(calls.len(), 3);
}

#[test]
fn export_dashboards_with_request_with_org_id_scopes_requests() {
    let temp = tempdir().unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        export_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: Some(7),
        all_orgs: false,
        flat: false,
        overwrite: true,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        dry_run: false,
        progress: false,
        verbose: false,
    };
    let mut calls = Vec::new();

    let count = export_dashboards_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            let scoped_org = params
                .iter()
                .find(|(key, _)| key == "orgId")
                .map(|(_, value)| value.as_str());
            match (path, scoped_org) {
                ("/api/org", Some("7")) => Ok(Some(json!({"id": 7, "name": "Scoped Org"}))),
                ("/api/search", Some("7")) => Ok(Some(
                    json!([{ "uid": "abc", "title": "CPU", "folderTitle": "Infra" }]),
                )),
                ("/api/dashboards/uid/abc", Some("7")) => Ok(Some(
                    json!({"dashboard": {"id": 7, "uid": "abc", "title": "CPU"}}),
                )),
                _ => Err(super::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert!(args.export_dir.join("raw/Infra/CPU__abc.json").is_file());
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params, _)| path == "/api/search"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "7"))
            .count(),
        1
    );
}

#[test]
fn build_external_export_document_adds_datasource_inputs() {
    let payload = json!({
        "dashboard": {
            "id": 9,
            "title": "Infra",
            "panels": [
                {
                    "type": "timeseries",
                    "datasource": {"type": "prometheus", "uid": "prom_uid"},
                    "targets": [
                        {
                            "datasource": {"type": "prometheus", "uid": "prom_uid"},
                            "expr": "up"
                        }
                    ]
                },
                {
                    "type": "stat",
                    "datasource": "Loki Logs"
                }
            ]
        }
    });
    let catalog = super::build_datasource_catalog(&vec![
        json!({"uid": "prom_uid", "name": "Prom Main", "type": "prometheus"})
            .as_object()
            .unwrap()
            .clone(),
        json!({"uid": "loki_uid", "name": "Loki Logs", "type": "loki"})
            .as_object()
            .unwrap()
            .clone(),
    ]);

    let document = build_external_export_document(&payload, &catalog).unwrap();

    assert_eq!(
        document["panels"][0]["datasource"]["uid"],
        "${DS_PROM_MAIN}"
    );
    assert_eq!(
        document["panels"][0]["targets"][0]["datasource"]["uid"],
        "${DS_PROM_MAIN}"
    );
    assert_eq!(document["panels"][1]["datasource"], "${DS_LOKI_LOGS}");
    assert_eq!(document["__inputs"][0]["name"], "DS_LOKI_LOGS");
    assert_eq!(document["__inputs"][1]["name"], "DS_PROM_MAIN");
    assert_eq!(document["__inputs"][0]["label"], "Loki Logs");
    assert_eq!(document["__inputs"][1]["label"], "Prom Main");
    assert_eq!(document["__inputs"][0]["pluginName"], "Loki");
    assert_eq!(document["__inputs"][1]["pluginName"], "Prometheus");
    assert_eq!(document["__elements"], json!({}));
}

#[test]
fn build_external_export_document_creates_input_from_datasource_template_variable() {
    let payload = json!({
        "dashboard": {
            "id": 15,
            "title": "Prometheus / Overview",
            "templating": {
                "list": [
                    {
                        "current": {"text": "default", "value": "default"},
                        "hide": 0,
                        "label": "Data source",
                        "name": "datasource",
                        "options": [],
                        "query": "prometheus",
                        "refresh": 1,
                        "regex": "",
                        "type": "datasource"
                    },
                    {
                        "allValue": ".+",
                        "current": {"selected": true, "text": "All", "value": "$__all"},
                        "datasource": "$datasource",
                        "includeAll": true,
                        "label": "job",
                        "multi": true,
                        "name": "job",
                        "options": [],
                        "query": "label_values(prometheus_build_info, job)",
                        "refresh": 1,
                        "regex": "",
                        "sort": 2,
                        "type": "query"
                    }
                ]
            },
            "panels": [
                {
                    "type": "timeseries",
                    "datasource": "$datasource",
                    "targets": [{"refId": "A", "expr": "up"}]
                }
            ]
        }
    });

    let document =
        build_external_export_document(&payload, &(BTreeMap::new(), BTreeMap::new())).unwrap();
    assert_eq!(document["__inputs"][0]["name"], "DS_PROMETHEUS");
    assert_eq!(document["templating"]["list"][0]["current"], json!({}));
    assert_eq!(document["templating"]["list"][0]["query"], "prometheus");
    assert_eq!(
        document["templating"]["list"][1]["datasource"]["uid"],
        "${DS_PROMETHEUS}"
    );
    assert_eq!(document["panels"][0]["datasource"]["uid"], "$datasource");
}

#[test]
fn export_dashboards_with_client_writes_prompt_variant_and_indexes() {
    let temp = tempdir().unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        export_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        flat: false,
        overwrite: true,
        without_dashboard_raw: false,
        without_dashboard_prompt: false,
        dry_run: false,
        progress: false,
        verbose: false,
    };

    let count = export_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/datasources" => Ok(Some(json!([
                {"uid": "prom_uid", "name": "Prom Main", "type": "prometheus"}
            ]))),
            "/api/org" => Ok(Some(json!({"id": 1, "name": "Main Org."}))),
            "/api/search" => Ok(Some(json!([{ "uid": "abc", "title": "CPU", "folderTitle": "Infra" }]))),
            "/api/dashboards/uid/abc" => Ok(Some(json!({
                "dashboard": {
                    "id": 7,
                    "uid": "abc",
                    "title": "CPU",
                    "panels": [
                        {"type": "timeseries", "datasource": {"type": "prometheus", "uid": "prom_uid"}}
                    ]
                }
            }))),
            _ => Err(super::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert!(args.export_dir.join("prompt/Infra/CPU__abc.json").is_file());
    assert!(args.export_dir.join("prompt/index.json").is_file());
    assert!(args
        .export_dir
        .join("prompt/export-metadata.json")
        .is_file());
}

#[test]
fn export_dashboards_with_request_all_orgs_aggregates_results() {
    let temp = tempdir().unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        export_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: true,
        flat: false,
        overwrite: true,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        dry_run: false,
        progress: false,
        verbose: false,
    };
    let mut calls = Vec::new();

    let count = export_dashboards_with_request(
        |method, path, params, payload| {
            calls.push((
                method.to_string(),
                path.to_string(),
                params.to_vec(),
                payload.cloned(),
            ));
            let scoped_org = params
                .iter()
                .find(|(key, _)| key == "orgId")
                .map(|(_, value)| value.as_str());
            match (path, scoped_org) {
                ("/api/orgs", None) => Ok(Some(json!([
                    {"id": 1, "name": "Main Org"},
                    {"id": 2, "name": "Ops Org"}
                ]))),
                ("/api/org", Some("1")) => Ok(Some(json!({"id": 1, "name": "Main Org"}))),
                ("/api/org", Some("2")) => Ok(Some(json!({"id": 2, "name": "Ops Org"}))),
                ("/api/search", Some("1")) => Ok(Some(
                    json!([{ "uid": "abc", "title": "CPU", "folderTitle": "Infra" }]),
                )),
                ("/api/search", Some("2")) => Ok(Some(
                    json!([{ "uid": "xyz", "title": "Logs", "folderTitle": "Ops" }]),
                )),
                ("/api/dashboards/uid/abc", Some("1")) => Ok(Some(
                    json!({"dashboard": {"id": 7, "uid": "abc", "title": "CPU"}}),
                )),
                ("/api/dashboards/uid/xyz", Some("2")) => Ok(Some(
                    json!({"dashboard": {"id": 8, "uid": "xyz", "title": "Logs"}}),
                )),
                _ => Err(super::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 2);
    assert!(args
        .export_dir
        .join("org_1_Main_Org/raw/Infra/CPU__abc.json")
        .is_file());
    assert!(args
        .export_dir
        .join("org_2_Ops_Org/raw/Ops/Logs__xyz.json")
        .is_file());
    assert!(args.export_dir.join("raw/index.json").is_file());
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, _, _)| path == "/api/orgs")
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params, _)| path == "/api/search"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "1"))
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|(_, path, params, _)| path == "/api/search"
                && params
                    .iter()
                    .any(|(key, value)| key == "orgId" && value == "2"))
            .count(),
        1
    );
}

#[test]
fn export_dashboards_with_dry_run_keeps_output_dir_empty() {
    let temp = tempdir().unwrap();
    let args = ExportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        export_dir: temp.path().join("dashboards"),
        page_size: 500,
        org_id: None,
        all_orgs: false,
        flat: false,
        overwrite: true,
        without_dashboard_raw: false,
        without_dashboard_prompt: true,
        dry_run: true,
        progress: false,
        verbose: false,
    };

    let count = export_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/org" => Ok(Some(json!({"id": 1, "name": "Main Org."}))),
            "/api/search" => Ok(Some(
                json!([{ "uid": "abc", "title": "CPU", "folderTitle": "Infra" }]),
            )),
            "/api/dashboards/uid/abc" => Ok(Some(
                json!({"dashboard": {"id": 7, "uid": "abc", "title": "CPU"}}),
            )),
            _ => Err(super::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert!(!args.export_dir.exists());
}

#[test]
fn import_dashboards_with_client_imports_discovered_files() {
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
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        import_dir: raw_dir,
        import_folder_uid: Some("new-folder".to_string()),
        ensure_folders: false,
        replace_existing: true,
        update_existing_only: false,
        import_message: "sync dashboards".to_string(),
        dry_run: false,
        table: false,
        json: false,
        no_header: false,
        progress: false,
        verbose: false,
    };
    let mut posted_payloads = Vec::new();
    let count = import_dashboards_with_request(
        |_method, path, _params, payload| {
            assert_eq!(path, "/api/dashboards/db");
            posted_payloads.push(payload.cloned().unwrap());
            Ok(Some(json!({"status": "success"})))
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(posted_payloads.len(), 1);
    assert_eq!(posted_payloads[0]["folderUid"], "new-folder");
    assert_eq!(posted_payloads[0]["dashboard"]["id"], Value::Null);
}

#[test]
fn import_dashboards_with_dry_run_skips_post_requests() {
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
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        import_dir: raw_dir,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: true,
        update_existing_only: false,
        import_message: "sync dashboards".to_string(),
        dry_run: true,
        table: false,
        json: false,
        no_header: false,
        progress: false,
        verbose: false,
    };

    let count = import_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/dashboards/uid/abc" => Ok(Some(json!({
                "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
                "meta": {"folderUid": "old-folder"}
            }))),
            "/api/folders/old-folder" => Ok(None),
            "/api/dashboards/db" => Err(super::message("dry-run must not post dashboards")),
            _ => Err(super::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn import_dashboards_rejects_unsupported_export_schema_version() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION + 1,
            "variant": "raw",
            "dashboardCount": 0,
            "indexFile": "index.json"
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        import_dir: raw_dir,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: false,
        import_message: "sync dashboards".to_string(),
        dry_run: false,
        table: false,
        json: false,
        no_header: false,
        progress: false,
        verbose: false,
    };

    let error = import_dashboards_with_request(|_method, _path, _params, _payload| Ok(None), &args)
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("Unsupported dashboard export schemaVersion"));
}

#[test]
fn import_dashboards_with_update_existing_only_skips_missing_dashboards() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 2,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("exists.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU"}
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("missing.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 8, "uid": "xyz", "title": "Memory"}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        import_dir: raw_dir,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: true,
        import_message: "sync dashboards".to_string(),
        dry_run: false,
        table: false,
        json: false,
        no_header: false,
        progress: false,
        verbose: false,
    };
    let mut posted_payloads = Vec::new();
    let count = import_dashboards_with_request(
        |_method, path, _params, payload| match path {
            "/api/dashboards/uid/abc" => Ok(Some(json!({
                "dashboard": {"id": 7, "uid": "abc", "title": "CPU"}
            }))),
            "/api/dashboards/uid/xyz" => Err(api_response(
                404,
                "http://127.0.0.1:3000/api/dashboards/uid/xyz",
                "{\"message\":\"not found\"}",
            )),
            "/api/dashboards/db" => {
                posted_payloads.push(payload.cloned().unwrap());
                Ok(Some(json!({"status": "success"})))
            }
            _ => Err(super::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(posted_payloads.len(), 1);
    assert_eq!(posted_payloads[0]["dashboard"]["uid"], "abc");
    assert_eq!(posted_payloads[0]["overwrite"], true);
}

#[test]
fn import_dashboards_with_update_existing_only_table_marks_missing_dashboards_as_skipped() {
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
        raw_dir.join("missing.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 8, "uid": "xyz", "title": "Memory"}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        import_dir: raw_dir,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: true,
        import_message: "sync dashboards".to_string(),
        dry_run: true,
        table: true,
        json: false,
        no_header: true,
        progress: false,
        verbose: false,
    };

    let count = import_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/dashboards/uid/xyz" => Err(api_response(
                404,
                "http://127.0.0.1:3000/api/dashboards/uid/xyz",
                "{\"message\":\"not found\"}",
            )),
            "/api/dashboards/db" => Err(super::message("dry-run must not post dashboards")),
            _ => Err(super::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
}

#[test]
fn import_dashboards_replace_existing_preserves_destination_folder() {
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
        raw_dir.join("exists.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
            "meta": {"folderUid": "source-folder"}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        import_dir: raw_dir,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: true,
        update_existing_only: false,
        import_message: "sync dashboards".to_string(),
        dry_run: false,
        table: false,
        json: false,
        no_header: false,
        progress: false,
        verbose: false,
    };
    let mut posted_payloads = Vec::new();
    let count = import_dashboards_with_request(
        |_method, path, _params, payload| match path {
            "/api/dashboards/uid/abc" => Ok(Some(json!({
                "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
                "meta": {"folderUid": "dest-folder"}
            }))),
            "/api/dashboards/db" => {
                posted_payloads.push(payload.cloned().unwrap());
                Ok(Some(json!({"status": "success"})))
            }
            _ => Err(super::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(posted_payloads.len(), 1);
    assert_eq!(posted_payloads[0]["folderUid"], "dest-folder");
    assert_eq!(posted_payloads[0]["overwrite"], true);
}

#[test]
fn import_dashboards_rejects_ensure_folders_with_import_folder_override() {
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
            "meta": {"folderUid": "child"}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        import_dir: raw_dir,
        import_folder_uid: Some("override-folder".to_string()),
        ensure_folders: true,
        replace_existing: false,
        update_existing_only: false,
        import_message: "sync dashboards".to_string(),
        dry_run: false,
        table: false,
        json: false,
        no_header: false,
        progress: false,
        verbose: false,
    };

    let error = import_dashboards_with_request(|_method, _path, _params, _payload| Ok(None), &args)
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("--ensure-folders cannot be combined with --import-folder-uid"));
}

#[test]
fn import_dashboards_rejects_json_without_dry_run() {
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
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        import_dir: raw_dir,
        import_folder_uid: None,
        ensure_folders: false,
        replace_existing: false,
        update_existing_only: false,
        import_message: "sync dashboards".to_string(),
        dry_run: false,
        table: false,
        json: true,
        no_header: false,
        progress: false,
        verbose: false,
    };

    let error = import_dashboards_with_request(|_method, _path, _params, _payload| Ok(None), &args)
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("--json is only supported with --dry-run"));
}

#[test]
fn import_dashboards_with_ensure_folders_creates_missing_folder_chain_from_raw_inventory() {
    let temp = tempdir().unwrap();
    let root_dir = temp.path();
    let raw_dir = root_dir.join("raw");
    fs::create_dir_all(&raw_dir).unwrap();
    fs::write(
        raw_dir.join(EXPORT_METADATA_FILENAME),
        serde_json::to_string_pretty(&json!({
            "kind": "grafana-utils-dashboard-export-index",
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "variant": "raw",
            "dashboardCount": 1,
            "indexFile": "index.json",
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": "folders.json"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join(FOLDER_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "platform",
                "title": "Platform",
                "path": "Platform",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "child",
                "title": "Child",
                "path": "Platform / Child",
                "parentUid": "platform",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
            "meta": {"folderUid": "child"}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        import_dir: raw_dir,
        import_folder_uid: None,
        ensure_folders: true,
        replace_existing: false,
        update_existing_only: false,
        import_message: "sync dashboards".to_string(),
        dry_run: false,
        table: false,
        json: false,
        no_header: false,
        progress: false,
        verbose: false,
    };
    let mut calls = Vec::new();
    let mut posted_payloads = Vec::new();

    let count = import_dashboards_with_request(
        |method, path, _params, payload| {
            calls.push(format!("{} {}", method.as_str(), path));
            match (method, path) {
                (reqwest::Method::GET, "/api/dashboards/uid/abc") => Err(api_response(
                    404,
                    "http://127.0.0.1:3000/api/dashboards/uid/abc",
                    "{\"message\":\"not found\"}",
                )),
                (reqwest::Method::GET, "/api/folders/child") => Ok(None),
                (reqwest::Method::GET, "/api/folders/platform") => Ok(None),
                (reqwest::Method::POST, "/api/folders") => {
                    posted_payloads.push(payload.cloned().unwrap());
                    Ok(Some(json!({"status": "success"})))
                }
                (reqwest::Method::POST, "/api/dashboards/db") => {
                    posted_payloads.push(payload.cloned().unwrap());
                    Ok(Some(json!({"status": "success"})))
                }
                _ => Err(super::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(
        posted_payloads,
        vec![
            json!({"uid": "platform", "title": "Platform"}),
            json!({"uid": "child", "title": "Child", "parentUid": "platform"}),
            json!({
                "dashboard": {"id": null, "uid": "abc", "title": "CPU"},
                "overwrite": false,
                "message": "sync dashboards",
                "folderUid": "child"
            })
        ]
    );
    assert_eq!(
        calls,
        vec![
            "GET /api/dashboards/uid/abc",
            "GET /api/folders/child",
            "GET /api/folders/platform",
            "GET /api/folders/platform",
            "POST /api/folders",
            "GET /api/folders/child",
            "POST /api/folders",
            "POST /api/dashboards/db"
        ]
    );
}

#[test]
fn import_dashboards_with_dry_run_and_ensure_folders_checks_folder_inventory() {
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
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": "folders.json"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join(FOLDER_INVENTORY_FILENAME),
        serde_json::to_string_pretty(&json!([
            {
                "uid": "platform",
                "title": "Platform",
                "path": "Platform",
                "org": "Main Org.",
                "orgId": "1"
            },
            {
                "uid": "child",
                "title": "Child",
                "path": "Platform / Child",
                "parentUid": "platform",
                "org": "Main Org.",
                "orgId": "1"
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
            "meta": {"folderUid": "child"}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        import_dir: raw_dir,
        import_folder_uid: None,
        ensure_folders: true,
        replace_existing: false,
        update_existing_only: false,
        import_message: "sync dashboards".to_string(),
        dry_run: true,
        table: false,
        json: false,
        no_header: false,
        progress: false,
        verbose: false,
    };
    let mut calls = Vec::new();

    let count = import_dashboards_with_request(
        |method, path, _params, _payload| {
            calls.push(format!("{} {}", method.as_str(), path));
            match (method, path) {
                (reqwest::Method::GET, "/api/folders/platform") => Ok(Some(json!({
                    "uid": "platform",
                    "title": "Platform",
                    "parents": []
                }))),
                (reqwest::Method::GET, "/api/folders/child") => Ok(None),
                (reqwest::Method::GET, "/api/dashboards/uid/abc") => Err(api_response(
                    404,
                    "http://127.0.0.1:3000/api/dashboards/uid/abc",
                    "{\"message\":\"not found\"}",
                )),
                (reqwest::Method::POST, "/api/folders") => {
                    Err(super::message("dry-run must not create folders"))
                }
                (reqwest::Method::POST, "/api/dashboards/db") => {
                    Err(super::message("dry-run must not post dashboards"))
                }
                _ => Err(super::message(format!("unexpected path {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
    assert_eq!(
        calls,
        vec![
            "GET /api/folders/platform",
            "GET /api/folders/child",
            "GET /api/dashboards/uid/abc",
            "GET /api/folders/child"
        ]
    );
}

#[test]
fn import_dashboards_with_ensure_folders_requires_inventory_manifest() {
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
            "format": "grafana-web-import-preserve-uid",
            "foldersFile": "folders.json"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        raw_dir.join("dash.json"),
        serde_json::to_string_pretty(&json!({
            "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
            "meta": {"folderUid": "child"}
        }))
        .unwrap(),
    )
    .unwrap();
    let args = ImportArgs {
        common: make_common_args("http://127.0.0.1:3000".to_string()),
        import_dir: raw_dir,
        import_folder_uid: None,
        ensure_folders: true,
        replace_existing: false,
        update_existing_only: false,
        import_message: "sync dashboards".to_string(),
        dry_run: false,
        table: false,
        json: false,
        no_header: false,
        progress: false,
        verbose: false,
    };

    let error = import_dashboards_with_request(|_method, _path, _params, _payload| Ok(None), &args)
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("Folder inventory file not found for --ensure-folders"));
}

#[test]
fn collect_folder_inventory_statuses_with_request_reports_match_mismatch_and_missing() {
    let folders = vec![
        super::FolderInventoryItem {
            uid: "platform".to_string(),
            title: "Platform".to_string(),
            path: "Platform".to_string(),
            parent_uid: None,
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        },
        super::FolderInventoryItem {
            uid: "child".to_string(),
            title: "Child".to_string(),
            path: "Platform / Child".to_string(),
            parent_uid: Some("platform".to_string()),
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        },
        super::FolderInventoryItem {
            uid: "missing".to_string(),
            title: "Missing".to_string(),
            path: "Missing".to_string(),
            parent_uid: None,
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
        },
    ];

    let statuses = super::collect_folder_inventory_statuses_with_request(
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
            _ => Err(super::message(format!("unexpected path {path}"))),
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
        import_dir: raw_dir,
        import_folder_uid: Some("old-folder".to_string()),
        context_lines: 3,
    };

    let count = diff_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/dashboards/uid/abc" => Ok(Some(json!({
                "dashboard": {"id": 7, "uid": "abc", "title": "CPU"},
                "meta": {"folderUid": "old-folder"}
            }))),
            _ => Err(super::message(format!("unexpected path {path}"))),
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
        import_dir: raw_dir,
        import_folder_uid: None,
        context_lines: 3,
    };

    let count = diff_dashboards_with_request(
        |_method, path, _params, _payload| match path {
            "/api/dashboards/uid/abc" => Ok(Some(json!({
                "dashboard": {"id": 7, "uid": "abc", "title": "Memory"}
            }))),
            _ => Err(super::message(format!("unexpected path {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 1);
}
