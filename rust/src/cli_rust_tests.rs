//! Unified CLI test suite.
//! Focuses on canonical command routing and ensures handlers receive the expected
//! domain payload shapes.
use super::{
    dispatch_with_handlers, maybe_render_unified_help_from_os_args, parse_cli_from,
    render_unified_help_full_text, render_unified_help_text, render_unified_version_text, CliArgs,
    UnifiedCommand,
};
use crate::alert::{parse_cli_from as parse_alert_cli_from, root_command as alert_root_command};
use crate::common::{
    render_json_value_with_choice, CliColorChoice, TOOL_BUILD_TIME, TOOL_GIT_COMMIT, TOOL_VERSION,
    TOOL_VERSION_SCHEMA_VERSION, TOOL_VERSION_TEXT,
};
use crate::dashboard::{
    DashboardCommand, RawToPromptLogFormat, RawToPromptOutputFormat, RawToPromptResolution,
    SimpleOutputFormat,
};
use crate::datasource::DatasourceGroupCommand;
use crate::migrate::{MigrateCommand, MigrateDashboardCommand};
use crate::overview::OverviewOutputFormat;
use crate::profile_cli::{root_command as profile_root_command, ProfileCommand};
use crate::resource::{ResourceCliArgs, ResourceCommand, ResourceKind, ResourceOutputFormat};
use crate::snapshot::root_command as snapshot_root_command;
use crate::sync::{SyncAdvancedCommand, SyncGroupCommand, SyncOutputFormat, DEFAULT_REVIEW_TOKEN};
use clap::{CommandFactory, Parser};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn render_unified_help() -> String {
    render_unified_help_text(false)
}

fn render_unified_help_full() -> String {
    render_unified_help_full_text(false)
}

fn render_alert_subcommand_help(path: &[&str]) -> String {
    let mut command = alert_root_command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing alert subcommand help for {segment}"));
    }
    let mut output = Vec::new();
    current.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn render_profile_subcommand_help(path: &[&str]) -> String {
    let mut command = profile_root_command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing profile subcommand help for {segment}"));
    }
    let mut output = Vec::new();
    current.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn render_snapshot_subcommand_help(path: &[&str]) -> String {
    let mut command = snapshot_root_command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing snapshot subcommand help for {segment}"));
    }
    let mut output = Vec::new();
    current.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn render_resource_subcommand_help(path: &[&str]) -> String {
    let mut command = ResourceCliArgs::command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing resource subcommand help for {segment}"));
    }
    let mut output = Vec::new();
    current.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn load_shared_diff_contract_fixture(domain: &str) -> Value {
    serde_json::from_str::<Vec<Value>>(include_str!(
        "../../fixtures/shared_diff_contract_cases.json"
    ))
    .unwrap()
    .into_iter()
    .find(|value| value.get("domain").and_then(Value::as_str) == Some(domain))
    .expect("shared diff contract fixture")
}

fn load_generated_diff_help(domain: &str) -> &'static str {
    match domain {
        "dashboard" => include_str!("../../schemas/help/diff/dashboard.txt"),
        "alert" => include_str!("../../schemas/help/diff/alert.txt"),
        "datasource" => include_str!("../../schemas/help/diff/datasource.txt"),
        _ => panic!("unknown diff help domain: {domain}"),
    }
}

fn assert_help_contains_all_keys(help: &str, keys: &[&str]) {
    for key in keys {
        assert!(
            help.contains(key),
            "help output is missing required schema key `{key}`"
        );
    }
}

fn assert_help_matches_shared_diff_contract(help: &str, domain: &str) {
    let fixture = load_shared_diff_contract_fixture(domain);
    let generated_help = load_generated_diff_help(domain);
    assert_eq!(help, generated_help);
    let expected_top_level_keys = fixture["topLevelKeys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let expected_summary_keys = fixture["summaryKeys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let expected_row_keys = fixture["rowKeys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<BTreeSet<_>>();

    assert!(help.contains(fixture["kind"].as_str().unwrap()));
    assert_help_contains_all_keys(
        help,
        &expected_top_level_keys.iter().cloned().collect::<Vec<_>>(),
    );
    assert_help_contains_all_keys(
        help,
        &expected_summary_keys.iter().cloned().collect::<Vec<_>>(),
    );
    assert_help_contains_all_keys(help, &expected_row_keys.iter().cloned().collect::<Vec<_>>());
}

fn repo_root_relative_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(relative)
}

fn read_repo_text(relative: &str) -> String {
    fs::read_to_string(repo_root_relative_path(relative)).unwrap()
}

fn read_repo_json(relative: &str) -> Value {
    serde_json::from_str(&read_repo_text(relative)).unwrap()
}

fn assert_generated_schema_route_exists(family: &str, contract_id: &str) {
    let schema_path = repo_root_relative_path(&format!(
        "schemas/jsonschema/{family}/{contract_id}.schema.json"
    ));
    let schema = serde_json::from_str::<Value>(&fs::read_to_string(&schema_path).unwrap()).unwrap();
    assert!(
        schema["title"].is_string(),
        "schema {contract_id} should have a title"
    );
    assert!(
        schema["properties"].is_object(),
        "schema {contract_id} should expose properties"
    );
}

fn route_help_target(route_id: &str) -> Option<&str> {
    let trimmed = route_id.strip_suffix(".help").unwrap_or(route_id);
    match trimmed {
        "root" => None,
        other => Some(other),
    }
}

fn assert_manifest_driven_schema_help(command_args: &[&str], family: &str) {
    let routes = read_repo_json(&format!("schemas/manifests/{family}/routes.json"));
    let contracts = read_repo_json(&format!("schemas/manifests/{family}/contracts.json"));

    for route in routes.as_array().unwrap() {
        let route_id = route["routeId"].as_str().unwrap();
        let help_path = repo_root_relative_path(&format!("schemas/help/{family}/{route_id}.txt"));
        let expected_help = fs::read_to_string(help_path).unwrap();
        let mut route_args = command_args.to_vec();
        if let Some(target) = route_help_target(route_id) {
            route_args.insert(route_args.len() - 1, target);
        }
        let actual_help = maybe_render_unified_help_from_os_args(route_args, false)
            .expect("expected route help output");
        assert_eq!(actual_help, expected_help);
    }

    let declared_contracts = contracts
        .as_array()
        .unwrap()
        .iter()
        .map(|contract| contract["contractId"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    for contract_id in declared_contracts {
        assert_generated_schema_route_exists(family, contract_id);
    }
}

#[test]
fn shared_diff_contract_docs_explain_versioning_rules() {
    let en = include_str!("../../docs/user-guide/en/diff-json-contract.md");
    assert!(en.contains("family-wide major version"));
    assert!(en.contains("current version is `1`"));
    assert!(en.contains("breaking changes"));
    assert!(en.contains("dashboard diff`, `alert diff`, and `datasource diff` together"));

    let zh = include_str!("../../docs/user-guide/zh-TW/diff-json-contract.md");
    assert!(zh.contains("family-wide major version"));
    assert!(zh.contains("目前版本是 `1`"));
    assert!(zh.contains("breaking change"));
    assert!(zh.contains("dashboard diff`、`alert diff`、`datasource diff`"));
}

#[test]
fn unified_help_mentions_screenshot_and_dashboard_analysis_examples() {
    let help = render_unified_help();
    assert!(help.contains("--version"));
    assert!(help.contains("--help-full"));
    assert!(help.contains("Print help with extended examples"));
    assert!(help.contains("[Dashboard Export] Export dashboards with Basic auth"));
    assert!(help.contains("[Dashboard Export] Export dashboards across all visible orgs"));
    assert!(help.contains("[Migrate Raw To Prompt]"));
    assert!(help
        .contains("migrate dashboard raw-to-prompt --input-file ./dashboards/raw/cpu-main.json"));
    assert!(help.contains("--basic-user admin --basic-password admin"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("dashboard screenshot"));
    assert!(help.contains("dashboard analyze --url http://localhost:3000"));
    assert!(help.contains("datasource list --input-dir ./datasources"));
    assert!(help.contains("--dashboard-url"));
    assert!(help.contains("dashboard review"));
    assert!(help.contains("snapshot export"));
    assert!(help.contains("snapshot review"));
    assert!(help.contains("Review a local snapshot inventory as JSON"));
    assert!(help.contains("grafana-util access user list --input-dir ./access-users --json"));
    assert!(help.contains("Run profile list, show, add, example, and init workflows."));
    assert!(help.contains("[Profile Show]"));
    assert!(help.contains("[Profile Add]"));
    assert!(help.contains("[Profile Example]"));
}

#[test]
fn dashboard_help_and_docs_remain_canonical_first() {
    let unified_help = render_unified_help();
    assert!(unified_help.contains("grafana-util dashboard export"));
    assert!(unified_help.contains("grafana-util dashboard browse"));
    assert!(unified_help.contains("grafana-util dashboard screenshot"));
    assert!(unified_help.contains("grafana-util dashboard list-vars"));
    assert!(unified_help.contains("grafana-util dashboard analyze"));
    assert!(unified_help.contains("grafana-util dashboard review"));
    assert!(!unified_help.contains("grafana-util export --"));
    assert!(!unified_help.contains("grafana-util list --"));
    assert!(!unified_help.contains("grafana-util diff --"));
    assert!(!unified_help.contains("grafana-util publish --"));

    let en_dashboard_index = include_str!("../../docs/commands/en/index.md");
    assert!(en_dashboard_index.contains("Compatibility Pages"));
    assert!(en_dashboard_index.contains("[dashboard analyze (local alias)]"));
    assert!(en_dashboard_index.contains("[dashboard analyze (live alias)]"));
    assert!(!en_dashboard_index.contains("[dashboard analyze (local)]"));
    assert!(!en_dashboard_index.contains("[dashboard analyze (live)]"));

    let zh_dashboard_index = include_str!("../../docs/commands/zh-TW/index.md");
    assert!(zh_dashboard_index.contains("相容頁面"));
    assert!(zh_dashboard_index.contains("[dashboard analyze（本地別名）]"));
    assert!(zh_dashboard_index.contains("[dashboard analyze（即時別名）]"));
    assert!(!zh_dashboard_index.contains("[dashboard analyze（本地）]"));
    assert!(!zh_dashboard_index.contains("[dashboard analyze（即時）]"));

    let en_dashboard_page = include_str!("../../docs/commands/en/dashboard.md");
    assert!(en_dashboard_page.contains("grafana-util dashboard browse"));
    assert!(en_dashboard_page.contains("grafana-util dashboard publish"));
    assert!(!en_dashboard_page.contains("grafana-util export --"));
    assert!(!en_dashboard_page.contains("grafana-util list --"));
    assert!(!en_dashboard_page.contains("grafana-util diff --"));
    assert!(!en_dashboard_page.contains("grafana-util publish --"));
}

#[test]
fn unified_cli_renders_root_version_flag_output() {
    let clap_version = CliArgs::command().render_version().to_string();
    let unified_version = render_unified_version_text();
    assert_eq!(clap_version, unified_version);
    assert_eq!(unified_version, TOOL_VERSION_TEXT);
    assert!(unified_version.contains("grafana-util"));
    assert!(unified_version.contains(TOOL_VERSION));
    assert!(unified_version.contains("commit:"));
    assert!(unified_version.contains(TOOL_GIT_COMMIT));
    assert!(unified_version.contains("build time:"));
    assert!(unified_version.contains(TOOL_BUILD_TIME));
}

#[test]
fn parse_cli_supports_version_subcommand() {
    let args: CliArgs = parse_cli_from(["grafana-util", "version"]);
    match args.command {
        UnifiedCommand::Version(inner) => assert!(!inner.json),
        _ => panic!("expected version command"),
    }
}

#[test]
fn parse_cli_supports_version_json_flag() {
    let args: CliArgs = parse_cli_from(["grafana-util", "version", "--json"]);
    match args.command {
        UnifiedCommand::Version(inner) => assert!(inner.json),
        _ => panic!("expected version command"),
    }
}

#[test]
fn version_subcommand_json_output_is_machine_readable() {
    let output = {
        let payload = serde_json::json!({
            "schemaVersion": TOOL_VERSION_SCHEMA_VERSION,
            "name": "grafana-util",
            "version": TOOL_VERSION,
            "commit": TOOL_GIT_COMMIT,
            "buildTime": TOOL_BUILD_TIME,
        });
        render_json_value_with_choice(&payload, CliColorChoice::Never, false).unwrap()
    };
    let value: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 5);
    assert_eq!(value["schemaVersion"], TOOL_VERSION_SCHEMA_VERSION);
    assert_eq!(value["name"], "grafana-util");
    assert_eq!(value["version"], TOOL_VERSION);
    assert_eq!(value["commit"], TOOL_GIT_COMMIT);
    assert_eq!(value["buildTime"], TOOL_BUILD_TIME);
}

#[test]
fn parse_cli_supports_dashboard_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "export",
        "--output-dir",
        "./dashboards",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::Export(inner) => {
                assert_eq!(inner.output_dir, Path::new("./dashboards"));
            }
            _ => panic!("expected dashboard export"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_rejects_legacy_export_import_dir_flags() {
    let dashboard_export = CliArgs::try_parse_from([
        "grafana-util",
        "dashboard",
        "export",
        "--export-dir",
        "./dashboards",
    ])
    .unwrap_err();
    assert!(dashboard_export.to_string().contains("--export-dir"));
    assert!(dashboard_export.to_string().contains("--output-dir"));

    let datasource_import = CliArgs::try_parse_from([
        "grafana-util",
        "datasource",
        "import",
        "--import-dir",
        "./datasources",
    ])
    .unwrap_err();
    assert!(datasource_import.to_string().contains("--import-dir"));
    assert!(datasource_import.to_string().contains("--input-dir"));

    let access_user_export = CliArgs::try_parse_from([
        "grafana-util",
        "access",
        "user",
        "export",
        "--export-dir",
        "./access-users",
    ])
    .unwrap_err();
    assert!(access_user_export.to_string().contains("--export-dir"));
    assert!(access_user_export.to_string().contains("--output-dir"));

    let snapshot_export = CliArgs::try_parse_from([
        "grafana-util",
        "snapshot",
        "export",
        "--export-dir",
        "./snapshot",
    ])
    .unwrap_err();
    assert!(snapshot_export.to_string().contains("--export-dir"));
    assert!(snapshot_export.to_string().contains("--output-dir"));
}

#[test]
fn parse_cli_supports_resource_list_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "resource",
        "list",
        "dashboards",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Resource(inner) => match inner.command {
            ResourceCommand::List(list_args) => {
                assert_eq!(list_args.kind, ResourceKind::Dashboards);
                assert_eq!(list_args.output_format, ResourceOutputFormat::Json);
            }
            _ => panic!("expected resource list"),
        },
        _ => panic!("expected resource command"),
    }
}

#[test]
fn parse_cli_supports_resource_describe_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "resource",
        "describe",
        "dashboards",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Resource(inner) => match inner.command {
            ResourceCommand::Describe(describe_args) => {
                assert_eq!(describe_args.kind, Some(ResourceKind::Dashboards));
                assert_eq!(describe_args.output_format, ResourceOutputFormat::Json);
            }
            _ => panic!("expected resource describe"),
        },
        _ => panic!("expected resource command"),
    }
}

#[test]
fn resource_help_mentions_describe() {
    let help = render_resource_subcommand_help(&[]);
    assert!(help.contains("describe"));
    assert!(help.contains(
        "List the resource kinds supported by the generic read-only resource query surface."
    ));
    assert!(
        help.contains("Describe the supported live Grafana resource kinds and selector patterns.")
    );
}

#[test]
fn unified_help_mentions_resource_escape_hatch() {
    let help = render_unified_help();
    assert!(help.contains("resource"));
    assert!(help.contains("resource describe"));
    assert!(help.contains("generic read-only query surface"));
}

#[test]
fn parse_cli_supports_migrate_raw_to_prompt_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "migrate",
        "dashboard",
        "raw-to-prompt",
        "--input-file",
        "./dashboards/raw/cpu-main.json",
        "--output-format",
        "yaml",
        "--log-format",
        "json",
        "--resolution",
        "strict",
        "--profile",
        "prod",
        "--org-id",
        "2",
    ]);

    match args.command {
        UnifiedCommand::Migrate(inner) => match inner.command {
            MigrateCommand::Dashboard { command } => match command {
                MigrateDashboardCommand::RawToPrompt(inner) => {
                    assert_eq!(
                        inner.input_file,
                        vec![Path::new("./dashboards/raw/cpu-main.json").to_path_buf()]
                    );
                    assert_eq!(inner.output_format, RawToPromptOutputFormat::Yaml);
                    assert_eq!(inner.log_format, RawToPromptLogFormat::Json);
                    assert_eq!(inner.resolution, RawToPromptResolution::Strict);
                    assert_eq!(inner.profile.as_deref(), Some("prod"));
                    assert_eq!(inner.org_id, Some(2));
                }
            },
        },
        _ => panic!("expected migrate group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_get_and_clone_live_commands() {
    let get_args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "fetch-live",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.json",
    ]);
    let clone_args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "clone-live",
        "--source-uid",
        "cpu-main",
        "--uid",
        "cpu-main-clone",
        "--folder-uid",
        "infra",
        "--output",
        "./cpu-main-clone.json",
    ]);

    match get_args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::Get(inner) => {
                assert_eq!(inner.dashboard_uid, "cpu-main");
                assert_eq!(inner.output, Path::new("./cpu-main.json"));
            }
            _ => panic!("expected dashboard fetch-live"),
        },
        _ => panic!("expected dashboard group"),
    }

    match clone_args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::CloneLive(inner) => {
                assert_eq!(inner.source_uid, "cpu-main");
                assert_eq!(inner.uid.as_deref(), Some("cpu-main-clone"));
                assert_eq!(inner.folder_uid.as_deref(), Some("infra"));
                assert_eq!(inner.output, Path::new("./cpu-main-clone.json"));
            }
            _ => panic!("expected dashboard clone-live"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_patch_file_and_publish_commands() {
    let patch_args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "patch-file",
        "--input",
        "./drafts/cpu-main.json",
        "--tag",
        "prod",
    ]);
    let publish_args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "publish",
        "--url",
        "https://grafana.example.com",
        "--basic-user",
        "admin",
        "--basic-password",
        "admin",
        "--input",
        "./drafts/cpu-main.json",
        "--dry-run",
    ]);

    match patch_args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::PatchFile(inner) => {
                assert_eq!(inner.input, Path::new("./drafts/cpu-main.json"));
                assert_eq!(inner.tags, vec!["prod".to_string()]);
            }
            _ => panic!("expected dashboard patch-file"),
        },
        _ => panic!("expected dashboard group"),
    }

    match publish_args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::Publish(inner) => {
                assert_eq!(inner.common.url, "https://grafana.example.com");
                assert_eq!(inner.common.username.as_deref(), Some("admin"));
                assert!(inner.dry_run);
            }
            _ => panic!("expected dashboard publish"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_review_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "review",
        "--input",
        "./drafts/cpu-main.json",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::Review(inner) => {
                assert_eq!(inner.input, Path::new("./drafts/cpu-main.json"));
                assert_eq!(inner.output_format, None);
                assert!(!inner.json);
                assert!(!inner.table);
                assert!(!inner.csv);
                assert!(!inner.yaml);
            }
            _ => panic!("expected dashboard review"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_snapshot_group_export_and_review_commands() {
    let export_args: CliArgs = parse_cli_from([
        "grafana-util",
        "snapshot",
        "export",
        "--url",
        "https://grafana.example.com",
        "--token",
        "abc",
        "--output-dir",
        "./snapshot",
        "--overwrite",
    ]);
    let review_args: CliArgs = parse_cli_from([
        "grafana-util",
        "snapshot",
        "review",
        "--input-dir",
        "./snapshot",
        "--output-format",
        "json",
    ]);

    match export_args.command {
        UnifiedCommand::Snapshot { command } => match command {
            super::SnapshotCommand::Export(inner) => {
                assert_eq!(inner.output_dir, Path::new("./snapshot"));
                assert_eq!(inner.common.url, "https://grafana.example.com");
                assert_eq!(inner.common.api_token.as_deref(), Some("abc"));
                assert!(inner.overwrite);
            }
            _ => panic!("expected snapshot export"),
        },
        _ => panic!("expected snapshot group"),
    }

    match review_args.command {
        UnifiedCommand::Snapshot { command } => match command {
            super::SnapshotCommand::Review(inner) => {
                assert_eq!(inner.input_dir, Path::new("./snapshot"));
                assert_eq!(inner.output_format, OverviewOutputFormat::Json);
                assert!(!inner.interactive);
            }
            _ => panic!("expected snapshot review"),
        },
        _ => panic!("expected snapshot group"),
    }
}

#[test]
fn parse_cli_supports_snapshot_review_interactive_shortcut() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "snapshot",
        "review",
        "--input-dir",
        "./snapshot",
        "--interactive",
    ]);

    match args.command {
        UnifiedCommand::Snapshot { command } => match command {
            super::SnapshotCommand::Review(inner) => {
                assert_eq!(inner.input_dir, Path::new("./snapshot"));
                assert_eq!(inner.output_format, OverviewOutputFormat::Text);
                assert!(inner.interactive);
            }
            _ => panic!("expected snapshot review"),
        },
        _ => panic!("expected snapshot group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_screenshot_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "screenshot",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.png",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::Screenshot(inner) => {
                assert_eq!(inner.dashboard_uid.as_deref(), Some("cpu-main"));
                assert_eq!(inner.output, Path::new("./cpu-main.png"));
                assert!(!inner.full_page);
                assert_eq!(inner.output_format, None);
            }
            _ => panic!("expected dashboard screenshot"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_profile_group_list_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "profile", "list"]);

    match args.command {
        UnifiedCommand::Profile(profile_args) => match profile_args.command {
            ProfileCommand::List(_) => {}
            _ => panic!("expected profile list"),
        },
        _ => panic!("expected profile group"),
    }
}

#[test]
fn parse_cli_supports_profile_group_current_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "profile",
        "current",
        "--profile",
        "prod",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Profile(profile_args) => match profile_args.command {
            ProfileCommand::Current(current_args) => {
                assert_eq!(current_args.profile.as_deref(), Some("prod"));
                assert_eq!(current_args.output_format, SimpleOutputFormat::Json);
            }
            _ => panic!("expected profile current"),
        },
        _ => panic!("expected profile group"),
    }
}

#[test]
fn parse_cli_supports_profile_group_validate_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "profile",
        "validate",
        "--profile",
        "prod",
        "--live",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        UnifiedCommand::Profile(profile_args) => match profile_args.command {
            ProfileCommand::Validate(validate_args) => {
                assert_eq!(validate_args.profile.as_deref(), Some("prod"));
                assert!(validate_args.live);
                assert_eq!(validate_args.output_format, SimpleOutputFormat::Yaml);
            }
            _ => panic!("expected profile validate"),
        },
        _ => panic!("expected profile group"),
    }
}

#[test]
fn parse_cli_supports_profile_group_show_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "profile",
        "show",
        "--profile",
        "prod",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        UnifiedCommand::Profile(profile_args) => match profile_args.command {
            ProfileCommand::Show(show_args) => {
                assert_eq!(show_args.profile.as_deref(), Some("prod"));
                assert!(!show_args.show_secrets);
                assert_eq!(show_args.output_format, SimpleOutputFormat::Yaml);
            }
            _ => panic!("expected profile show"),
        },
        _ => panic!("expected profile group"),
    }
}

#[test]
fn parse_cli_supports_profile_group_show_secrets_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "profile",
        "show",
        "--profile",
        "prod",
        "--show-secrets",
    ]);

    match args.command {
        UnifiedCommand::Profile(profile_args) => match profile_args.command {
            ProfileCommand::Show(show_args) => {
                assert_eq!(show_args.profile.as_deref(), Some("prod"));
                assert!(show_args.show_secrets);
            }
            _ => panic!("expected profile show"),
        },
        _ => panic!("expected profile group"),
    }
}

#[test]
fn parse_cli_supports_profile_group_init_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "profile", "init", "--overwrite"]);

    match args.command {
        UnifiedCommand::Profile(profile_args) => match profile_args.command {
            ProfileCommand::Init(init_args) => {
                assert!(init_args.overwrite);
            }
            _ => panic!("expected profile init"),
        },
        _ => panic!("expected profile group"),
    }
}

#[test]
fn parse_cli_supports_profile_group_add_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "profile",
        "add",
        "prod",
        "--url",
        "https://grafana.example.com",
        "--basic-user",
        "admin",
        "--prompt-password",
        "--store-secret",
        "encrypted-file",
    ]);

    match args.command {
        UnifiedCommand::Profile(profile_args) => match profile_args.command {
            ProfileCommand::Add(add_args) => {
                assert_eq!(add_args.name.as_str(), "prod");
                assert_eq!(add_args.url.as_str(), "https://grafana.example.com");
                assert_eq!(add_args.basic_user.as_deref(), Some("admin"));
                assert!(add_args.prompt_password);
                assert_eq!(
                    add_args.store_secret,
                    crate::profile_cli::ProfileSecretStorageMode::EncryptedFile
                );
            }
            _ => panic!("expected profile add"),
        },
        _ => panic!("expected profile group"),
    }
}

#[test]
fn parse_cli_supports_profile_group_example_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "profile", "example", "--mode", "basic"]);

    match args.command {
        UnifiedCommand::Profile(profile_args) => match profile_args.command {
            ProfileCommand::Example(example_args) => {
                assert_eq!(
                    example_args.mode,
                    crate::profile_cli::ProfileExampleMode::Basic
                );
            }
            _ => panic!("expected profile example"),
        },
        _ => panic!("expected profile group"),
    }
}

#[test]
fn parse_cli_supports_profile_group_example_full_mode_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "profile", "example", "--mode", "full"]);

    match args.command {
        UnifiedCommand::Profile(profile_args) => match profile_args.command {
            ProfileCommand::Example(example_args) => {
                assert_eq!(
                    example_args.mode,
                    crate::profile_cli::ProfileExampleMode::Full
                );
            }
            _ => panic!("expected profile example"),
        },
        _ => panic!("expected profile group"),
    }
}

#[test]
fn profile_show_subcommand_help_mentions_output_format() {
    let help = render_profile_subcommand_help(&["show"]);

    assert!(help.contains("--output-format"));
    assert!(help.contains("--profile"));
    assert!(help.contains("--show-secrets"));
}

#[test]
fn profile_add_subcommand_help_mentions_secret_modes() {
    let help = render_profile_subcommand_help(&["add"]);

    assert!(help.contains("--store-secret"));
    assert!(help.contains("--prompt-password"));
    assert!(help.contains("--prompt-secret-passphrase"));
}

#[test]
fn profile_example_subcommand_help_mentions_mode() {
    let help = render_profile_subcommand_help(&["example"]);

    assert!(help.contains("--mode"));
    assert!(help.contains("annotated profile config example"));
}

#[test]
fn profile_example_subcommand_help_mentions_modes() {
    let help = render_profile_subcommand_help(&["example"]);

    assert!(help.contains("--mode"));
    assert!(help.contains("basic"));
    assert!(help.contains("full"));
}

#[test]
fn profile_root_help_mentions_current_and_validate() {
    let help = render_profile_subcommand_help(&[]);

    assert!(help.contains("grafana-util profile current"));
    assert!(help.contains("grafana-util profile validate"));
    assert!(help.contains("Show the currently selected profile"));
    assert!(help.contains("Validate the selected profile"));
}

#[test]
fn parse_cli_supports_datasource_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "datasource",
        "import",
        "--input-dir",
        "./datasources",
        "--dry-run",
    ]);

    match args.command {
        UnifiedCommand::Datasource { command, .. } => match command {
            DatasourceGroupCommand::Import(inner) => {
                assert_eq!(inner.input_dir, Path::new("./datasources"));
                assert!(inner.dry_run);
            }
            _ => panic!("expected datasource import"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_datasource_diff_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "datasource",
        "diff",
        "--diff-dir",
        "./datasources",
    ]);

    match args.command {
        UnifiedCommand::Datasource { command, .. } => match command {
            DatasourceGroupCommand::Diff(inner) => {
                assert_eq!(inner.diff_dir, Path::new("./datasources"));
            }
            _ => panic!("expected datasource diff"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_datasource_group_alias() {
    let args: CliArgs = parse_cli_from(["grafana-util", "ds", "list", "--json"]);

    match args.command {
        UnifiedCommand::Datasource { command, .. } => match command {
            DatasourceGroupCommand::List(inner) => {
                assert!(inner.json);
            }
            _ => panic!("expected datasource list"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_inspect_export_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "analyze-export",
        "--input-dir",
        "./dashboards/raw",
        "--json",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::InspectExport(inner) => {
                assert_eq!(inner.input_dir, Path::new("./dashboards/raw"));
                assert!(inner.json);
            }
            _ => panic!("expected dashboard analyze-export"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_inspect_live_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "analyze-live",
        "--url",
        "http://127.0.0.1:3000",
        "--output-format",
        "queries-json",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::InspectLive(inner) => {
                assert_eq!(inner.common.url, "http://127.0.0.1:3000");
                assert_eq!(
                    inner.output_format,
                    Some(crate::dashboard::InspectOutputFormat::QueriesJson)
                );
            }
            _ => panic!("expected dashboard analyze-live"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_alias() {
    let args: CliArgs = parse_cli_from(["grafana-util", "db", "list", "--json"]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::List(inner) => assert!(inner.json),
            _ => panic!("expected dashboard list"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_graph_alias() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "graph",
        "--governance",
        "./governance.json",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::Topology(topology_args) => {
                assert_eq!(
                    topology_args.governance,
                    Some(PathBuf::from("./governance.json"))
                );
            }
            _ => panic!("expected dashboard topology"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_impact_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "impact",
        "--input-dir",
        "./grafana-oac-repo",
        "--input-format",
        "git-sync",
        "--datasource-uid",
        "smoke-prom",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::Impact(inner) => {
                assert_eq!(inner.input_dir, Some(PathBuf::from("./grafana-oac-repo")));
                assert_eq!(
                    inner.input_format,
                    crate::dashboard::DashboardImportInputFormat::Raw
                );
                assert_eq!(inner.datasource_uid, "smoke-prom");
                assert_eq!(
                    inner.output_format,
                    crate::dashboard::ImpactOutputFormat::Json
                );
            }
            _ => panic!("expected dashboard impact"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_group_validate_export_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "validate-export",
        "--input-dir",
        "./grafana-oac-repo",
        "--input-format",
        "git-sync",
        "--reject-custom-plugins",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::ValidateExport(inner) => {
                assert_eq!(inner.input_dir, PathBuf::from("./grafana-oac-repo"));
                assert_eq!(
                    inner.input_format,
                    crate::dashboard::DashboardImportInputFormat::Raw
                );
                assert!(inner.reject_custom_plugins);
            }
            _ => panic!("expected dashboard validate-export"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_history_list() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "history",
        "list",
        "--dashboard-uid",
        "cpu-main",
        "--limit",
        "15",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::History(history_args) => match history_args.command {
                crate::dashboard::DashboardHistorySubcommand::List(inner) => {
                    assert_eq!(inner.dashboard_uid.as_deref(), Some("cpu-main"));
                    assert_eq!(inner.limit, 15);
                    assert_eq!(
                        inner.output_format,
                        crate::dashboard::HistoryOutputFormat::Yaml
                    );
                }
                _ => panic!("expected dashboard history list"),
            },
            _ => panic!("expected dashboard history"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_history_export() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "history",
        "export",
        "--dashboard-uid",
        "cpu-main",
        "--output",
        "./cpu-main.history.json",
        "--limit",
        "30",
        "--overwrite",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::History(history_args) => match history_args.command {
                crate::dashboard::DashboardHistorySubcommand::Export(inner) => {
                    assert_eq!(inner.dashboard_uid, "cpu-main");
                    assert_eq!(inner.output, PathBuf::from("./cpu-main.history.json"));
                    assert_eq!(inner.limit, 30);
                    assert!(inner.overwrite);
                }
                _ => panic!("expected dashboard history export"),
            },
            _ => panic!("expected dashboard history"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_supports_dashboard_history_diff() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "history",
        "diff",
        "--base-input",
        "./exports-2026-04-01/history/cpu-main.history.json",
        "--base-version",
        "17",
        "--new-input",
        "./exports-2026-04-07/history/cpu-main.history.json",
        "--new-version",
        "21",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Dashboard { command } => match command {
            super::DashboardGroupCommand::History(history_args) => match history_args.command {
                crate::dashboard::DashboardHistorySubcommand::Diff(inner) => {
                    assert_eq!(
                        inner.base_input,
                        Some(PathBuf::from(
                            "./exports-2026-04-01/history/cpu-main.history.json"
                        ))
                    );
                    assert_eq!(inner.base_version, 17);
                    assert_eq!(
                        inner.new_input,
                        Some(PathBuf::from(
                            "./exports-2026-04-07/history/cpu-main.history.json"
                        ))
                    );
                    assert_eq!(inner.new_version, 21);
                    assert_eq!(inner.output_format, crate::common::DiffOutputFormat::Json);
                }
                _ => panic!("expected dashboard history diff"),
            },
            _ => panic!("expected dashboard history"),
        },
        _ => panic!("expected dashboard group"),
    }
}

#[test]
fn parse_cli_rejects_dashboard_list_datasources_subcommand() {
    let error =
        CliArgs::try_parse_from(["grafana-util", "dashboard", "list-data-sources", "--json"])
            .unwrap_err();

    assert!(error.to_string().contains("unrecognized subcommand"));
    assert!(error.to_string().contains("list-data-sources"));
}

#[test]
fn parse_cli_supports_datasource_list_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "datasource", "list", "--json"]);

    match args.command {
        UnifiedCommand::Datasource { command, .. } => match command {
            DatasourceGroupCommand::List(inner) => assert!(inner.json),
            _ => panic!("expected datasource list"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_datasource_types_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "datasource",
        "types",
        "--output-format",
        "csv",
    ]);

    match args.command {
        UnifiedCommand::Datasource { command, .. } => match command {
            DatasourceGroupCommand::Types(inner) => {
                assert_eq!(inner.output_format, SimpleOutputFormat::Csv);
            }
            _ => panic!("expected datasource types"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_datasource_group_list_local_input_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--json",
    ]);

    match args.command {
        UnifiedCommand::Datasource { command, .. } => match command {
            DatasourceGroupCommand::List(inner) => {
                assert!(inner.json);
                assert!(!inner.table);
                assert!(!inner.csv);
                assert!(!inner.text);
                assert!(!inner.yaml);
            }
            _ => panic!("expected datasource list"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_datasource_browse_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "datasource", "browse", "--org-id", "4"]);

    match args.command {
        UnifiedCommand::Datasource { command, .. } => match command {
            DatasourceGroupCommand::Browse(inner) => {
                assert_eq!(inner.org_id, Some(4));
                assert!(!inner.all_orgs);
            }
            _ => panic!("expected datasource browse"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_datasource_browse_all_orgs() {
    let args: CliArgs = parse_cli_from(["grafana-util", "datasource", "browse", "--all-orgs"]);

    match args.command {
        UnifiedCommand::Datasource { command, .. } => match command {
            DatasourceGroupCommand::Browse(inner) => {
                assert!(inner.all_orgs);
                assert_eq!(inner.org_id, None);
            }
            _ => panic!("expected datasource browse"),
        },
        _ => panic!("expected datasource group"),
    }
}

#[test]
fn parse_cli_supports_alert_group() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "export",
        "--output-dir",
        "./alerts",
        "--overwrite",
    ]);

    match args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::Export(export_args)) => {
                assert_eq!(export_args.output_dir, Path::new("./alerts"));
                assert!(export_args.overwrite);
            }
            _ => panic!("expected alert export"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_plan_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "plan",
        "--desired-dir",
        "./alerts/desired",
        "--prune",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::Plan(plan_args)) => {
                assert_eq!(plan_args.desired_dir, Path::new("./alerts/desired"));
                assert!(plan_args.prune);
                assert_eq!(format!("{:?}", plan_args.output_format), "Json");
            }
            _ => panic!("expected alert plan"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_apply_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "apply",
        "--plan-file",
        "./plan.json",
        "--approve",
    ]);

    match args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::Apply(apply_args)) => {
                assert_eq!(apply_args.plan_file, Path::new("./plan.json"));
                assert!(apply_args.approve);
            }
            _ => panic!("expected alert apply"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_delete_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "delete",
        "--kind",
        "policy-tree",
        "--identity",
        "default",
        "--allow-policy-reset",
    ]);

    match args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::Delete(delete_args)) => {
                assert_eq!(format!("{:?}", delete_args.kind), "PolicyTree");
                assert_eq!(delete_args.identity, "default");
                assert!(delete_args.allow_policy_reset);
            }
            _ => panic!("expected alert delete"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_scaffolding_group_commands() {
    let init_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "init",
        "--desired-dir",
        "./alerts/desired",
    ]);
    match init_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::Init(init_args)) => {
                assert_eq!(init_args.desired_dir, Path::new("./alerts/desired"));
            }
            _ => panic!("expected alert init"),
        },
        _ => panic!("expected alert group"),
    }

    let new_rule_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "new-rule",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "cpu-main",
    ]);
    match new_rule_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::NewRule(new_args)) => {
                assert_eq!(new_args.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(new_args.name, "cpu-main");
            }
            _ => panic!("expected alert new-rule"),
        },
        _ => panic!("expected alert group"),
    }

    let new_contact_point_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "new-contact-point",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "pagerduty-primary",
    ]);
    match new_contact_point_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::NewContactPoint(new_args)) => {
                assert_eq!(new_args.name, "pagerduty-primary");
            }
            _ => panic!("expected alert new-contact-point"),
        },
        _ => panic!("expected alert group"),
    }

    let new_template_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "new-template",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "sev1-notification",
    ]);
    match new_template_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::NewTemplate(new_args)) => {
                assert_eq!(new_args.name, "sev1-notification");
            }
            _ => panic!("expected alert new-template"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_authoring_group_commands() {
    let add_rule_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "add-rule",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "cpu-high",
        "--folder",
        "platform-alerts",
        "--rule-group",
        "cpu",
        "--receiver",
        "pagerduty-primary",
        "--label",
        "team=platform",
        "--annotation",
        "summary=CPU high",
        "--severity",
        "critical",
        "--for",
        "5m",
        "--expr",
        "A",
        "--threshold",
        "80",
        "--above",
        "--dry-run",
    ]);
    match add_rule_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::AddRule(add_args)) => {
                assert_eq!(add_args.base.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(add_args.name, "cpu-high");
                assert_eq!(add_args.folder, "platform-alerts");
                assert_eq!(add_args.rule_group, "cpu");
                assert_eq!(add_args.receiver.as_deref(), Some("pagerduty-primary"));
                assert_eq!(add_args.labels, vec!["team=platform".to_string()]);
                assert_eq!(add_args.annotations, vec!["summary=CPU high".to_string()]);
                assert_eq!(add_args.severity.as_deref(), Some("critical"));
                assert_eq!(add_args.for_duration.as_deref(), Some("5m"));
                assert_eq!(add_args.expr.as_deref(), Some("A"));
                assert_eq!(add_args.threshold, Some(80.0));
                assert!(add_args.above);
                assert!(!add_args.below);
                assert!(add_args.dry_run);
            }
            _ => panic!("expected alert add-rule"),
        },
        _ => panic!("expected alert group"),
    }

    let clone_rule_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "clone-rule",
        "--desired-dir",
        "./alerts/desired",
        "--source",
        "cpu-high",
        "--name",
        "cpu-high-staging",
        "--dry-run",
    ]);
    match clone_rule_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::CloneRule(clone_args)) => {
                assert_eq!(clone_args.base.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(clone_args.source, "cpu-high");
                assert_eq!(clone_args.name, "cpu-high-staging");
                assert!(clone_args.dry_run);
            }
            _ => panic!("expected alert clone-rule"),
        },
        _ => panic!("expected alert group"),
    }

    let add_contact_point_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "add-contact-point",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "pagerduty-primary",
        "--dry-run",
    ]);
    match add_contact_point_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::AddContactPoint(add_args)) => {
                assert_eq!(add_args.base.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(add_args.name, "pagerduty-primary");
                assert!(add_args.dry_run);
            }
            _ => panic!("expected alert add-contact-point"),
        },
        _ => panic!("expected alert group"),
    }

    let set_route_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "set-route",
        "--desired-dir",
        "./alerts/desired",
        "--receiver",
        "pagerduty-primary",
        "--label",
        "team=platform",
        "--severity",
        "critical",
        "--dry-run",
    ]);
    match set_route_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::SetRoute(set_args)) => {
                assert_eq!(set_args.base.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(set_args.receiver, "pagerduty-primary");
                assert_eq!(set_args.labels, vec!["team=platform".to_string()]);
                assert_eq!(set_args.severity.as_deref(), Some("critical"));
                assert!(set_args.dry_run);
            }
            _ => panic!("expected alert set-route"),
        },
        _ => panic!("expected alert group"),
    }

    let preview_route_args: CliArgs = parse_cli_from([
        "grafana-util",
        "alert",
        "preview-route",
        "--desired-dir",
        "./alerts/desired",
        "--label",
        "team=platform",
        "--severity",
        "critical",
    ]);
    match preview_route_args.command {
        UnifiedCommand::Alert(inner) => match inner.command {
            Some(crate::alert::AlertGroupCommand::PreviewRoute(preview_args)) => {
                assert_eq!(preview_args.base.desired_dir, Path::new("./alerts/desired"));
                assert_eq!(preview_args.labels, vec!["team=platform".to_string()]);
                assert_eq!(preview_args.severity.as_deref(), Some("critical"));
            }
            _ => panic!("expected alert preview-route"),
        },
        _ => panic!("expected alert group"),
    }
}

#[test]
fn parse_cli_supports_alert_plan_normalized_args() {
    let args = parse_alert_cli_from([
        "grafana-util alert",
        "plan",
        "--desired-dir",
        "./alerts/desired",
        "--prune",
        "--dashboard-uid-map",
        "./dashboard-map.json",
        "--panel-id-map",
        "./panel-map.json",
        "--output-format",
        "json",
    ]);

    assert_eq!(format!("{:?}", args.command_kind), "Some(Plan)");
    assert_eq!(
        args.desired_dir.as_deref(),
        Some(Path::new("./alerts/desired"))
    );
    assert!(args.prune);
    assert_eq!(
        args.dashboard_uid_map.as_deref(),
        Some(Path::new("./dashboard-map.json"))
    );
    assert_eq!(
        args.panel_id_map.as_deref(),
        Some(Path::new("./panel-map.json"))
    );
    assert_eq!(format!("{:?}", args.command_output), "Some(Json)");
    assert!(args.json);
}

#[test]
fn parse_cli_supports_alert_apply_normalized_args() {
    let args = parse_alert_cli_from([
        "grafana-util alert",
        "apply",
        "--plan-file",
        "./plan.json",
        "--approve",
    ]);

    assert_eq!(format!("{:?}", args.command_kind), "Some(Apply)");
    assert_eq!(args.plan_file.as_deref(), Some(Path::new("./plan.json")));
    assert!(args.approve);
}

#[test]
fn parse_cli_supports_alert_delete_normalized_args() {
    let args = parse_alert_cli_from([
        "grafana-util alert",
        "delete",
        "--kind",
        "rule",
        "--identity",
        "cpu-main",
    ]);

    assert_eq!(format!("{:?}", args.command_kind), "Some(Delete)");
    assert_eq!(format!("{:?}", args.resource_kind), "Some(Rule)");
    assert_eq!(args.resource_identity.as_deref(), Some("cpu-main"));
    assert!(!args.allow_policy_reset);
}

#[test]
fn parse_cli_supports_alert_scaffolding_normalized_args() {
    let init_args = parse_alert_cli_from([
        "grafana-util alert",
        "init",
        "--desired-dir",
        "./alerts/desired",
    ]);
    assert_eq!(format!("{:?}", init_args.command_kind), "Some(Init)");
    assert_eq!(
        init_args.desired_dir.as_deref(),
        Some(Path::new("./alerts/desired"))
    );

    let rule_args = parse_alert_cli_from([
        "grafana-util alert",
        "new-rule",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "cpu-main",
    ]);
    assert_eq!(format!("{:?}", rule_args.command_kind), "Some(NewRule)");
    assert_eq!(rule_args.scaffold_name.as_deref(), Some("cpu-main"));

    let contact_point_args = parse_alert_cli_from([
        "grafana-util alert",
        "new-contact-point",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "pagerduty-primary",
    ]);
    assert_eq!(
        format!("{:?}", contact_point_args.command_kind),
        "Some(NewContactPoint)"
    );
    assert_eq!(
        contact_point_args.scaffold_name.as_deref(),
        Some("pagerduty-primary")
    );

    let template_args = parse_alert_cli_from([
        "grafana-util alert",
        "new-template",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "sev1-notification",
    ]);
    assert_eq!(
        format!("{:?}", template_args.command_kind),
        "Some(NewTemplate)"
    );
    assert_eq!(
        template_args.scaffold_name.as_deref(),
        Some("sev1-notification")
    );
}

#[test]
fn parse_cli_supports_alert_authoring_normalized_args() {
    let add_rule_args = parse_alert_cli_from([
        "grafana-util alert",
        "add-rule",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "cpu-high",
        "--folder",
        "platform-alerts",
        "--rule-group",
        "cpu",
        "--receiver",
        "pagerduty-primary",
        "--label",
        "team=platform",
        "--annotation",
        "summary=CPU high",
        "--severity",
        "critical",
        "--for",
        "5m",
        "--expr",
        "A",
        "--threshold",
        "80",
        "--above",
        "--dry-run",
    ]);
    assert_eq!(
        format!("{:?}", add_rule_args.authoring_command_kind),
        "Some(AddRule)"
    );
    assert_eq!(
        add_rule_args.desired_dir.as_deref(),
        Some(Path::new("./alerts/desired"))
    );
    assert_eq!(add_rule_args.scaffold_name.as_deref(), Some("cpu-high"));
    assert_eq!(add_rule_args.folder.as_deref(), Some("platform-alerts"));
    assert_eq!(add_rule_args.rule_group.as_deref(), Some("cpu"));
    assert_eq!(add_rule_args.receiver.as_deref(), Some("pagerduty-primary"));
    assert_eq!(add_rule_args.labels, vec!["team=platform".to_string()]);
    assert_eq!(
        add_rule_args.annotations,
        vec!["summary=CPU high".to_string()]
    );
    assert_eq!(add_rule_args.severity.as_deref(), Some("critical"));
    assert_eq!(add_rule_args.for_duration.as_deref(), Some("5m"));
    assert_eq!(add_rule_args.expr.as_deref(), Some("A"));
    assert_eq!(add_rule_args.threshold, Some(80.0));
    assert!(add_rule_args.above);
    assert!(!add_rule_args.below);
    assert!(add_rule_args.dry_run);

    let clone_rule_args = parse_alert_cli_from([
        "grafana-util alert",
        "clone-rule",
        "--desired-dir",
        "./alerts/desired",
        "--source",
        "cpu-high",
        "--name",
        "cpu-high-staging",
        "--no-route",
        "--dry-run",
    ]);
    assert_eq!(
        format!("{:?}", clone_rule_args.authoring_command_kind),
        "Some(CloneRule)"
    );
    assert_eq!(clone_rule_args.source_name.as_deref(), Some("cpu-high"));
    assert_eq!(
        clone_rule_args.scaffold_name.as_deref(),
        Some("cpu-high-staging")
    );
    assert!(clone_rule_args.no_route);
    assert!(clone_rule_args.dry_run);

    let add_contact_point_args = parse_alert_cli_from([
        "grafana-util alert",
        "add-contact-point",
        "--desired-dir",
        "./alerts/desired",
        "--name",
        "pagerduty-primary",
        "--dry-run",
    ]);
    assert_eq!(
        format!("{:?}", add_contact_point_args.authoring_command_kind),
        "Some(AddContactPoint)"
    );
    assert_eq!(
        add_contact_point_args.scaffold_name.as_deref(),
        Some("pagerduty-primary")
    );
    assert!(add_contact_point_args.dry_run);

    let set_route_args = parse_alert_cli_from([
        "grafana-util alert",
        "set-route",
        "--desired-dir",
        "./alerts/desired",
        "--receiver",
        "pagerduty-primary",
        "--label",
        "team=platform",
        "--dry-run",
    ]);
    assert_eq!(
        format!("{:?}", set_route_args.authoring_command_kind),
        "Some(SetRoute)"
    );
    assert_eq!(
        set_route_args.receiver.as_deref(),
        Some("pagerduty-primary")
    );
    assert_eq!(set_route_args.labels, vec!["team=platform".to_string()]);
    assert!(set_route_args.dry_run);

    let preview_route_args = parse_alert_cli_from([
        "grafana-util alert",
        "preview-route",
        "--desired-dir",
        "./alerts/desired",
        "--label",
        "team=platform",
        "--severity",
        "critical",
    ]);
    assert_eq!(
        format!("{:?}", preview_route_args.authoring_command_kind),
        "Some(PreviewRoute)"
    );
    assert_eq!(
        preview_route_args.desired_dir.as_deref(),
        Some(Path::new("./alerts/desired"))
    );
    assert_eq!(preview_route_args.labels, vec!["team=platform".to_string()]);
    assert_eq!(preview_route_args.severity.as_deref(), Some("critical"));
}

#[test]
fn parse_cli_supports_alert_apply_requires_approve_flag() {
    let error = crate::alert::root_command()
        .try_get_matches_from(["grafana-util alert", "apply", "--plan-file", "./plan.json"])
        .unwrap_err();

    assert!(error.to_string().contains("--approve"));
}

#[test]
fn parse_cli_supports_access_group() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "access",
        "user",
        "list",
        "--json",
        "--token",
        "abc",
    ]);

    match args.command {
        UnifiedCommand::Access(inner) => match inner.command {
            crate::access::AccessCommand::User { command } => match command {
                crate::access::UserCommand::List(list_args) => {
                    assert!(list_args.json);
                    assert_eq!(list_args.common.api_token.as_deref(), Some("abc"));
                }
                _ => panic!("expected user list"),
            },
            _ => panic!("expected access user"),
        },
        _ => panic!("expected access group"),
    }
}

#[test]
fn parse_cli_supports_overview_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "overview",
        "--dashboard-export-dir",
        "./dashboards/raw",
    ]);

    match args.command {
        UnifiedCommand::Overview(inner) => {
            assert!(inner.command.is_none());
            assert_eq!(
                inner.staged.dashboard_export_dir.as_deref(),
                Some(Path::new("./dashboards/raw"))
            );
        }
        _ => panic!("expected overview group"),
    }
}

#[test]
fn parse_cli_supports_overview_live_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "overview", "live"]);

    match args.command {
        UnifiedCommand::Overview(inner) => match inner.command {
            Some(crate::overview::OverviewCommand::Live(_)) => {}
            _ => panic!("expected overview live"),
        },
        _ => panic!("expected overview group"),
    }
}

#[test]
fn parse_cli_supports_overview_live_org_scope_flags() {
    let args: CliArgs = parse_cli_from(["grafana-util", "overview", "live", "--org-id", "7"]);

    match args.command {
        UnifiedCommand::Overview(inner) => match inner.command {
            Some(crate::overview::OverviewCommand::Live(live)) => {
                assert_eq!(live.org_id, Some(7));
                assert!(!live.all_orgs);
            }
            _ => panic!("expected overview live"),
        },
        _ => panic!("expected overview group"),
    }
}

#[test]
fn parse_cli_supports_status_command() {
    let args: CliArgs = parse_cli_from(["grafana-util", "status", "staged"]);

    match args.command {
        UnifiedCommand::Status(_) => {}
        _ => panic!("expected status group"),
    }
}

#[test]
fn parse_cli_supports_status_live_staged_inputs() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "status",
        "live",
        "--sync-summary-file",
        "./sync-summary.json",
        "--bundle-preflight-file",
        "./bundle-preflight.json",
        "--promotion-summary-file",
        "./promotion-summary.json",
        "--mapping-file",
        "./mapping.json",
        "--availability-file",
        "./availability.json",
    ]);

    match args.command {
        UnifiedCommand::Status(inner) => match inner.command {
            crate::project_status_command::ProjectStatusSubcommand::Live(live) => {
                assert_eq!(
                    live.sync_summary_file.as_deref(),
                    Some(Path::new("./sync-summary.json"))
                );
                assert_eq!(
                    live.bundle_preflight_file.as_deref(),
                    Some(Path::new("./bundle-preflight.json"))
                );
                assert_eq!(
                    live.promotion_summary_file.as_deref(),
                    Some(Path::new("./promotion-summary.json"))
                );
                assert_eq!(
                    live.mapping_file.as_deref(),
                    Some(Path::new("./mapping.json"))
                );
                assert_eq!(
                    live.availability_file.as_deref(),
                    Some(Path::new("./availability.json"))
                );
            }
            _ => panic!("expected status live"),
        },
        _ => panic!("expected status group"),
    }
}

#[test]
fn parse_cli_supports_status_live_org_scope_flags() {
    let args: CliArgs = parse_cli_from(["grafana-util", "status", "live", "--all-orgs"]);

    match args.command {
        UnifiedCommand::Status(inner) => match inner.command {
            crate::project_status_command::ProjectStatusSubcommand::Live(live) => {
                assert!(live.all_orgs);
                assert_eq!(live.org_id, None);
            }
            _ => panic!("expected status live"),
        },
        _ => panic!("expected status group"),
    }
}

#[test]
fn unified_help_mentions_alert_access_and_shims() {
    let help = render_unified_help();
    assert!(help.contains("grafana-util access user list"));
    assert!(help.contains("[Alert Export]"));
    assert!(help.contains("[Datasource Inventory]"));
    assert!(help.contains("grafana-util datasource list --input-dir ./datasources --json"));
    assert!(help.contains("Run datasource list, browse-live, export, import, and diff workflows."));
    assert!(help.contains("[Access Local Inventory]"));
    assert!(help.contains("[Change Planning]"));
    assert!(help.contains("[Change Apply]"));
    assert!(help.contains("datasource"));
    assert!(help.contains("grafana-util change preview --fetch-live"));
    assert!(help.contains(
        "grafana-util change apply --preview-file ./change-preview.json --approve --execute-live"
    ));
    assert!(help.contains(
        "Run review-first change workflows with optional live Grafana fetch/apply paths."
    ));
    assert!(help.contains("overview"));
    assert!(help.contains("Summarize project artifacts into a project-wide overview."));
    assert!(help.contains("overview live"));
    assert!(help.contains("Staged overview is the default"));
    assert!(help.contains("status"));
    assert!(help.contains("Render shared project-wide staged or live status."));
    assert!(help.contains("dashboard"));
    assert!(help.contains("[aliases: db]"));
    assert!(help.contains("[aliases: ds]"));
    assert!(!help.contains("Compatibility direct form"));
}

#[test]
fn render_unified_help_text_colorizes_example_labels_when_requested() {
    let help = render_unified_help_text(true);
    assert!(help.contains("\u{1b}[1;36m[Dashboard Export]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;31m[Alert Export]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;32m[Datasource Inventory]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;32m[Datasource Local Inventory]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;33m[Access Local Inventory]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;34m[Change Planning]\u{1b}[0m"));
}

#[test]
fn render_unified_help_text_colorizes_bracketed_usage_tokens_when_requested() {
    let help = render_unified_help_text(true);
    assert!(help.contains("\u{1b}[1m\u{1b}[32mUsage:\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1m\u{1b}[36m<COMMAND>\u{1b}[0m"));
    assert!(help.contains("\u{1b}[33m[aliases: \u{1b}[0m\u{1b}[33mdb\u{1b}[0m\u{1b}[33m]\u{1b}[0m"));
}

#[test]
fn unified_help_full_appends_extended_examples() {
    let help = render_unified_help_full();
    assert!(help.contains("Extended Examples:"));
    assert!(help.contains("[Dashboard Analyze]"));
    assert!(help.contains("[Datasource Diff]"));
    assert!(help.contains("--input-format provisioning"));
    assert!(help.contains("grafana-util change advanced review --plan-file ./sync-plan.json"));
}

#[test]
fn unified_help_full_colorizes_extended_example_labels_when_requested() {
    let help = render_unified_help_full_text(true);
    assert!(help.contains("\u{1b}[1;36m[Dashboard Analyze]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;31m[Alert Import]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;32m[Datasource Import]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;33m[Access Local Inventory]\u{1b}[0m"));
    assert!(help.contains("\u{1b}[1;34m[Change Review]\u{1b}[0m"));
}

#[test]
fn maybe_render_unified_help_from_os_args_handles_root_help_and_help_full_flags() {
    let default_help = maybe_render_unified_help_from_os_args(["grafana-util"], false).unwrap();
    assert!(default_help.contains("[Dashboard Export]"));
    assert!(default_help.contains("Print help with extended examples"));

    let default_help_colorized =
        maybe_render_unified_help_from_os_args(["grafana-util"], true).unwrap_or_default();
    assert!(default_help_colorized.contains("\u{1b}[1;36m[Dashboard Export]\u{1b}[0m"));

    let root_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "--help"], false).unwrap();
    assert!(root_help.contains("[Dashboard Export]"));
    assert!(root_help.contains("--help-full"));

    let short_help = maybe_render_unified_help_from_os_args(["grafana-util", "-h"], false).unwrap();
    assert!(short_help.contains("[Change Apply]"));
    assert!(short_help.contains("Print help with extended examples"));

    let full_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "--help-full"], false).unwrap();
    assert!(full_help.contains("Extended Examples:"));
    assert!(full_help.contains("[Alert Import]"));
    assert!(full_help.contains(
        "Open a local snapshot inventory in the interactive browser with `--interactive`:"
    ));
    assert!(full_help.contains("grafana-util snapshot review --input-dir ./snapshot --interactive"));

    let alert_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "alert", "--help-full"], false)
            .unwrap();
    assert!(alert_help.contains("Extended Examples:"));
    assert!(alert_help.contains("[Alert List]"));
    assert!(alert_help.contains("[Alert Plan]"));
    assert!(alert_help.contains("[Alert Apply]"));
    assert!(alert_help.contains("[Alert Delete]"));
    assert!(alert_help.contains("[Alert Add Rule]"));
    assert!(alert_help.contains("[Alert Clone Rule]"));
    assert!(alert_help.contains("[Alert Add Contact Point]"));
    assert!(alert_help.contains("[Alert Set Route]"));
    assert!(alert_help.contains("[Alert Preview Route]"));
    assert!(alert_help.contains("alert import --url http://localhost:3000 --input-dir ./alerts/raw --replace-existing --dry-run --json"));
    assert!(alert_help.contains(
        "alert diff --url http://localhost:3000 --diff-dir ./alerts/raw --output-format json"
    ));
    assert!(alert_help.contains("alert plan --desired-dir ./alerts/desired --prune --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json --output-format json"));
    assert!(alert_help.contains("alert apply --plan-file ./alert-plan-reviewed.json --approve"));
    assert!(alert_help
        .contains("alert delete --kind policy-tree --identity default --allow-policy-reset"));
    assert!(alert_help.contains("alert add-rule --desired-dir ./alerts/desired --name cpu-high"));
    assert!(alert_help.contains("--dry-run"));
    assert!(alert_help.contains("alert preview-route --desired-dir ./alerts/desired --label team=platform --severity critical"));
    assert!(alert_help.contains("fully replaced on rerun instead of merged field-by-field"));
    assert!(alert_help.contains("low-level rule scaffold"));

    let datasource_help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "datasource", "--help-full"],
        false,
    )
    .unwrap();
    assert!(datasource_help.contains("[Datasource Diff]"));

    let access_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "access", "--help-full"], false)
            .unwrap();
    assert!(access_help.contains("[Access Token Add]"));

    let change_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "change", "--help-full"], false)
            .unwrap();
    assert!(change_help.contains("[Change Apply]"));
    assert!(change_help.contains("[Change Bundle]"));
    assert!(change_help.contains("[Change Bundle Preflight]"));

    let alert_short_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "alert", "-h"], false).unwrap();
    assert!(alert_short_help.contains("--help-full"));
    assert!(alert_short_help.contains("plan"));
    assert!(alert_short_help.contains("apply"));
    assert!(alert_short_help.contains("delete"));
    assert!(alert_short_help.contains("add-rule"));
    assert!(alert_short_help.contains("clone-rule"));
    assert!(alert_short_help.contains("add-contact-point"));
    assert!(alert_short_help.contains("set-route"));
    assert!(alert_short_help.contains("preview-route"));
    assert!(alert_short_help.contains("init"));
    assert!(alert_short_help.contains("new-rule"));
    assert!(alert_short_help.contains("new-contact-point"));
    assert!(alert_short_help.contains("new-template"));

    let datasource_short_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "datasource", "-h"], false)
            .unwrap();
    assert!(datasource_short_help.contains("--help-full"));

    let access_short_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "access", "-h"], false).unwrap();
    assert!(access_short_help.contains("--help-full"));

    let snapshot_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "snapshot", "--help"], false)
            .unwrap();
    assert!(snapshot_help.contains("Export and review Grafana snapshot inventory bundles."));
    assert!(snapshot_help.contains("Review a local snapshot inventory without touching Grafana."));
    assert!(snapshot_help
        .contains("grafana-util snapshot review --input-dir ./snapshot --output-format table"));
    assert!(
        snapshot_help.contains("grafana-util snapshot review --input-dir ./snapshot --interactive")
    );
    assert!(!snapshot_help.contains("overview"));

    let snapshot_review_help = render_snapshot_subcommand_help(&["review"]);
    assert!(snapshot_review_help.contains(
        "Render the snapshot inventory review as table, csv, text, json, yaml, or interactive browser output."
    ));
    assert!(snapshot_review_help.contains("Shortcut for --output-format interactive."));
    assert!(snapshot_review_help
        .contains("grafana-util snapshot review --input-dir ./snapshot --output-format table"));
    assert!(snapshot_review_help
        .contains("grafana-util snapshot review --input-dir ./snapshot --output-format csv"));
    assert!(snapshot_review_help
        .contains("grafana-util snapshot review --input-dir ./snapshot --output-format text"));
    assert!(snapshot_review_help
        .contains("grafana-util snapshot review --input-dir ./snapshot --output-format json"));
    assert!(snapshot_review_help
        .contains("grafana-util snapshot review --input-dir ./snapshot --output-format yaml"));
    assert!(snapshot_review_help
        .contains("grafana-util snapshot review --input-dir ./snapshot --interactive"));

    let overview_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "overview", "--help-full"], false)
            .unwrap();
    assert!(overview_help.contains("overview"));
    assert!(overview_help.contains("project-wide overview"));
    assert!(overview_help.contains("overview live"));
    assert!(overview_help.contains("shared live status"));

    let project_status_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "status", "--help-full"], false)
            .unwrap();
    assert!(project_status_help.contains("status"));
    assert!(project_status_help
        .contains("Render project-wide staged or live status through the shared status contract."));
    assert!(project_status_help.contains("staged"));
    assert!(project_status_help.contains("live"));
    assert!(project_status_help.contains("Schema guide:"));
    assert!(project_status_help.contains("grafana-util status --help-schema"));

    let project_status_schema_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "status", "--help-schema"], false)
            .unwrap();
    assert!(project_status_schema_help.contains("Status JSON schema guide"));
    assert!(project_status_schema_help.contains("grafana-util-project-status"));
    assert!(project_status_schema_help.contains("grafana-util status staged --help-schema"));
    assert!(project_status_schema_help.contains("grafana-util status live --help-schema"));

    let project_status_staged_schema_help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "status", "staged", "--help-schema"],
        false,
    )
    .unwrap();
    assert!(project_status_staged_schema_help.contains("Status staged JSON schema"));
    assert!(project_status_staged_schema_help.contains("grafana-util-project-status"));
    assert!(project_status_staged_schema_help.contains(
        "grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output-format json"
    ));

    let project_status_live_schema_help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "status", "live", "--help-schema"],
        false,
    )
    .unwrap();
    assert!(project_status_live_schema_help.contains("Status live JSON schema"));
    assert!(project_status_live_schema_help.contains("grafana-util-project-status"));
    assert!(project_status_live_schema_help
        .contains("grafana-util status live --profile prod --output-format json"));

    let change_short_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "change", "-h"], false).unwrap();
    assert!(change_short_help.contains("--help-full"));

    let dashboard_short_help =
        maybe_render_unified_help_from_os_args(["grafana-util", "dashboard", "--help"], false)
            .unwrap();
    assert!(dashboard_short_help.contains("Usage: grafana-util dashboard <COMMAND>"));
    assert!(dashboard_short_help.contains("Choose the task first:"));
    assert!(dashboard_short_help.contains("Work with dashboard trees:"));
    assert!(dashboard_short_help.contains("Work with local drafts:"));
    assert!(dashboard_short_help.contains("Analyze and review risk:"));
    assert!(dashboard_short_help
        .contains("analyze          Analyze live Grafana or a local export tree"));
    assert!(dashboard_short_help.contains("Move dashboards:"));
    assert!(dashboard_short_help
        .contains("fetch-live       Fetch one live dashboard into an API-safe local JSON draft."));
    assert!(dashboard_short_help.contains(
        "clone-live       Clone one live dashboard into a local draft with optional overrides."
    ));
    assert!(dashboard_short_help.contains("screenshot       Open one dashboard in a headless browser and capture image or PDF output."));
    assert!(dashboard_short_help.contains("grafana-util dashboard <COMMAND> --help-full"));
    assert!(maybe_render_unified_help_from_os_args(
        ["grafana-util", "dashboard", "--help-full"],
        false
    )
    .is_none());
    assert!(maybe_render_unified_help_from_os_args(
        ["grafana-util", "alert", "export", "--help-full"],
        false
    )
    .is_none());
}

#[test]
fn maybe_render_unified_help_from_os_args_supports_change_schema_root() {
    assert_manifest_driven_schema_help(&["grafana-util", "change", "--help-schema"], "change");
}

#[test]
fn maybe_render_unified_help_from_os_args_supports_change_subcommand_schema_help() {
    assert_manifest_driven_schema_help(&["grafana-util", "change", "--help-schema"], "change");
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "change", "apply", "--help-schema"],
        false,
    )
    .unwrap();
    assert!(help.contains("Change apply JSON schema"));
    assert!(help.contains("grafana-utils-sync-apply-intent"));
    assert!(help.contains("Live execute shape (`--execute-live`)"));
    assert!(help.contains("appliedCount"));
    assert!(help.contains("results[]"));
}

#[test]
fn maybe_render_unified_help_from_os_args_supports_dashboard_history_schema_root() {
    assert_manifest_driven_schema_help(
        &["grafana-util", "dashboard", "history", "--help-schema"],
        "dashboard-history",
    );
}

#[test]
fn maybe_render_unified_help_from_os_args_supports_dashboard_history_subcommand_schema_help() {
    assert_manifest_driven_schema_help(
        &["grafana-util", "dashboard", "history", "--help-schema"],
        "dashboard-history",
    );
    let restore_help = maybe_render_unified_help_from_os_args(
        [
            "grafana-util",
            "dashboard",
            "history",
            "restore",
            "--help-schema",
        ],
        false,
    )
    .unwrap();
    assert!(restore_help.contains("Dashboard history restore JSON schema"));
    assert!(restore_help.contains("grafana-util-dashboard-history-restore"));
    assert!(restore_help.contains("createsNewRevision"));
    assert!(restore_help.contains("A live restore still creates a new latest revision"));

    let diff_help = maybe_render_unified_help_from_os_args(
        [
            "grafana-util",
            "dashboard",
            "history",
            "diff",
            "--help-schema",
        ],
        false,
    )
    .unwrap();
    assert!(diff_help.contains("Dashboard history diff JSON schema"));
    assert!(diff_help.contains("grafana-util-dashboard-history-diff"));
    assert!(diff_help.contains("baseVersion"));
    assert!(diff_help.contains("newVersion"));

    let export_help = maybe_render_unified_help_from_os_args(
        [
            "grafana-util",
            "dashboard",
            "history",
            "export",
            "--help-schema",
        ],
        false,
    )
    .unwrap();
    assert!(export_help.contains("Dashboard history export JSON schema"));
    assert!(export_help.contains("grafana-util-dashboard-history-export"));
    assert!(export_help.contains("versions[]"));
    assert!(export_help.contains("dashboard"));
}

#[test]
fn maybe_render_unified_help_from_os_args_supports_dashboard_diff_schema_help() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "dashboard", "diff", "--help-schema"],
        false,
    )
    .unwrap();
    assert!(help.contains("Dashboard diff JSON schema"));
    assert_help_matches_shared_diff_contract(&help, "dashboard");
}

#[test]
fn maybe_render_unified_help_from_os_args_supports_alert_diff_schema_help() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "alert", "diff", "--help-schema"],
        false,
    )
    .unwrap();
    assert!(help.contains("Alert diff JSON schema"));
    assert_help_matches_shared_diff_contract(&help, "alert");
    assert!(help.contains("--output-format json"));
    assert!(help.contains("--json"));
}

#[test]
fn maybe_render_unified_help_from_os_args_supports_datasource_diff_schema_help() {
    let help = maybe_render_unified_help_from_os_args(
        ["grafana-util", "datasource", "diff", "--help-schema"],
        false,
    )
    .unwrap();
    assert!(help.contains("Datasource diff JSON schema"));
    assert_help_matches_shared_diff_contract(&help, "datasource");
}

#[test]
fn alert_help_subcommands_document_management_flags_and_examples() {
    let plan_help = render_alert_subcommand_help(&["plan"]);
    assert!(plan_help.contains("--desired-dir"));
    assert!(plan_help.contains("--prune"));
    assert!(plan_help.contains("--dashboard-uid-map"));
    assert!(plan_help.contains("--panel-id-map"));
    assert!(plan_help.contains("--output-format"));
    assert!(plan_help.contains("grafana-util alert plan"));

    let apply_help = render_alert_subcommand_help(&["apply"]);
    assert!(apply_help.contains("--plan-file"));
    assert!(apply_help.contains("--approve"));
    assert!(apply_help.contains("grafana-util alert apply"));

    let delete_help = render_alert_subcommand_help(&["delete"]);
    assert!(delete_help.contains("--kind"));
    assert!(delete_help.contains("--identity"));
    assert!(delete_help.contains("--allow-policy-reset"));
    assert!(delete_help.contains("grafana-util alert delete"));

    let init_help = render_alert_subcommand_help(&["init"]);
    assert!(init_help.contains("--desired-dir"));
    assert!(init_help.contains("grafana-util alert init"));

    let add_rule_help = render_alert_subcommand_help(&["add-rule"]);
    assert!(add_rule_help.contains("--desired-dir"));
    assert!(add_rule_help.contains("--name"));
    assert!(add_rule_help.contains("--folder"));
    assert!(add_rule_help.contains("--rule-group"));
    assert!(add_rule_help.contains("--receiver"));
    assert!(add_rule_help.contains("--no-route"));
    assert!(add_rule_help.contains("--label"));
    assert!(add_rule_help.contains("--annotation"));
    assert!(add_rule_help.contains("--severity"));
    assert!(add_rule_help.contains("--for"));
    assert!(add_rule_help.contains("--expr"));
    assert!(add_rule_help.contains("--threshold"));
    assert!(add_rule_help.contains("--above"));
    assert!(add_rule_help.contains("--below"));
    assert!(add_rule_help.contains("--dry-run"));
    assert!(add_rule_help.contains("grafana-util alert add-rule"));

    let clone_rule_help = render_alert_subcommand_help(&["clone-rule"]);
    assert!(clone_rule_help.contains("--desired-dir"));
    assert!(clone_rule_help.contains("--source"));
    assert!(clone_rule_help.contains("--name"));
    assert!(clone_rule_help.contains("--dry-run"));
    assert!(clone_rule_help.contains("grafana-util alert clone-rule"));

    let add_contact_point_help = render_alert_subcommand_help(&["add-contact-point"]);
    assert!(add_contact_point_help.contains("--desired-dir"));
    assert!(add_contact_point_help.contains("--name"));
    assert!(add_contact_point_help.contains("--dry-run"));
    assert!(add_contact_point_help.contains("grafana-util alert add-contact-point"));

    let set_route_help = render_alert_subcommand_help(&["set-route"]);
    assert!(set_route_help.contains("--desired-dir"));
    assert!(set_route_help.contains("--receiver"));
    assert!(set_route_help.contains("--label"));
    assert!(set_route_help.contains("--severity"));
    assert!(set_route_help.contains("--dry-run"));
    assert!(set_route_help.contains("fully replaces that managed route instead of merging fields"));
    assert!(set_route_help.contains("grafana-util alert set-route"));

    let preview_route_help = render_alert_subcommand_help(&["preview-route"]);
    assert!(preview_route_help.contains("--desired-dir"));
    assert!(preview_route_help.contains("--label"));
    assert!(preview_route_help.contains("--severity"));
    assert!(preview_route_help.contains("fully replaces the tool-owned route on rerun"));
    assert!(preview_route_help.contains("grafana-util alert preview-route"));

    let new_rule_help = render_alert_subcommand_help(&["new-rule"]);
    assert!(new_rule_help.contains("--desired-dir"));
    assert!(new_rule_help.contains("--name"));
    assert!(new_rule_help.contains("low-level staged alert rule scaffold"));
    assert!(new_rule_help.contains("grafana-util alert new-rule"));
}

#[test]
fn parse_cli_rejects_legacy_dashboard_direct_command() {
    let error = CliArgs::try_parse_from(["grafana-util", "list-dashboard", "--json"]).unwrap_err();

    assert!(error.to_string().contains("unrecognized subcommand"));
    assert!(error.to_string().contains("list-dashboard"));
}

#[test]
fn parse_cli_rejects_legacy_alert_direct_command() {
    let error =
        CliArgs::try_parse_from(["grafana-util", "export-alert", "--output-dir", "./alerts"])
            .unwrap_err();

    assert!(error.to_string().contains("unrecognized subcommand"));
    assert!(error.to_string().contains("export-alert"));
}

#[test]
fn parse_cli_supports_change_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
        "inspect",
        "--dashboard-export-dir",
        "./dashboards/raw",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Inspect(inner) => {
                assert_eq!(
                    inner.inputs.dashboard_export_dir,
                    Some(Path::new("./dashboards/raw").to_path_buf())
                );
                assert_eq!(inner.output.output_format, SyncOutputFormat::Json);
            }
            _ => panic!("expected change inspect"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_assess_alerts_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
        "advanced",
        "assess-alerts",
        "--alerts-file",
        "./alerts.json",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Advanced(inner) => match inner.command {
                SyncAdvancedCommand::AssessAlerts(inner) => {
                    assert_eq!(inner.alerts_file, Path::new("./alerts.json"));
                    assert_eq!(inner.output_format, SyncOutputFormat::Json);
                }
                _ => panic!("expected change advanced assess-alerts"),
            },
            _ => panic!("expected change advanced"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_plan_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
        "preview",
        "--desired-file",
        "./desired.json",
        "--live-file",
        "./live.json",
        "--trace-id",
        "trace-explicit",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Preview(inner) => {
                assert_eq!(
                    inner.inputs.desired_file,
                    Some(Path::new("./desired.json").to_path_buf())
                );
                assert_eq!(
                    inner.live_file,
                    Some(Path::new("./live.json").to_path_buf())
                );
                assert_eq!(inner.trace_id, Some("trace-explicit".to_string()));
                assert_eq!(inner.output.output_format, SyncOutputFormat::Json);
            }
            _ => panic!("expected change preview"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_plan_fetch_live_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
        "preview",
        "--desired-file",
        "./desired.json",
        "--fetch-live",
        "--org-id",
        "7",
        "--page-size",
        "250",
        "--url",
        "http://localhost:3000",
        "--token",
        "token-value",
    ]);

    match args.command {
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Preview(inner) => {
                assert_eq!(
                    inner.inputs.desired_file,
                    Some(Path::new("./desired.json").to_path_buf())
                );
                assert!(inner.fetch_live);
                assert_eq!(inner.org_id, Some(7));
                assert_eq!(inner.page_size, 250);
                assert_eq!(inner.common.url, "http://localhost:3000");
                assert_eq!(inner.common.api_token, Some("token-value".to_string()));
            }
            _ => panic!("expected change preview"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_apply_group_command_with_reason_and_note() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
        "apply",
        "--plan-file",
        "./plan.json",
        "--approve",
        "--approval-reason",
        "change-approved",
        "--apply-note",
        "local apply intent only",
    ]);

    match args.command {
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Apply(inner) => {
                assert_eq!(inner.approval_reason, Some("change-approved".to_string()));
                assert_eq!(
                    inner.apply_note,
                    Some("local apply intent only".to_string())
                );
            }
            _ => panic!("expected change apply"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_apply_execute_live_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
        "apply",
        "--plan-file",
        "./plan.json",
        "--approve",
        "--execute-live",
        "--allow-folder-delete",
        "--org-id",
        "9",
        "--url",
        "http://localhost:3000",
        "--token",
        "token-value",
    ]);

    match args.command {
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Apply(inner) => {
                assert_eq!(inner.plan_file.as_deref(), Some(Path::new("./plan.json")));
                assert!(inner.approve);
                assert!(inner.execute_live);
                assert!(inner.allow_folder_delete);
                assert_eq!(inner.org_id, Some(9));
                assert_eq!(inner.common.url, "http://localhost:3000");
                assert_eq!(inner.common.api_token, Some("token-value".to_string()));
            }
            _ => panic!("expected change apply"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn parse_cli_supports_change_review_group_command() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
        "advanced",
        "review",
        "--plan-file",
        "./plan.json",
        "--review-token",
        "reviewed-change-plan",
        "--output-format",
        "json",
    ]);

    match args.command {
        UnifiedCommand::Change { command } => match command {
            SyncGroupCommand::Advanced(inner) => match inner.command {
                SyncAdvancedCommand::Review(inner) => {
                    assert_eq!(inner.plan_file, Path::new("./plan.json"));
                    assert_eq!(inner.review_token, DEFAULT_REVIEW_TOKEN);
                    assert_eq!(inner.output_format, SyncOutputFormat::Json);
                    assert_eq!(inner.reviewed_by, None);
                    assert_eq!(inner.reviewed_at, None);
                    assert_eq!(inner.review_note, None);
                }
                _ => panic!("expected change advanced review"),
            },
            _ => panic!("expected change advanced"),
        },
        _ => panic!("expected change group"),
    }
}

#[test]
fn dispatch_routes_dashboard_group_to_dashboard_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "diff",
        "--input-dir",
        "./dashboards/raw",
    ]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |dashboard_args| {
            routed.borrow_mut().push(match dashboard_args.command {
                DashboardCommand::Diff(_) => "dashboard-diff".to_string(),
                _ => "other".to_string(),
            });
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
        |_migrate_args| {
            routed.borrow_mut().push("migrate".to_string());
            Ok(())
        },
        |_profile_args| {
            routed.borrow_mut().push("profile".to_string());
            Ok(())
        },
        |_snapshot_args| {
            routed.borrow_mut().push("snapshot".to_string());
            Ok(())
        },
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["dashboard-diff".to_string()]);
}

#[test]
fn dispatch_routes_dashboard_review_group_to_dashboard_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "dashboard",
        "review",
        "--input",
        "./drafts/cpu-main.json",
    ]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |dashboard_args| {
            routed.borrow_mut().push(match dashboard_args.command {
                DashboardCommand::Review(_) => "dashboard-review".to_string(),
                _ => "other".to_string(),
            });
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
        |_migrate_args| {
            routed.borrow_mut().push("migrate".to_string());
            Ok(())
        },
        |_profile_args| {
            routed.borrow_mut().push("profile".to_string());
            Ok(())
        },
        |_snapshot_args| {
            routed.borrow_mut().push("snapshot".to_string());
            Ok(())
        },
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["dashboard-review".to_string()]);
}

#[test]
fn dispatch_routes_snapshot_group_to_snapshot_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "snapshot",
        "export",
        "--output-dir",
        "./snapshot",
    ]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
        |_migrate_args| {
            routed.borrow_mut().push("migrate".to_string());
            Ok(())
        },
        |_profile_args| {
            routed.borrow_mut().push("profile".to_string());
            Ok(())
        },
        |_snapshot_args| {
            routed.borrow_mut().push("snapshot".to_string());
            Ok(())
        },
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["snapshot".to_string()]);
}

#[test]
fn dispatch_routes_access_group_to_access_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "access",
        "service-account",
        "list",
        "--json",
        "--token",
        "abc",
    ]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
        |_migrate_args| {
            routed.borrow_mut().push("migrate".to_string());
            Ok(())
        },
        |_profile_args| {
            routed.borrow_mut().push("profile".to_string());
            Ok(())
        },
        |_snapshot_args| {
            routed.borrow_mut().push("snapshot".to_string());
            Ok(())
        },
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["access".to_string()]);
}

#[test]
fn dispatch_routes_overview_group_to_overview_handler() {
    let args: CliArgs = parse_cli_from(["grafana-util", "overview"]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
        |_migrate_args| {
            routed.borrow_mut().push("migrate".to_string());
            Ok(())
        },
        |_profile_args| {
            routed.borrow_mut().push("profile".to_string());
            Ok(())
        },
        |_snapshot_args| {
            routed.borrow_mut().push("snapshot".to_string());
            Ok(())
        },
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["overview".to_string()]);
}

#[test]
fn dispatch_routes_change_group_to_change_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "change",
        "preflight",
        "--desired-file",
        "./desired.json",
    ]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
        |_migrate_args| {
            routed.borrow_mut().push("migrate".to_string());
            Ok(())
        },
        |_profile_args| {
            routed.borrow_mut().push("profile".to_string());
            Ok(())
        },
        |_snapshot_args| {
            routed.borrow_mut().push("snapshot".to_string());
            Ok(())
        },
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["change".to_string()]);
}

#[test]
fn dispatch_routes_datasource_group_to_datasource_handler() {
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "datasource",
        "list",
        "--json",
        "--token",
        "abc",
    ]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
        |_migrate_args| {
            routed.borrow_mut().push("migrate".to_string());
            Ok(())
        },
        |_profile_args| {
            routed.borrow_mut().push("profile".to_string());
            Ok(())
        },
        |_snapshot_args| {
            routed.borrow_mut().push("snapshot".to_string());
            Ok(())
        },
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["datasource".to_string()]);
}

#[test]
fn dispatch_routes_status_group_to_status_handler() {
    let args: CliArgs = parse_cli_from(["grafana-util", "status", "staged"]);
    let routed = RefCell::new(Vec::new());

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| {
            routed.borrow_mut().push("datasource".to_string());
            Ok(())
        },
        |_change_args| {
            routed.borrow_mut().push("change".to_string());
            Ok(())
        },
        |_alert_args| {
            routed.borrow_mut().push("alert".to_string());
            Ok(())
        },
        |_access_args| {
            routed.borrow_mut().push("access".to_string());
            Ok(())
        },
        |_migrate_args| {
            routed.borrow_mut().push("migrate".to_string());
            Ok(())
        },
        |_profile_args| {
            routed.borrow_mut().push("profile".to_string());
            Ok(())
        },
        |_snapshot_args| {
            routed.borrow_mut().push("snapshot".to_string());
            Ok(())
        },
        |_overview_args| {
            routed.borrow_mut().push("overview".to_string());
            Ok(())
        },
        |_project_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["status".to_string()]);
}

#[test]
fn overview_live_help_exposes_shared_live_status_contract() {
    let mut command = crate::overview::OverviewCliArgs::command();
    let live_command = command
        .find_subcommand_mut("live")
        .expect("overview live subcommand should exist");
    let help = live_command.render_long_help().to_string();

    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("grafana-util overview live"));
}

#[test]
fn status_live_help_exposes_live_org_scope_contract() {
    let mut command = crate::project_status_command::ProjectStatusCliArgs::command();
    let live_command = command
        .find_subcommand_mut("live")
        .expect("status live subcommand should exist");
    let help = live_command.render_long_help().to_string();

    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("grafana-util status live"));
}
