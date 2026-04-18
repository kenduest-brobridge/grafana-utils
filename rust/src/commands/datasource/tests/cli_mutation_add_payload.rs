use super::super::*;

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
