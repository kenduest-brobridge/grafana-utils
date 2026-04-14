//! Datasource payload construction and mutation behavior tests.

use super::*;

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
            org_name: normalized
                .get("org")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            org_id: normalized
                .get("orgId")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
            basic_auth: None,
            basic_auth_user: String::new(),
            database: String::new(),
            json_data: None,
            secure_json_data_placeholders: None,
            user: String::new(),
            with_credentials: None,
        };

        assert_eq!(build_import_payload(&record), expected_payload);
    }
}

#[test]
fn datasource_import_record_round_trips_through_inventory_shape() {
    let record = DatasourceImportRecord {
        uid: "loki-main".to_string(),
        name: "Loki Logs".to_string(),
        datasource_type: "loki".to_string(),
        access: "proxy".to_string(),
        url: "http://loki:3100".to_string(),
        is_default: false,
        org_name: "Observability".to_string(),
        org_id: "7".to_string(),
        basic_auth: Some(true),
        basic_auth_user: "loki-user".to_string(),
        database: "logs".to_string(),
        json_data: Some(
            json!({
                "maxLines": 1000
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
        secure_json_data_placeholders: Some(
            json!({
                "basicAuthPassword": "${secret:loki-main-basicauthpassword}"
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
        user: "query-user".to_string(),
        with_credentials: Some(true),
    };

    let inventory_record = record.to_inventory_record();
    let reparsed = DatasourceImportRecord::from_inventory_record(
        &inventory_record,
        "datasource inventory roundtrip test",
    )
    .unwrap();

    assert_eq!(reparsed, record);
}

#[test]
fn build_add_payload_keeps_optional_json_fields() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--uid",
        "prom-main",
        "--name",
        "Prometheus Main",
        "--type",
        "prometheus",
        "--access",
        "proxy",
        "--datasource-url",
        "http://prometheus:9090",
        "--default",
        "--json-data",
        r#"{"httpMethod":"POST"}"#,
        "--secure-json-data",
        r#"{"httpHeaderValue1":"secret"}"#,
    ]);
    let add_args = match args.command {
        crate::datasource::DatasourceGroupCommand::Add(inner) => inner,
        _ => panic!("expected datasource add"),
    };

    let payload = build_add_payload(&add_args).unwrap();

    assert_eq!(
        payload,
        json!({
            "uid": "prom-main",
            "name": "Prometheus Main",
            "type": "prometheus",
            "access": "proxy",
            "url": "http://prometheus:9090",
            "isDefault": true,
            "jsonData": {"httpMethod": "POST"},
            "secureJsonData": {"httpHeaderValue1": "secret"}
        })
    );
}

#[test]
fn build_add_payload_supports_datasource_auth_and_header_flags() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--uid",
        "prom-main",
        "--name",
        "Prometheus Main",
        "--type",
        "prometheus",
        "--datasource-url",
        "http://prometheus:9090",
        "--apply-supported-defaults",
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
    let add_args = match args.command {
        crate::datasource::DatasourceGroupCommand::Add(inner) => inner,
        _ => panic!("expected datasource add"),
    };

    let payload = build_add_payload(&add_args).unwrap();

    assert_eq!(payload["basicAuth"], json!(true));
    assert_eq!(payload["basicAuthUser"], json!("metrics-user"));
    assert_eq!(payload["user"], json!("query-user"));
    assert_eq!(payload["withCredentials"], json!(true));
    assert_eq!(payload["jsonData"]["httpMethod"], json!("POST"));
    assert_eq!(
        payload["jsonData"]["httpHeaderName1"],
        json!("X-Scope-OrgID")
    );
    assert_eq!(payload["jsonData"]["tlsSkipVerify"], json!(true));
    assert_eq!(
        payload["jsonData"]["serverName"],
        json!("prometheus.internal")
    );
    assert_eq!(
        payload["secureJsonData"]["basicAuthPassword"],
        json!("metrics-pass")
    );
    assert_eq!(payload["secureJsonData"]["password"], json!("query-pass"));
    assert_eq!(
        payload["secureJsonData"]["httpHeaderValue1"],
        json!("tenant-a")
    );
}

#[test]
fn build_add_payload_resolves_secret_placeholders_into_secure_json_data() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--uid",
        "loki-main",
        "--name",
        "Loki Main",
        "--type",
        "loki",
        "--secure-json-data-placeholders",
        r#"{"basicAuthPassword":"${secret:loki-basic-auth}","httpHeaderValue1":"${secret:loki-tenant-token}"}"#,
        "--secret-values",
        r#"{"loki-basic-auth":"secret-value","loki-tenant-token":"tenant-token"}"#,
    ]);
    let add_args = match args.command {
        crate::datasource::DatasourceGroupCommand::Add(inner) => inner,
        _ => panic!("expected datasource add"),
    };

    let payload = build_add_payload(&add_args).unwrap();

    assert_eq!(
        payload["secureJsonData"]["basicAuthPassword"],
        json!("secret-value")
    );
    assert_eq!(
        payload["secureJsonData"]["httpHeaderValue1"],
        json!("tenant-token")
    );
}

#[test]
fn build_add_payload_rejects_secret_values_without_placeholders() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Loki Main",
        "--type",
        "loki",
        "--secret-values",
        r#"{"loki-basic-auth":"secret-value"}"#,
    ]);
    let add_args = match args.command {
        crate::datasource::DatasourceGroupCommand::Add(inner) => inner,
        _ => panic!("expected datasource add"),
    };

    let error = build_add_payload(&add_args).unwrap_err().to_string();
    assert!(error.contains("--secret-values requires --secure-json-data-placeholders"));
}

#[test]
fn build_add_payload_rejects_missing_secret_values_with_visibility_summary() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Loki Main",
        "--type",
        "loki",
        "--secure-json-data-placeholders",
        r#"{"basicAuthPassword":"${secret:loki-basic-auth}","httpHeaderValue1":"${secret:loki-tenant-token}"}"#,
    ]);
    let add_args = match args.command {
        crate::datasource::DatasourceGroupCommand::Add(inner) => inner,
        _ => panic!("expected datasource add"),
    };

    let error = build_add_payload(&add_args).unwrap_err().to_string();

    assert!(error.contains("--secure-json-data-placeholders requires --secret-values"));
    assert!(error.contains("\"providerKind\":\"inline-placeholder-map\""));
    assert!(error.contains("\"provider\":{\"inputFlag\":\"--secret-values\""));
    assert!(error.contains("\"placeholderNames\":[\"loki-basic-auth\",\"loki-tenant-token\"]"));
    assert!(error.contains("\"secretFields\":[\"basicAuthPassword\",\"httpHeaderValue1\"]"));
}

#[test]
fn build_import_payload_resolves_secret_placeholders_into_secure_json_data() {
    let record = DatasourceImportRecord {
        uid: "loki-main".to_string(),
        name: "Loki Main".to_string(),
        datasource_type: "loki".to_string(),
        access: "proxy".to_string(),
        url: "http://loki:3100".to_string(),
        is_default: false,
        org_name: String::new(),
        org_id: "1".to_string(),
        basic_auth: None,
        basic_auth_user: String::new(),
        database: String::new(),
        json_data: None,
        secure_json_data_placeholders: json!({
            "basicAuthPassword": "${secret:loki-basic-auth}",
            "httpHeaderValue1": "${secret:loki-tenant-token}"
        })
        .as_object()
        .cloned(),
        user: String::new(),
        with_credentials: None,
    };

    let payload = build_import_payload_with_secret_values(
        &record,
        json!({
            "loki-basic-auth": "secret-value",
            "loki-tenant-token": "tenant-token"
        })
        .as_object(),
    )
    .unwrap();

    assert_eq!(
        payload["secureJsonData"]["basicAuthPassword"],
        json!("secret-value")
    );
    assert_eq!(
        payload["secureJsonData"]["httpHeaderValue1"],
        json!("tenant-token")
    );
}

#[test]
fn build_import_payload_rejects_missing_secret_values_for_placeholders() {
    let record = DatasourceImportRecord {
        uid: "loki-main".to_string(),
        name: "Loki Main".to_string(),
        datasource_type: "loki".to_string(),
        access: "proxy".to_string(),
        url: "http://loki:3100".to_string(),
        is_default: false,
        org_name: String::new(),
        org_id: "1".to_string(),
        basic_auth: None,
        basic_auth_user: String::new(),
        database: String::new(),
        json_data: None,
        secure_json_data_placeholders: json!({
            "basicAuthPassword": "${secret:loki-basic-auth}"
        })
        .as_object()
        .cloned(),
        user: String::new(),
        with_credentials: None,
    };

    let error = build_import_payload_with_secret_values(&record, None)
        .unwrap_err()
        .to_string();
    assert!(error.contains("requires --secret-values"));
}

#[test]
fn build_datasource_import_dry_run_json_value_includes_secret_visibility() {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let input_dir = std::env::temp_dir().join(format!(
        "grafana-utils-datasource-secret-{}-{}",
        std::process::id(),
        unique_suffix
    ));
    std::fs::create_dir_all(&input_dir).unwrap();
    std::fs::write(
        input_dir.join(crate::datasource::EXPORT_METADATA_FILENAME),
        format!(
            "{{\n  \"schemaVersion\": {},\n  \"kind\": \"{}\",\n  \"variant\": \"root\",\n  \"resource\": \"datasource\",\n  \"datasourceCount\": 1,\n  \"datasourcesFile\": \"{}\",\n  \"indexFile\": \"index.json\",\n  \"format\": \"grafana-datasource-inventory-v1\"\n}}\n",
            1,
            "grafana-utils-datasource-export-index",
            crate::datasource::DATASOURCE_EXPORT_FILENAME
        ),
    )
    .unwrap();
    std::fs::write(
        input_dir.join(crate::datasource::DATASOURCE_EXPORT_FILENAME),
        r#"[
  {
    "uid": "loki-main",
    "name": "Loki Main",
    "type": "loki",
    "access": "proxy",
    "url": "http://loki:3100",
    "isDefault": false,
    "orgId": "1",
    "secureJsonDataPlaceholders": {
      "basicAuthPassword": "${secret:loki-basic-auth}",
      "httpHeaderValue1": "${secret:loki-tenant-token}"
    }
  }
]
"#,
    )
    .unwrap();

    let report = crate::datasource::DatasourceImportDryRunReport {
        mode: "create-or-update".to_string(),
        input_dir: input_dir.clone(),
        input_format: DatasourceImportInputFormat::Inventory,
        source_org_id: "1".to_string(),
        target_org_id: "7".to_string(),
        rows: vec![vec![
            "loki-main".to_string(),
            "Loki Main".to_string(),
            "loki".to_string(),
            "name".to_string(),
            "missing".to_string(),
            "would-create".to_string(),
            "7".to_string(),
            "datasources.json#0".to_string(),
        ]],
        datasource_count: 1,
        would_create: 1,
        would_update: 0,
        would_skip: 0,
        would_block: 0,
    };

    let value =
        crate::datasource::datasource_import_export::build_datasource_import_dry_run_json_value(
            &report,
        );

    assert_eq!(
        value["kind"],
        json!("grafana-util-datasource-import-dry-run")
    );
    assert_eq!(value["schemaVersion"], json!(1));
    assert!(value.get("toolVersion").is_some());
    assert_eq!(value["reviewRequired"], json!(true));
    assert_eq!(value["reviewed"], json!(false));
    assert_eq!(value["summary"]["secretVisibilityCount"], json!(1));
    assert_eq!(value["secretVisibility"].as_array().unwrap().len(), 1);
    assert_eq!(
        value["secretVisibility"][0]["providerKind"],
        json!("inline-placeholder-map")
    );
    assert_eq!(
        value["secretVisibility"][0]["provider"]["inputFlag"],
        json!("--secret-values")
    );
    assert_eq!(
        value["secretVisibility"][0]["provider"]["placeholderNameStrategy"],
        json!("sanitize(<datasource-uid|name|type>-<secure-json-field>).lowercase")
    );
    assert_eq!(
        value["secretVisibility"][0]["placeholderNames"],
        json!(["loki-basic-auth", "loki-tenant-token"])
    );
    assert_eq!(
        value["secretVisibility"][0]["secretFields"],
        json!(["basicAuthPassword", "httpHeaderValue1"])
    );
    assert_eq!(value["datasources"][0]["matchBasis"], json!("name"));
}

#[test]
fn build_add_payload_rejects_basic_auth_password_without_user() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "add",
        "--name",
        "Prometheus Main",
        "--type",
        "prometheus",
        "--basic-auth-password",
        "metrics-pass",
    ]);
    let add_args = match args.command {
        crate::datasource::DatasourceGroupCommand::Add(inner) => inner,
        _ => panic!("expected datasource add"),
    };

    let error = build_add_payload(&add_args).unwrap_err().to_string();
    assert!(error.contains("requires --basic-auth-user"));
}

#[test]
fn build_add_payload_merges_nested_json_data_override_from_shared_fixture() {
    for case in load_nested_json_data_merge_cases() {
        if case["operation"] != json!("add") {
            continue;
        }
        let args = case["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>();
        let parsed = DatasourceCliArgs::parse_normalized_from(args);
        let add_args = match parsed.command {
            crate::datasource::DatasourceGroupCommand::Add(inner) => inner,
            _ => panic!("expected datasource add"),
        };

        let payload = build_add_payload(&add_args).unwrap();
        let expected = case["expected"].as_object().unwrap();

        assert_json_subset(&payload, &Value::Object(expected.clone()));
    }
}

#[test]
fn build_modify_updates_keeps_optional_json_fields() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "modify",
        "--uid",
        "prom-main",
        "--set-url",
        "http://prometheus-v2:9090",
        "--set-access",
        "direct",
        "--set-default",
        "true",
        "--json-data",
        r#"{"httpMethod":"POST"}"#,
        "--secure-json-data",
        r#"{"token":"abc123"}"#,
    ]);
    let modify_args = match args.command {
        crate::datasource::DatasourceGroupCommand::Modify(inner) => inner,
        _ => panic!("expected datasource modify"),
    };

    let updates = build_modify_updates(&modify_args).unwrap();

    assert_eq!(updates["url"], json!("http://prometheus-v2:9090"));
    assert_eq!(updates["access"], json!("direct"));
    assert_eq!(updates["isDefault"], json!(true));
    assert_eq!(updates["jsonData"]["httpMethod"], json!("POST"));
    assert_eq!(updates["secureJsonData"]["token"], json!("abc123"));
}

#[test]
fn build_modify_updates_supports_datasource_auth_and_header_flags() {
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
    let modify_args = match args.command {
        crate::datasource::DatasourceGroupCommand::Modify(inner) => inner,
        _ => panic!("expected datasource modify"),
    };

    let updates = build_modify_updates(&modify_args).unwrap();

    assert_eq!(updates["basicAuth"], json!(true));
    assert_eq!(updates["basicAuthUser"], json!("metrics-user"));
    assert_eq!(updates["user"], json!("query-user"));
    assert_eq!(updates["withCredentials"], json!(true));
    assert_eq!(
        updates["jsonData"]["httpHeaderName1"],
        json!("X-Scope-OrgID")
    );
    assert_eq!(updates["jsonData"]["tlsSkipVerify"], json!(true));
    assert_eq!(
        updates["jsonData"]["serverName"],
        json!("prometheus.internal")
    );
    assert_eq!(
        updates["secureJsonData"]["basicAuthPassword"],
        json!("metrics-pass")
    );
    assert_eq!(updates["secureJsonData"]["password"], json!("query-pass"));
    assert_eq!(
        updates["secureJsonData"]["httpHeaderValue1"],
        json!("tenant-b")
    );
}

#[test]
fn build_modify_updates_resolves_secret_placeholders_into_secure_json_data() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "modify",
        "--uid",
        "loki-main",
        "--secure-json-data-placeholders",
        r#"{"basicAuthPassword":"${secret:loki-basic-auth}","httpHeaderValue1":"${secret:loki-tenant-token}"}"#,
        "--secret-values",
        r#"{"loki-basic-auth":"secret-value","loki-tenant-token":"tenant-token"}"#,
    ]);
    let modify_args = match args.command {
        crate::datasource::DatasourceGroupCommand::Modify(inner) => inner,
        _ => panic!("expected datasource modify"),
    };

    let updates = build_modify_updates(&modify_args).unwrap();

    assert_eq!(
        updates["secureJsonData"]["basicAuthPassword"],
        json!("secret-value")
    );
    assert_eq!(
        updates["secureJsonData"]["httpHeaderValue1"],
        json!("tenant-token")
    );
}

#[test]
fn build_modify_payload_merges_existing_json_data() {
    let existing = json!({
        "id": 7,
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "url": "http://prometheus:9090",
        "access": "proxy",
        "isDefault": false,
        "basicAuth": true,
        "basicAuthUser": "metrics-user",
        "user": "query-user",
        "withCredentials": true,
        "jsonData": {
            "httpMethod": "POST"
        }
    })
    .as_object()
    .unwrap()
    .clone();
    let updates = json!({
        "url": "http://prometheus-v2:9090",
        "jsonData": {
            "timeInterval": "30s"
        },
        "secureJsonData": {
            "token": "abc123"
        }
    })
    .as_object()
    .unwrap()
    .clone();

    let payload = build_modify_payload(&existing, &updates).unwrap();

    assert_eq!(payload["url"], json!("http://prometheus-v2:9090"));
    assert_eq!(payload["basicAuth"], json!(true));
    assert_eq!(payload["basicAuthUser"], json!("metrics-user"));
    assert_eq!(payload["user"], json!("query-user"));
    assert_eq!(payload["withCredentials"], json!(true));
    assert_eq!(payload["jsonData"]["httpMethod"], json!("POST"));
    assert_eq!(payload["jsonData"]["timeInterval"], json!("30s"));
    assert_eq!(payload["secureJsonData"]["token"], json!("abc123"));
}

#[test]
fn build_modify_payload_rejects_basic_auth_password_without_basic_auth_user() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "modify",
        "--uid",
        "prom-main",
        "--basic-auth-password",
        "metrics-pass",
    ]);
    let modify_args = match args.command {
        crate::datasource::DatasourceGroupCommand::Modify(inner) => inner,
        _ => panic!("expected datasource modify"),
    };
    let updates = build_modify_updates(&modify_args).unwrap();
    let existing = json!({
        "id": 7,
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "url": "http://prometheus:9090",
        "access": "proxy",
        "isDefault": false
    })
    .as_object()
    .unwrap()
    .clone();

    let error = build_modify_payload(&existing, &updates).unwrap_err();

    assert!(error
        .to_string()
        .contains("--basic-auth-password requires --basic-auth-user or an existing basicAuthUser"));
}

#[test]
fn build_modify_payload_deep_merges_nested_json_data_override_from_shared_fixture() {
    for case in load_nested_json_data_merge_cases() {
        if case["operation"] != json!("modify") {
            continue;
        }
        let args = case["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>();
        let parsed = DatasourceCliArgs::parse_normalized_from(args);
        let modify_args = match parsed.command {
            crate::datasource::DatasourceGroupCommand::Modify(inner) => inner,
            _ => panic!("expected datasource modify"),
        };

        let updates = build_modify_updates(&modify_args).unwrap();
        let existing = case["existing"].as_object().unwrap().clone();
        let payload = build_modify_payload(&existing, &updates).unwrap();
        let expected = case["expected"].as_object().unwrap();

        assert_json_subset(&payload, &Value::Object(expected.clone()));
    }
}

#[test]
fn build_add_payload_preserves_explicit_secure_json_data_from_shared_fixture() {
    for case in load_secure_json_merge_cases() {
        if case["operation"] != json!("add") {
            continue;
        }
        let args = case["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>();
        let parsed = DatasourceCliArgs::parse_normalized_from(args);
        let add_args = match parsed.command {
            crate::datasource::DatasourceGroupCommand::Add(inner) => inner,
            _ => panic!("expected datasource add"),
        };

        let payload = build_add_payload(&add_args).unwrap();
        let expected = case["expected"].as_object().unwrap();

        assert_json_subset(&payload, &Value::Object(expected.clone()));
    }
}

#[test]
fn build_modify_payload_replaces_secure_json_data_from_shared_fixture() {
    for case in load_secure_json_merge_cases() {
        if case["operation"] != json!("modify") {
            continue;
        }
        let args = case["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>();
        let parsed = DatasourceCliArgs::parse_normalized_from(args);
        let modify_args = match parsed.command {
            crate::datasource::DatasourceGroupCommand::Modify(inner) => inner,
            _ => panic!("expected datasource modify"),
        };

        let updates = build_modify_updates(&modify_args).unwrap();
        let existing = case["existing"].as_object().unwrap().clone();
        let payload = build_modify_payload(&existing, &updates).unwrap();
        let expected = case["expected"].as_object().unwrap();

        assert_json_subset(&payload, &Value::Object(expected.clone()));
    }
}
