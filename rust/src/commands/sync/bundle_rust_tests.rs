//! Sync bundle test helpers and nested regression modules.
//!
//! The shared fixture helpers stay here so the split test modules can reuse
//! them without duplicating setup logic.
#![allow(unused_imports)]

use crate::dashboard::CommonCliArgs;
use serde_json::{json, Value};

fn sync_common_args() -> CommonCliArgs {
    CommonCliArgs {
        color: crate::common::CliColorChoice::Auto,
        profile: None,
        url: "http://127.0.0.1:3000".to_string(),
        api_token: Some("test-token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

fn load_alert_export_contract_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../fixtures/alert_export_contract_cases.json"
    ))
    .unwrap()
}

fn load_sync_source_bundle_contract_fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../fixtures/sync_source_bundle_contract_cases.json"
    ))
    .unwrap()
}

fn build_alert_replay_artifact_fixture(include_rules: bool) -> Value {
    let fixture = load_alert_export_contract_fixture();
    let mut summary = fixture["syncAlertingArtifacts"]["summary"].clone();
    let artifacts = fixture["syncAlertingArtifacts"]["artifacts"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut alerting = serde_json::Map::new();
    if !include_rules {
        if let Some(summary_object) = summary.as_object_mut() {
            summary_object.insert("ruleCount".to_string(), json!(0));
        }
    }
    alerting.insert("summary".to_string(), summary);
    for artifact in artifacts {
        let section = artifact["section"].as_str().unwrap_or_default();
        if !include_rules && section == "rules" {
            continue;
        }
        let identity_field = artifact["identityField"].as_str().unwrap_or_default();
        let identity = artifact["identity"].clone();
        let source_path = artifact["sourcePath"].clone();
        let kind = match section {
            "rules" => "grafana-alert-rule",
            "contactPoints" => "grafana-contact-point",
            "muteTimings" => "grafana-mute-timing",
            "policies" => "grafana-notification-policies",
            "templates" => "grafana-notification-template",
            _ => "",
        };
        let mut spec = serde_json::Map::new();
        spec.insert(identity_field.to_string(), identity);
        let mut document = serde_json::Map::new();
        document.insert("kind".to_string(), json!(kind));
        document.insert("spec".to_string(), Value::Object(spec));
        let entry = json!({
            "sourcePath": source_path,
            "document": Value::Object(document)
        });
        alerting
            .entry(section.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        alerting
            .get_mut(section)
            .and_then(Value::as_array_mut)
            .unwrap()
            .push(entry);
    }
    Value::Object(alerting)
}

#[cfg(test)]
#[path = "bundle_contract_rust_tests.rs"]
mod sync_bundle_contract_rust_tests;

#[cfg(test)]
#[path = "bundle_exec_rust_tests.rs"]
mod sync_bundle_exec_rust_tests;
