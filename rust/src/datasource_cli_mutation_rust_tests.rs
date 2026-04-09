//! CLI definitions for Core command surface and option compatibility behavior.

use super::*;
use crate::datasource::DatasourceGroupCommand;

#[test]
fn datasource_root_help_includes_examples() {
    let mut command = DatasourceCliArgs::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("Examples:"));
    assert!(help.contains("grafana-util datasource browse"));
    assert!(help.contains("grafana-util datasource types"));
    assert!(help.contains("grafana-util datasource list"));
    assert!(help.contains("grafana-util datasource list --input-dir ./datasources"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("grafana-util datasource add"));
    assert!(help.contains("grafana-util datasource import"));
}

#[test]
fn types_help_includes_examples() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("types")
        .unwrap_or_else(|| panic!("missing datasource types help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--output-format"));
    assert!(help.contains("yaml"));
    assert!(help.contains("grafana-util datasource types"));
}

#[test]
fn list_help_explains_org_scope_flags() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("list")
        .unwrap_or_else(|| panic!("missing datasource list help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("Requires Basic auth"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("--input-dir"));
    assert!(help.contains("--input-format"));
    assert!(help.contains("--text"));
    assert!(help.contains("--table"));
    assert!(help.contains("--csv"));
    assert!(help.contains("--json"));
    assert!(help.contains("--yaml"));
    assert!(help.contains("--output-columns"));
    assert!(help.contains("--list-columns"));
}

#[test]
fn browse_help_mentions_edit_delete_and_examples() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("browse")
        .unwrap_or_else(|| panic!("missing datasource browse help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("Live-only browse against Grafana"));
    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("grafana-util datasource browse"));
    assert!(help.contains("edit"));
    assert!(help.contains("delete"));
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

    assert!(help.contains("--input-dir"));
    assert!(help.contains("--org-id"));
    assert!(help.contains("--use-export-org"));
    assert!(help.contains("--only-org-id"));
    assert!(help.contains("--create-missing-orgs"));
    assert!(help.contains("--require-matching-export-org"));
    assert!(help.contains("--replace-existing"));
    assert!(help.contains("--update-existing-only"));
    assert!(help.contains("--secret-values"));
    assert!(help.contains("--dry-run"));
    assert!(help.contains("--table"));
    assert!(help.contains("--json"));
    assert!(help.contains("--output-format"));
    assert!(help.contains("--output-columns"));
    assert!(help.contains("--progress"));
    assert!(help.contains("--verbose"));
    assert!(help.contains("secureJsonDataPlaceholders"));
    assert!(help.contains("Secrets"));
    assert!(help.contains("Examples:"));
    assert!(help.contains("Input Options"));
}

#[test]
fn parse_datasource_browse_supports_org_scope_flag() {
    let args: DatasourceCliArgs =
        DatasourceCliArgs::parse_from(["grafana-util datasource", "browse", "--org-id", "7"]);

    match args.command {
        DatasourceGroupCommand::Browse(inner) => {
            assert_eq!(inner.org_id, Some(7));
            assert!(!inner.all_orgs);
        }
        _ => panic!("expected datasource browse"),
    }
}

#[test]
fn parse_datasource_browse_supports_all_orgs_flag() {
    let args: DatasourceCliArgs =
        DatasourceCliArgs::parse_from(["grafana-util datasource", "browse", "--all-orgs"]);

    match args.command {
        DatasourceGroupCommand::Browse(inner) => {
            assert!(inner.all_orgs);
            assert_eq!(inner.org_id, None);
        }
        _ => panic!("expected datasource browse"),
    }
}

#[test]
fn parse_datasource_browse_rejects_conflicting_org_scope_flags() {
    let result = DatasourceCliArgs::try_parse_from([
        "grafana-util datasource",
        "browse",
        "--org-id",
        "7",
        "--all-orgs",
    ]);

    assert!(result.is_err());
}

#[test]
fn parse_datasource_list_supports_input_dir_and_json() {
    let args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--json",
    ]);

    match args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.json);
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(!inner.text);
            assert!(!inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_list_supports_all_output_flags_and_aliases() {
    let table_args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--table",
    ]);
    let csv_args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--csv",
    ]);
    let text_args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--text",
    ]);
    let json_args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--json",
    ]);
    let yaml_args: DatasourceCliArgs = DatasourceCliArgs::parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--yaml",
    ]);
    let table_alias_args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "table",
    ]);
    let csv_alias_args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "csv",
    ]);
    let text_alias_args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "text",
    ]);
    let json_alias_args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "json",
    ]);
    let yaml_alias_args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "yaml",
    ]);

    match table_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.table);
            assert!(!inner.csv);
            assert!(!inner.text);
            assert!(!inner.json);
            assert!(!inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }

    match csv_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(!inner.table);
            assert!(inner.csv);
            assert!(!inner.text);
            assert!(!inner.json);
            assert!(!inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }

    match text_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(inner.text);
            assert!(!inner.json);
            assert!(!inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }

    match json_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(!inner.text);
            assert!(inner.json);
            assert!(!inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }

    match yaml_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(!inner.text);
            assert!(!inner.json);
            assert!(inner.yaml);
        }
        _ => panic!("expected datasource list"),
    }

    match table_alias_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.table);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Table)
            );
        }
        _ => panic!("expected datasource list"),
    }

    match csv_alias_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.csv);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Csv)
            );
        }
        _ => panic!("expected datasource list"),
    }

    match text_alias_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.text);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Text)
            );
        }
        _ => panic!("expected datasource list"),
    }

    match json_alias_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.json);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Json)
            );
        }
        _ => panic!("expected datasource list"),
    }

    match yaml_alias_args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.yaml);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Yaml)
            );
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_list_supports_output_format_aliases() {
    let args: DatasourceCliArgs = DatasourceCliArgs::parse_normalized_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--output-format",
        "yaml",
    ]);

    match args.command {
        DatasourceGroupCommand::List(inner) => {
            assert!(inner.yaml);
            assert!(!inner.table);
            assert!(!inner.csv);
            assert!(!inner.text);
            assert!(!inner.json);
            assert_eq!(
                inner.output_format,
                Some(crate::datasource::ListOutputFormat::Yaml)
            );
        }
        _ => panic!("expected datasource list"),
    }
}

#[test]
fn parse_datasource_list_rejects_conflicting_output_flags() {
    let result = DatasourceCliArgs::try_parse_from([
        "grafana-util datasource",
        "list",
        "--input-dir",
        "./datasources",
        "--interactive",
        "--json",
    ]);

    assert!(result.is_err());
}

#[test]
fn export_help_explains_org_scope_flags() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("export")
        .unwrap_or_else(|| panic!("missing datasource export help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--org-id"));
    assert!(help.contains("--all-orgs"));
    assert!(help.contains("--overwrite"));
    assert!(help.contains("--dry-run"));
    assert!(help.contains("Examples:"));
}

#[test]
fn add_help_explains_live_mutation_flags() {
    let mut command = DatasourceCliArgs::command();
    let subcommand = command
        .find_subcommand_mut("add")
        .unwrap_or_else(|| panic!("missing datasource add help"));
    let mut output = Vec::new();
    subcommand.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("--name"));
    assert!(help.contains("--type"));
    assert!(help.contains("--apply-supported-defaults"));
    assert!(help.contains("--preset-profile"));
    assert!(help.contains("starter"));
    assert!(help.contains("full"));
    assert!(help.contains("--datasource-url"));
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
    assert!(help.contains("--dry-run"));
    assert!(help.contains("Examples:"));
}

#[test]
fn build_add_payload_normalizes_supported_type_alias() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Prometheus Main",
        "--type",
        "grafana-prometheus-datasource",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["type"], json!("prometheus"));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_applies_supported_defaults_when_requested() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Prometheus Main",
        "--type",
        "prometheus",
        "--apply-supported-defaults",
        "--json-data",
        "{\"httpMethod\":\"GET\",\"timeInterval\":\"30s\"}",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["access"], json!("proxy"));
            assert!(!payload.as_object().unwrap().contains_key("httpMethod"));
            assert_eq!(payload["jsonData"]["httpMethod"], json!("GET"));
            assert_eq!(payload["jsonData"]["timeInterval"], json!("30s"));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_applies_full_preset_profile_defaults() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Prometheus Main",
        "--type",
        "prometheus",
        "--preset-profile",
        "full",
        "--json-data",
        "{\"httpMethod\":\"GET\",\"timeInterval\":\"30s\"}",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["access"], json!("proxy"));
            assert_eq!(payload["httpMethod"], json!("POST"));
            assert_eq!(payload["jsonData"]["httpMethod"], json!("GET"));
            assert_eq!(payload["jsonData"]["timeInterval"], json!("30s"));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_applies_full_preset_profile_scaffold_for_loki() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Loki Main",
        "--type",
        "loki",
        "--preset-profile",
        "full",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["type"], json!("loki"));
            assert_eq!(payload["access"], json!("proxy"));
            assert_eq!(payload["jsonData"]["maxLines"], json!(1000));
            assert_eq!(payload["jsonData"]["timeout"], json!(60));
            assert_eq!(
                payload["jsonData"]["derivedFields"][0]["name"],
                json!("TraceID")
            );
            assert_eq!(
                payload["jsonData"]["derivedFields"][0]["matcherRegex"],
                json!("traceID=(\\w+)")
            );
            assert_eq!(
                payload["jsonData"]["derivedFields"][0]["datasourceUid"],
                json!("tempo")
            );
            assert_eq!(
                payload["jsonData"]["derivedFields"][0]["urlDisplayLabel"],
                json!("View Trace")
            );
            assert_eq!(
                payload["jsonData"]["derivedFields"][0]["url"],
                json!("$${__value.raw}")
            );
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_applies_full_preset_profile_scaffold_for_tempo() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Tempo Main",
        "--type",
        "tempo",
        "--preset-profile",
        "full",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["type"], json!("tempo"));
            assert_eq!(payload["access"], json!("proxy"));
            assert_eq!(
                payload["jsonData"]["serviceMap"]["datasourceUid"],
                json!("prometheus")
            );
            assert_eq!(
                payload["jsonData"]["tracesToLogsV2"]["datasourceUid"],
                json!("loki")
            );
            assert_eq!(
                payload["jsonData"]["tracesToLogsV2"]["spanStartTimeShift"],
                json!("-1h")
            );
            assert_eq!(
                payload["jsonData"]["tracesToLogsV2"]["spanEndTimeShift"],
                json!("1h")
            );
            assert_eq!(
                payload["jsonData"]["tracesToMetrics"]["datasourceUid"],
                json!("prometheus")
            );
            assert_eq!(
                payload["jsonData"]["tracesToMetrics"]["spanStartTimeShift"],
                json!("-1h")
            );
            assert_eq!(
                payload["jsonData"]["tracesToMetrics"]["spanEndTimeShift"],
                json!("1h")
            );
            assert_eq!(payload["jsonData"]["nodeGraph"]["enabled"], json!(true));
            assert_eq!(payload["jsonData"]["search"]["hide"], json!(false));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_applies_full_preset_profile_scaffold_for_mysql() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "MySQL Main",
        "--type",
        "mysql",
        "--preset-profile",
        "full",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["type"], json!("mysql"));
            assert_eq!(payload["access"], json!("proxy"));
            assert_eq!(payload["jsonData"]["database"], json!("grafana"));
            assert_eq!(payload["jsonData"]["maxOpenConns"], json!(100));
            assert_eq!(payload["jsonData"]["maxIdleConns"], json!(100));
            assert_eq!(payload["jsonData"]["maxIdleConnsAuto"], json!(true));
            assert_eq!(payload["jsonData"]["connMaxLifetime"], json!(14400));
            assert_eq!(payload["jsonData"]["tlsAuth"], json!(true));
            assert_eq!(payload["jsonData"]["tlsSkipVerify"], json!(true));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_applies_full_preset_profile_scaffold_for_postgresql() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Postgres Main",
        "--type",
        "postgresql",
        "--preset-profile",
        "full",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["type"], json!("postgresql"));
            assert_eq!(payload["access"], json!("proxy"));
            assert_eq!(payload["jsonData"]["database"], json!("grafana"));
            assert_eq!(payload["jsonData"]["sslmode"], json!("disable"));
            assert_eq!(payload["jsonData"]["maxOpenConns"], json!(100));
            assert_eq!(payload["jsonData"]["maxIdleConns"], json!(100));
            assert_eq!(payload["jsonData"]["maxIdleConnsAuto"], json!(true));
            assert_eq!(payload["jsonData"]["connMaxLifetime"], json!(14400));
            assert_eq!(payload["jsonData"]["postgresVersion"], json!(903));
            assert_eq!(payload["jsonData"]["timescaledb"], json!(false));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_matches_shared_preset_profile_fixture() {
    for case in load_preset_profile_add_payload_cases() {
        let args = case["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>();
        let parsed = DatasourceCliArgs::parse_normalized_from(args);
        let add_args = match parsed.command {
            DatasourceGroupCommand::Add(inner) => inner,
            _ => panic!("expected datasource add"),
        };

        let payload = build_add_payload(&add_args).unwrap();
        assert_json_subset(&payload, &case["expectedSubset"]);
    }
}

#[test]
fn build_add_payload_applies_full_preset_profile_time_field_defaults() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Elastic Main",
        "--type",
        "elasticsearch",
        "--preset-profile",
        "full",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["access"], json!("proxy"));
            assert_eq!(payload["timeField"], json!("@timestamp"));
            assert_eq!(payload["jsonData"]["timeField"], json!("@timestamp"));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_applies_supported_defaults_for_elasticsearch() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Elastic Main",
        "--type",
        "elasticsearch",
        "--apply-supported-defaults",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["type"], json!("elasticsearch"));
            assert_eq!(payload["access"], json!("proxy"));
            assert_eq!(payload["jsonData"]["timeField"], json!("@timestamp"));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_applies_supported_defaults_for_influxdb() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Influx Main",
        "--type",
        "influxdb",
        "--apply-supported-defaults",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["type"], json!("influxdb"));
            assert_eq!(payload["access"], json!("proxy"));
            assert_eq!(payload["jsonData"]["version"], json!("Flux"));
            assert_eq!(payload["jsonData"]["organization"], json!("main-org"));
            assert_eq!(payload["jsonData"]["defaultBucket"], json!("metrics"));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_applies_supported_defaults_for_loki() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Loki Main",
        "--type",
        "loki",
        "--apply-supported-defaults",
        "--json-data",
        "{\"maxLines\":250}",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["type"], json!("loki"));
            assert_eq!(payload["access"], json!("proxy"));
            assert_eq!(payload["jsonData"]["maxLines"], json!(250));
            assert_eq!(payload["jsonData"]["timeout"], json!(60));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_applies_supported_defaults_for_tempo() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Tempo Main",
        "--type",
        "tempo",
        "--apply-supported-defaults",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["type"], json!("tempo"));
            assert_eq!(payload["access"], json!("proxy"));
            assert_eq!(payload["jsonData"]["nodeGraph"]["enabled"], json!(true));
            assert_eq!(payload["jsonData"]["search"]["hide"], json!(false));
            assert_eq!(
                payload["jsonData"]["traceQuery"]["timeShiftEnabled"],
                json!(true)
            );
            assert_eq!(
                payload["jsonData"]["traceQuery"]["spanStartTimeShift"],
                json!("-1h")
            );
            assert_eq!(
                payload["jsonData"]["traceQuery"]["spanEndTimeShift"],
                json!("1h")
            );
            assert_eq!(
                payload["jsonData"]["streamingEnabled"]["search"],
                json!(true)
            );
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn build_add_payload_applies_supported_defaults_for_postgresql() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Postgres Main",
        "--type",
        "postgres",
        "--apply-supported-defaults",
        "--dry-run",
    ]);

    match args.command {
        DatasourceGroupCommand::Add(inner) => {
            let payload = build_add_payload(&inner).unwrap();
            assert_eq!(payload["type"], json!("postgresql"));
            assert_eq!(payload["access"], json!("proxy"));
            assert_eq!(payload["jsonData"]["database"], json!("grafana"));
            assert_eq!(payload["jsonData"]["sslmode"], json!("disable"));
            assert_eq!(payload["jsonData"]["maxOpenConns"], json!(100));
            assert_eq!(payload["jsonData"]["maxIdleConns"], json!(100));
            assert_eq!(payload["jsonData"]["maxIdleConnsAuto"], json!(true));
            assert_eq!(payload["jsonData"]["connMaxLifetime"], json!(14400));
        }
        _ => panic!("expected datasource add"),
    }
}

#[test]
fn supported_catalog_json_includes_prometheus_profile_metadata() {
    let document = render_supported_datasource_catalog_json();
    let prometheus = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Metrics"))
        .and_then(|row| row["types"].as_array())
        .and_then(|rows| rows.iter().find(|row| row["type"] == json!("prometheus")))
        .unwrap();

    assert_eq!(prometheus["profile"], json!("metrics-http"));
    assert_eq!(prometheus["queryLanguage"], json!("promql"));
    assert_eq!(prometheus["requiresDatasourceUrl"], json!(true));
    assert!(prometheus["suggestedFlags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "--basic-auth"));
    assert_eq!(prometheus["presetProfiles"], json!(["starter"]));
    assert_eq!(prometheus["addDefaults"]["access"], json!("proxy"));
    assert_eq!(
        prometheus["addDefaults"]["jsonData"]["httpMethod"],
        json!("POST")
    );
    assert_eq!(prometheus["fullAddDefaults"], prometheus["addDefaults"]);
}

#[test]
fn supported_catalog_json_matches_shared_supported_types_fixture() {
    let document = render_supported_datasource_catalog_json();

    assert_eq!(
        project_supported_types_catalog(&document),
        load_supported_types_catalog_fixture()
    );
}

#[test]
fn supported_catalog_json_includes_database_profile_metadata() {
    let document = render_supported_datasource_catalog_json();
    let sqlite = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Databases"))
        .and_then(|row| row["types"].as_array())
        .and_then(|rows| rows.iter().find(|row| row["type"] == json!("sqlite")))
        .unwrap();

    assert_eq!(sqlite["profile"], json!("sql-database"));
    assert_eq!(sqlite["queryLanguage"], json!("sql"));
    assert_eq!(sqlite["requiresDatasourceUrl"], json!(false));
    assert!(sqlite["suggestedFlags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "--user"));
    assert_eq!(sqlite["presetProfiles"], json!(["starter"]));
}

#[test]
fn supported_catalog_json_includes_family_level_json_data_defaults() {
    let document = render_supported_datasource_catalog_json();
    let metrics_types = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Metrics"))
        .and_then(|row| row["types"].as_array())
        .unwrap();
    let influxdb = metrics_types
        .iter()
        .find(|row| row["type"] == json!("influxdb"))
        .unwrap();
    assert_eq!(influxdb["addDefaults"]["access"], json!("proxy"));
    assert_eq!(
        influxdb["addDefaults"]["jsonData"]["version"],
        json!("Flux")
    );
    assert_eq!(
        influxdb["addDefaults"]["jsonData"]["organization"],
        json!("main-org")
    );
    assert_eq!(
        influxdb["addDefaults"]["jsonData"]["defaultBucket"],
        json!("metrics")
    );
    let graphite = metrics_types
        .iter()
        .find(|row| row["type"] == json!("graphite"))
        .unwrap();
    assert_eq!(
        graphite["addDefaults"]["jsonData"]["graphiteVersion"],
        json!("1.1")
    );

    let logs_types = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Logs"))
        .and_then(|row| row["types"].as_array())
        .unwrap();
    let loki = logs_types
        .iter()
        .find(|row| row["type"] == json!("loki"))
        .unwrap();
    assert_eq!(loki["addDefaults"]["access"], json!("proxy"));
    assert_eq!(loki["presetProfiles"], json!(["starter", "full"]));
    assert_eq!(loki["addDefaults"]["jsonData"]["maxLines"], json!(1000));
    assert_eq!(loki["addDefaults"]["jsonData"]["timeout"], json!(60));
    assert_eq!(
        loki["fullAddDefaults"]["jsonData"]["derivedFields"][0]["datasourceUid"],
        json!("tempo")
    );

    let tracing_types = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Tracing"))
        .and_then(|row| row["types"].as_array())
        .unwrap();
    let tempo = tracing_types
        .iter()
        .find(|row| row["type"] == json!("tempo"))
        .unwrap();
    assert_eq!(tempo["presetProfiles"], json!(["starter", "full"]));
    assert_eq!(
        tempo["addDefaults"]["jsonData"]["nodeGraph"]["enabled"],
        json!(true)
    );
    assert_eq!(
        tempo["addDefaults"]["jsonData"]["search"]["hide"],
        json!(false)
    );
    assert_eq!(
        tempo["addDefaults"]["jsonData"]["traceQuery"]["timeShiftEnabled"],
        json!(true)
    );
    assert_eq!(
        tempo["fullAddDefaults"]["jsonData"]["serviceMap"]["datasourceUid"],
        json!("prometheus")
    );
    assert_eq!(
        tempo["fullAddDefaults"]["jsonData"]["tracesToLogsV2"]["datasourceUid"],
        json!("loki")
    );

    let database_types = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Databases"))
        .and_then(|row| row["types"].as_array())
        .unwrap();
    let postgresql = database_types
        .iter()
        .find(|row| row["type"] == json!("postgresql"))
        .unwrap();
    assert_eq!(postgresql["presetProfiles"], json!(["starter", "full"]));
    assert_eq!(
        postgresql["addDefaults"]["jsonData"]["database"],
        json!("grafana")
    );
    assert_eq!(
        postgresql["addDefaults"]["jsonData"]["sslmode"],
        json!("disable")
    );
    let mysql = database_types
        .iter()
        .find(|row| row["type"] == json!("mysql"))
        .unwrap();
    assert_eq!(mysql["presetProfiles"], json!(["starter", "full"]));
    assert_eq!(mysql["fullAddDefaults"]["jsonData"]["tlsAuth"], json!(true));
}

#[test]
fn supported_catalog_text_mentions_profile_and_flags() {
    let lines = crate::datasource_catalog::render_supported_datasource_catalog_text();
    let prometheus_line = lines
        .iter()
        .find(|line| line.contains("Prometheus (prometheus)"))
        .unwrap();
    assert!(prometheus_line.contains("profile=metrics-http"));
    assert!(prometheus_line.contains("query=promql"));
    assert!(prometheus_line.contains("flags: --basic-auth"));
}

#[test]
fn supported_catalog_text_mentions_family_level_defaults() {
    let lines = crate::datasource_catalog::render_supported_datasource_catalog_text();
    let influxdb_line = lines
        .iter()
        .find(|line| line.contains("InfluxDB (influxdb)"))
        .unwrap();
    assert!(influxdb_line.contains("defaults: access=proxy, jsonData.version=Flux"));
    assert!(influxdb_line.contains("jsonData.organization=main-org"));
    assert!(influxdb_line.contains("jsonData.defaultBucket=metrics"));

    let loki_line = lines
        .iter()
        .find(|line| line.contains("Loki (loki)"))
        .unwrap();
    assert!(loki_line.contains("jsonData.maxLines=1000"));
    assert!(loki_line.contains("jsonData.timeout=60"));

    let tempo_line = lines
        .iter()
        .find(|line| line.contains("Tempo (tempo)"))
        .unwrap();
    assert!(tempo_line.contains("jsonData.nodeGraph.enabled=true"));
    assert!(tempo_line.contains("jsonData.traceQuery.timeShiftEnabled=true"));

    let postgresql_line = lines
        .iter()
        .find(|line| line.contains("PostgreSQL (postgresql)"))
        .unwrap();
    assert!(postgresql_line.contains("jsonData.database=grafana"));
    assert!(postgresql_line.contains("jsonData.sslmode=disable"));
}

#[test]
fn supported_catalog_table_mentions_category_and_aliases() {
    let lines = crate::datasource_catalog::render_supported_datasource_catalog_table();
    assert!(lines[0].contains("category"));
    assert!(lines[0].contains("display_name"));
    assert!(lines.iter().any(|line| line.contains("Prometheus")));
    assert!(lines
        .iter()
        .any(|line| line.contains("grafana-loki-datasource")));
}

#[test]
fn supported_catalog_csv_mentions_headers_and_defaults() {
    let lines = crate::datasource_catalog::render_supported_datasource_catalog_csv();
    assert!(lines[0].contains("category,display_name,type"));
    assert!(lines.iter().any(|line| line.contains("Prometheus")));
    assert!(lines.iter().any(|line| line.contains("required")));
}

#[test]
fn supported_catalog_yaml_serializes_supported_types_document() {
    let yaml = crate::datasource_catalog::render_supported_datasource_catalog_yaml().unwrap();
    assert!(yaml.contains("kind: grafana-utils-datasource-supported-types"));
    assert!(yaml.contains("category:"));
    assert!(yaml.contains("displayName: Prometheus"));
}
