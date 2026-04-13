//! Rust regression coverage for Core behavior at this module boundary.

use crate::datasource_secret::{
    build_inline_secret_placeholder_name, build_inline_secret_placeholder_token,
    build_secret_placeholder_plan, collect_secret_placeholders, describe_secret_placeholder_plan,
    iter_secret_placeholder_names, resolve_secret_placeholders, summarize_secret_placeholder_plan,
};
use serde_json::json;

#[test]
fn collect_secret_placeholders_rejects_raw_secret_values() {
    let secure_json_data = json!({
        "basicAuthPassword": "plain-text-secret"
    });
    let error = collect_secret_placeholders(secure_json_data.as_object())
        .unwrap_err()
        .to_string();

    assert!(error.contains("${secret:...} placeholders"));
}

#[test]
fn build_secret_placeholder_plan_shapes_review_summary() {
    let datasource = json!({
        "uid": "loki-main",
        "name": "Loki Main",
        "type": "loki",
        "secureJsonDataPlaceholders": {
            "basicAuthPassword": "${secret:loki-basic-auth}",
            "httpHeaderValue1": "${secret:loki-tenant-token}",
            "httpHeaderValue2": "${secret:loki-tenant-token}"
        }
    });
    let plan = build_secret_placeholder_plan(datasource.as_object().unwrap()).unwrap();

    assert_eq!(plan.datasource_uid.as_deref(), Some("loki-main"));
    assert_eq!(plan.placeholders.len(), 3);
    assert_eq!(plan.provider.kind, "inline-placeholder-map");
    assert_eq!(plan.provider.input_flag, "--secret-values");
    assert_eq!(
        iter_secret_placeholder_names(&plan.placeholders).collect::<Vec<_>>(),
        vec!["loki-basic-auth", "loki-tenant-token"]
    );
}

#[test]
fn inline_secret_provider_builds_stable_placeholder_name_contract() {
    assert_eq!(
        build_inline_secret_placeholder_name("Loki Main", "httpHeaderValue1"),
        "loki_main-httpheadervalue1"
    );
    assert_eq!(
        build_inline_secret_placeholder_token("Loki Main", "httpHeaderValue1"),
        "${secret:loki_main-httpheadervalue1}"
    );
}

#[test]
fn summarize_secret_placeholder_plan_exposes_provider_contract_for_error_reporting() {
    let datasource = json!({
        "uid": "loki-main",
        "name": "Loki Main",
        "type": "loki",
        "secureJsonDataPlaceholders": {
            "basicAuthPassword": "${secret:loki-basic-auth}",
            "httpHeaderValue1": "${secret:loki-tenant-token}"
        }
    });
    let plan = build_secret_placeholder_plan(datasource.as_object().unwrap()).unwrap();

    let summary = summarize_secret_placeholder_plan(&plan);
    let described = describe_secret_placeholder_plan(&plan);

    assert_eq!(
        summary,
        json!({
            "datasourceUid": "loki-main",
            "datasourceName": "Loki Main",
            "datasourceType": "loki",
            "providerKind": "inline-placeholder-map",
            "provider": {
                "kind": "inline-placeholder-map",
                "inputFlag": "--secret-values",
                "placeholderFormat": "${secret:<placeholder-name>}",
                "placeholderNameStrategy": "sanitize(<datasource-uid|name|type>-<secure-json-field>).lowercase"
            },
            "action": "inject-secrets",
            "reviewRequired": true,
            "secretFields": ["basicAuthPassword", "httpHeaderValue1"],
            "placeholderNames": ["loki-basic-auth", "loki-tenant-token"]
        })
    );
    assert!(described.contains("\"providerKind\":\"inline-placeholder-map\""));
    assert!(described.contains("\"provider\":{\"inputFlag\":\"--secret-values\""));
}

#[test]
fn resolve_secret_placeholders_reports_all_missing_or_empty_values() {
    let datasource = json!({
        "uid": "loki-main",
        "name": "Loki Main",
        "type": "loki",
        "secureJsonDataPlaceholders": {
            "basicAuthPassword": "${secret:loki-basic-auth}",
            "httpHeaderValue1": "${secret:loki-tenant-token}"
        }
    });
    let plan = build_secret_placeholder_plan(datasource.as_object().unwrap()).unwrap();

    let missing_error = resolve_secret_placeholders(
        &plan.placeholders,
        json!({"loki-basic-auth": "secret-value"})
            .as_object()
            .unwrap(),
    )
    .unwrap_err()
    .to_string();
    assert!(missing_error.contains("must resolve to non-empty strings before import"));
    assert!(missing_error.contains("loki-tenant-token"));

    let empty_error = resolve_secret_placeholders(
        &plan.placeholders,
        json!({
            "loki-basic-auth": "",
        })
        .as_object()
        .unwrap(),
    )
    .unwrap_err()
    .to_string();
    assert!(empty_error.contains("must resolve to non-empty strings before import"));
    assert!(empty_error.contains("loki-basic-auth"));
    assert!(empty_error.contains("loki-tenant-token"));
}

#[test]
fn resolve_secret_placeholders_builds_secure_json_data_map() {
    let datasource = json!({
        "uid": "loki-main",
        "name": "Loki Main",
        "type": "loki",
        "secureJsonDataPlaceholders": {
            "basicAuthPassword": "${secret:loki-basic-auth}",
            "httpHeaderValue1": "${secret:loki-tenant-token}"
        }
    });
    let plan = build_secret_placeholder_plan(datasource.as_object().unwrap()).unwrap();

    let resolved = resolve_secret_placeholders(
        &plan.placeholders,
        json!({
            "loki-basic-auth": "secret-value",
            "loki-tenant-token": "tenant-token"
        })
        .as_object()
        .unwrap(),
    )
    .unwrap();

    assert_eq!(resolved["basicAuthPassword"], json!("secret-value"));
    assert_eq!(resolved["httpHeaderValue1"], json!("tenant-token"));
}
