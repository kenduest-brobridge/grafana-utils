// Datasource provider contract unit tests.
// Keeps staged secret-provider parsing and review-summary behavior aligned with Python.

use crate::datasource_provider::{
    build_provider_plan, collect_provider_references, iter_provider_names,
    summarize_provider_plan,
};
use serde_json::json;

#[test]
fn collect_provider_references_rejects_opaque_secret_replay() {
    let secure_json_data = json!({
        "basicAuthPassword": "already-a-secret"
    });

    let error = collect_provider_references(secure_json_data.as_object())
        .unwrap_err()
        .to_string();

    assert!(error.contains("opaque replay is not allowed"));
}

#[test]
fn build_provider_plan_shapes_review_summary() {
    let datasource_spec = json!({
        "uid": "loki-main",
        "name": "Loki Main",
        "type": "loki",
        "secureJsonDataProviders": {
            "basicAuthPassword": "${provider:vault:secret/data/loki/basic-auth}",
            "httpHeaderValue1": "${provider:aws-sm:prod/loki/token}",
        }
    });

    let plan = build_provider_plan(datasource_spec.as_object().unwrap()).unwrap();

    assert_eq!(plan.provider_kind, "external-provider-reference");
    assert!(plan.review_required);
    assert_eq!(
        summarize_provider_plan(&plan),
        json!({
            "datasourceUid": "loki-main",
            "datasourceName": "Loki Main",
            "datasourceType": "loki",
            "providerKind": "external-provider-reference",
            "action": "resolve-provider-secrets",
            "reviewRequired": true,
            "providers": [
                {
                    "fieldName": "basicAuthPassword",
                    "providerName": "vault",
                    "secretPath": "secret/data/loki/basic-auth",
                },
                {
                    "fieldName": "httpHeaderValue1",
                    "providerName": "aws-sm",
                    "secretPath": "prod/loki/token",
                },
            ],
        })
    );
}

#[test]
fn iter_provider_names_deduplicates_names() {
    let datasource_spec = json!({
        "name": "Prometheus Main",
        "type": "prometheus",
        "secureJsonDataProviders": {
            "password": "${provider:vault:secret/a}",
            "httpHeaderValue1": "${provider:vault:secret/b}",
            "httpHeaderValue2": "${provider:aws-sm:secret/c}",
        }
    });

    let plan = build_provider_plan(datasource_spec.as_object().unwrap()).unwrap();
    let provider_names = iter_provider_names(&plan.references).collect::<Vec<_>>();

    assert_eq!(provider_names, vec!["vault", "aws-sm"]);
}

#[test]
fn build_provider_plan_rejects_missing_datasource_name() {
    let datasource_spec = json!({
        "type": "loki",
        "secureJsonDataProviders": {
            "basicAuthPassword": "${provider:vault:secret/data/loki/basic-auth}",
        }
    });

    let error = build_provider_plan(datasource_spec.as_object().unwrap())
        .unwrap_err()
        .to_string();

    assert!(error.contains("requires a datasource name"));
}
