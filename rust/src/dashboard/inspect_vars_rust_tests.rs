//! Inspection path for Dashboard resources: analysis, extraction, and report shaping.

use super::test_support::{
    extract_dashboard_variables, parse_cli_from, DashboardCommand, SimpleOutputFormat,
};
use crate::dashboard::vars::{
    render_dashboard_variable_output, DashboardVariableDocument, DashboardVariableRow,
};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn parse_inspect_vars_args_supports_dashboard_url_only() {
    let args = match parse_cli_from([
        "grafana-util",
        "inspect-vars",
        "--dashboard-url",
        "https://grafana.example.com/d/infra-main/infra-overview?var-datasource=prom-main",
        "--output-format",
        "json",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::InspectVars(args) => args,
        other => panic!("expected inspect-vars args, got {other:?}"),
    };

    assert_eq!(args.dashboard_uid, None);
    assert_eq!(
        args.dashboard_url.as_deref(),
        Some("https://grafana.example.com/d/infra-main/infra-overview?var-datasource=prom-main")
    );
    assert_eq!(args.output_format, Some(SimpleOutputFormat::Json));
    assert_eq!(args.vars_query, None);
}

#[test]
fn parse_inspect_vars_args_accepts_vars_query() {
    let args = match parse_cli_from([
        "grafana-util",
        "inspect-vars",
        "--dashboard-uid",
        "infra-main",
        "--vars-query",
        "var-datasource=prom-main&var-cluster=prod-a",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::InspectVars(args) => args,
        other => panic!("expected inspect-vars args, got {other:?}"),
    };

    assert_eq!(args.dashboard_uid.as_deref(), Some("infra-main"));
    assert_eq!(
        args.vars_query.as_deref(),
        Some("var-datasource=prom-main&var-cluster=prod-a")
    );
}

#[test]
fn parse_inspect_vars_args_supports_output_file() {
    let args = match parse_cli_from([
        "grafana-util",
        "inspect-vars",
        "--dashboard-uid",
        "infra-main",
        "--output-file",
        "/tmp/inspect-vars.json",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::InspectVars(args) => args,
        other => panic!("expected inspect-vars args, got {other:?}"),
    };

    assert_eq!(args.dashboard_uid.as_deref(), Some("infra-main"));
    assert_eq!(
        args.output_file,
        Some(PathBuf::from("/tmp/inspect-vars.json"))
    );
}

#[test]
fn parse_inspect_vars_args_supports_text_and_yaml_output_formats() {
    let text_args = match parse_cli_from([
        "grafana-util",
        "inspect-vars",
        "--dashboard-uid",
        "infra-main",
        "--output-format",
        "text",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::InspectVars(args) => args,
        other => panic!("expected inspect-vars args, got {other:?}"),
    };

    let yaml_args = match parse_cli_from([
        "grafana-util",
        "inspect-vars",
        "--dashboard-uid",
        "infra-main",
        "--output-format",
        "yaml",
        "--token",
        "secret",
    ])
    .command
    {
        DashboardCommand::InspectVars(args) => args,
        other => panic!("expected inspect-vars args, got {other:?}"),
    };

    assert_eq!(text_args.output_format, Some(SimpleOutputFormat::Text));
    assert_eq!(yaml_args.output_format, Some(SimpleOutputFormat::Yaml));
}

#[test]
fn extract_dashboard_variables_reads_current_and_options() {
    let dashboard = json!({
        "templating": {
            "list": [
                {
                    "name": "datasource",
                    "type": "datasource",
                    "label": "Datasource",
                    "query": "prometheus",
                    "current": {"text": "Prom Main", "value": "prom-main"},
                    "options": [
                        {"text": "Prom Main", "value": "prom-main"},
                        {"text": "Prom DR", "value": "prom-dr"}
                    ]
                },
                {
                    "name": "cluster",
                    "type": "query",
                    "label": "Cluster",
                    "datasource": {"uid": "prom-main", "type": "prometheus"},
                    "current": {"text": ["prod-a", "prod-b"], "value": ["prod-a", "prod-b"]},
                    "multi": true,
                    "includeAll": true,
                    "options": [{"text": "prod-a", "value": "prod-a"}]
                }
            ]
        }
    });

    let rows = extract_dashboard_variables(dashboard.as_object().unwrap()).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].name, "datasource");
    assert_eq!(rows[0].current, "Prom Main (prom-main)");
    assert_eq!(rows[0].option_count, 2);
    assert_eq!(rows[1].name, "cluster");
    assert_eq!(rows[1].datasource, "prom-main");
    assert_eq!(rows[1].current, "prod-a|prod-b");
    assert!(rows[1].multi);
    assert!(rows[1].include_all);
}

#[test]
fn render_dashboard_variable_output_supports_text_yaml_and_json() {
    let document = DashboardVariableDocument {
        dashboard_uid: "infra-main".to_string(),
        dashboard_title: "Infra Overview".to_string(),
        variable_count: 1,
        variables: vec![DashboardVariableRow {
            name: "cluster".to_string(),
            variable_type: "query".to_string(),
            label: "Cluster".to_string(),
            current: "prod-a".to_string(),
            datasource: "prom-main".to_string(),
            query: "label_values(up, cluster)".to_string(),
            multi: false,
            include_all: false,
            option_count: 1,
            options: vec!["prod-a".to_string()],
        }],
    };

    let base_args = super::test_support::InspectVarsArgs {
        common: super::test_support::CommonCliArgs {
            color: crate::common::CliColorChoice::Auto,
            profile: None,
            url: "https://grafana.example.com".to_string(),
            api_token: Some("secret".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        },
        dashboard_uid: Some("infra-main".to_string()),
        dashboard_url: None,
        vars_query: None,
        org_id: None,
        output_format: None,
        no_header: false,
        output_file: None,
    };

    let text_output = render_dashboard_variable_output(
        &super::test_support::InspectVarsArgs {
            output_format: Some(SimpleOutputFormat::Text),
            ..base_args.clone()
        },
        &document,
    )
    .unwrap();
    assert!(text_output.contains("Dashboard variables: Infra Overview (infra-main)"));
    assert!(text_output.contains("Variable count: 1"));
    assert!(text_output.contains("name=cluster"));

    let yaml_output = render_dashboard_variable_output(
        &super::test_support::InspectVarsArgs {
            output_format: Some(SimpleOutputFormat::Yaml),
            ..base_args.clone()
        },
        &document,
    )
    .unwrap();
    assert!(yaml_output.contains("dashboard_uid: infra-main"));
    assert!(yaml_output.contains("variable_count: 1"));

    let json_output = render_dashboard_variable_output(
        &super::test_support::InspectVarsArgs {
            output_format: Some(SimpleOutputFormat::Json),
            ..base_args
        },
        &document,
    )
    .unwrap();
    assert!(json_output.contains("\"dashboard_uid\": \"infra-main\""));
    assert!(json_output.contains("\"variable_count\": 1"));
}
