//! Alert domain test suite.
//! Validates export/import/diff document shaping, kind detection, and CLI parser/help
//! behavior.
use super::alert_list::serialize_contact_point_list_rows;
use super::{
    build_alert_delete_preview_from_files, build_alert_diff_document,
    build_alert_import_dry_run_document, build_alert_live_project_status_domain,
    build_alert_plan_document, build_alert_plan_with_request, build_alert_project_status_domain,
    build_compare_diff_text, build_contact_point_export_document, build_contact_point_output_path,
    build_empty_root_index, build_import_operation, build_rule_export_document,
    build_rule_output_path, detect_document_kind, determine_import_action_with_request,
    execute_alert_plan_with_request, expect_object_list, fetch_live_compare_document_with_request,
    get_rule_linkage, import_resource_document_with_request, init_alert_runtime_layout,
    load_alert_resource_file, load_panel_id_map, load_string_map, parse_cli_from,
    parse_template_list_response, root_command, run_alert_cli, serialize_compare_document,
    serialize_rule_list_rows, write_new_contact_point_scaffold, write_new_rule_scaffold,
    write_new_template_scaffold, AlertCliArgs, AlertLiveProjectStatusInputs, CONTACT_POINT_KIND,
    MUTE_TIMING_KIND, POLICIES_KIND, ROOT_INDEX_KIND, RULE_KIND, TEMPLATE_KIND, TOOL_API_VERSION,
    TOOL_SCHEMA_VERSION,
};
use crate::common::api_response;
use crate::common::{message, DiffOutputFormat, Result, TOOL_VERSION};
use reqwest::Method;
use serde_json::json;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn render_alert_help() -> String {
    let mut command = root_command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn render_alert_subcommand_help(path: &[&str]) -> String {
    let mut command = root_command();
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

fn load_alert_export_contract_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../fixtures/alert_export_contract_cases.json"
    ))
    .unwrap()
}

fn load_alert_recreate_contract_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../fixtures/alert_recreate_contract_cases.json"
    ))
    .unwrap()
}

fn load_shared_diff_golden_fixture(domain: &str) -> Value {
    serde_json::from_str::<Vec<Value>>(include_str!("../../fixtures/shared_diff_golden_cases.json"))
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

fn write_pretty_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(value).unwrap()),
    )
    .unwrap();
}

fn assert_exact_object_keys(value: &Value, expected_keys: &[&str]) {
    let object = value.as_object().expect("expected JSON object");
    let actual_keys = object.keys().cloned().collect::<BTreeSet<_>>();
    let expected_keys = expected_keys
        .iter()
        .map(|key| (*key).to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_keys, expected_keys);
}

#[test]
fn build_rule_output_path_keeps_folder_structure() {
    let rule = json!({
        "folderUID": "infra folder",
        "ruleGroup": "CPU Alerts",
        "title": "DB CPU > 90%",
        "uid": "rule-1",
    });
    let path = build_rule_output_path(
        Path::new("alerts/raw/rules"),
        rule.as_object().unwrap(),
        false,
    );
    assert_eq!(
        path,
        Path::new("alerts/raw/rules/infra_folder/CPU_Alerts/DB_CPU_90__rule-1.json")
    );
}

#[test]
fn build_alert_project_status_domain_is_partial_without_core_counts() {
    let summary_document = json!({
        "summary": {
            "ruleCount": 0,
            "contactPointCount": 0,
            "policyCount": 0,
            "muteTimingCount": 2,
            "templateCount": 1
        }
    });
    let domain = build_alert_project_status_domain(Some(&summary_document)).unwrap();
    let value = serde_json::to_value(domain).unwrap();

    assert_eq!(value["id"], json!("alert"));
    assert_eq!(value["scope"], json!("staged"));
    assert_eq!(value["mode"], json!("artifact-summary"));
    assert_eq!(value["status"], json!("partial"));
    assert_eq!(value["reasonCode"], json!("partial-no-data"));
    assert_eq!(value["primaryCount"], json!(0));
    assert_eq!(value["blockerCount"], json!(0));
    assert_eq!(value["warningCount"], json!(0));
    assert_eq!(value["sourceKinds"], json!(["alert-export"]));
    assert_eq!(
        value["signalKeys"],
        json!([
            "summary.ruleCount",
            "summary.contactPointCount",
            "summary.policyCount",
            "summary.muteTimingCount",
            "summary.templateCount",
        ])
    );
    assert_eq!(value["blockers"], json!([]));
    assert_eq!(value["warnings"], json!([]));
    assert_eq!(
        value["nextActions"],
        json!(["export at least one alert rule, contact point, or policy"])
    );
}

#[test]
fn build_alert_project_status_domain_is_ready_from_core_counts() {
    let summary_document = json!({
        "summary": {
            "ruleCount": 4,
            "contactPointCount": 2,
            "policyCount": 3,
            "muteTimingCount": 1,
            "templateCount": 5
        }
    });
    let domain = build_alert_project_status_domain(Some(&summary_document)).unwrap();
    let value = serde_json::to_value(domain).unwrap();

    assert_eq!(value["status"], json!("ready"));
    assert_eq!(value["reasonCode"], json!("ready"));
    assert_eq!(value["primaryCount"], json!(4));
    assert_eq!(
        value["nextActions"],
        json!(["re-run alert export after alerting changes"])
    );
}

#[test]
fn build_alert_live_project_status_domain_is_ready_from_live_counts() {
    let rules = json!([{"uid": "cpu-high"}]);
    let contact_points = json!([{"uid": "cp-main"}]);
    let mute_timings = json!([{"name": "off-hours"}]);
    let policies = json!({"receiver": "grafana-default-email"});
    let templates = json!([{"name": "slack.default"}]);

    let domain = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
        rules_document: Some(&rules),
        contact_points_document: Some(&contact_points),
        mute_timings_document: Some(&mute_timings),
        policies_document: Some(&policies),
        templates_document: Some(&templates),
    })
    .unwrap();
    let value = serde_json::to_value(domain).unwrap();

    assert_eq!(value["scope"], json!("live"));
    assert_eq!(value["mode"], json!("live-alert-surfaces"));
    assert_eq!(value["primaryCount"], json!(5));
    assert_eq!(
        value["sourceKinds"],
        json!([
            "alert",
            "alert-contact-point",
            "alert-mute-timing",
            "alert-policy",
            "alert-template"
        ])
    );
}

#[test]
fn build_contact_point_output_path_uses_name_and_uid() {
    let contact_point = json!({
        "name": "Webhook Main",
        "uid": "cp-uid",
    });
    let path = build_contact_point_output_path(
        Path::new("alerts/raw/contact-points"),
        contact_point.as_object().unwrap(),
        false,
    );
    assert_eq!(
        path,
        Path::new("alerts/raw/contact-points/Webhook_Main/Webhook_Main__cp-uid.json")
    );
}

#[test]
fn build_rule_export_document_strips_server_managed_fields() {
    let document = build_rule_export_document(
        json!({
            "uid": "rule-uid",
            "title": "CPU High",
            "folderUID": "infra-folder",
            "ruleGroup": "cpu-alerts",
            "condition": "C",
            "data": [],
            "updated": "2026-03-10T10:00:00Z",
            "provenance": "api"
        })
        .as_object()
        .unwrap(),
    );
    assert_eq!(document["kind"], RULE_KIND);
    assert_eq!(document["toolVersion"], json!(TOOL_VERSION));
    assert!(document["spec"].get("updated").is_none());
    assert!(document["spec"].get("provenance").is_none());
}

#[test]
fn detect_document_kind_accepts_plain_contact_point_shape() {
    let kind = detect_document_kind(
        json!({
            "name": "Webhook Main",
            "type": "webhook",
            "settings": {"url": "http://127.0.0.1/notify"}
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();
    assert_eq!(kind, CONTACT_POINT_KIND);
}

#[test]
fn build_import_operation_accepts_plain_rule_document() {
    let (kind, payload) = build_import_operation(&json!({
        "uid": "rule-uid",
        "title": "CPU High",
        "folderUID": "infra-folder",
        "ruleGroup": "cpu-alerts",
        "condition": "C",
        "data": [],
    }))
    .unwrap();
    assert_eq!(kind, RULE_KIND);
    assert_eq!(payload["title"], "CPU High");
}

#[test]
fn build_contact_point_export_document_wraps_tool_document() {
    let document = build_contact_point_export_document(
        json!({
            "uid": "cp-uid",
            "name": "Webhook Main",
            "type": "webhook",
            "settings": {"url": "http://127.0.0.1/notify"},
            "provenance": "api"
        })
        .as_object()
        .unwrap(),
    );
    assert_eq!(document["kind"], CONTACT_POINT_KIND);
    assert!(document["spec"].get("provenance").is_none());
}

#[test]
fn get_rule_linkage_returns_typed_dashboard_and_panel_ids() {
    let linkage = get_rule_linkage(
        json!({
            "annotations": {
                "__dashboardUid__": "dash-uid",
                "__panelId__": 7
            }
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();
    assert_eq!(linkage.dashboard_uid, "dash-uid");
    assert_eq!(linkage.panel_id.as_deref(), Some("7"));
}

#[test]
fn load_string_map_returns_empty_map_without_input_file() {
    let mapping = load_string_map(None, "Dashboard UID map").unwrap();
    assert!(mapping.is_empty());
}

#[test]
fn load_panel_id_map_parses_nested_dashboard_panel_mapping() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("panel-map.json");
    fs::write(
        &path,
        serde_json::to_string_pretty(&json!({
            "source-dashboard": {
                "7": "17",
                "8": 18
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let mapping = load_panel_id_map(Some(&path)).unwrap();

    assert_eq!(
        mapping
            .get("source-dashboard")
            .and_then(|items| items.get("7"))
            .map(String::as_str),
        Some("17")
    );
    assert_eq!(
        mapping
            .get("source-dashboard")
            .and_then(|items| items.get("8"))
            .map(String::as_str),
        Some("18")
    );
}

#[test]
fn parse_cli_supports_diff_dir_and_dry_run() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-alert-utils",
        "diff",
        "--url",
        "https://grafana.example.com",
        "--diff-dir",
        "./alerts/raw",
    ]);
    assert_eq!(args.url, "https://grafana.example.com");
    assert_eq!(args.diff_dir.as_deref(), Some(Path::new("./alerts/raw")));
    assert!(args.input_dir.is_none());
    assert!(!args.dry_run);
}

#[test]
fn parse_cli_supports_diff_json() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-util alert",
        "diff",
        "--diff-dir",
        "./alerts/raw",
        "--json",
    ]);
    assert_eq!(args.diff_dir.as_deref(), Some(Path::new("./alerts/raw")));
    assert!(args.json);
    assert_eq!(args.diff_output, Some(DiffOutputFormat::Json));
}

#[test]
fn parse_cli_supports_diff_output_format_json() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-util alert",
        "diff",
        "--diff-dir",
        "./alerts/raw",
        "--output-format",
        "json",
    ]);
    assert_eq!(args.diff_dir.as_deref(), Some(Path::new("./alerts/raw")));
    assert!(args.json);
    assert_eq!(args.diff_output, Some(DiffOutputFormat::Json));
}

#[test]
fn parse_cli_supports_preferred_auth_aliases() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-alert-utils",
        "--token",
        "abc123",
        "--basic-user",
        "user",
        "--basic-password",
        "pass",
    ]);
    assert_eq!(args.api_token.as_deref(), Some("abc123"));
    assert_eq!(args.username.as_deref(), Some("user"));
    assert_eq!(args.password.as_deref(), Some("pass"));
    assert!(!args.prompt_password);
}

#[test]
fn parse_cli_supports_prompt_password() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-alert-utils",
        "--basic-user",
        "user",
        "--prompt-password",
    ]);
    assert_eq!(args.username.as_deref(), Some("user"));
    assert_eq!(args.password.as_deref(), None);
    assert!(args.prompt_password);
}

#[test]
fn parse_cli_supports_prompt_token() {
    let args: AlertCliArgs = parse_cli_from(["grafana-alert-utils", "--prompt-token"]);
    assert_eq!(args.api_token.as_deref(), None);
    assert!(args.prompt_token);
    assert!(!args.prompt_password);
}

#[test]
fn help_explains_flat_layout() {
    let help = render_alert_help();
    assert!(help.contains("export"));
    assert!(help.contains("import"));
    assert!(help.contains("diff"));
    assert!(help.contains("Write rule, contact-point, mute-timing, and template files directly"));
    assert!(help.contains("instead of nested subdirectories"));
    assert!(!help.contains("--username"));
    assert!(!help.contains("--password "));
}

#[test]
fn parse_cli_supports_import_subcommand() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-alert-utils",
        "import",
        "--input-dir",
        "./alerts/raw",
        "--replace-existing",
        "--dry-run",
        "--json",
    ]);
    assert_eq!(args.input_dir.as_deref(), Some(Path::new("./alerts/raw")));
    assert!(args.replace_existing);
    assert!(args.dry_run);
    assert!(args.json);
    assert!(args.diff_dir.is_none());
}

#[test]
fn import_help_mentions_structured_dry_run_json() {
    let help = render_alert_subcommand_help(&["import"]);
    assert!(help.contains("--json"));
    assert!(help.contains("Only supported with --dry-run."));
}

#[test]
fn diff_help_mentions_structured_json() {
    let help = render_alert_subcommand_help(&["diff"]);
    assert!(help.contains("--json"));
    assert!(help.contains("Deprecated compatibility flag. Equivalent to --output-format json."));
    assert!(help.contains("--output-format"));
}

#[test]
fn parse_cli_supports_list_rules_subcommand() {
    let args: AlertCliArgs = parse_cli_from(["grafana-util alert", "list-rules", "--json"]);
    assert_eq!(args.list_kind, Some(super::AlertListKind::Rules));
    assert!(args.json);
    assert!(!args.text);
    assert!(!args.csv);
    assert!(!args.yaml);
    assert_eq!(args.org_id, None);
    assert!(!args.all_orgs);
}

#[test]
fn parse_cli_supports_list_alert_output_formats() {
    fn assert_output_mode(args: &AlertCliArgs, mode: &str) {
        match mode {
            "text" => {
                assert!(args.text);
                assert!(!args.table);
                assert!(!args.csv);
                assert!(!args.json);
                assert!(!args.yaml);
            }
            "table" => {
                assert!(args.table);
                assert!(!args.text);
                assert!(!args.csv);
                assert!(!args.json);
                assert!(!args.yaml);
            }
            "csv" => {
                assert!(args.csv);
                assert!(!args.text);
                assert!(!args.table);
                assert!(!args.json);
                assert!(!args.yaml);
            }
            "json" => {
                assert!(args.json);
                assert!(!args.text);
                assert!(!args.table);
                assert!(!args.csv);
                assert!(!args.yaml);
            }
            "yaml" => {
                assert!(args.yaml);
                assert!(!args.text);
                assert!(!args.table);
                assert!(!args.csv);
                assert!(!args.json);
            }
            other => panic!("unexpected output mode {other}"),
        }
    }

    let cases = vec![
        (
            vec![
                "grafana-util alert",
                "list-rules",
                "--output-format",
                "text",
            ],
            super::AlertListKind::Rules,
            "text",
        ),
        (
            vec![
                "grafana-util alert",
                "list-contact-points",
                "--output-format",
                "yaml",
            ],
            super::AlertListKind::ContactPoints,
            "yaml",
        ),
        (
            vec!["grafana-util alert", "list-mute-timings", "--csv"],
            super::AlertListKind::MuteTimings,
            "csv",
        ),
        (
            vec!["grafana-util alert", "list-templates", "--json"],
            super::AlertListKind::Templates,
            "json",
        ),
    ];

    for (argv, kind, mode) in cases {
        let args: AlertCliArgs = parse_cli_from(argv);
        assert_eq!(args.list_kind, Some(kind));
        assert_output_mode(&args, mode);
    }
}

#[test]
fn parse_cli_supports_list_rules_output_format_yaml() {
    let args: AlertCliArgs = parse_cli_from([
        "grafana-util alert",
        "list-rules",
        "--output-format",
        "yaml",
    ]);
    assert_eq!(args.list_kind, Some(super::AlertListKind::Rules));
    assert!(args.yaml);
    assert!(!args.table);
    assert!(!args.csv);
    assert!(!args.json);
}

#[test]
fn parse_cli_supports_list_rules_output_format_csv() {
    let args: AlertCliArgs =
        parse_cli_from(["grafana-util alert", "list-rules", "--output-format", "csv"]);
    assert_eq!(args.list_kind, Some(super::AlertListKind::Rules));
    assert!(args.csv);
    assert!(!args.table);
    assert!(!args.json);
    assert!(!args.text);
    assert!(!args.yaml);
}

#[test]
fn parse_cli_supports_list_rules_org_routing_flags() {
    let org_args: AlertCliArgs = parse_cli_from([
        "grafana-util alert",
        "list-rules",
        "--org-id",
        "7",
        "--json",
    ]);
    assert_eq!(org_args.org_id, Some(7));
    assert!(!org_args.all_orgs);

    let all_orgs_args: AlertCliArgs =
        parse_cli_from(["grafana-util alert", "list-rules", "--all-orgs", "--json"]);
    assert_eq!(all_orgs_args.org_id, None);
    assert!(all_orgs_args.all_orgs);
}

#[test]
fn parse_cli_rejects_list_rules_org_id_with_all_orgs() {
    let error = root_command()
        .try_get_matches_from([
            "grafana-util alert",
            "list-rules",
            "--org-id",
            "7",
            "--all-orgs",
        ])
        .unwrap_err();
    assert!(error.to_string().contains("--org-id"));
    assert!(error.to_string().contains("--all-orgs"));
}

#[test]
fn help_mentions_list_org_routing_flags() {
    let help = render_alert_subcommand_help(&["list-rules"]);
    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("This requires Basic auth."));
}

#[test]
fn help_mentions_list_output_formats() {
    let help = render_alert_subcommand_help(&["list-rules"]);
    assert!(help.contains("--text"));
    assert!(help.contains("--table"));
    assert!(help.contains("--csv"));
    assert!(help.contains("--json"));
    assert!(help.contains("--yaml"));
    assert!(help.contains("Use text, table, csv, json, or yaml."));
}

#[test]
fn serialize_rule_list_rows_includes_org_scope_columns_when_present() {
    let rows = serialize_rule_list_rows(&[json!({
        "uid": "rule-uid",
        "title": "CPU High",
        "folderUID": "infra-folder",
        "ruleGroup": "cpu",
        "org": {
            "id": 7,
            "name": "Platform"
        }
    })
    .as_object()
    .unwrap()
    .clone()]);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("org").map(String::as_str), Some("Platform"));
    assert_eq!(rows[0].get("orgId").map(String::as_str), Some("7"));
}

#[test]
fn build_import_operation_accepts_legacy_tool_document_without_schema_version() {
    let (kind, payload) = build_import_operation(&json!({
        "apiVersion": TOOL_API_VERSION,
        "kind": RULE_KIND,
        "metadata": {"uid": "rule-uid"},
        "spec": {
            "uid": "rule-uid",
            "title": "CPU High",
            "folderUID": "infra-folder",
            "ruleGroup": "cpu-alerts",
            "condition": "C",
            "data": [],
        }
    }))
    .unwrap();
    assert_eq!(kind, RULE_KIND);
    assert_eq!(payload["uid"], "rule-uid");
}

#[test]
fn build_import_operation_rejects_unsupported_schema_version() {
    let error = build_import_operation(&json!({
        "apiVersion": TOOL_API_VERSION,
        "schemaVersion": TOOL_SCHEMA_VERSION + 1,
        "kind": RULE_KIND,
        "spec": {
            "uid": "rule-uid",
            "title": "CPU High",
            "folderUID": "infra-folder",
            "ruleGroup": "cpu-alerts",
            "condition": "C",
            "data": [],
        }
    }))
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("Unsupported grafana-alert-rule schema version"));
}

#[test]
fn build_empty_root_index_contains_version_markers() {
    let index = build_empty_root_index();
    assert_eq!(index["schemaVersion"], json!(TOOL_SCHEMA_VERSION));
    assert_eq!(index["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(index["apiVersion"], json!(TOOL_API_VERSION));
    assert_eq!(index["kind"], json!(ROOT_INDEX_KIND));
    assert_eq!(index["rules"], json!([]));
    assert_eq!(index["contact-points"], json!([]));
    assert_eq!(index["mute-timings"], json!([]));
    assert_eq!(index["policies"], json!([]));
    assert_eq!(index["templates"], json!([]));
}

#[test]
fn alert_export_contract_fixture_matches_root_index_and_resource_subdirs() {
    let fixture = load_alert_export_contract_fixture();
    let root_index = build_empty_root_index();

    assert_eq!(fixture["rootIndex"]["kind"], json!(ROOT_INDEX_KIND));
    assert_eq!(
        fixture["rootIndex"]["schemaVersion"],
        json!(TOOL_SCHEMA_VERSION)
    );
    assert_eq!(fixture["rootIndex"]["apiVersion"], json!(TOOL_API_VERSION));

    for section in fixture["rootIndex"]["requiredSections"]
        .as_array()
        .unwrap_or(&Vec::new())
    {
        let key = section.as_str().unwrap_or("");
        assert_eq!(root_index.get(key), Some(&json!([])));
    }

    let subdirs = super::resource_subdir_by_kind();
    for case in fixture["cases"].as_array().unwrap_or(&Vec::new()) {
        let kind = case["kind"].as_str().unwrap_or("");
        let subdir = case["subdir"].as_str().unwrap_or("");
        assert_eq!(subdirs.get(kind).copied(), Some(subdir));
    }
}

#[test]
fn build_alert_import_dry_run_document_reports_summary_and_rows() {
    let document = build_alert_import_dry_run_document(&[
        json!({
            "path": "alerts/raw/contact-points/smoke.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "smoke-webhook",
            "action": "would-update",
        }),
        json!({
            "path": "alerts/raw/policies/notification-policies.json",
            "kind": "grafana-notification-policies",
            "identity": "grafana-default-email",
            "action": "would-create",
        }),
        json!({
            "path": "alerts/raw/templates/template.json",
            "kind": "grafana-message-template",
            "identity": "slack",
            "action": "would-fail-existing",
        }),
    ]);

    assert_eq!(document["reviewRequired"], json!(true));
    assert_eq!(document["reviewed"], json!(false));
    assert_eq!(document["summary"]["processed"], json!(3));
    assert_eq!(document["summary"]["wouldCreate"], json!(1));
    assert_eq!(document["summary"]["wouldUpdate"], json!(1));
    assert_eq!(document["summary"]["wouldFailExisting"], json!(1));
    assert_eq!(document["rows"].as_array().map(Vec::len), Some(3));
    assert_eq!(document["rows"][0]["identity"], json!("smoke-webhook"));
}

#[test]
fn build_alert_diff_document_reports_summary_and_rows() {
    let document = build_alert_diff_document(&[
        json!({
            "path": "alerts/raw/contact-points/smoke.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "smoke-webhook",
            "action": "same",
        }),
        json!({
            "path": "alerts/raw/policies/notification-policies.json",
            "kind": "grafana-notification-policies",
            "identity": "grafana-default-email",
            "action": "different",
        }),
        json!({
            "path": "alerts/raw/contact-points/missing.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "smoke-webhook",
            "action": "missing-remote",
        }),
    ]);

    assert_eq!(document["kind"], json!("grafana-util-alert-diff"));
    assert_eq!(document["schemaVersion"], json!(1));
    assert_eq!(document["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(document["summary"]["checked"], json!(3));
    assert_eq!(document["summary"]["same"], json!(1));
    assert_eq!(document["summary"]["different"], json!(1));
    assert_eq!(document["summary"]["missingRemote"], json!(1));
    assert_eq!(document["rows"].as_array().map(Vec::len), Some(3));
    assert_exact_object_keys(
        &document,
        &["kind", "rows", "schemaVersion", "summary", "toolVersion"],
    );
    for row in document["rows"].as_array().unwrap() {
        assert_exact_object_keys(row, &["action", "identity", "kind", "path"]);
    }
    assert_eq!(document["rows"][0]["action"], json!("same"));
    assert_eq!(document["rows"][1]["action"], json!("different"));
    assert_eq!(document["rows"][2]["action"], json!("missing-remote"));
}

#[test]
fn build_alert_diff_document_matches_shared_contract_fixture() {
    let fixture = load_shared_diff_golden_fixture("alert");
    let document = build_alert_diff_document(&[json!({
        "domain": "alert",
        "resourceKind": "contact-point",
        "identity": "smoke-webhook",
        "status": "different",
        "path": "alerts/raw/contact-points/smoke.json",
        "changedFields": ["spec"],
    })]);
    assert_eq!(document, fixture["document"]);
}

#[test]
fn contact_point_list_and_export_document_share_identity_fields() {
    let contact_point = json!({
        "uid": "cp-uid",
        "name": "Webhook Main",
        "type": "webhook",
        "settings": {"url": "http://127.0.0.1/notify"},
    });

    let rows = serialize_contact_point_list_rows(&[contact_point.as_object().unwrap().clone()]);
    let document = build_contact_point_export_document(contact_point.as_object().unwrap());

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("uid").map(String::as_str), Some("cp-uid"));
    assert_eq!(
        rows[0].get("name").map(String::as_str),
        Some("Webhook Main")
    );
    assert_eq!(document["spec"]["uid"], json!("cp-uid"));
    assert_eq!(document["spec"]["name"], json!("Webhook Main"));
}

#[test]
fn mute_timing_list_and_export_document_share_identity_fields() {
    let mute_timing = json!({
        "name": "Off Hours",
        "time_intervals": [{"times": [{"start_time": "00:00", "end_time": "06:00"}]}]
    });

    let rows = super::alert_list::serialize_mute_timing_list_rows(&[mute_timing
        .as_object()
        .unwrap()
        .clone()]);
    let document = super::build_mute_timing_export_document(mute_timing.as_object().unwrap());

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("name").map(String::as_str), Some("Off Hours"));
    assert_eq!(rows[0].get("intervals").map(String::as_str), Some("1"));
    assert_eq!(document["spec"]["name"], json!("Off Hours"));
}

#[test]
fn template_list_and_export_document_share_identity_fields() {
    let template = json!({
        "name": "slack.default",
        "template": "{{ define \"slack.default\" }}ok{{ end }}",
        "version": "template-version-1"
    });

    let rows =
        super::alert_list::serialize_template_list_rows(&[template.as_object().unwrap().clone()]);
    let document = super::build_template_export_document(template.as_object().unwrap());

    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].get("name").map(String::as_str),
        Some("slack.default")
    );
    assert_eq!(document["spec"]["name"], json!("slack.default"));
    assert!(document["spec"].get("version").is_none());
}

#[test]
fn rule_list_and_export_document_share_identity_fields() {
    let rule = json!({
        "uid": "cpu-high",
        "title": "CPU High",
        "folderUID": "infra",
        "ruleGroup": "cpu-alerts",
        "condition": "A",
        "data": []
    });

    let rows = serialize_rule_list_rows(&[rule.as_object().unwrap().clone()]);
    let document = build_rule_export_document(rule.as_object().unwrap());

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("uid").map(String::as_str), Some("cpu-high"));
    assert_eq!(rows[0].get("title").map(String::as_str), Some("CPU High"));
    assert_eq!(document["spec"]["uid"], json!("cpu-high"));
    assert_eq!(document["spec"]["title"], json!("CPU High"));
}

#[test]
fn alert_diff_and_import_documents_align_for_update_and_create_actions() {
    let diff_document = build_alert_diff_document(&[
        json!({
            "path": "alerts/raw/contact-points/smoke.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "smoke-webhook",
            "action": "different",
        }),
        json!({
            "path": "alerts/raw/contact-points/missing.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "new-webhook",
            "action": "missing-remote",
        }),
    ]);
    let import_document = build_alert_import_dry_run_document(&[
        json!({
            "path": "alerts/raw/contact-points/smoke.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "smoke-webhook",
            "action": "would-update",
        }),
        json!({
            "path": "alerts/raw/contact-points/missing.json",
            "kind": CONTACT_POINT_KIND,
            "identity": "new-webhook",
            "action": "would-create",
        }),
    ]);

    assert!(diff_document["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["identity"] == "smoke-webhook" && row["action"] == "different"));
    assert!(import_document["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["identity"] == "smoke-webhook" && row["action"] == "would-update"));
    assert!(diff_document["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["identity"] == "new-webhook" && row["action"] == "missing-remote"));
    assert!(import_document["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["identity"] == "new-webhook" && row["action"] == "would-create"));
}

#[test]
fn load_alert_resource_file_accepts_json_and_yaml_desired_documents() {
    let temp = tempdir().unwrap();
    let json_path = temp.path().join("rule.json");
    let yaml_path = temp.path().join("contact-point.yaml");

    write_pretty_json(
        &json_path,
        &json!({
            "uid": "rule-json",
            "title": "JSON Rule",
            "folderUID": "general",
            "ruleGroup": "default",
            "condition": "A",
            "data": [],
        }),
    );
    fs::write(
        &yaml_path,
        r#"name: yaml-contact-point
type: webhook
settings:
  url: http://127.0.0.1:9000/notify
"#,
    )
    .unwrap();

    let (json_kind, json_payload) =
        build_import_operation(&load_alert_resource_file(&json_path, "Alert resource").unwrap())
            .unwrap();
    let (yaml_kind, yaml_payload) =
        build_import_operation(&load_alert_resource_file(&yaml_path, "Alert resource").unwrap())
            .unwrap();

    assert_eq!(json_kind, RULE_KIND);
    assert_eq!(json_payload["uid"], json!("rule-json"));
    assert_eq!(yaml_kind, CONTACT_POINT_KIND);
    assert_eq!(yaml_payload["name"], json!("yaml-contact-point"));
}

#[test]
fn request_optional_object_with_request_treats_http_404_as_missing() {
    let result = crate::grafana_api::alert_live::request_optional_object_with_request(
        |_method, path, _params, _payload| {
            Err(api_response(
                404,
                format!("http://127.0.0.1:3000{path}"),
                "",
            ))
        },
        Method::GET,
        "/api/v1/provisioning/alert-rules/missing-rule",
        None,
    )
    .unwrap();

    assert!(result.is_none());
}

#[test]
fn build_alert_plan_with_request_generates_create_update_noop_and_blocked_rows() {
    let temp = tempdir().unwrap();
    write_new_rule_scaffold(
        &temp.path().join("rules/create-rule.json"),
        "create-rule",
        true,
    )
    .unwrap();
    write_pretty_json(
        &temp.path().join("contact-points/update-contact-point.yaml"),
        &json!({
            "kind": CONTACT_POINT_KIND,
            "apiVersion": TOOL_API_VERSION,
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "spec": {
                "uid": "cp-update",
                "name": "Update Me",
                "type": "webhook",
                "settings": {"url": "http://127.0.0.1/new"}
            }
        }),
    );
    write_new_template_scaffold(
        &temp.path().join("templates/example-template.json"),
        "example-template",
        true,
    )
    .unwrap();

    let plan = build_alert_plan_with_request(
        |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/v1/provisioning/alert-rules/create-rule") => Ok(None),
            (Method::GET, "/api/v1/provisioning/contact-points") => Ok(Some(json!([
                {
                    "uid": "cp-update",
                    "name": "Update Me",
                    "type": "webhook",
                    "settings": {"url": "http://127.0.0.1/old"}
                }
            ]))),
            (Method::GET, "/api/v1/provisioning/templates/example-template") => Ok(Some(json!({
                "name": "example-template",
                "template": "{{ define \"example-template\" }}replace me{{ end }}"
            }))),
            (Method::GET, "/api/v1/provisioning/alert-rules") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/mute-timings") => Ok(Some(json!([
                {
                    "name": "off-hours",
                    "time_intervals": []
                }
            ]))),
            (Method::GET, "/api/v1/provisioning/templates") => Ok(Some(json!([
                {
                    "name": "example-template",
                    "template": "{{ define \"example-template\" }}replace me{{ end }}"
                }
            ]))),
            (Method::GET, "/api/v1/provisioning/policies") => Ok(Some(json!({
                "receiver": "grafana-default-email"
            }))),
            _ => panic!("unexpected request {method:?} {path}"),
        },
        temp.path(),
        false,
    )
    .unwrap();

    assert_eq!(plan["summary"]["create"], json!(1));
    assert_eq!(plan["summary"]["update"], json!(1));
    assert_eq!(plan["summary"]["noop"], json!(1));
    assert_eq!(plan["summary"]["blocked"], json!(2));

    let rows = plan["rows"].as_array().unwrap();
    assert!(rows.iter().any(|row| {
        row["kind"] == json!(RULE_KIND)
            && row["identity"] == json!("create-rule")
            && row["action"] == json!("create")
    }));
    assert!(rows.iter().any(|row| {
        row["kind"] == json!(CONTACT_POINT_KIND)
            && row["identity"] == json!("cp-update")
            && row["action"] == json!("update")
    }));
    assert!(rows.iter().any(|row| {
        row["kind"] == json!(TEMPLATE_KIND)
            && row["identity"] == json!("example-template")
            && row["action"] == json!("noop")
    }));
    assert!(rows.iter().any(|row| {
        row["kind"] == json!(MUTE_TIMING_KIND)
            && row["identity"] == json!("off-hours")
            && row["action"] == json!("blocked")
            && row["reason"] == json!("prune-required")
    }));
    assert!(rows.iter().any(|row| {
        row["kind"] == json!(POLICIES_KIND)
            && row["identity"] == json!("grafana-default-email")
            && row["action"] == json!("blocked")
    }));
}

#[test]
fn build_alert_plan_with_request_marks_live_only_resources_delete_when_prune_enabled() {
    let temp = tempdir().unwrap();
    write_new_rule_scaffold(
        &temp.path().join("rules/create-rule.json"),
        "create-rule",
        true,
    )
    .unwrap();

    let plan = build_alert_plan_with_request(
        |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/v1/provisioning/alert-rules/create-rule") => Ok(None),
            (Method::GET, "/api/v1/provisioning/alert-rules") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/contact-points") => Ok(Some(json!([
                {
                    "uid": "cp-delete",
                    "name": "Delete Me",
                    "type": "webhook",
                    "settings": {"url": "http://127.0.0.1/delete"}
                }
            ]))),
            (Method::GET, "/api/v1/provisioning/mute-timings") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/templates") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/policies") => Ok(None),
            _ => panic!("unexpected request {method:?} {path}"),
        },
        temp.path(),
        true,
    )
    .unwrap();

    assert!(plan["rows"].as_array().unwrap().iter().any(|row| {
        row["kind"] == json!(CONTACT_POINT_KIND)
            && row["identity"] == json!("cp-delete")
            && row["action"] == json!("delete")
    }));
}

#[test]
fn normalize_compare_payload_erases_authoring_round_trip_drift_defaults() {
    let contact_point = json!({
        "uid": "cp-authoring",
        "name": "Authoring Webhook",
        "type": "webhook",
        "settings": {"url": "http://127.0.0.1/notify"},
    });
    let contact_point_live = json!({
        "uid": "cp-authoring",
        "name": "Authoring Webhook",
        "type": "webhook",
        "settings": {"url": "http://127.0.0.1/notify"},
        "disableResolveMessage": false,
    });
    assert_eq!(
        super::normalize_compare_payload(CONTACT_POINT_KIND, contact_point.as_object().unwrap()),
        super::normalize_compare_payload(
            CONTACT_POINT_KIND,
            contact_point_live.as_object().unwrap()
        )
    );

    let policies_desired = json!({
        "receiver": "pagerduty-primary",
        "group_by": ["grafana_folder", "alertname"],
        "routes": [{
            "receiver": "pagerduty-primary",
            "continue": false,
            "group_by": ["grafana_folder", "alertname"],
            "object_matchers": [
                ["team", "=", "platform"],
                ["severity", "=", "critical"],
                ["grafana_utils_route", "=", "pagerduty-primary"]
            ]
        }]
    });
    let policies_live = json!({
        "receiver": "pagerduty-primary",
        "group_by": ["grafana_folder", "alertname"],
        "routes": [{
            "receiver": "pagerduty-primary",
            "group_by": ["grafana_folder", "alertname"],
            "object_matchers": [
                ["grafana_utils_route", "=", "pagerduty-primary"],
                ["severity", "=", "critical"],
                ["team", "=", "platform"]
            ]
        }]
    });
    assert_eq!(
        super::normalize_compare_payload(POLICIES_KIND, policies_desired.as_object().unwrap()),
        super::normalize_compare_payload(POLICIES_KIND, policies_live.as_object().unwrap())
    );

    let rule_desired = json!({
        "uid": "cpu-high",
        "title": "cpu-high",
        "folderUID": "platform-alerts",
        "ruleGroup": "cpu",
        "condition": "A",
        "for": "5m",
        "noDataState": "NoData",
        "execErrState": "Alerting",
        "labels": {
            "grafana_utils_route": "pagerduty-primary",
            "severity": "critical",
            "team": "platform"
        },
        "annotations": {},
        "data": [{
            "refId": "A",
            "datasourceUid": "__expr__",
            "relativeTimeRange": {"from": 0, "to": 0},
            "model": {
                "refId": "A",
                "type": "classic_conditions",
                "datasource": {"type": "__expr__", "uid": "__expr__"},
                "expression": "A",
                "conditions": [{
                    "type": "query",
                    "query": {"params": ["A"]},
                    "reducer": {"type": "last", "params": []},
                    "evaluator": {"type": "gt", "params": [80.0]},
                    "operator": {"type": "and"}
                }],
                "intervalMs": 1000,
                "maxDataPoints": 43200
            }
        }]
    });
    let rule_live = json!({
        "uid": "cpu-high",
        "title": "cpu-high",
        "folderUID": "platform-alerts",
        "ruleGroup": "cpu",
        "condition": "A",
        "for": "5m",
        "noDataState": "NoData",
        "execErrState": "Alerting",
        "isPaused": false,
        "keep_firing_for": "0s",
        "notification_settings": null,
        "record": null,
        "orgID": 1,
        "labels": {
            "grafana_utils_route": "pagerduty-primary",
            "severity": "critical",
            "team": "platform"
        },
        "annotations": {},
        "data": [{
            "refId": "A",
            "queryType": "",
            "datasourceUid": "__expr__",
            "relativeTimeRange": {"from": 0, "to": 0},
            "model": {
                "refId": "A",
                "type": "classic_conditions",
                "datasource": {"type": "__expr__", "uid": "__expr__"},
                "expression": "A",
                "conditions": [{
                    "type": "query",
                    "query": {"params": ["A"]},
                    "reducer": {"type": "last", "params": []},
                    "evaluator": {"type": "gt", "params": [80.0]},
                    "operator": {"type": "and"}
                }],
                "intervalMs": 1000,
                "maxDataPoints": 43200
            }
        }]
    });
    assert_eq!(
        super::normalize_compare_payload(RULE_KIND, rule_desired.as_object().unwrap()),
        super::normalize_compare_payload(RULE_KIND, rule_live.as_object().unwrap())
    );
}

#[test]
fn build_alert_plan_with_request_treats_authoring_round_trip_defaults_as_noop() {
    let temp = tempdir().unwrap();
    write_pretty_json(
        &temp
            .path()
            .join("contact-points/authoring-contact-point.json"),
        &json!({
            "kind": CONTACT_POINT_KIND,
            "apiVersion": TOOL_API_VERSION,
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "spec": {
                "uid": "cp-authoring",
                "name": "Authoring Webhook",
                "type": "webhook",
                "settings": {"url": "http://127.0.0.1/notify"}
            }
        }),
    );
    write_pretty_json(
        &temp.path().join("policies/notification-policies.json"),
        &json!({
            "kind": POLICIES_KIND,
            "apiVersion": TOOL_API_VERSION,
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "spec": {
                "receiver": "pagerduty-primary",
                "group_by": ["grafana_folder", "alertname"],
                "routes": [{
                    "receiver": "pagerduty-primary",
                    "continue": false,
                    "group_by": ["grafana_folder", "alertname"],
                    "object_matchers": [
                        ["team", "=", "platform"],
                        ["severity", "=", "critical"],
                        ["grafana_utils_route", "=", "pagerduty-primary"]
                    ]
                }]
            }
        }),
    );
    write_pretty_json(
        &temp.path().join("rules/cpu-high.json"),
        &json!({
            "kind": RULE_KIND,
            "apiVersion": TOOL_API_VERSION,
            "schemaVersion": TOOL_SCHEMA_VERSION,
            "spec": {
                "uid": "cpu-high",
                "title": "cpu-high",
                "folderUID": "platform-alerts",
                "ruleGroup": "cpu",
                "condition": "A",
                "for": "5m",
                "noDataState": "NoData",
                "execErrState": "Alerting",
                "labels": {
                    "grafana_utils_route": "pagerduty-primary",
                    "severity": "critical",
                    "team": "platform"
                },
                "annotations": {},
                "data": [{
                    "refId": "A",
                    "datasourceUid": "__expr__",
                    "relativeTimeRange": {"from": 0, "to": 0},
                    "model": {
                        "refId": "A",
                        "type": "classic_conditions",
                        "datasource": {"type": "__expr__", "uid": "__expr__"},
                        "expression": "A",
                        "conditions": [{
                            "type": "query",
                            "query": {"params": ["A"]},
                            "reducer": {"type": "last", "params": []},
                            "evaluator": {"type": "gt", "params": [80.0]},
                            "operator": {"type": "and"}
                        }],
                        "intervalMs": 1000,
                        "maxDataPoints": 43200
                    }
                }]
            }
        }),
    );

    let plan = build_alert_plan_with_request(
        |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/v1/provisioning/contact-points") => Ok(Some(json!([
                {
                    "uid": "cp-authoring",
                    "name": "Authoring Webhook",
                    "type": "webhook",
                    "settings": {"url": "http://127.0.0.1/notify"},
                    "disableResolveMessage": false
                }
            ]))),
            (Method::GET, "/api/v1/provisioning/policies") => Ok(Some(json!({
                "receiver": "pagerduty-primary",
                "group_by": ["grafana_folder", "alertname"],
                "routes": [{
                    "receiver": "pagerduty-primary",
                    "group_by": ["grafana_folder", "alertname"],
                    "object_matchers": [
                        ["grafana_utils_route", "=", "pagerduty-primary"],
                        ["severity", "=", "critical"],
                        ["team", "=", "platform"]
                    ]
                }]
            }))),
            (Method::GET, "/api/v1/provisioning/alert-rules/cpu-high") => Ok(Some(json!({
                "uid": "cpu-high",
                "title": "cpu-high",
                "folderUID": "platform-alerts",
                "ruleGroup": "cpu",
                "condition": "A",
                "for": "5m",
                "noDataState": "NoData",
                "execErrState": "Alerting",
                "isPaused": false,
                "keep_firing_for": "0s",
                "notification_settings": null,
                "record": null,
                "orgID": 1,
                "data": [{
                    "refId": "A",
                    "queryType": "",
                    "datasourceUid": "__expr__",
                    "relativeTimeRange": {"from": 0, "to": 0},
                    "model": {
                        "refId": "A",
                        "type": "classic_conditions",
                        "datasource": {"type": "__expr__", "uid": "__expr__"},
                        "expression": "A",
                        "conditions": [{
                            "type": "query",
                            "query": {"params": ["A"]},
                            "reducer": {"type": "last", "params": []},
                            "evaluator": {"type": "gt", "params": [80.0]},
                            "operator": {"type": "and"}
                        }],
                        "intervalMs": 1000,
                        "maxDataPoints": 43200
                    }
                }],
                "labels": {
                    "grafana_utils_route": "pagerduty-primary",
                    "severity": "critical",
                    "team": "platform"
                },
                "annotations": {}
            }))),
            (Method::GET, "/api/v1/provisioning/alert-rules") => Ok(Some(json!([
                {"uid": "cpu-high"}
            ]))),
            (Method::GET, "/api/v1/provisioning/mute-timings") => Ok(Some(json!([]))),
            (Method::GET, "/api/v1/provisioning/templates") => Ok(Some(json!([]))),
            _ => panic!("unexpected request {method:?} {path}"),
        },
        temp.path(),
        true,
    )
    .unwrap();

    assert_eq!(plan["summary"]["create"], json!(0));
    assert_eq!(plan["summary"]["update"], json!(0));
    assert_eq!(plan["summary"]["noop"], json!(3));
    assert_eq!(plan["summary"]["delete"], json!(0));
}

#[test]
fn execute_alert_plan_with_request_applies_create_update_and_delete_rows() {
    let plan = build_alert_plan_document(
        &[
            json!({
                "kind": RULE_KIND,
                "identity": "rule-create",
                "action": "create",
                "desired": {
                    "uid": "rule-create",
                    "title": "Create Me",
                    "folderUID": "general",
                    "ruleGroup": "default",
                    "condition": "A",
                    "data": []
                }
            }),
            json!({
                "kind": CONTACT_POINT_KIND,
                "identity": "cp-update",
                "action": "update",
                "desired": {
                    "uid": "cp-update",
                    "name": "Update Me",
                    "type": "webhook",
                    "settings": {"url": "http://127.0.0.1/new"}
                }
            }),
            json!({
                "kind": TEMPLATE_KIND,
                "identity": "template-delete",
                "action": "delete",
                "desired": null
            }),
            json!({
                "kind": RULE_KIND,
                "identity": "rule-noop",
                "action": "noop",
                "desired": null
            }),
        ],
        true,
    );
    let calls = RefCell::new(Vec::new());

    let result = execute_alert_plan_with_request(
        |method, path, _params, payload| {
            calls
                .borrow_mut()
                .push((method.clone(), path.to_string(), payload.cloned()));
            match (method.clone(), path) {
                (Method::POST, "/api/v1/provisioning/alert-rules") => {
                    Ok(Some(json!({"uid": "rule-create"})))
                }
                (Method::PUT, "/api/v1/provisioning/contact-points/cp-update") => {
                    Ok(Some(json!({"uid": "cp-update"})))
                }
                (Method::DELETE, "/api/v1/provisioning/templates/template-delete") => Ok(None),
                _ => panic!("unexpected request {method:?} {path}"),
            }
        },
        &plan,
        false,
    )
    .unwrap();

    assert_eq!(result["appliedCount"], json!(3));
    let calls = calls.borrow();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].0, Method::POST);
    assert_eq!(calls[0].1, "/api/v1/provisioning/alert-rules");
    assert_eq!(calls[1].0, Method::PUT);
    assert_eq!(calls[1].1, "/api/v1/provisioning/contact-points/cp-update");
    assert_eq!(calls[2].0, Method::DELETE);
    assert_eq!(calls[2].1, "/api/v1/provisioning/templates/template-delete");
}

#[test]
fn execute_alert_plan_with_request_rejects_policy_delete_without_guard() {
    let plan = build_alert_plan_document(
        &[json!({
            "kind": POLICIES_KIND,
            "identity": "grafana-default-email",
            "action": "delete",
            "desired": null
        })],
        true,
    );

    let error =
        execute_alert_plan_with_request(|_method, _path, _params, _payload| Ok(None), &plan, false)
            .unwrap_err()
            .to_string();

    assert!(error.contains("--allow-policy-reset"));
}

#[test]
fn alert_runtime_init_and_scaffolds_write_valid_desired_files() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("alerts-managed");

    let init = init_alert_runtime_layout(&root).unwrap();
    assert_eq!(init["root"], json!(root.to_string_lossy().to_string()));
    assert!(root.join("rules").is_dir());
    assert!(root.join("contact-points").is_dir());
    assert!(root.join("templates").is_dir());

    let rule_path = root.join("rules").join("rule.yaml");
    let contact_point_path = root.join("contact-points").join("contact-point.json");
    let template_path = root.join("templates").join("template.yaml");

    write_new_rule_scaffold(&rule_path, "cpu-main", true).unwrap();
    write_new_contact_point_scaffold(&contact_point_path, "pagerduty-primary", true).unwrap();
    write_new_template_scaffold(&template_path, "sev1-notification", true).unwrap();

    let (rule_kind, _) =
        build_import_operation(&load_alert_resource_file(&rule_path, "rule scaffold").unwrap())
            .unwrap();
    let (contact_point_kind, _) = build_import_operation(
        &load_alert_resource_file(&contact_point_path, "contact point scaffold").unwrap(),
    )
    .unwrap();
    let (template_kind, _) = build_import_operation(
        &load_alert_resource_file(&template_path, "template scaffold").unwrap(),
    )
    .unwrap();

    assert_eq!(rule_kind, RULE_KIND);
    assert_eq!(contact_point_kind, CONTACT_POINT_KIND);
    assert_eq!(template_kind, TEMPLATE_KIND);
    assert_eq!(
        load_alert_resource_file(&rule_path, "rule scaffold").unwrap()["spec"]["uid"],
        json!("cpu-main")
    );
    assert_eq!(
        load_alert_resource_file(&contact_point_path, "contact point scaffold").unwrap()["spec"]
            ["name"],
        json!("pagerduty-primary")
    );
    assert_eq!(
        load_alert_resource_file(&template_path, "template scaffold").unwrap()["spec"]["name"],
        json!("sev1-notification")
    );
}

#[test]
fn managed_route_helpers_build_stable_rule_authoring_contracts() {
    assert_eq!(
        super::alert_support::stable_route_label_key(),
        "grafana_utils_route"
    );
    assert_eq!(
        super::alert_support::build_stable_route_label_value("Team Alerts / Primary"),
        "Team_Alerts_Primary"
    );
    assert_eq!(
        super::alert_support::build_stable_route_matcher("Team Alerts / Primary"),
        json!(["grafana_utils_route", "=", "Team_Alerts_Primary"])
    );

    let folder_contract =
        super::alert_support::build_folder_resolution_contract("infra", Some("Infrastructure"));
    assert_eq!(folder_contract["folderUid"], json!("infra"));
    assert_eq!(folder_contract["folderTitle"], json!("Infrastructure"));
    assert_eq!(folder_contract["resolution"], json!("uid-or-title"));

    let rule_body =
        super::alert_support::build_simple_rule_body("CPU High", "infra", "cpu", "Team Alerts");
    assert_eq!(rule_body["folderUID"], json!("infra"));
    assert_eq!(rule_body["ruleGroup"], json!("cpu"));
    assert_eq!(
        rule_body["labels"]["grafana_utils_route"],
        json!("Team_Alerts")
    );

    let scaffold = super::alert_support::build_new_rule_scaffold_document_with_route(
        "CPU High",
        "infra",
        "cpu",
        "Team Alerts",
    );
    assert_eq!(scaffold["kind"], json!(RULE_KIND));
    assert_eq!(
        scaffold["spec"]["labels"]["grafana_utils_route"],
        json!("Team_Alerts")
    );
    assert_eq!(scaffold["metadata"]["folder"]["folderUid"], json!("infra"));
    assert_eq!(
        scaffold["metadata"]["route"]["labelKey"],
        json!("grafana_utils_route")
    );
    assert_eq!(
        scaffold["metadata"]["route"]["labelValue"],
        json!("Team_Alerts")
    );
}

#[test]
fn managed_policy_subtree_upsert_is_idempotent_and_leaves_unmanaged_routes_untouched() {
    let current_policy = json!({
        "receiver": "grafana-default-email",
        "routes": [
            {
                "receiver": "legacy-email",
                "object_matchers": [["team", "=", "legacy"]],
                "routes": [{"receiver": "legacy-nested"}]
            }
        ]
    });
    let desired_route = json!({
        "receiver": "team-webhook",
        "group_by": ["grafana_folder", "alertname"],
        "routes": [{"receiver": "team-slack"}]
    });

    let (first_policy, first_action) = super::alert_support::upsert_managed_policy_subtree(
        current_policy.as_object().unwrap(),
        "Team Alerts",
        desired_route.as_object().unwrap(),
    )
    .unwrap();
    assert_eq!(first_action, "created");
    assert_eq!(first_policy["routes"].as_array().unwrap().len(), 2);
    assert_eq!(first_policy["routes"][0], current_policy["routes"][0]);
    let managed_route = first_policy["routes"][1].as_object().unwrap();
    assert!(super::alert_support::route_matches_stable_label(
        managed_route,
        "Team Alerts"
    ));
    assert_eq!(managed_route["receiver"], json!("team-webhook"));
    assert_eq!(
        managed_route["object_matchers"],
        json!([["grafana_utils_route", "=", "Team_Alerts"]])
    );

    let preview = super::alert_support::build_managed_policy_route_preview(
        current_policy.as_object().unwrap(),
        "Team Alerts",
        Some(desired_route.as_object().unwrap()),
    )
    .unwrap();
    assert_eq!(preview["action"], json!("created"));
    assert_eq!(preview["managedRouteValue"], json!("Team_Alerts"));

    let (second_policy, second_action) = super::alert_support::upsert_managed_policy_subtree(
        &first_policy,
        "Team Alerts",
        desired_route.as_object().unwrap(),
    )
    .unwrap();
    assert_eq!(second_action, "noop");
    assert_eq!(second_policy, first_policy);
}

#[test]
fn managed_policy_subtree_remove_only_touches_tool_owned_route() {
    let current_policy = json!({
        "receiver": "grafana-default-email",
        "routes": [
            {
                "receiver": "legacy-email",
                "object_matchers": [["team", "=", "legacy"]]
            },
            {
                "receiver": "team-webhook",
                "object_matchers": [["grafana_utils_route", "=", "Team_Alerts"]]
            }
        ]
    });

    let (next_policy, action) = super::alert_support::remove_managed_policy_subtree(
        current_policy.as_object().unwrap(),
        "Team Alerts",
    )
    .unwrap();
    assert_eq!(action, "deleted");
    assert_eq!(next_policy["routes"].as_array().unwrap().len(), 1);
    assert_eq!(next_policy["routes"][0], current_policy["routes"][0]);

    let preview = super::alert_support::build_managed_policy_route_preview(
        current_policy.as_object().unwrap(),
        "Team Alerts",
        None,
    )
    .unwrap();
    assert_eq!(preview["action"], json!("deleted"));
    assert_eq!(preview["nextRoute"], Value::Null);

    let (noop_policy, noop_action) =
        super::alert_support::remove_managed_policy_subtree(&next_policy, "Team Alerts").unwrap();
    assert_eq!(noop_action, "noop");
    assert_eq!(noop_policy, next_policy);
}

#[test]
fn runtime_managed_policy_helpers_produce_idempotent_documents() {
    let current_policy = json!({
        "receiver": "grafana-default-email",
        "routes": [
            {
                "receiver": "legacy-email",
                "object_matchers": [["team", "=", "legacy"]]
            }
        ]
    });
    let desired_route = json!({
        "receiver": "team-webhook",
        "object_matchers": [["grafana_utils_route", "=", "old-value"]],
        "routes": [{"receiver": "team-slack"}]
    });

    let preview = super::alert_runtime_support::build_managed_policy_edit_preview_document(
        &current_policy,
        "Team Alerts",
        Some(&desired_route),
    )
    .unwrap();
    assert_eq!(
        preview["kind"],
        json!("grafana-util-alert-managed-policy-preview")
    );
    assert_eq!(preview["schemaVersion"], json!(1));
    assert_eq!(preview["toolVersion"], json!(TOOL_VERSION));
    assert_eq!(preview["reviewRequired"], json!(true));
    assert_eq!(preview["reviewed"], json!(false));
    assert_eq!(preview["preview"]["action"], json!("created"));

    let first_apply = super::alert_runtime_support::apply_managed_policy_subtree_edit_document(
        &current_policy,
        "Team Alerts",
        Some(&desired_route),
    )
    .unwrap();
    assert_eq!(first_apply["kind"], json!(POLICIES_KIND));
    assert_eq!(first_apply["action"], json!("created"));
    assert_eq!(
        first_apply["spec"]["routes"][1]["object_matchers"],
        json!([["grafana_utils_route", "=", "Team_Alerts"]])
    );
    assert_eq!(
        first_apply["spec"]["routes"][0],
        current_policy["routes"][0]
    );

    let second_apply = super::alert_runtime_support::apply_managed_policy_subtree_edit_document(
        &first_apply["spec"],
        "Team Alerts",
        Some(&desired_route),
    )
    .unwrap();
    assert_eq!(second_apply["action"], json!("noop"));
    assert_eq!(second_apply["spec"], first_apply["spec"]);
}

#[test]
fn contact_point_scaffolds_cover_webhook_email_and_slack_authoring_shapes() {
    let webhook =
        super::alert_support::build_contact_point_scaffold_document("team-webhook", "webhook");
    let email = super::alert_support::build_contact_point_scaffold_document("team-email", "email");
    let slack = super::alert_support::build_contact_point_scaffold_document("team-slack", "slack");

    assert_eq!(webhook["spec"]["type"], json!("webhook"));
    assert_eq!(
        webhook["spec"]["settings"]["url"],
        json!("http://127.0.0.1:9000/notify")
    );
    assert_eq!(email["spec"]["type"], json!("email"));
    assert_eq!(
        email["spec"]["settings"]["addresses"],
        json!(["alerts@example.com"])
    );
    assert_eq!(slack["spec"]["type"], json!("slack"));
    assert_eq!(slack["spec"]["settings"]["recipient"], json!("#alerts"));
    assert_eq!(
        slack["metadata"]["authoring"]["settingsKeys"],
        json!(["recipient", "text", "token"])
    );

    let temp = tempdir().unwrap();
    let slack_path = temp.path().join("team-slack.yaml");
    super::alert_runtime_support::write_contact_point_scaffold(
        &slack_path,
        "team-slack",
        "slack",
        true,
    )
    .unwrap();
    let written = load_alert_resource_file(&slack_path, "typed contact point scaffold").unwrap();
    let (kind, payload) = build_import_operation(&written).unwrap();
    assert_eq!(kind, CONTACT_POINT_KIND);
    assert_eq!(payload["type"], json!("slack"));
    assert_eq!(payload["settings"]["recipient"], json!("#alerts"));
}

#[test]
fn run_alert_cli_add_rule_writes_desired_rule_and_managed_policy_files() {
    let temp = tempdir().unwrap();
    let desired_dir = temp.path().join("alerts-desired");
    init_alert_runtime_layout(&desired_dir).unwrap();

    let args = parse_cli_from([
        "grafana-util alert",
        "add-rule",
        "--desired-dir",
        desired_dir.to_string_lossy().as_ref(),
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
        "--severity",
        "critical",
        "--expr",
        "A",
        "--threshold",
        "80",
        "--above",
    ]);
    run_alert_cli(args).unwrap();

    let rule_path = desired_dir.join("rules").join("cpu-high.yaml");
    let policy_path = desired_dir
        .join("policies")
        .join("notification-policies.yaml");
    let rule = load_alert_resource_file(&rule_path, "authored rule").unwrap();
    let policy = load_alert_resource_file(&policy_path, "managed policy").unwrap();

    assert_eq!(rule["kind"], json!(RULE_KIND));
    assert_eq!(rule["spec"]["folderUID"], json!("platform-alerts"));
    assert_eq!(rule["spec"]["ruleGroup"], json!("cpu"));
    assert_eq!(rule["spec"]["labels"]["team"], json!("platform"));
    assert_eq!(rule["spec"]["labels"]["severity"], json!("critical"));
    assert_eq!(
        rule["spec"]["labels"]["grafana_utils_route"],
        json!("pagerduty-primary")
    );
    assert_eq!(policy["kind"], json!(POLICIES_KIND));
    assert_eq!(
        policy["spec"]["routes"][0]["receiver"],
        json!("pagerduty-primary")
    );
    assert_eq!(
        policy["spec"]["routes"][0]["object_matchers"][0],
        json!(["team", "=", "platform"])
    );
}

#[test]
fn run_alert_cli_clone_rule_dry_run_leaves_target_files_absent() {
    let temp = tempdir().unwrap();
    let desired_dir = temp.path().join("alerts-desired");
    init_alert_runtime_layout(&desired_dir).unwrap();

    let source_path = desired_dir.join("rules").join("cpu-high.yaml");
    write_new_rule_scaffold(&source_path, "cpu-high", true).unwrap();

    let args = parse_cli_from([
        "grafana-util alert",
        "clone-rule",
        "--desired-dir",
        desired_dir.to_string_lossy().as_ref(),
        "--source",
        "cpu-high",
        "--name",
        "cpu-high-staging",
        "--no-route",
        "--dry-run",
    ]);
    run_alert_cli(args).unwrap();

    assert!(!desired_dir
        .join("rules")
        .join("cpu-high-staging.yaml")
        .exists());
    assert!(!desired_dir
        .join("policies")
        .join("notification-policies.yaml")
        .exists());
}

#[test]
fn run_alert_cli_set_route_overwrites_managed_route_in_place() {
    let temp = tempdir().unwrap();
    let desired_dir = temp.path().join("alerts-desired");
    init_alert_runtime_layout(&desired_dir).unwrap();

    let first = parse_cli_from([
        "grafana-util alert",
        "set-route",
        "--desired-dir",
        desired_dir.to_string_lossy().as_ref(),
        "--receiver",
        "pagerduty-primary",
        "--label",
        "team=platform",
    ]);
    run_alert_cli(first).unwrap();

    let second = parse_cli_from([
        "grafana-util alert",
        "set-route",
        "--desired-dir",
        desired_dir.to_string_lossy().as_ref(),
        "--receiver",
        "pagerduty-primary",
        "--label",
        "team=infra",
    ]);
    run_alert_cli(second).unwrap();

    let policy = load_alert_resource_file(
        &desired_dir
            .join("policies")
            .join("notification-policies.yaml"),
        "managed policy",
    )
    .unwrap();
    let routes = policy["spec"]["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0]["receiver"], json!("pagerduty-primary"));
    assert_eq!(
        routes[0]["object_matchers"],
        json!([
            ["team", "=", "infra"],
            ["grafana_utils_route", "=", "pagerduty-primary"]
        ])
    );
}

#[test]
fn build_alert_delete_preview_from_files_blocks_policy_reset_without_guard() {
    let temp = tempdir().unwrap();
    let policy_path = temp.path().join("notification-policies.yaml");
    fs::write(&policy_path, "receiver: grafana-default-email\n").unwrap();

    let preview = build_alert_delete_preview_from_files(&[policy_path], false).unwrap();
    assert_eq!(preview["reviewRequired"], json!(true));
    assert_eq!(preview["reviewed"], json!(false));
    assert_eq!(preview["summary"]["delete"], json!(0));
    assert_eq!(preview["summary"]["blocked"], json!(1));
    assert_eq!(
        preview["rows"][0]["reason"],
        json!("policy-reset-requires-allow-policy-reset")
    );
}

#[test]
fn compare_diff_output_includes_headers_and_local_payload() {
    let remote = json!({
        "kind": RULE_KIND,
        "spec": {
            "uid": "rule-uid",
            "title": "CPU High",
            "folderUID": "infra-folder",
            "ruleGroup": "cpu-alerts",
            "condition": "C",
            "data": [],
        }
    });
    let local = json!({
        "kind": RULE_KIND,
        "spec": {
            "uid": "rule-uid",
            "title": "CPU Critical",
            "folderUID": "infra-folder",
            "ruleGroup": "cpu-alerts",
            "condition": "C",
            "data": [],
        }
    });
    let diff = build_compare_diff_text(
        &remote,
        &local,
        "rule-uid",
        Path::new("alerts/raw/rules/infra-folder/rule-uid.json"),
    )
    .unwrap();
    assert!(diff.contains("--- remote:rule-uid"));
    assert!(diff.contains("+++ alerts/raw/rules/infra-folder/rule-uid.json"));
    assert!(diff.contains("+    \"title\": \"CPU Critical\""));
    assert!(diff.contains("-    \"title\": \"CPU High\""));
}

#[test]
fn serialize_compare_document_sorts_object_keys_stably() {
    let first = json!({
        "spec": {
            "ruleGroup": "cpu-alerts",
            "title": "CPU High",
        },
        "kind": RULE_KIND,
    });
    let second = json!({
        "kind": RULE_KIND,
        "spec": {
            "title": "CPU High",
            "ruleGroup": "cpu-alerts",
        },
    });
    assert_eq!(
        serialize_compare_document(&first).unwrap(),
        serialize_compare_document(&second).unwrap()
    );
}

#[test]
fn expect_object_list_rejects_json_null() {
    let error = expect_object_list(
        Some(serde_json::Value::Null),
        "Unexpected template list response from Grafana.",
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("Unexpected template list response from Grafana."));
}

#[test]
fn template_list_null_is_treated_as_empty_in_live_client_path() {
    let templates = parse_template_list_response(Some(serde_json::Value::Null)).unwrap();
    assert!(templates.is_empty());
}

#[test]
fn alert_recreate_matrix_with_request_covers_rule_contact_point_mute_timing_and_template() {
    let fixture = load_alert_recreate_contract_fixture();
    for case in fixture["recreateCases"].as_array().unwrap_or(&Vec::new()) {
        let kind = case["kind"].as_str().unwrap_or("");
        let identity = case["identity"].as_str().unwrap_or("");
        let expected_dry_run_action = case["expectedDryRunAction"]
            .as_str()
            .unwrap_or("would-create");
        let expected_replay_action = case["expectedReplayAction"].as_str().unwrap_or("created");
        let request_contract = case["requestContract"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        let payload = case["payload"].as_object().cloned().unwrap_or_default();
        run_alert_recreate_case(
            kind,
            payload,
            identity,
            expected_dry_run_action,
            expected_replay_action,
            request_contract,
        );
    }
}

#[test]
fn policies_with_request_stay_update_only_and_return_to_same_state() {
    let fixture = load_alert_recreate_contract_fixture();
    let expected_identity = fixture["policiesCase"]["identity"].as_str().unwrap_or("");
    let expected_dry_run_action = fixture["policiesCase"]["expectedDryRunAction"]
        .as_str()
        .unwrap_or("would-update");
    let expected_replay_action = fixture["policiesCase"]["expectedReplayAction"]
        .as_str()
        .unwrap_or("updated");
    let request_contract = fixture["policiesCase"]["requestContract"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let local_payload = fixture["policiesCase"]["payload"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let remote_policy = RefCell::new(json!({
        "receiver": "legacy-email",
        "routes": [{"receiver": "legacy-email"}]
    }));
    let request_log = RefCell::new(Vec::<String>::new());

    let initial_compare = fetch_live_compare_document_with_request(
        |method, path, _params, payload| -> Result<Option<Value>> {
            let method_name = method.as_str().to_string();
            request_log
                .borrow_mut()
                .push(format!("{} {}", method_name, path));
            match (method, path) {
                (Method::GET, "/api/v1/provisioning/policies") => {
                    Ok(Some(remote_policy.borrow().clone()))
                }
                (Method::PUT, "/api/v1/provisioning/policies") => {
                    let next = payload.cloned().ok_or_else(|| {
                        message("policies update payload must be present".to_string())
                    })?;
                    *remote_policy.borrow_mut() = next.clone();
                    Ok(Some(next))
                }
                _ => Err(message(format!(
                    "unexpected alert runtime request {} {}",
                    method_name, path
                ))),
            }
        },
        POLICIES_KIND,
        &local_payload,
    )
    .unwrap()
    .unwrap();
    assert_ne!(
        serialize_compare_document(&initial_compare).unwrap(),
        serialize_compare_document(&json!({
            "kind": POLICIES_KIND,
            "spec": local_payload,
        }))
        .unwrap()
    );

    assert_eq!(
        determine_import_action_with_request(
            |method, path, _params, _payload| -> Result<Option<Value>> {
                let method_name = method.as_str().to_string();
                request_log
                    .borrow_mut()
                    .push(format!("{} {}", method_name, path));
                match (method, path) {
                    (Method::GET, "/api/v1/provisioning/policies") => {
                        Ok(Some(remote_policy.borrow().clone()))
                    }
                    _ => Err(message(format!(
                        "unexpected alert runtime request {} {}",
                        method_name, path
                    ))),
                }
            },
            POLICIES_KIND,
            &local_payload,
            true,
        )
        .unwrap(),
        expected_dry_run_action
    );

    let (action, identity) = import_resource_document_with_request(
        |method, path, _params, payload| -> Result<Option<Value>> {
            let method_name = method.as_str().to_string();
            request_log
                .borrow_mut()
                .push(format!("{} {}", method_name, path));
            match (method, path) {
                (Method::PUT, "/api/v1/provisioning/policies") => {
                    let next = payload.cloned().ok_or_else(|| {
                        message("policies update payload must be present".to_string())
                    })?;
                    *remote_policy.borrow_mut() = next.clone();
                    Ok(Some(next))
                }
                _ => Err(message(format!(
                    "unexpected alert runtime request {} {}",
                    method_name, path
                ))),
            }
        },
        POLICIES_KIND,
        &local_payload,
        true,
    )
    .unwrap();
    assert_eq!(action, expected_replay_action);
    assert_eq!(identity, expected_identity);

    let update_request_prefix = request_contract
        .get("updateRequestPrefix")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let update_request_count = request_contract
        .get("updateRequestCount")
        .and_then(Value::as_u64)
        .unwrap_or(1) as usize;
    let actual_update_count = request_log
        .borrow()
        .iter()
        .filter(|entry| entry.starts_with(update_request_prefix))
        .count();
    assert_eq!(
        actual_update_count, update_request_count,
        "expected {update_request_count} update request(s) for policies"
    );
    assert!(
        !update_request_prefix.is_empty(),
        "policies request contract is missing updateRequestPrefix"
    );

    let live_compare = fetch_live_compare_document_with_request(
        |method, path, _params, _payload| -> Result<Option<Value>> {
            let method_name = method.as_str().to_string();
            request_log
                .borrow_mut()
                .push(format!("{} {}", method_name, path));
            match (method, path) {
                (Method::GET, "/api/v1/provisioning/policies") => {
                    Ok(Some(remote_policy.borrow().clone()))
                }
                _ => Err(message(format!(
                    "unexpected alert runtime request {} {}",
                    method_name, path
                ))),
            }
        },
        POLICIES_KIND,
        &local_payload,
    )
    .unwrap()
    .unwrap();

    assert_eq!(
        serialize_compare_document(&live_compare).unwrap(),
        serialize_compare_document(&json!({
            "kind": POLICIES_KIND,
            "spec": local_payload,
        }))
        .unwrap()
    );
}

fn run_alert_recreate_case(
    kind: &str,
    payload: serde_json::Map<String, Value>,
    identity: &str,
    expected_dry_run_action: &str,
    expected_replay_action: &str,
    request_contract: serde_json::Map<String, Value>,
) {
    let remote_resources = RefCell::new(Vec::<Value>::new());
    let request_log = RefCell::new(Vec::<String>::new());

    let initial_compare = fetch_live_compare_document_with_request(
        |method, path, _params, payload_json| -> Result<Option<Value>> {
            handle_alert_runtime_request(
                &request_log,
                &remote_resources,
                kind,
                method,
                path,
                payload_json,
            )
        },
        kind,
        &payload,
    )
    .unwrap();
    assert!(
        initial_compare.is_none(),
        "expected missing remote for {kind}"
    );

    let action = determine_import_action_with_request(
        |method, path, _params, payload_json| -> Result<Option<Value>> {
            handle_alert_runtime_request(
                &request_log,
                &remote_resources,
                kind,
                method,
                path,
                payload_json,
            )
        },
        kind,
        &payload,
        true,
    )
    .unwrap();
    assert_eq!(
        action, expected_dry_run_action,
        "expected {expected_dry_run_action} for {kind}"
    );

    let (replay_action, replay_identity) = import_resource_document_with_request(
        |method, path, _params, payload_json| -> Result<Option<Value>> {
            handle_alert_runtime_request(
                &request_log,
                &remote_resources,
                kind,
                method,
                path,
                payload_json,
            )
        },
        kind,
        &payload,
        true,
    )
    .unwrap();
    assert_eq!(
        replay_action, expected_replay_action,
        "expected {expected_replay_action} for {kind}"
    );
    assert_eq!(
        replay_identity, identity,
        "expected identity parity for {kind}"
    );

    let live_compare = fetch_live_compare_document_with_request(
        |method, path, _params, payload_json| -> Result<Option<Value>> {
            handle_alert_runtime_request(
                &request_log,
                &remote_resources,
                kind,
                method,
                path,
                payload_json,
            )
        },
        kind,
        &payload,
    )
    .unwrap()
    .unwrap();

    assert_eq!(
        serialize_compare_document(&live_compare).unwrap(),
        serialize_compare_document(&super::build_compare_document(
            kind,
            &super::strip_server_managed_fields(kind, &payload),
        ))
        .unwrap(),
        "expected same-state after recreate for {kind}"
    );

    let create_request_prefix = request_contract
        .get("createRequestPrefix")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let create_request_count = request_contract
        .get("createRequestCount")
        .and_then(Value::as_u64)
        .unwrap_or(1) as usize;
    let disallow_update_prefix = request_contract
        .get("disallowUpdatePrefix")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let create_like_count = request_log
        .borrow()
        .iter()
        .filter(|entry| entry.starts_with(create_request_prefix))
        .count();
    assert!(
        create_like_count == create_request_count,
        "expected {create_request_count} create replay request(s) for {kind}, got {create_like_count}"
    );
    if !disallow_update_prefix.is_empty() {
        assert!(
            !request_log
                .borrow()
                .iter()
                .any(|entry| entry.starts_with(disallow_update_prefix)),
            "unexpected update path during recreate for {kind}"
        );
    }
}

fn handle_alert_runtime_request(
    request_log: &RefCell<Vec<String>>,
    remote_resources: &RefCell<Vec<Value>>,
    kind: &str,
    method: Method,
    path: &str,
    payload: Option<&Value>,
) -> Result<Option<Value>> {
    let method_name = method.as_str().to_string();
    request_log
        .borrow_mut()
        .push(format!("{} {}", method_name, path));
    match kind {
        RULE_KIND => match (method, path) {
            (Method::GET, path) if path.starts_with("/api/v1/provisioning/alert-rules/") => {
                Ok(remote_resources
                    .borrow()
                    .iter()
                    .find(|item| item.get("uid").and_then(Value::as_str) == path.rsplit('/').next())
                    .cloned())
            }
            (Method::POST, "/api/v1/provisioning/alert-rules") => {
                let created = payload.cloned().ok_or_else(|| {
                    message("alert-rule create payload must be present".to_string())
                })?;
                remote_resources.borrow_mut().push(created.clone());
                Ok(Some(created))
            }
            (Method::PUT, path) if path.starts_with("/api/v1/provisioning/alert-rules/") => {
                Err(message(format!(
                    "unexpected alert-rule update during recreate path {}",
                    path
                )))
            }
            _ => Err(message(format!(
                "unexpected alert runtime request {} {}",
                method_name, path
            ))),
        },
        CONTACT_POINT_KIND => match (method, path) {
            (Method::GET, "/api/v1/provisioning/contact-points") => {
                Ok(Some(Value::Array(remote_resources.borrow().clone())))
            }
            (Method::POST, "/api/v1/provisioning/contact-points") => {
                let created = payload.cloned().ok_or_else(|| {
                    message("contact-point create payload must be present".to_string())
                })?;
                remote_resources.borrow_mut().push(created.clone());
                Ok(Some(created))
            }
            (Method::PUT, path) if path.starts_with("/api/v1/provisioning/contact-points/") => {
                Err(message(format!(
                    "unexpected contact-point update during recreate path {}",
                    path
                )))
            }
            _ => Err(message(format!(
                "unexpected alert runtime request {} {}",
                method_name, path
            ))),
        },
        MUTE_TIMING_KIND => match (method, path) {
            (Method::GET, "/api/v1/provisioning/mute-timings") => {
                Ok(Some(Value::Array(remote_resources.borrow().clone())))
            }
            (Method::POST, "/api/v1/provisioning/mute-timings") => {
                let created = payload.cloned().ok_or_else(|| {
                    message("mute-timing create payload must be present".to_string())
                })?;
                remote_resources.borrow_mut().push(created.clone());
                Ok(Some(created))
            }
            (Method::PUT, path) if path.starts_with("/api/v1/provisioning/mute-timings/") => {
                Err(message(format!(
                    "unexpected mute-timing update during recreate path {}",
                    path
                )))
            }
            _ => Err(message(format!(
                "unexpected alert runtime request {} {}",
                method_name, path
            ))),
        },
        TEMPLATE_KIND => match (method, path) {
            (Method::GET, path) if path.starts_with("/api/v1/provisioning/templates/") => {
                Ok(remote_resources
                    .borrow()
                    .iter()
                    .find(|item| {
                        item.get("name").and_then(Value::as_str) == path.rsplit('/').next()
                    })
                    .cloned())
            }
            (Method::PUT, path) if path.starts_with("/api/v1/provisioning/templates/") => {
                let name = path.rsplit('/').next().unwrap_or_default().to_string();
                let mut created = payload.cloned().ok_or_else(|| {
                    message("template update payload must be present".to_string())
                })?;
                if let Some(object) = created.as_object_mut() {
                    object.insert("name".to_string(), Value::String(name.clone()));
                }
                let existing_index = remote_resources.borrow().iter().position(|item| {
                    item.get("name").and_then(Value::as_str) == Some(name.as_str())
                });
                if let Some(index) = existing_index {
                    remote_resources.borrow_mut()[index] = created.clone();
                } else {
                    remote_resources.borrow_mut().push(created.clone());
                }
                Ok(Some(created))
            }
            _ => Err(message(format!(
                "unexpected alert runtime request {} {}",
                method_name, path
            ))),
        },
        _ => Err(message(format!("unexpected alert kind {}", kind))),
    }
}
