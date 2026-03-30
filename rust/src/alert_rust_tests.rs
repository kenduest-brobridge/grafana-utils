//! Alert domain test suite.
//! Validates export/import/diff document shaping, kind detection, and CLI parser/help
//! behavior.
use super::alert_list::serialize_contact_point_list_rows;
use super::{
    build_alert_diff_document, build_alert_import_dry_run_document,
    build_alert_live_project_status_domain, build_alert_project_status_domain,
    build_compare_diff_text, build_contact_point_export_document, build_contact_point_output_path,
    build_empty_root_index, build_import_operation, build_rule_export_document,
    build_rule_output_path, detect_document_kind, determine_import_action_with_request,
    expect_object_list, fetch_live_compare_document_with_request, get_rule_linkage,
    import_resource_document_with_request, load_panel_id_map, load_string_map, parse_cli_from,
    parse_template_list_response, root_command, serialize_compare_document,
    serialize_rule_list_rows, AlertCliArgs, AlertLiveProjectStatusInputs, CONTACT_POINT_KIND,
    MUTE_TIMING_KIND, POLICIES_KIND, ROOT_INDEX_KIND, RULE_KIND, TEMPLATE_KIND, TOOL_API_VERSION,
    TOOL_SCHEMA_VERSION,
};
use crate::common::{message, Result, TOOL_VERSION};
use reqwest::Method;
use serde_json::json;
use serde_json::Value;
use std::cell::RefCell;
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
    assert!(args.import_dir.is_none());
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
        "--json",
    ]);
    assert_eq!(args.import_dir.as_deref(), Some(Path::new("./alerts/raw")));
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
    assert!(help.contains("Render diff output as structured JSON."));
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

    assert_eq!(document["summary"]["checked"], json!(3));
    assert_eq!(document["summary"]["same"], json!(1));
    assert_eq!(document["summary"]["different"], json!(1));
    assert_eq!(document["summary"]["missingRemote"], json!(1));
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
