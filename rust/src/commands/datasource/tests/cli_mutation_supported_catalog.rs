//! Datasource supported-catalog output contract tests.

use super::super::{load_supported_types_catalog_fixture, project_supported_types_catalog};
use serde_json::json;

#[test]
fn supported_catalog_json_includes_prometheus_profile_metadata() {
    let document = crate::datasource_catalog::render_supported_datasource_catalog_json();
    let prometheus = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Metrics"))
        .and_then(|row| row["types"].as_array())
        .and_then(|rows| rows.iter().find(|row| row["type"] == json!("prometheus")))
        .unwrap();

    assert_eq!(prometheus["profile"], json!("metrics-http"));
    assert_eq!(prometheus["queryLanguage"], json!("promql"));
    assert_eq!(prometheus["requiresDatasourceUrl"], json!(true));
    assert!(prometheus["suggestedFlags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "--basic-auth"));
    assert_eq!(prometheus["presetProfiles"], json!(["starter"]));
    assert_eq!(prometheus["addDefaults"]["access"], json!("proxy"));
    assert_eq!(
        prometheus["addDefaults"]["jsonData"]["httpMethod"],
        json!("POST")
    );
    assert_eq!(prometheus["fullAddDefaults"], prometheus["addDefaults"]);
}

#[test]
fn supported_catalog_json_matches_shared_supported_types_fixture() {
    let document = crate::datasource_catalog::render_supported_datasource_catalog_json();

    assert_eq!(
        project_supported_types_catalog(&document),
        load_supported_types_catalog_fixture()
    );
}

#[test]
fn supported_catalog_json_includes_database_profile_metadata() {
    let document = crate::datasource_catalog::render_supported_datasource_catalog_json();
    let sqlite = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Databases"))
        .and_then(|row| row["types"].as_array())
        .and_then(|rows| rows.iter().find(|row| row["type"] == json!("sqlite")))
        .unwrap();

    assert_eq!(sqlite["profile"], json!("sql-database"));
    assert_eq!(sqlite["queryLanguage"], json!("sql"));
    assert_eq!(sqlite["requiresDatasourceUrl"], json!(false));
    assert!(sqlite["suggestedFlags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "--user"));
    assert_eq!(sqlite["presetProfiles"], json!(["starter"]));
}

#[test]
fn supported_catalog_json_includes_family_level_json_data_defaults() {
    let document = crate::datasource_catalog::render_supported_datasource_catalog_json();
    let metrics_types = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Metrics"))
        .and_then(|row| row["types"].as_array())
        .unwrap();
    let influxdb = metrics_types
        .iter()
        .find(|row| row["type"] == json!("influxdb"))
        .unwrap();
    assert_eq!(influxdb["addDefaults"]["access"], json!("proxy"));
    assert_eq!(
        influxdb["addDefaults"]["jsonData"]["version"],
        json!("Flux")
    );
    assert_eq!(
        influxdb["addDefaults"]["jsonData"]["organization"],
        json!("main-org")
    );
    assert_eq!(
        influxdb["addDefaults"]["jsonData"]["defaultBucket"],
        json!("metrics")
    );
    let graphite = metrics_types
        .iter()
        .find(|row| row["type"] == json!("graphite"))
        .unwrap();
    assert_eq!(
        graphite["addDefaults"]["jsonData"]["graphiteVersion"],
        json!("1.1")
    );

    let logs_types = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Logs"))
        .and_then(|row| row["types"].as_array())
        .unwrap();
    let loki = logs_types
        .iter()
        .find(|row| row["type"] == json!("loki"))
        .unwrap();
    assert_eq!(loki["addDefaults"]["access"], json!("proxy"));
    assert_eq!(loki["presetProfiles"], json!(["starter", "full"]));
    assert_eq!(loki["addDefaults"]["jsonData"]["maxLines"], json!(1000));
    assert_eq!(loki["addDefaults"]["jsonData"]["timeout"], json!(60));
    assert_eq!(
        loki["fullAddDefaults"]["jsonData"]["derivedFields"][0]["datasourceUid"],
        json!("tempo")
    );

    let tracing_types = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Tracing"))
        .and_then(|row| row["types"].as_array())
        .unwrap();
    let tempo = tracing_types
        .iter()
        .find(|row| row["type"] == json!("tempo"))
        .unwrap();
    assert_eq!(tempo["presetProfiles"], json!(["starter", "full"]));
    assert_eq!(
        tempo["addDefaults"]["jsonData"]["nodeGraph"]["enabled"],
        json!(true)
    );
    assert_eq!(
        tempo["addDefaults"]["jsonData"]["search"]["hide"],
        json!(false)
    );
    assert_eq!(
        tempo["addDefaults"]["jsonData"]["traceQuery"]["timeShiftEnabled"],
        json!(true)
    );
    assert_eq!(
        tempo["fullAddDefaults"]["jsonData"]["serviceMap"]["datasourceUid"],
        json!("prometheus")
    );
    assert_eq!(
        tempo["fullAddDefaults"]["jsonData"]["tracesToLogsV2"]["datasourceUid"],
        json!("loki")
    );

    let database_types = document["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["category"] == json!("Databases"))
        .and_then(|row| row["types"].as_array())
        .unwrap();
    let postgresql = database_types
        .iter()
        .find(|row| row["type"] == json!("postgresql"))
        .unwrap();
    assert_eq!(postgresql["presetProfiles"], json!(["starter", "full"]));
    assert_eq!(
        postgresql["addDefaults"]["jsonData"]["database"],
        json!("grafana")
    );
    assert_eq!(
        postgresql["addDefaults"]["jsonData"]["sslmode"],
        json!("disable")
    );
    let mysql = database_types
        .iter()
        .find(|row| row["type"] == json!("mysql"))
        .unwrap();
    assert_eq!(mysql["presetProfiles"], json!(["starter", "full"]));
    assert_eq!(mysql["fullAddDefaults"]["jsonData"]["tlsAuth"], json!(true));
}

#[test]
fn supported_catalog_text_mentions_profile_and_flags() {
    let lines = crate::datasource_catalog::render_supported_datasource_catalog_text();
    let prometheus_line = lines
        .iter()
        .find(|line| line.contains("Prometheus (prometheus)"))
        .unwrap();
    assert!(prometheus_line.contains("profile=metrics-http"));
    assert!(prometheus_line.contains("query=promql"));
    assert!(prometheus_line.contains("flags: --basic-auth"));
}

#[test]
fn supported_catalog_text_mentions_family_level_defaults() {
    let lines = crate::datasource_catalog::render_supported_datasource_catalog_text();
    let influxdb_line = lines
        .iter()
        .find(|line| line.contains("InfluxDB (influxdb)"))
        .unwrap();
    assert!(influxdb_line.contains("defaults: access=proxy, jsonData.version=Flux"));
    assert!(influxdb_line.contains("jsonData.organization=main-org"));
    assert!(influxdb_line.contains("jsonData.defaultBucket=metrics"));

    let loki_line = lines
        .iter()
        .find(|line| line.contains("Loki (loki)"))
        .unwrap();
    assert!(loki_line.contains("jsonData.maxLines=1000"));
    assert!(loki_line.contains("jsonData.timeout=60"));

    let tempo_line = lines
        .iter()
        .find(|line| line.contains("Tempo (tempo)"))
        .unwrap();
    assert!(tempo_line.contains("jsonData.nodeGraph.enabled=true"));
    assert!(tempo_line.contains("jsonData.traceQuery.timeShiftEnabled=true"));

    let postgresql_line = lines
        .iter()
        .find(|line| line.contains("PostgreSQL (postgresql)"))
        .unwrap();
    assert!(postgresql_line.contains("jsonData.database=grafana"));
    assert!(postgresql_line.contains("jsonData.sslmode=disable"));
}

#[test]
fn supported_catalog_table_mentions_category_and_aliases() {
    let lines = crate::datasource_catalog::render_supported_datasource_catalog_table();
    assert!(lines[0].contains("category"));
    assert!(lines[0].contains("display_name"));
    assert!(lines.iter().any(|line| line.contains("Prometheus")));
    assert!(lines
        .iter()
        .any(|line| line.contains("grafana-loki-datasource")));
}

#[test]
fn supported_catalog_csv_mentions_headers_and_defaults() {
    let lines = crate::datasource_catalog::render_supported_datasource_catalog_csv();
    assert!(lines[0].contains("category,display_name,type"));
    assert!(lines.iter().any(|line| line.contains("Prometheus")));
    assert!(lines.iter().any(|line| line.contains("required")));
}

#[test]
fn supported_catalog_yaml_serializes_supported_types_document() {
    let yaml = crate::datasource_catalog::render_supported_datasource_catalog_yaml().unwrap();
    assert!(yaml.contains("kind: grafana-utils-datasource-supported-types"));
    assert!(yaml.contains("category:"));
    assert!(yaml.contains("displayName: Prometheus"));
}
