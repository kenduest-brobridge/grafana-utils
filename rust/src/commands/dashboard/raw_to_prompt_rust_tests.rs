use super::test_support::{run_raw_to_prompt, RawToPromptArgs, RawToPromptLogFormat};
use crate::common::CliColorChoice;
use crate::dashboard::{RawToPromptOutputFormat, RawToPromptResolution, EXPORT_METADATA_FILENAME};
use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
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

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/commands/dashboard/fixtures")
        .join(name)
}

fn load_fixture(name: &str) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(fixture_path(name)).unwrap()).unwrap()
}

fn start_live_export_mock_server() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let mut served = 0usize;
        while served < 3 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();

            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                let bytes_read = stream.read(&mut buffer).unwrap();
                if bytes_read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..bytes_read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }

            let request_line_end = request
                .windows(2)
                .position(|window| window == b"\r\n")
                .unwrap_or(request.len());
            let request_line = String::from_utf8_lossy(&request[..request_line_end]);
            let path = request_line
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_string();

            let body = match path.as_str() {
                "/api/org" => json!({"id": 1, "name": "Main Org"}),
                "/api/datasources" => json!([
                    {
                        "uid": "prom-main",
                        "name": "Prometheus Main",
                        "type": "prometheus",
                        "access": "proxy",
                        "url": "http://prometheus:9090",
                        "isDefault": true
                    }
                ]),
                "/api/library-elements/shared-panel" => json!({
                    "result": {
                        "uid": "shared-panel",
                        "name": "Shared Panel",
                        "kind": 1,
                        "type": "graph",
                        "model": {
                            "id": 11,
                            "type": "graph",
                            "datasource": {"uid": "prom-main", "type": "prometheus"},
                            "targets": [{
                                "refId": "A",
                                "datasource": {"uid": "prom-main", "type": "prometheus"}
                            }]
                        }
                    }
                }),
                other => panic!("unexpected request path: {other}"),
            };
            let body = serde_json::to_string(&body).unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            let _ = stream.flush();
            served += 1;
        }
    });
    (format!("http://{address}"), server)
}

fn write_fixture(path: &Path, name: &str) {
    write_json(path, load_fixture(name));
}

fn run_prompt_fixture(temp: &Path, name: &str) -> serde_json::Value {
    let input = temp.join(name);
    write_fixture(&input, name);

    let mut args = make_args();
    args.input_file = vec![input];

    run_raw_to_prompt(&args).unwrap();

    let prompt_path = temp.join(name.replace(".json", ".prompt.json"));
    serde_json::from_str(&fs::read_to_string(prompt_path).unwrap()).unwrap()
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
fn raw_to_prompt_keeps_concrete_single_family_datasource_refs_without_synthetic_variable() {
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
    assert!(prompt
        .get("templating")
        .and_then(serde_json::Value::as_object)
        .and_then(|templating| templating.get("list"))
        .and_then(serde_json::Value::as_array)
        .map(|variables| {
            !variables.iter().any(|variable| {
                variable["type"] == serde_json::Value::String("datasource".to_string())
            })
        })
        .unwrap_or(true));
    assert_eq!(prompt["__inputs"].as_array().unwrap().len(), 3);
    assert!(prompt["panels"][0]["datasource"]["uid"]
        .as_str()
        .is_some_and(|value| value.starts_with("${DS_INFLUXDB_")));
}

#[test]
fn raw_to_prompt_does_not_turn_datasource_template_variables_into_inputs() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("host-list.json");
    write_json(
        &input,
        json!({
            "uid": "host-list",
            "title": "Host List",
            "templating": {
                "list": [{
                    "name": "datasource",
                    "type": "datasource",
                    "query": "influxdb",
                    "current": {},
                    "options": []
                }]
            },
            "panels": [{
                "id": 1,
                "type": "table",
                "datasource": {"type": "influxdb", "uid": "influx-a"},
                "targets": []
            }]
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];

    run_raw_to_prompt(&args).unwrap();

    let output = temp.path().join("host-list.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    let inputs = prompt["__inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(
        prompt["templating"]["list"][0]["type"],
        serde_json::Value::String("datasource".to_string())
    );
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
fn raw_to_prompt_maps_constant_variables_to_var_inputs() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("constants.json");
    write_json(
        &input,
        json!({
            "uid": "constants",
            "title": "Constants",
            "templating": {
                "list": [{
                    "name": "env name",
                    "label": "Environment",
                    "type": "constant",
                    "query": "prod",
                    "current": {"text": "prod", "value": "prod"},
                    "options": []
                }]
            },
            "panels": []
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];

    run_raw_to_prompt(&args).unwrap();

    let output = temp.path().join("constants.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    let inputs = prompt["__inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0]["name"], "VAR_ENV_NAME");
    assert_eq!(inputs[0]["type"], "constant");
    assert_eq!(inputs[0]["value"], "prod");
    assert_eq!(prompt["templating"]["list"][0]["query"], "${VAR_ENV_NAME}");
    assert_eq!(
        prompt["templating"]["list"][0]["current"]["value"],
        "${VAR_ENV_NAME}"
    );
}

#[test]
fn raw_to_prompt_keeps_expression_datasource_refs_outside_prompt_slots() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("expressions.json");
    write_json(
        &input,
        json!({
            "uid": "expressions",
            "title": "Expressions",
            "panels": [{
                "id": 1,
                "type": "timeseries",
                "datasource": {"type": "__expr__", "uid": "__expr__", "name": "Expression"},
                "targets": []
            }]
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];

    run_raw_to_prompt(&args).unwrap();

    let output = temp.path().join("expressions.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert!(prompt["__inputs"].as_array().unwrap().is_empty());
    assert_eq!(prompt["panels"][0]["datasource"]["uid"], "__expr__");
}

#[test]
fn raw_to_prompt_rejects_dashboard_v2_resource_shape() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("v2.json");
    write_json(
        &input,
        json!({
            "apiVersion": "dashboard.grafana.app/v2",
            "kind": "Dashboard",
            "metadata": {"name": "v2-main"},
            "spec": {
                "title": "V2 Main",
                "elements": {},
                "variables": [],
                "annotations": []
            }
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];

    let error = run_raw_to_prompt(&args).unwrap_err().to_string();
    assert!(error.contains("dashboard raw-to-prompt completed with 1 failure"));

    let output = temp.path().join("v2.prompt.json");
    assert!(!output.exists());
}

#[test]
fn raw_to_prompt_warns_for_library_panel_references_without_inlining() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("library.json");
    write_json(
        &input,
        json!({
            "uid": "library",
            "title": "Library",
            "panels": [{
                "id": 1,
                "type": "timeseries",
                "libraryPanel": {"uid": "lib-panel", "name": "Shared Panel"},
                "datasource": {"type": "prometheus", "uid": "prom-main"},
                "targets": []
            }]
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];
    args.log_file = Some(temp.path().join("raw-to-prompt.log"));

    run_raw_to_prompt(&args).unwrap();

    let output = temp.path().join("library.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert_eq!(prompt["panels"][0]["libraryPanel"]["uid"], "lib-panel");
    assert_eq!(prompt["__elements"], json!({}));
    let log = fs::read_to_string(temp.path().join("raw-to-prompt.log")).unwrap();
    assert!(log.contains("library panel external export is not fully portable yet"));
}

#[test]
fn raw_to_prompt_preserves_datasource_placeholder_refs_without_inputs() {
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
    assert!(inputs.is_empty());
    assert_eq!(
        prompt["panels"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
    assert_eq!(
        prompt["panels"][0]["targets"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
}

#[test]
fn raw_to_prompt_maps_used_datasource_variable_current_to_prompt_input() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("kube-current.json");
    write_json(
        &input,
        json!({
            "uid": "kube-current",
            "title": "Kube Current",
            "templating": {
                "list": [{
                    "name": "datasource",
                    "type": "datasource",
                    "query": "prometheus",
                    "current": {
                        "text": "Prometheus Main",
                        "value": "prom-main",
                        "selected": true
                    },
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

    let output = temp.path().join("kube-current.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    let inputs = prompt["__inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0]["pluginId"], "prometheus");
    assert_eq!(
        prompt["templating"]["list"][0]["current"]["value"],
        serde_json::Value::String("${DS_PROMETHEUS_MAIN}".to_string())
    );
    assert_eq!(
        prompt["panels"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
    assert_eq!(
        prompt["panels"][0]["targets"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
}

#[test]
fn raw_to_prompt_preserves_string_datasource_placeholder_refs_without_resolution() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("linux.json");
    write_json(
        &input,
        json!({
            "uid": "linux",
            "title": "Linux",
            "templating": {
                "list": [{
                    "name": "datasource",
                    "type": "datasource",
                    "query": "influxdb",
                    "current": {},
                    "options": []
                }]
            },
            "panels": [{
                "id": 1,
                "type": "timeseries",
                "datasource": "$datasource",
                "targets": [{"datasource": "$datasource"}]
            }]
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input.clone()];

    run_raw_to_prompt(&args).unwrap();

    let output = temp.path().join("linux.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    assert!(prompt["__inputs"].as_array().unwrap().is_empty());
    assert_eq!(
        prompt["panels"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
    assert_eq!(
        prompt["panels"][0]["targets"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
}

#[test]
fn raw_to_prompt_matches_datasource_variable_fixture_parity() {
    let temp = tempdir().unwrap();
    let prompt = run_prompt_fixture(temp.path(), "datasource-variable.json");

    let inputs = prompt["__inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0]["type"], "datasource");
    assert_eq!(inputs[0]["pluginId"], "sqlite-datasource");
    assert!(prompt["templating"]["list"][0]["current"]["value"]
        .as_str()
        .is_some_and(|value| value.starts_with("${DS_")));
    assert_eq!(
        prompt["panels"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
}

#[test]
fn raw_to_prompt_preserves_all_selected_datasource_variable_without_prompt_inputs() {
    let temp = tempdir().unwrap();
    let prompt = run_prompt_fixture(temp.path(), "all-selected-single-datasource-variable.json");

    assert!(prompt["__inputs"].as_array().unwrap().is_empty());
    assert_eq!(
        prompt["templating"]["list"][0]["current"]["value"],
        serde_json::Value::String("$__all".to_string())
    );
    assert_eq!(
        prompt["panels"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
}

#[test]
fn raw_to_prompt_maps_default_datasource_variable_to_prompt_input() {
    let temp = tempdir().unwrap();
    let prompt = run_prompt_fixture(temp.path(), "default-datasource-variable.json");

    let inputs = prompt["__inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0]["pluginId"], "testdata");
    assert!(prompt["templating"]["list"][0]["current"]["value"]
        .as_str()
        .is_some_and(|value| value.starts_with("${DS_")));
    assert_eq!(
        prompt["panels"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
}

#[test]
fn raw_to_prompt_excludes_builtin_and_expression_datasources_from_inputs() {
    let temp = tempdir().unwrap();
    let prompt = run_prompt_fixture(temp.path(), "special-datasource-types.json");

    let inputs = prompt["__inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0]["pluginId"], "frser-sqlite-datasource");
    assert!(prompt["panels"][0]["datasource"]["uid"]
        .as_str()
        .is_some_and(|value| value.starts_with("${DS_")));
    assert_eq!(prompt["panels"][1]["datasource"]["uid"], "grafana");
    assert_eq!(
        prompt["panels"][0]["targets"][0]["datasource"]["uid"],
        "__expr__"
    );
}

#[test]
fn raw_to_prompt_resolves_string_datasource_ids_using_mapping() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("check-string-datasource-id.json");
    let mapping = temp.path().join("datasource-map.json");
    write_fixture(&input, "check-string-datasource-id.json");
    write_json(
        &mapping,
        json!({
            "kind": "grafana-utils-dashboard-datasource-map",
            "datasources": [{
                "match": {"name": "sqlite-1"},
                "replace": {
                    "uid": "sqlite-1",
                    "name": "sqlite-1",
                    "type": "sqlite-datasource"
                }
            }]
        }),
    );

    let mut args = make_args();
    args.input_file = vec![input];
    args.datasource_map = Some(mapping);

    run_raw_to_prompt(&args).unwrap();

    let prompt: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(temp.path().join("check-string-datasource-id.prompt.json")).unwrap(),
    )
    .unwrap();

    let inputs = prompt["__inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0]["pluginId"], "sqlite-datasource");
    assert!(prompt["panels"][0]["datasource"]["uid"]
        .as_str()
        .is_some_and(|value| value.starts_with("${DS_")));
    assert!(prompt["panels"][0]["targets"][0]["datasource"]["uid"]
        .as_str()
        .is_some_and(|value| value.starts_with("${DS_")));
    assert!(prompt["panels"][1]["datasource"]["uid"]
        .as_str()
        .is_some_and(|value| value.starts_with("${DS_")));
    assert!(prompt["panels"][1]["targets"][0]["datasource"]["uid"]
        .as_str()
        .is_some_and(|value| value.starts_with("${DS_")));
}

#[test]
fn raw_to_prompt_warns_for_library_panel_references_without_inlining_fixture() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("with-library-panels.json");
    write_fixture(&input, "with-library-panels.json");
    let log_file = temp.path().join("raw-to-prompt.log");

    let mut args = make_args();
    args.input_file = vec![input.clone()];
    args.log_file = Some(log_file.clone());

    run_raw_to_prompt(&args).unwrap();

    let prompt: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(temp.path().join("with-library-panels.prompt.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(prompt["__elements"], json!({}));
    assert_eq!(
        prompt["panels"][0]["libraryPanel"]["uid"],
        "a7975b7a-fb53-4ab7-951d-15810953b54f"
    );
    assert_eq!(
        prompt["panels"][2]["panels"][0]["libraryPanel"]["uid"],
        "l3d2s634-fdgf-75u4-3fg3-67j966ii7jur"
    );
    let log = fs::read_to_string(log_file).unwrap();
    assert!(log.contains("library panel external export is not fully portable yet"));
}

#[test]
fn raw_to_prompt_inlines_live_library_panel_models_without_losing_prompt_semantics() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("library-live.json");
    write_json(
        &input,
        json!({
            "uid": "library-live",
            "title": "Library Live",
            "templating": {
                "list": [{
                    "name": "datasource",
                    "type": "datasource",
                    "query": "prometheus",
                    "current": {
                        "selected": true,
                        "text": "Prometheus Main",
                        "value": "prom-main"
                    },
                    "options": []
                }]
            },
            "panels": [{
                "id": 1,
                "title": "Shared Panel",
                "type": "graph",
                "libraryPanel": {"uid": "shared-panel", "name": "Shared Panel"},
                "datasource": {"uid": "$datasource", "type": "prometheus"},
                "targets": [{
                    "refId": "A",
                    "datasource": {"uid": "$datasource", "type": "prometheus"}
                }]
            }]
        }),
    );

    let (base_url, server) = start_live_export_mock_server();

    let mut args = make_args();
    args.input_file = vec![input.clone()];
    args.url = Some(base_url);
    args.api_token = Some("token".to_string());
    args.log_file = Some(temp.path().join("raw-to-prompt.log"));

    run_raw_to_prompt(&args).unwrap();
    server.join().unwrap();

    let output = temp.path().join("library-live.prompt.json");
    let prompt: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(output).unwrap()).unwrap();
    let inputs = prompt["__inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0]["pluginId"], "prometheus");
    assert_eq!(
        prompt["templating"]["list"][0]["type"],
        serde_json::Value::String("datasource".to_string())
    );
    assert_eq!(
        prompt["templating"]["list"][0]["query"],
        serde_json::Value::String("prometheus".to_string())
    );
    assert_eq!(
        prompt["templating"]["list"][0]["current"]["value"],
        serde_json::Value::String("${DS_PROM_MAIN}".to_string())
    );
    assert_eq!(
        prompt["panels"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
    assert_eq!(
        prompt["panels"][0]["targets"][0]["datasource"]["uid"],
        serde_json::Value::String("$datasource".to_string())
    );
    assert_eq!(prompt["__elements"]["shared-panel"]["uid"], "shared-panel");
    assert_eq!(
        prompt["__elements"]["shared-panel"]["model"]["datasource"]["uid"],
        serde_json::Value::String("${DS_PROM_MAIN}".to_string())
    );
    assert_eq!(
        prompt["__elements"]["shared-panel"]["model"]["targets"][0]["datasource"]["uid"],
        serde_json::Value::String("${DS_PROM_MAIN}".to_string())
    );
}

#[test]
fn raw_to_prompt_rejects_dashboard_v2_fixture() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("v2-elements.json");
    write_fixture(&input, "v2-elements.json");

    let mut args = make_args();
    args.input_file = vec![input];

    let error = run_raw_to_prompt(&args).unwrap_err().to_string();
    assert!(error.contains("dashboard raw-to-prompt completed with 1 failure"));
}

#[test]
fn raw_to_prompt_rejects_dashboard_k8s_wrapper_fixture() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("k8s-wrapper.json");
    write_fixture(&input, "k8s-wrapper.json");

    let mut args = make_args();
    args.input_file = vec![input];

    let error = run_raw_to_prompt(&args).unwrap_err().to_string();
    assert!(error.contains("dashboard raw-to-prompt completed with 1 failure"));
}
