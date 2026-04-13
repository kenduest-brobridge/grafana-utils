use super::test_support::{run_raw_to_prompt, RawToPromptArgs, RawToPromptLogFormat};
use crate::common::CliColorChoice;
use crate::dashboard::{RawToPromptOutputFormat, RawToPromptResolution, EXPORT_METADATA_FILENAME};
use serde_json::json;
use std::fs;
use tempfile::tempdir;

fn write_json(path: &std::path::Path, value: serde_json::Value) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, serde_json::to_string_pretty(&value).unwrap() + "\n").unwrap();
}

fn make_args() -> RawToPromptArgs {
    RawToPromptArgs {
        input_file: Vec::new(),
        input_dir: None,
        output_file: None,
        output_dir: None,
        overwrite: false,
        output_format: RawToPromptOutputFormat::Json,
        no_header: false,
        color: CliColorChoice::Never,
        progress: false,
        verbose: false,
        dry_run: false,
        log_file: None,
        log_format: RawToPromptLogFormat::Text,
        resolution: RawToPromptResolution::InferFamily,
        datasource_map: None,
        profile: None,
        url: None,
        api_token: None,
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        org_id: None,
        timeout: None,
        verify_ssl: false,
    }
}

#[test]
fn raw_to_prompt_single_file_writes_sibling_prompt_json() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("cpu-main.json");
    write_json(
        &input,
        json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "panels": [{
                "id": 1,
                "type": "timeseries",
                "datasource": "legacy-prom",
                "targets": [{"refId": "A", "expr": "rate(cpu_usage_total[5m])"}]
            }]
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];

    run_raw_to_prompt(&args).unwrap();

    let output = temp.path().join("cpu-main.prompt.json");
    assert!(output.is_file());
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert_eq!(prompt["__inputs"].as_array().unwrap().len(), 1);
    assert!(prompt["__requires"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["type"] == "datasource"));
}

#[test]
fn raw_to_prompt_plain_directory_requires_output_dir() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("raw-json");
    fs::create_dir_all(&input_dir).unwrap();
    write_json(
        &input_dir.join("cpu-main.json"),
        json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "panels": []
        }),
    );

    let mut args = make_args();
    args.input_dir = Some(input_dir);

    let error = run_raw_to_prompt(&args).unwrap_err().to_string();
    assert!(error.contains("requires --output-dir"));
}

#[test]
fn raw_to_prompt_raw_dir_defaults_to_sibling_prompt_and_writes_metadata() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("dashboards");
    let raw_dir = export_root.join("raw");
    write_json(
        &raw_dir.join("cpu-main.json"),
        json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "panels": [{
                "id": 1,
                "type": "timeseries",
                "datasource": "legacy-prom",
                "targets": [{"refId": "A", "expr": "rate(cpu_usage_total[5m])"}]
            }]
        }),
    );

    let mut args = make_args();
    args.input_dir = Some(raw_dir.clone());
    args.overwrite = true;

    run_raw_to_prompt(&args).unwrap();

    let prompt_dir = export_root.join("prompt");
    assert!(prompt_dir.join("cpu-main.json").is_file());
    assert!(prompt_dir.join("index.json").is_file());
    assert!(prompt_dir.join(EXPORT_METADATA_FILENAME).is_file());
}

#[test]
fn raw_to_prompt_repo_root_normalizes_to_dashboard_raw_lane() {
    let temp = tempdir().unwrap();
    fs::create_dir_all(temp.path().join(".git")).unwrap();
    let dashboards_root = temp.path().join("dashboards");
    let raw_dir = dashboards_root.join("raw");
    write_json(
        &raw_dir.join("cpu-main.json"),
        json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "panels": [{
                "id": 1,
                "type": "timeseries",
                "datasource": "legacy-prom",
                "targets": [{"refId": "A", "expr": "rate(cpu_usage_total[5m])"}]
            }]
        }),
    );

    let mut args = make_args();
    args.input_dir = Some(temp.path().to_path_buf());
    args.overwrite = true;

    run_raw_to_prompt(&args).unwrap();

    let prompt_dir = dashboards_root.join("prompt");
    assert!(prompt_dir.join("cpu-main.json").is_file());
    assert!(prompt_dir.join("index.json").is_file());
    assert!(prompt_dir.join(EXPORT_METADATA_FILENAME).is_file());
}

#[test]
fn raw_to_prompt_uses_datasource_map_for_exact_resolution() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("cpu-main.json");
    let mapping = temp.path().join("datasource-map.json");
    write_json(
        &input,
        json!({
            "uid": "cpu-main",
            "title": "CPU Main",
            "panels": [{
                "id": 1,
                "type": "timeseries",
                "datasource": "legacy-prom",
                "targets": [{"refId": "A", "expr": "rate(cpu_usage_total[5m])"}]
            }]
        }),
    );
    write_json(
        &mapping,
        json!({
            "kind": "grafana-utils-dashboard-datasource-map",
            "datasources": [{
                "match": {"name": "legacy-prom"},
                "replace": {
                    "uid": "prom-main",
                    "name": "Prometheus Main",
                    "type": "prometheus"
                }
            }]
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];
    args.datasource_map = Some(mapping);
    args.resolution = RawToPromptResolution::Exact;
    args.log_file = Some(temp.path().join("raw-to-prompt.log"));

    run_raw_to_prompt(&args).unwrap();

    let output = temp.path().join("cpu-main.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    let inputs = prompt["__inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0]["pluginName"], "Prometheus");
    let log = fs::read_to_string(temp.path().join("raw-to-prompt.log")).unwrap();
    assert!(log.contains("OK"));
}

#[test]
fn raw_to_prompt_keeps_single_family_datasource_template_and_slot_requires() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("host-list.json");
    write_json(
        &input,
        json!({
            "uid": "host-list",
            "title": "Host List",
            "panels": [
                {
                    "id": 1,
                    "type": "table",
                    "datasource": {"type": "influxdb", "uid": "influx-a"},
                    "targets": []
                },
                {
                    "id": 2,
                    "type": "table",
                    "datasource": {"type": "influxdb", "uid": "influx-b"},
                    "targets": []
                },
                {
                    "id": 3,
                    "type": "table",
                    "datasource": {"type": "influxdb", "uid": "influx-c"},
                    "targets": []
                }
            ]
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];

    run_raw_to_prompt(&args).unwrap();

    let output = temp.path().join("host-list.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert_eq!(
        prompt["templating"]["list"][0]["type"],
        serde_json::Value::String("datasource".to_string())
    );
    assert_eq!(
        prompt["panels"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
    let datasource_requires = prompt["__requires"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["type"] == "datasource" && item["id"] == "influxdb")
        .count();
    assert_eq!(datasource_requires, 3);
}

#[test]
fn raw_to_prompt_rewrites_generic_mixed_datasource_refs_to_prompt_slot() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("overview.json");
    write_json(
        &input,
        json!({
            "uid": "overview",
            "title": "Overview",
            "panels": [{
                "id": 1,
                "type": "timeseries",
                "datasource": {"type": "datasource", "uid": "-- Mixed --"},
                "targets": []
            }]
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];

    run_raw_to_prompt(&args).unwrap();

    let output = temp.path().join("overview.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    let inputs = prompt["__inputs"].as_array().unwrap();
    assert!(inputs.iter().any(|item| item["pluginId"] == "datasource"
        && item["name"].as_str().unwrap().starts_with("DS_DATASOURCE")));
    assert_eq!(
        prompt["panels"][0]["datasource"]["uid"].as_str(),
        Some("${DS_DATASOURCE}")
    );
}

#[test]
fn raw_to_prompt_keeps_builtin_grafana_datasource_objects_outside_prompt_slots() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("annotations.json");
    write_json(
        &input,
        json!({
            "uid": "annotations",
            "title": "Annotations",
            "annotations": {
                "list": [{
                    "name": "Annotations & Alerts",
                    "datasource": {"type": "datasource", "uid": "grafana"}
                }]
            },
            "panels": [{
                "id": 1,
                "type": "timeseries",
                "datasource": {"type": "influxdb", "uid": "influx-a"},
                "targets": []
            }]
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];

    run_raw_to_prompt(&args).unwrap();

    let output = temp.path().join("annotations.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    let inputs = prompt["__inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0]["pluginId"], "influxdb");
}

#[test]
fn raw_to_prompt_reuses_datasource_variable_slot_for_typed_placeholder_refs() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("kube.json");
    write_json(
        &input,
        json!({
            "uid": "kube",
            "title": "Kube",
            "templating": {
                "list": [{
                    "name": "datasource",
                    "type": "datasource",
                    "query": "prometheus",
                    "current": {},
                    "options": []
                }]
            },
            "panels": [{
                "id": 1,
                "type": "timeseries",
                "datasource": {"type": "prometheus", "uid": "$datasource"},
                "targets": [{"datasource": {"type": "prometheus", "uid": "$datasource"}}]
            }]
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];

    run_raw_to_prompt(&args).unwrap();

    let output = temp.path().join("kube.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    let inputs = prompt["__inputs"].as_array().unwrap();
    let prometheus_inputs = inputs
        .iter()
        .filter(|item| item["pluginId"] == "prometheus")
        .count();
    assert_eq!(prometheus_inputs, 1);
    assert_eq!(
        prompt["panels"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
    assert_eq!(
        prompt["panels"][0]["targets"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
}
