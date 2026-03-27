//! Datasource domain test suite.
//! Exercises parsing + import/export/diff helpers, including mocked datasource matching
//! and contract fixtures.
use super::{
    build_add_payload, build_import_payload, build_modify_payload, build_modify_updates,
    parse_json_object_argument, render_data_source_csv, render_data_source_json,
    render_data_source_table, render_import_table, render_live_mutation_json,
    render_live_mutation_table, resolve_delete_match, resolve_live_mutation_match, resolve_match,
    CommonCliArgs, DatasourceCliArgs, DatasourceImportRecord,
};
use crate::datasource_catalog::render_supported_datasource_catalog_json;
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

fn load_contract_cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../fixtures/datasource_contract_cases.json"
    ))
    .unwrap()
}

fn load_nested_json_data_merge_cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../fixtures/datasource_nested_json_data_merge_cases.json"
    ))
    .unwrap()
}

fn load_secure_json_merge_cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../fixtures/datasource_secure_json_merge_cases.json"
    ))
    .unwrap()
}

fn load_preset_profile_add_payload_cases() -> Vec<Value> {
    let document: Value = serde_json::from_str(include_str!(
        "../../fixtures/datasource_preset_profile_add_payload_cases.json"
    ))
    .unwrap();
    document["cases"].as_array().cloned().unwrap()
}

fn load_supported_types_catalog_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../fixtures/datasource_supported_types_catalog.json"
    ))
    .unwrap()
}

fn project_supported_types_catalog(document: &Value) -> Value {
    json!({
        "kind": document["kind"].clone(),
        "categories": document["categories"]
            .as_array()
            .unwrap()
            .iter()
            .map(|category| {
                json!({
                    "category": category["category"].clone(),
                    "types": category["types"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|datasource_type| {
                            json!({
                                "type": datasource_type["type"].clone(),
                                "profile": datasource_type["profile"].clone(),
                                "queryLanguage": datasource_type["queryLanguage"].clone(),
                                "requiresDatasourceUrl": datasource_type["requiresDatasourceUrl"].clone(),
                                "suggestedFlags": datasource_type["suggestedFlags"].clone(),
                                "presetProfiles": datasource_type["presetProfiles"].clone(),
                                "addDefaults": datasource_type["addDefaults"].clone(),
                                "fullAddDefaults": datasource_type["fullAddDefaults"].clone(),
                            })
                        })
                        .collect::<Vec<_>>(),
                })
            })
            .collect::<Vec<_>>(),
    })
}

fn assert_json_subset(actual: &Value, expected: &Value) {
    match expected {
        Value::Object(expected_object) => {
            let actual_object = actual
                .as_object()
                .unwrap_or_else(|| panic!("expected object, got {actual:?}"));
            for (key, expected_value) in expected_object {
                let actual_value = actual_object
                    .get(key)
                    .unwrap_or_else(|| panic!("missing key {key} in {actual:?}"));
                assert_json_subset(actual_value, expected_value);
            }
        }
        Value::Array(expected_items) => {
            let actual_items = actual
                .as_array()
                .unwrap_or_else(|| panic!("expected array, got {actual:?}"));
            assert_eq!(actual_items.len(), expected_items.len());
            for (actual_item, expected_item) in actual_items.iter().zip(expected_items.iter()) {
                assert_json_subset(actual_item, expected_item);
            }
        }
        _ => assert_eq!(actual, expected),
    }
}

fn test_datasource_common_args() -> CommonCliArgs {
    CommonCliArgs {
        url: "http://grafana.example".to_string(),
        api_token: None,
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

#[path = "datasource_cli_mutation_rust_tests.rs"]
mod datasource_cli_mutation_rust_tests;

#[path = "datasource_cli_mutation_tail_rust_tests.rs"]
mod datasource_cli_mutation_tail_rust_tests;

#[path = "datasource_rust_tests_tail_rust_tests.rs"]
mod datasource_rust_tests_tail_rust_tests;

#[test]
fn render_import_table_honors_selected_columns() {
    let rows = vec![vec![
        "prom-main".to_string(),
        "Prometheus Main".to_string(),
        "prometheus".to_string(),
        "exists-uid".to_string(),
        "would-update".to_string(),
        "7".to_string(),
        "datasources.json#0".to_string(),
    ]];

    let lines = render_import_table(
        &rows,
        true,
        Some(&[
            "uid".to_string(),
            "action".to_string(),
            "org_id".to_string(),
        ]),
    );

    assert!(lines[0].contains("UID"));
    assert!(lines[0].contains("ACTION"));
    assert!(lines[0].contains("ORG_ID"));
    assert!(!lines[0].contains("NAME"));
    assert!(lines[2].contains("prom-main"));
    assert!(lines[2].contains("would-update"));
    assert!(lines[2].contains("7"));
}

#[test]
fn parse_datasource_import_preserves_requested_path() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
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

#[test]
fn parse_datasource_import_supports_output_format_table() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./datasources",
        "--dry-run",
        "--output-format",
        "table",
    ]);

    match args.command {
        super::DatasourceGroupCommand::Import(inner) => {
            assert!(inner.dry_run);
            assert!(inner.table);
            assert!(!inner.json);
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_import_supports_output_columns() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./datasources",
        "--dry-run",
        "--output-format",
        "table",
        "--output-columns",
        "uid,action,orgId,file",
    ]);

    match args.command {
        super::DatasourceGroupCommand::Import(inner) => {
            assert!(inner.table);
            assert_eq!(
                inner.output_columns,
                vec!["uid", "action", "org_id", "file"]
            );
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_export_supports_org_scope_flags() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "export",
        "--export-dir",
        "./datasources",
        "--org-id",
        "7",
    ]);

    match args.command {
        super::DatasourceGroupCommand::Export(inner) => {
            assert_eq!(inner.export_dir, Path::new("./datasources"));
            assert_eq!(inner.org_id, Some(7));
            assert!(!inner.all_orgs);
        }
        _ => panic!("expected datasource export"),
    }
}

#[test]
fn parse_datasource_export_supports_all_orgs_flag() {
    let args = DatasourceCliArgs::parse_normalized_from(["grafana-util", "export", "--all-orgs"]);

    match args.command {
        super::DatasourceGroupCommand::Export(inner) => {
            assert!(inner.all_orgs);
            assert_eq!(inner.org_id, None);
        }
        _ => panic!("expected datasource export"),
    }
}

#[test]
fn parse_datasource_import_supports_use_export_org_flags() {
    let args = DatasourceCliArgs::parse_normalized_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./datasources",
        "--use-export-org",
        "--only-org-id",
        "2",
        "--only-org-id",
        "5",
        "--create-missing-orgs",
    ]);

    match args.command {
        super::DatasourceGroupCommand::Import(inner) => {
            assert!(inner.use_export_org);
            assert_eq!(inner.only_org_id, vec![2, 5]);
            assert!(inner.create_missing_orgs);
            assert_eq!(inner.org_id, None);
        }
        _ => panic!("expected datasource import"),
    }
}

#[test]
fn parse_datasource_import_rejects_org_id_with_use_export_org() {
    let error = DatasourceCliArgs::try_parse_from([
        "grafana-util",
        "import",
        "--import-dir",
        "./datasources",
        "--org-id",
        "7",
        "--use-export-org",
    ])
    .unwrap_err();

    assert!(error.to_string().contains("--org-id"));
    assert!(error.to_string().contains("--use-export-org"));
}

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
            org_id: normalized
                .get("orgId")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
        };

        assert_eq!(build_import_payload(&record), expected_payload);
    }
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
        super::DatasourceGroupCommand::Add(inner) => inner,
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
        super::DatasourceGroupCommand::Add(inner) => inner,
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
        super::DatasourceGroupCommand::Add(inner) => inner,
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
            super::DatasourceGroupCommand::Add(inner) => inner,
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
        super::DatasourceGroupCommand::Modify(inner) => inner,
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
        super::DatasourceGroupCommand::Modify(inner) => inner,
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
        super::DatasourceGroupCommand::Modify(inner) => inner,
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
            super::DatasourceGroupCommand::Modify(inner) => inner,
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
            super::DatasourceGroupCommand::Add(inner) => inner,
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
            super::DatasourceGroupCommand::Modify(inner) => inner,
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
fn parse_json_object_argument_rejects_non_object_values() {
    let error = parse_json_object_argument(Some("[]"), "--json-data").unwrap_err();

    assert!(error
        .to_string()
        .contains("--json-data must decode to a JSON object."));
}

#[test]
fn render_live_mutation_table_can_omit_header() {
    let rows = vec![vec![
        "add".to_string(),
        "prom-main".to_string(),
        "Prometheus Main".to_string(),
        "prometheus".to_string(),
        "missing".to_string(),
        "would-create".to_string(),
        String::new(),
    ]];

    let lines = render_live_mutation_table(&rows, false);

    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("would-create"));
    assert!(!lines[0].contains("OPERATION"));
}

#[test]
fn render_live_mutation_json_summarizes_actions() {
    let value = render_live_mutation_json(&[
        vec![
            "add".to_string(),
            "prom-main".to_string(),
            "Prometheus Main".to_string(),
            "prometheus".to_string(),
            "missing".to_string(),
            "would-create".to_string(),
            String::new(),
        ],
        vec![
            "modify".to_string(),
            "prom-mid".to_string(),
            "Prometheus Updated".to_string(),
            "prometheus".to_string(),
            "exists-uid".to_string(),
            "would-update".to_string(),
            "9".to_string(),
        ],
        vec![
            "delete".to_string(),
            "prom-main".to_string(),
            "Prometheus Main".to_string(),
            String::new(),
            "exists-uid".to_string(),
            "would-delete".to_string(),
            "7".to_string(),
        ],
        vec![
            "add".to_string(),
            String::new(),
            "Prometheus Main".to_string(),
            "prometheus".to_string(),
            "exists-name".to_string(),
            "would-fail-existing-name".to_string(),
            "7".to_string(),
        ],
    ]);

    assert_eq!(value["summary"]["itemCount"], json!(4));
    assert_eq!(value["summary"]["createCount"], json!(1));
    assert_eq!(value["summary"]["updateCount"], json!(1));
    assert_eq!(value["summary"]["deleteCount"], json!(1));
    assert_eq!(value["summary"]["blockedCount"], json!(1));
}
