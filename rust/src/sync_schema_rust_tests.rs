use crate::sync_bundle_preflight::build_sync_bundle_preflight_document;
use crate::sync_contracts::{
    build_sync_apply_intent_document, build_sync_plan_document, build_sync_summary_document,
};
use crate::sync_preflight::build_sync_preflight_document;
use serde_json::Value;
use std::path::Path;

fn load_contract_cases() -> Value {
    serde_json::from_str(include_str!(
        "../../tests/fixtures/rust_sync_contract_cases.json"
    ))
    .unwrap()
}

fn load_preflight_cases() -> Value {
    serde_json::from_str(include_str!(
        "../../tests/fixtures/rust_sync_preflight_cases.json"
    ))
    .unwrap()
}

fn load_schema(path: &str) -> Value {
    match path {
        "summary" => serde_json::from_str(include_str!(
            "../../docs/internal/schemas/grafana-utils-sync-summary.schema.json"
        ))
        .unwrap(),
        "plan" => serde_json::from_str(include_str!(
            "../../docs/internal/schemas/grafana-utils-sync-plan.schema.json"
        ))
        .unwrap(),
        "preflight" => serde_json::from_str(include_str!(
            "../../docs/internal/schemas/grafana-utils-sync-preflight.schema.json"
        ))
        .unwrap(),
        "bundle-preflight" => serde_json::from_str(include_str!(
            "../../docs/internal/schemas/grafana-utils-sync-bundle-preflight.schema.json"
        ))
        .unwrap(),
        "apply-intent" => serde_json::from_str(include_str!(
            "../../docs/internal/schemas/grafana-utils-sync-apply-intent.schema.json"
        ))
        .unwrap(),
        other => panic!("unsupported schema key {other}"),
    }
}

fn load_schema_index() -> Value {
    serde_json::from_str(include_str!("../../docs/internal/schemas/index.json")).unwrap()
}

fn schema_kind_const(schema: &Value) -> String {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .and_then(|properties| properties.get("kind"))
        .and_then(Value::as_object)
        .and_then(|kind_schema| kind_schema.get("const"))
        .and_then(Value::as_str)
        .unwrap()
        .to_string()
}

fn attach_lineage(
    document: &Value,
    trace_id: &str,
    stage: &str,
    step_index: i64,
    parent_trace_id: Option<&str>,
) -> Value {
    let mut object = document.as_object().cloned().unwrap();
    object.insert("traceId".to_string(), Value::String(trace_id.to_string()));
    object.insert("stage".to_string(), Value::String(stage.to_string()));
    object.insert("stepIndex".to_string(), Value::Number(step_index.into()));
    if let Some(parent) = parent_trace_id {
        object.insert(
            "parentTraceId".to_string(),
            Value::String(parent.to_string()),
        );
    }
    Value::Object(object)
}

fn assert_required_and_consts(schema: &Value, document: &Value, label: &str) {
    let schema_object = schema
        .as_object()
        .unwrap_or_else(|| panic!("{label}: schema root must be object"));
    let document_object = document
        .as_object()
        .unwrap_or_else(|| panic!("{label}: document root must be object"));

    if let Some(required) = schema_object.get("required").and_then(Value::as_array) {
        for field in required {
            let key = field
                .as_str()
                .unwrap_or_else(|| panic!("{label}: required entry must be a string"));
            assert!(
                document_object.contains_key(key),
                "{label}: missing required field {key:?}"
            );
        }
    }

    let properties = schema_object
        .get("properties")
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("{label}: schema missing properties"));
    for (key, property_schema) in properties {
        let Some(expected) = property_schema.get("const") else {
            continue;
        };
        let actual = document_object
            .get(key)
            .unwrap_or_else(|| panic!("{label}: missing const field {key:?}"));
        assert_eq!(
            actual, expected,
            "{label}: field {key:?} did not match schema const"
        );
    }
}

#[test]
fn rust_sync_schemas_are_machine_readable_and_define_required_contract_fields() {
    let entries = vec![
        (
            "summary",
            "https://grafana-utils.local/docs/internal/schemas/grafana-utils-sync-summary.schema.json",
            "grafana-utils-sync-summary",
        ),
        (
            "plan",
            "https://grafana-utils.local/docs/internal/schemas/grafana-utils-sync-plan.schema.json",
            "grafana-utils-sync-plan",
        ),
        (
            "preflight",
            "https://grafana-utils.local/docs/internal/schemas/grafana-utils-sync-preflight.schema.json",
            "grafana-utils-sync-preflight",
        ),
        (
            "bundle-preflight",
            "https://grafana-utils.local/docs/internal/schemas/grafana-utils-sync-bundle-preflight.schema.json",
            "grafana-utils-sync-bundle-preflight",
        ),
        (
            "apply-intent",
            "https://grafana-utils.local/docs/internal/schemas/grafana-utils-sync-apply-intent.schema.json",
            "grafana-utils-sync-apply-intent",
        ),
    ];
    for (key, schema_id, kind) in entries {
        let schema = load_schema(key);
        assert_eq!(
            schema.get("$schema").and_then(Value::as_str),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
        assert_eq!(schema.get("$id").and_then(Value::as_str), Some(schema_id));
        assert_eq!(
            schema
                .get("properties")
                .and_then(Value::as_object)
                .and_then(|properties| properties.get("kind"))
                .and_then(Value::as_object)
                .and_then(|kind_schema| kind_schema.get("const"))
                .and_then(Value::as_str),
            Some(kind)
        );
    }
}

#[test]
fn rust_sync_schema_index_matches_schema_files() {
    let index = load_schema_index();
    assert_eq!(
        index.get("kind").and_then(Value::as_str),
        Some("grafana-utils-sync-schema-index")
    );
    assert_eq!(index.get("schemaVersion").and_then(Value::as_i64), Some(1));
    let artifacts = index
        .get("artifacts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap();
    assert_eq!(artifacts.len(), 5);

    let expected = vec![
        ("summary", false),
        ("plan", true),
        ("preflight", true),
        ("bundle-preflight", true),
        ("apply-intent", true),
    ];
    let schemas = expected
        .iter()
        .map(|(key, _)| load_schema(key))
        .collect::<Vec<Value>>();

    for artifact in artifacts {
        let entry = artifact.as_object().unwrap();
        let kind = entry.get("kind").and_then(Value::as_str).unwrap();
        let path = entry.get("path").and_then(Value::as_str).unwrap();
        let id = entry.get("id").and_then(Value::as_str).unwrap();
        let staged = entry.get("staged").and_then(Value::as_bool).unwrap();

        let absolute = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
        assert!(
            absolute.exists(),
            "schema path from index does not exist: {}",
            absolute.display()
        );

        let schema = schemas
            .iter()
            .find(|item| schema_kind_const(item) == kind)
            .unwrap_or_else(|| panic!("schema kind from index not found: {kind}"));
        assert_eq!(schema.get("$id").and_then(Value::as_str), Some(id));
        if staged {
            assert!(schema
                .get("required")
                .and_then(Value::as_array)
                .unwrap()
                .iter()
                .any(|field| field.as_str() == Some("traceId")));
        }
    }
}

#[test]
fn rust_sync_summary_document_matches_summary_schema_contract() {
    let contract_cases = load_contract_cases();
    let raw_specs = contract_cases["summaryCase"]["rawSpecs"]
        .as_array()
        .unwrap();
    let schema = load_schema("summary");
    let document = build_sync_summary_document(raw_specs).unwrap();

    assert_required_and_consts(&schema, &document, "sync-summary");
}

#[test]
fn rust_sync_plan_document_matches_plan_schema_contract() {
    let contract_cases = load_contract_cases();
    let desired_specs = contract_cases["planCase"]["desiredSpecs"]
        .as_array()
        .unwrap();
    let live_specs = contract_cases["planCase"]["liveSpecs"].as_array().unwrap();
    let allow_prune = contract_cases["planCase"]["allowPrune"].as_bool().unwrap();
    let schema = load_schema("plan");
    let document = attach_lineage(
        &build_sync_plan_document(desired_specs, live_specs, allow_prune).unwrap(),
        "sync-trace-schema-test",
        "plan",
        1,
        None,
    );

    assert_required_and_consts(&schema, &document, "sync-plan");
}

#[test]
fn rust_sync_preflight_document_matches_preflight_schema_contract() {
    let preflight_cases = load_preflight_cases();
    let case = preflight_cases["preflightCases"][0]
        .as_object()
        .unwrap()
        .clone();
    let desired_specs = case.get("desiredSpecs").and_then(Value::as_array).unwrap();
    let availability = case.get("availability").cloned().unwrap();
    let schema = load_schema("preflight");
    let document = attach_lineage(
        &build_sync_preflight_document(desired_specs, Some(&availability)).unwrap(),
        "sync-trace-schema-test",
        "preflight",
        2,
        Some("sync-trace-schema-test"),
    );

    assert_required_and_consts(&schema, &document, "sync-preflight");
}

#[test]
fn rust_sync_bundle_preflight_document_matches_bundle_schema_contract() {
    let preflight_cases = load_preflight_cases();
    let case = preflight_cases["bundlePreflightCases"][0]
        .as_object()
        .unwrap()
        .clone();
    let source_bundle = case.get("sourceBundle").cloned().unwrap();
    let target_inventory = case.get("targetInventory").cloned().unwrap();
    let availability = case.get("availability").cloned().unwrap();
    let schema = load_schema("bundle-preflight");
    let document = attach_lineage(
        &build_sync_bundle_preflight_document(
            &source_bundle,
            &target_inventory,
            Some(&availability),
        )
        .unwrap(),
        "sync-trace-schema-test",
        "bundle-preflight",
        2,
        Some("sync-trace-schema-test"),
    );

    assert_required_and_consts(&schema, &document, "sync-bundle-preflight");
}

#[test]
fn rust_sync_apply_intent_document_matches_apply_intent_schema_contract() {
    let contract_cases = load_contract_cases();
    let reviewed_plan = contract_cases["applyCase"]["reviewedPlan"].clone();
    let schema = load_schema("apply-intent");
    let document = attach_lineage(
        &build_sync_apply_intent_document(&reviewed_plan, true).unwrap(),
        "sync-trace-schema-test",
        "apply",
        3,
        Some("sync-trace-schema-test"),
    );

    assert_required_and_consts(&schema, &document, "sync-apply-intent");
}
