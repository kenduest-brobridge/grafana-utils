//! Alert domain test suite.
//! Validates export/import/diff document shaping, kind detection, and CLI parser/help
//! behavior.
use super::{
    build_compare_diff_text, build_contact_point_export_document, build_contact_point_output_path,
    build_empty_root_index, build_import_operation, build_rule_export_document,
    build_rule_output_path, detect_document_kind, expect_object_list, get_rule_linkage,
    load_panel_id_map, load_string_map, parse_cli_from, parse_template_list_response, root_command,
    serialize_compare_document, serialize_rule_list_rows, AlertCliArgs, CONTACT_POINT_KIND,
    ROOT_INDEX_KIND, RULE_KIND, TOOL_API_VERSION, TOOL_SCHEMA_VERSION,
};
use serde_json::json;
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
    assert!(args.import_dir.is_none());
    assert!(!args.dry_run);
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
        "--import-dir",
        "./alerts/raw",
        "--replace-existing",
        "--dry-run",
    ]);
    assert_eq!(args.import_dir.as_deref(), Some(Path::new("./alerts/raw")));
    assert!(args.replace_existing);
    assert!(args.dry_run);
    assert!(args.diff_dir.is_none());
}

#[test]
fn parse_cli_supports_list_rules_subcommand() {

    let args: AlertCliArgs = parse_cli_from(["grafana-util alert", "list-rules", "--json"]);
    assert_eq!(args.list_kind, Some(super::AlertListKind::Rules));
    assert!(args.json);
    assert!(!args.csv);
    assert_eq!(args.org_id, None);
    assert!(!args.all_orgs);
}

#[test]
fn parse_cli_supports_list_rules_output_format_csv() {

    let args: AlertCliArgs =
        parse_cli_from(["grafana-util alert", "list-rules", "--output-format", "csv"]);
    assert_eq!(args.list_kind, Some(super::AlertListKind::Rules));
    assert!(args.csv);
    assert!(!args.table);
    assert!(!args.json);
}

#[test]
fn parse_cli_supports_list_rules_org_routing_flags() {

    let org_args: AlertCliArgs =
        parse_cli_from(["grafana-util alert", "list-rules", "--org-id", "7", "--json"]);
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
    assert_eq!(index["apiVersion"], json!(TOOL_API_VERSION));
    assert_eq!(index["kind"], json!(ROOT_INDEX_KIND));
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
