use crate::datasource_secret::{
    build_secret_placeholder_plan, collect_secret_placeholders, iter_secret_placeholder_names,
    resolve_secret_placeholders,
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
    assert_eq!(
        iter_secret_placeholder_names(&plan.placeholders).collect::<Vec<_>>(),
        vec!["loki-basic-auth", "loki-tenant-token"]
    );
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
