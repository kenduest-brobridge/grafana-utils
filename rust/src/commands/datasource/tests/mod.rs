//! Datasource domain test suite.
//! Exercises parsing + import/export/diff helpers, including mocked datasource matching
//! and contract fixtures.
use super::{
    build_add_payload, build_import_payload, build_import_payload_with_secret_values,
    build_modify_payload, build_modify_updates, parse_json_object_argument, render_data_source_csv,
    render_data_source_json, render_data_source_table, render_import_table,
    render_live_mutation_json, render_live_mutation_table, resolve_delete_match,
    resolve_live_mutation_match, resolve_match, CommonCliArgs, DatasourceCliArgs,
    DatasourceImportInputFormat, DatasourceImportRecord,
};
use crate::common::CliColorChoice;
use crate::datasource_catalog::render_supported_datasource_catalog_json;
use serde_json::{json, Value};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

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
        "../../../../../fixtures/datasource_contract_cases.json"
    ))
    .unwrap()
}

fn load_nested_json_data_merge_cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../../../fixtures/datasource_nested_json_data_merge_cases.json"
    ))
    .unwrap()
}

fn load_secure_json_merge_cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../../../fixtures/datasource_secure_json_merge_cases.json"
    ))
    .unwrap()
}

fn load_preset_profile_add_payload_cases() -> Vec<Value> {
    let document: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/datasource_preset_profile_add_payload_cases.json"
    ))
    .unwrap();
    document["cases"].as_array().cloned().unwrap()
}

fn load_supported_types_catalog_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../fixtures/datasource_supported_types_catalog.json"
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
        color: CliColorChoice::Auto,
        profile: None,
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

#[path = "cli_mutation.rs"]
mod datasource_cli_mutation_rust_tests;

#[path = "cli_mutation_tail.rs"]
mod datasource_cli_mutation_tail_rust_tests;

#[path = "payload.rs"]
mod datasource_payload_rust_tests;

#[path = "parse_inventory.rs"]
mod datasource_parse_inventory_rust_tests;

#[path = "render.rs"]
mod datasource_render_rust_tests;

#[path = "tail.rs"]
mod datasource_rust_tests_tail_rust_tests;

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

#[test]
fn resolve_delete_preview_type_uses_matching_live_datasource_type() {
    let live = vec![
        live_datasource(7, "prom-main", "Prometheus Main", "prometheus"),
        live_datasource(9, "loki-ops", "Loki Ops", "loki"),
    ];

    assert_eq!(
        super::resolve_delete_preview_type(Some(7), &live),
        "prometheus"
    );
    assert_eq!(super::resolve_delete_preview_type(Some(9), &live), "loki");
    assert_eq!(super::resolve_delete_preview_type(Some(42), &live), "");
    assert_eq!(super::resolve_delete_preview_type(None, &live), "");
}
