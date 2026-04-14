//! Datasource render output behavior tests.

use super::*;

#[test]
fn render_import_table_honors_selected_columns() {
    let rows = vec![vec![
        "prom-main".to_string(),
        "Prometheus Main".to_string(),
        "prometheus".to_string(),
        "uid".to_string(),
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
fn render_import_table_supports_all_columns() {
    let rows = vec![vec![
        "prom-main".to_string(),
        "Prometheus Main".to_string(),
        "prometheus".to_string(),
        "uid".to_string(),
        "exists-uid".to_string(),
        "would-update".to_string(),
        "7".to_string(),
        "datasources.json#0".to_string(),
    ]];

    let lines = render_import_table(&rows, true, Some(&["all".to_string()]));

    assert!(lines[0].contains("UID"));
    assert!(lines[0].contains("NAME"));
    assert!(lines[0].contains("TYPE"));
    assert!(lines[0].contains("MATCH_BASIS"));
    assert!(lines[0].contains("DESTINATION"));
    assert!(lines[0].contains("ACTION"));
    assert!(lines[0].contains("ORG_ID"));
    assert!(lines[0].contains("FILE"));
}

#[test]
fn render_datasource_list_table_supports_all_columns() {
    let rows = vec![json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "access": "proxy",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "database": "metrics",
        "jsonData": {
            "organization": "acme",
            "defaultBucket": "main"
        },
        "org": "Main Org.",
        "orgId": "1"
    })
    .as_object()
    .unwrap()
    .clone()];

    let lines = render_data_source_table(&rows, true, Some(&["all".to_string()]));

    assert!(lines[0].contains("UID"));
    assert!(lines[0].contains("NAME"));
    assert!(lines[0].contains("TYPE"));
    assert!(lines[0].contains("URL"));
    assert!(lines[0].contains("IS_DEFAULT"));
    assert!(lines[0].contains("DATABASE"));
    assert!(lines[0].contains("JSONDATA.ORGANIZATION"));
    assert!(lines[0].contains("ORG"));
    assert!(lines[0].contains("ORG_ID"));
}

#[test]
fn render_datasource_list_csv_honors_selected_columns() {
    let rows = vec![json!({
        "uid": "prom-main",
        "name": "Prometheus Main",
        "type": "prometheus",
        "url": "http://prometheus:9090",
        "isDefault": true,
        "jsonData": {
            "organization": "acme"
        }
    })
    .as_object()
    .unwrap()
    .clone()];

    let lines = render_data_source_csv(
        &rows,
        Some(&["uid".to_string(), "jsonData.organization".to_string()]),
    );

    assert_eq!(lines[0], "uid,jsonData.organization");
    assert_eq!(lines[1], "prom-main,acme");
}

#[test]
fn render_datasource_list_json_defaults_to_full_records() {
    let rows = vec![
        json!({
            "uid": "influx-main",
            "name": "Influx Main",
            "type": "influxdb",
            "access": "proxy",
            "url": "http://influx:8086",
            "database": "metrics",
            "user": "influx-user",
            "isDefault": true,
            "jsonData": {
                "version": "Flux",
                "organization": "acme",
                "defaultBucket": "main"
            },
            "secureJsonFields": {
                "token": true
            }
        })
        .as_object()
        .unwrap()
        .clone(),
        json!({
            "uid": "prom-auth",
            "name": "Prometheus Auth",
            "type": "prometheus",
            "access": "proxy",
            "url": "http://prometheus:9090",
            "basicAuth": true,
            "basicAuthUser": "metrics-user",
            "withCredentials": true,
            "isDefault": false,
            "jsonData": {
                "httpMethod": "POST",
                "timeInterval": "30s"
            },
            "secureJsonFields": {
                "basicAuthPassword": true,
                "httpHeaderValue1": true
            }
        })
        .as_object()
        .unwrap()
        .clone(),
    ];

    let json_value = render_data_source_json(&rows, None);

    assert_eq!(
        json_value[0]["database"],
        Value::String("metrics".to_string())
    );
    assert_eq!(
        json_value[0]["user"],
        Value::String("influx-user".to_string())
    );
    assert_eq!(
        json_value[0]["jsonData"]["organization"],
        Value::String("acme".to_string())
    );
    assert_eq!(
        json_value[0]["secureJsonFields"]["token"],
        Value::Bool(true)
    );
    assert_eq!(json_value[1]["basicAuth"], Value::Bool(true));
    assert_eq!(
        json_value[1]["basicAuthUser"],
        Value::String("metrics-user".to_string())
    );
    assert_eq!(json_value[1]["withCredentials"], Value::Bool(true));
    assert_eq!(
        json_value[1]["jsonData"]["httpMethod"],
        Value::String("POST".to_string())
    );
    assert_eq!(
        json_value[1]["secureJsonFields"]["basicAuthPassword"],
        Value::Bool(true)
    );
    assert_eq!(
        json_value[1]["secureJsonFields"]["httpHeaderValue1"],
        Value::Bool(true)
    );
}

#[test]
fn render_datasource_list_json_honors_selected_columns() {
    let rows = vec![json!({
        "uid": "influx-main",
        "name": "Influx Main",
        "type": "influxdb",
        "access": "proxy",
        "url": "http://influx:8086",
        "database": "metrics",
        "isDefault": true,
        "basicAuth": true,
        "basicAuthUser": "metrics-user",
        "jsonData": {
            "organization": "acme",
            "defaultBucket": "main"
        }
    })
    .as_object()
    .unwrap()
    .clone()];

    let json_value = render_data_source_json(
        &rows,
        Some(&[
            "uid".to_string(),
            "database".to_string(),
            "basicAuthUser".to_string(),
            "jsonData.organization".to_string(),
        ]),
    );

    assert_eq!(
        json_value[0]["uid"],
        Value::String("influx-main".to_string())
    );
    assert_eq!(
        json_value[0]["database"],
        Value::String("metrics".to_string())
    );
    assert_eq!(
        json_value[0]["basicAuthUser"],
        Value::String("metrics-user".to_string())
    );
    assert_eq!(
        json_value[0]["jsonData"]["organization"],
        Value::String("acme".to_string())
    );
    assert!(json_value[0].get("type").is_none());
    assert!(json_value[0].get("basicAuth").is_none());
}
