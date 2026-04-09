//! CLI definitions for Core command surface and option compatibility behavior.

use super::*;
use crate::dashboard::SimpleOutputFormat;
use crate::datasource::DatasourceGroupCommand;

#[test]
fn delete_help_explains_live_mutation_flags() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("delete")
        .unwrap_or_else(|| panic!("missing datasource delete help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--uid"));
    assert!(help.contains("--name"));
    assert!(help.contains("--dry-run"));
    assert!(help.contains("--yes"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("Safety Options"));
}

#[test]
fn modify_help_explains_live_mutation_flags() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("modify")
        .unwrap_or_else(|| panic!("missing datasource modify help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--uid"));
    assert!(help.contains("--set-url"));
    assert!(help.contains("--set-access"));
    assert!(help.contains("--set-default"));
    assert!(help.contains("--basic-auth"));
    assert!(help.contains("--basic-auth-user"));
    assert!(help.contains("--basic-auth-password"));
    assert!(help.contains("--user"));
    assert!(help.contains("--password"));
    assert!(help.contains("--with-credentials"));
    assert!(help.contains("--http-header"));
    assert!(help.contains("--tls-skip-verify"));
    assert!(help.contains("--server-name"));
    assert!(help.contains("--json-data"));
    assert!(help.contains("--secure-json-data"));
    assert!(help.contains("--secure-json-data-placeholders"));
    assert!(help.contains("--secret-values"));
    assert!(help.contains("--dry-run"));
    assert!(help.contains("Examples:"));
}

#[test]
fn parse_datasource_list_supports_output_format_json() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "list",
        "--output-format",
        "json",
    ]);

    match args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.json);
            assert!(!inner.table);
            assert!(!inner.csv);
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_list_supports_output_format_text_and_yaml() {
    let text_args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "list",
        "--output-format",
        "text",
    ]);
    let yaml_args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "list",
        "--output-format",
        "yaml",
    ]);

    match text_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.text);
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(!inner.json);
            assert!(!inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }

    match yaml_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.yaml);
            assert!(!inner.text);
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(!inner.json);
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_types_supports_output_format_json() {
    for (flag, expected) in [
        ("json", SimpleOutputFormat::Json),
        ("table", SimpleOutputFormat::Table),
        ("csv", SimpleOutputFormat::Csv),
        ("yaml", SimpleOutputFormat::Yaml),
    ] {
        let args = DatasourceCliArgs::parse_normalized_from([
            "grafana-util",
            "types",
            "--output-format",
            flag,
        ]);

        match args.command {
            DatasourceGroupCommand::Types(inner) => {
                assert_eq!(inner.output_format, expected);
            }
            _ => panic!("expected datasource types"),
        }
    }
}

#[test]
fn parse_datasource_types_defaults_to_text_output() {
    let args = DatasourceCliArgs::parse_normalized_from(["grafana-util", "types"]);

    match args.command {
        DatasourceGroupCommand::Types(inner) => {
            assert_eq!(inner.output_format, SimpleOutputFormat::Text);
        }
        _ => panic!("expected datasource types"),
    }
}

#[test]
fn parse_datasource_list_supports_org_scope_flags() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "list",
        "--org-id",
        "7",
        "--output-format",
        "csv",
    ]);

    match args.command {
        DatasourceGroupCommand::List(inner) => {
            assert_eq!(inner.org_id, Some(7));
            assert!(!inner.all_orgs);
            assert!(inner.csv);
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_list_supports_all_orgs_flag() {
    let args =
        DatasourceCliArgs::parse_normalized_from(["grafana-util", "list", "--all-orgs", "--json"]);

    match args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.all_orgs);
            assert_eq!(inner.org_id, None);
            assert!(inner.json);
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_list_rejects_conflicting_org_scope_flags() {
    let error =
        DatasourceCliArgs::try_parse_from(["grafana-util", "list", "--org-id", "7", "--all-orgs"])
            .unwrap_err();

    assert!(error.to_string().contains("--org-id"));
    assert!(error.to_string().contains("--all-orgs"));
}

#[test]
fn render_data_source_table_includes_org_columns_when_present() {
    let datasources = vec![json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "org": "Main Org.",
        "orgId": "1"
    })
    .as_object()
    .unwrap()
    .clone()];

    let lines = render_data_source_table(&datasources, true, None);

    assert!(lines[0].contains("ORG"));
    assert!(lines[0].contains("ORG_ID"));
    assert!(lines[2].contains("Main Org."));
    assert!(lines[2].contains("1"));
}

#[test]
fn render_data_source_csv_and_json_include_org_fields_when_present() {
    let datasources = vec![json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "org": "Main Org.",
        "orgId": "1"
    })
    .as_object()
    .unwrap()
    .clone()];

    let csv = render_data_source_csv(&datasources, None);
    let json_value = render_data_source_json(&datasources, None);

    assert_eq!(csv[0], "uid,name,type,url,isDefault,org,orgId");
    assert!(csv[1].contains("Main Org."));
    assert_eq!(json_value[0]["org"], Value::String("Main Org.".to_string()));
    assert_eq!(json_value[0]["orgId"], Value::String("1".to_string()));
}

#[test]
fn parse_datasource_add_supports_output_format_table() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Prometheus Main",
        "--type",
        "prometheus",
        "--dry-run",
        "--output-format",
        "table",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            assert!(inner.dry_run);
            assert!(inner.table);
            assert!(!inner.json);
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn parse_datasource_add_supports_datasource_auth_flags() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Prometheus Main",
        "--type",
        "prometheus",
        "--basic-auth",
        "--basic-auth-user",
        "metrics-user",
        "--basic-auth-password",
        "metrics-pass",
        "--user",
        "query-user",
        "--password",
        "query-pass",
        "--with-credentials",
        "--http-header",
        "X-Scope-OrgID=tenant-a",
        "--tls-skip-verify",
        "--server-name",
        "prometheus.internal",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            assert!(inner.basic_auth);
            assert_eq!(inner.basic_auth_user.as_deref(), Some("metrics-user"));
            assert_eq!(inner.basic_auth_password.as_deref(), Some("metrics-pass"));
            assert_eq!(inner.user.as_deref(), Some("query-user"));
            assert_eq!(inner.datasource_password.as_deref(), Some("query-pass"));
            assert!(inner.with_credentials);
            assert_eq!(inner.http_header, vec!["X-Scope-OrgID=tenant-a"]);
            assert!(inner.tls_skip_verify);
            assert_eq!(inner.server_name.as_deref(), Some("prometheus.internal"));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn parse_datasource_delete_supports_output_format_json() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "delete",
        "--uid",
        "prom-main",
        "--dry-run",
        "--output-format",
        "json",
    ]);

    match args.command {
        DatasourceGroupCommand::Delete(inner) => {
            assert_eq!(inner.uid.as_deref(), Some("prom-main"));
            assert!(inner.dry_run);
            assert!(inner.json);
            assert!(!inner.table);
        }
        _ => panic!("expected datasource delete"),
    }
}

#[test]
fn parse_datasource_delete_accepts_yes_confirmation() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "delete",
        "--uid",
        "prom-main",
        "--yes",
    ]);

    match args.command {
        DatasourceGroupCommand::Delete(inner) => {
            assert_eq!(inner.uid.as_deref(), Some("prom-main"));
            assert!(inner.yes);
            assert!(!inner.dry_run);
        }
        _ => panic!("expected datasource delete"),
    }
}

#[test]
fn parse_datasource_modify_supports_output_format_table() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "modify",
        "--uid",
        "prom-main",
        "--set-url",
        "http://prometheus-v2:9090",
        "--dry-run",
        "--output-format",
        "table",
    ]);

    match args.command {
        DatasourceGroupCommand::Modify(inner) => {
            assert_eq!(inner.uid, "prom-main");
            assert_eq!(inner.set_url.as_deref(), Some("http://prometheus-v2:9090"));
            assert!(inner.dry_run);
            assert!(inner.table);
            assert!(!inner.json);
        }
        _ => panic!("expected datasource modify"),
    }
}

#[test]
fn parse_datasource_modify_supports_datasource_auth_flags() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "modify",
        "--uid",
        "prom-main",
        "--basic-auth",
        "--basic-auth-user",
        "metrics-user",
        "--basic-auth-password",
        "metrics-pass",
        "--user",
        "query-user",
        "--password",
        "query-pass",
        "--with-credentials",
        "--http-header",
        "X-Scope-OrgID=tenant-b",
        "--tls-skip-verify",
        "--server-name",
        "prometheus.internal",
    ]);

    match args.command {
        DatasourceGroupCommand::Modify(inner) => {
            assert!(inner.basic_auth);
            assert_eq!(inner.basic_auth_user.as_deref(), Some("metrics-user"));
            assert_eq!(inner.basic_auth_password.as_deref(), Some("metrics-pass"));
            assert_eq!(inner.user.as_deref(), Some("query-user"));
            assert_eq!(inner.datasource_password.as_deref(), Some("query-pass"));
            assert!(inner.with_credentials);
            assert_eq!(inner.http_header, vec!["X-Scope-OrgID=tenant-b"]);
            assert!(inner.tls_skip_verify);
            assert_eq!(inner.server_name.as_deref(), Some("prometheus.internal"));
        }
        _ => panic!("expected datasource modify"),
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
        org_name: String::new(),
        org_id: "1".to_string(),
        basic_auth: None,
        basic_auth_user: String::new(),
        database: String::new(),
        json_data: None,
        secure_json_data_placeholders: None,
        user: String::new(),
        with_credentials: None,
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
fn resolve_live_mutation_match_distinguishes_uid_name_mismatch() {
    let live = vec![live_datasource(
        7,
        "prom-main",
        "Prometheus Main",
        "prometheus",
    )];

    let matching = resolve_live_mutation_match(Some("prom-main"), Some("Other Name"), &live);

    assert_eq!(matching.destination, "uid-name-mismatch");
    assert_eq!(matching.action, "would-fail-uid-name-mismatch");
    assert_eq!(matching.target_id, Some(7));
}

#[test]
fn resolve_delete_match_returns_would_delete_for_existing_uid() {
    let live = vec![live_datasource(
        7,
        "prom-main",
        "Prometheus Main",
        "prometheus",
    )];

    let matching = resolve_delete_match(Some("prom-main"), None, &live);

    assert_eq!(matching.destination, "exists-uid");
    assert_eq!(matching.action, "would-delete");
    assert_eq!(matching.target_id, Some(7));
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
        org_name: String::new(),
        org_id: "1".to_string(),
        basic_auth: None,
        basic_auth_user: String::new(),
        database: String::new(),
        json_data: None,
        secure_json_data_placeholders: None,
        user: String::new(),
        with_credentials: None,
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
fn resolve_match_blocks_name_match_when_uid_differs() {
    let record = DatasourceImportRecord {
        uid: "prom-export".to_string(),
        name: "Prometheus Main".to_string(),
        datasource_type: "prometheus".to_string(),
        access: "proxy".to_string(),
        url: "http://prometheus:9090".to_string(),
        is_default: true,
        org_name: String::new(),
        org_id: "1".to_string(),
        basic_auth: None,
        basic_auth_user: String::new(),
        database: String::new(),
        json_data: None,
        secure_json_data_placeholders: None,
        user: String::new(),
        with_credentials: None,
    };
    let live = vec![live_datasource(
        9,
        "prom-live",
        "Prometheus Main",
        "prometheus",
    )];

    let matching = resolve_match(&record, &live, true, false);

    assert_eq!(matching.destination, "exists-name");
    assert_eq!(matching.action, "would-fail-uid-mismatch");
    assert_eq!(matching.target_uid, "prom-live");
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
