//! Dashboard governance output contract regressions.
use super::super::test_support;
use super::super::{
    render_dashboard_governance_gate_result, GovernanceGateArgs, GovernanceGateOutputFormat,
};
use crate::dashboard::GovernancePolicySource;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn read_json_output_file(path: &Path) -> Value {
    let raw = fs::read_to_string(path).unwrap();
    assert!(
        raw.ends_with('\n'),
        "expected output file {} to end with a newline",
        path.display()
    );
    serde_json::from_str(&raw).unwrap()
}

#[test]
fn render_dashboard_governance_gate_result_lists_violations_and_warnings() {
    let result = test_support::DashboardGovernanceGateResult {
        ok: false,
        summary: test_support::DashboardGovernanceGateSummary {
            dashboard_count: 1,
            query_record_count: 2,
            violation_count: 1,
            warning_count: 1,
            checked_rules: json!({
                "datasourceAllowedFamilies": [],
                "datasourceAllowedUids": [],
                "allowedFolderPrefixes": [],
                "forbidUnknown": false,
                "forbidHighBlastRadius": false,
                "forbidMixedFamilies": false,
                "forbidSelectStar": false,
                "requireSqlTimeFilter": false,
                "forbidBroadLokiRegex": false,
                "forbidBroadPrometheusSelectors": false,
                "forbidRegexHeavyPrometheus": false,
                "forbidHighCardinalityRegex": false,
                "maxPrometheusRangeWindowSeconds": null,
                "maxPrometheusAggregationDepth": null,
                "maxPrometheusCostScore": null,
                "forbidUnscopedLokiSearch": false,
                "maxPanelsPerDashboard": null,
                "minRefreshIntervalSeconds": null,
                "maxAuditScore": null,
                "maxReasonCount": null,
                "blockReasons": [],
                "maxDashboardLoadScore": null,
                "maxQueryComplexityScore": null,
                "maxDashboardComplexityScore": null,
                "maxQueriesPerDashboard": 1,
                "maxQueriesPerPanel": null,
                "failOnWarnings": false
            }),
        },
        violations: vec![test_support::DashboardGovernanceGateFinding {
            severity: "error".to_string(),
            code: "max-queries-per-dashboard".to_string(),
            message: "Dashboard query count 2 exceeds policy maxQueriesPerDashboard=1.".to_string(),
            dashboard_uid: "core-main".to_string(),
            dashboard_title: "Core Main".to_string(),
            panel_id: String::new(),
            panel_title: String::new(),
            ref_id: String::new(),
            datasource: String::new(),
            datasource_uid: String::new(),
            datasource_family: String::new(),
            risk_kind: String::new(),
        }],
        warnings: vec![test_support::DashboardGovernanceGateFinding {
            severity: "warning".to_string(),
            code: "broad-loki-selector".to_string(),
            message: "Narrow the Loki stream selector.".to_string(),
            dashboard_uid: "core-main".to_string(),
            dashboard_title: String::new(),
            panel_id: "7".to_string(),
            panel_title: String::new(),
            ref_id: String::new(),
            datasource: "Logs Main".to_string(),
            datasource_uid: String::new(),
            datasource_family: String::new(),
            risk_kind: "broad-loki-selector".to_string(),
        }],
    };

    let output = render_dashboard_governance_gate_result(&result);
    assert!(output.contains("Dashboard governance gate: FAIL"));
    assert!(output.contains("Violations:"));
    assert!(output.contains("Warnings:"));
    assert!(output.contains("max-queries-per-dashboard"));
    assert!(output.contains("broad-loki-selector"));
}

#[test]
fn run_dashboard_governance_gate_writes_json_output_file() {
    let temp = tempdir().unwrap();
    let policy_path = temp.path().join("policy.json");
    let governance_path = temp.path().join("governance.json");
    let queries_path = temp.path().join("queries.json");
    let json_output = temp.path().join("governance-check.json");

    fs::write(
        &policy_path,
        serde_json::to_string_pretty(&json!({
            "version": 1,
            "datasources": {
                "allowedFamilies": [],
                "allowedUids": []
            },
            "queries": {
                "maxQueriesPerDashboard": 4,
                "maxQueriesPerPanel": 2
            },
            "enforcement": {
                "failOnWarnings": false
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &governance_path,
        serde_json::to_string_pretty(&json!({
            "summary": {
                "dashboardCount": 1,
                "queryRecordCount": 2
            },
            "dashboardGovernance": [],
            "queryAudits": [],
            "dashboardAudits": [],
            "riskRecords": []
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &queries_path,
        serde_json::to_string_pretty(&json!({
            "summary": {
                "dashboardCount": 1,
                "queryRecordCount": 2
            },
            "queries": [
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "panelId": "7",
                    "panelTitle": "Errors",
                    "refId": "A"
                },
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "panelId": "8",
                    "panelTitle": "Latency",
                    "refId": "B"
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let args = GovernanceGateArgs {
        policy_source: GovernancePolicySource::File,
        policy: Some(policy_path),
        builtin_policy: None,
        governance: governance_path,
        queries: queries_path,
        output_format: GovernanceGateOutputFormat::Json,
        json_output: Some(json_output.clone()),
        interactive: false,
    };

    test_support::run_dashboard_governance_gate(&args).unwrap();
    let output = read_json_output_file(&json_output);
    assert_eq!(output["ok"], json!(true));
    assert_eq!(output["summary"]["violationCount"], json!(0));
    assert_eq!(output["summary"]["warningCount"], json!(0));
    assert_eq!(
        output["summary"]["checkedRules"],
        json!({
            "datasourceAllowedFamilies": [],
            "datasourceAllowedUids": [],
            "allowedFolderPrefixes": [],
            "forbidUnknown": false,
            "forbidHighBlastRadius": false,
            "forbidMixedFamilies": false,
            "forbidSelectStar": false,
            "requireSqlTimeFilter": false,
            "forbidBroadLokiRegex": false,
            "forbidBroadPrometheusSelectors": false,
            "forbidRegexHeavyPrometheus": false,
            "forbidHighCardinalityRegex": false,
            "maxPrometheusRangeWindowSeconds": null,
            "maxPrometheusAggregationDepth": null,
            "maxPrometheusCostScore": null,
            "forbidUnscopedLokiSearch": false,
            "maxPanelsPerDashboard": null,
            "minRefreshIntervalSeconds": null,
            "maxAuditScore": null,
            "maxReasonCount": null,
            "blockReasons": [],
            "maxDashboardLoadScore": null,
            "maxQueryComplexityScore": null,
            "maxDashboardComplexityScore": null,
            "maxQueriesPerDashboard": 4,
            "maxQueriesPerPanel": 2,
            "failOnWarnings": false
        })
    );
    assert_eq!(output["violations"], json!([]));
    assert_eq!(output["warnings"], json!([]));
}

#[test]
fn run_dashboard_governance_gate_reads_yaml_policy_file() {
    let temp = tempdir().unwrap();
    let policy_path = temp.path().join("policy.yaml");
    let governance_path = temp.path().join("governance.json");
    let queries_path = temp.path().join("queries.json");

    fs::write(
        &policy_path,
        r#"version: 1
datasources:
  allowedFamilies: []
  allowedUids: []
queries:
  maxQueriesPerDashboard: 4
  maxQueriesPerPanel: 2
enforcement:
  failOnWarnings: false
"#,
    )
    .unwrap();
    fs::write(
        &governance_path,
        serde_json::to_string_pretty(&json!({
            "summary": {
                "dashboardCount": 1,
                "queryRecordCount": 2
            },
            "dashboardGovernance": [],
            "riskRecords": []
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &queries_path,
        serde_json::to_string_pretty(&json!({
            "summary": {
                "dashboardCount": 1,
                "queryRecordCount": 2
            },
            "queries": [
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "panelId": "7",
                    "panelTitle": "Errors",
                    "refId": "A"
                },
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "panelId": "8",
                    "panelTitle": "Latency",
                    "refId": "B"
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let args = GovernanceGateArgs {
        policy_source: GovernancePolicySource::File,
        policy: Some(policy_path),
        builtin_policy: None,
        governance: governance_path,
        queries: queries_path,
        output_format: GovernanceGateOutputFormat::Text,
        json_output: None,
        interactive: false,
    };

    let result = test_support::run_dashboard_governance_gate(&args);
    assert!(
        result.is_ok(),
        "expected YAML policy file to load successfully"
    );
}

#[test]
fn run_dashboard_governance_gate_uses_builtin_policy_source() {
    let temp = tempdir().unwrap();
    let governance_path = temp.path().join("governance.json");
    let queries_path = temp.path().join("queries.json");
    let json_output = temp.path().join("builtin-governance-check.json");

    fs::write(
        &governance_path,
        serde_json::to_string_pretty(&json!({
            "summary": {
                "dashboardCount": 1,
                "queryRecordCount": 2
            },
            "dashboardGovernance": [
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "panelCount": 2,
                    "mixedDatasource": false,
                    "datasourceFamilies": ["prometheus"]
                }
            ],
            "queryAudits": [],
            "dashboardAudits": [],
            "riskRecords": []
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        &queries_path,
        serde_json::to_string_pretty(&json!({
            "summary": {
                "dashboardCount": 1,
                "queryRecordCount": 2
            },
            "queries": [
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "panelId": "7",
                    "panelTitle": "Errors",
                    "refId": "A",
                    "folderPath": "General",
                    "datasource": "Prometheus Main",
                    "datasourceUid": "prom-main",
                    "datasourceFamily": "prometheus",
                    "query": "sum(rate(http_requests_total[5m]))",
                    "metrics": ["http_requests_total"],
                    "functions": ["sum", "rate"],
                    "measurements": [],
                    "buckets": [],
                    "panelType": "timeseries"
                },
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "panelId": "8",
                    "panelTitle": "Latency",
                    "refId": "B",
                    "folderPath": "General",
                    "datasource": "Prometheus Main",
                    "datasourceUid": "prom-main",
                    "datasourceFamily": "prometheus",
                    "query": "sum(rate(http_requests_total[5m]))",
                    "metrics": ["http_requests_total"],
                    "functions": ["sum", "rate"],
                    "measurements": [],
                    "buckets": [],
                    "panelType": "timeseries"
                }
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let args = GovernanceGateArgs {
        policy_source: GovernancePolicySource::Builtin,
        policy: None,
        builtin_policy: Some("default".to_string()),
        governance: governance_path,
        queries: queries_path,
        output_format: GovernanceGateOutputFormat::Json,
        json_output: Some(json_output.clone()),
        interactive: false,
    };

    test_support::run_dashboard_governance_gate(&args).unwrap();
    let output = read_json_output_file(&json_output);
    assert_eq!(output["ok"], json!(true));
    assert_eq!(
        output["summary"]["checkedRules"]["maxQueriesPerDashboard"],
        json!(80)
    );
    assert_eq!(output["summary"]["violationCount"], json!(0));
    assert_eq!(output["summary"]["warningCount"], json!(0));
}

#[test]
fn load_governance_policy_source_uses_builtin_policy_source() {
    let policy = test_support::load_governance_policy_source(
        GovernancePolicySource::Builtin,
        None,
        Some("default"),
    )
    .unwrap();

    assert_eq!(policy["version"], json!(1));
    assert_eq!(policy["queries"]["maxQueriesPerDashboard"], json!(80));
    assert_eq!(policy["queries"]["maxQueriesPerPanel"], json!(8));
}

struct BuiltinPolicyExpectations {
    max_queries_per_dashboard: i64,
    max_queries_per_panel: i64,
    max_query_complexity_score: i64,
    max_dashboard_complexity_score: i64,
    fail_on_warnings: bool,
    forbid_select_star: bool,
    require_sql_time_filter: bool,
    forbid_broad_loki_regex: bool,
}

fn assert_builtin_policy_profile(name: &str, expected: BuiltinPolicyExpectations) {
    let policy = test_support::load_governance_policy_source(
        GovernancePolicySource::Builtin,
        None,
        Some(name),
    )
    .unwrap();

    assert_eq!(policy["version"], json!(1));
    assert_eq!(
        policy["queries"]["maxQueriesPerDashboard"],
        json!(expected.max_queries_per_dashboard)
    );
    assert_eq!(
        policy["queries"]["maxQueriesPerPanel"],
        json!(expected.max_queries_per_panel)
    );
    assert_eq!(
        policy["queries"]["maxQueryComplexityScore"],
        json!(expected.max_query_complexity_score)
    );
    assert_eq!(
        policy["queries"]["maxDashboardComplexityScore"],
        json!(expected.max_dashboard_complexity_score)
    );
    assert_eq!(
        policy["enforcement"]["failOnWarnings"],
        json!(expected.fail_on_warnings)
    );
    assert_eq!(
        policy["queries"]["forbidSelectStar"],
        json!(expected.forbid_select_star)
    );
    assert_eq!(
        policy["queries"]["requireSqlTimeFilter"],
        json!(expected.require_sql_time_filter)
    );
    assert_eq!(
        policy["queries"]["forbidBroadLokiRegex"],
        json!(expected.forbid_broad_loki_regex)
    );
}

#[test]
fn load_governance_policy_source_supports_builtin_strict_balanced_and_lenient_profiles() {
    assert_builtin_policy_profile(
        "strict",
        BuiltinPolicyExpectations {
            max_queries_per_dashboard: 40,
            max_queries_per_panel: 4,
            max_query_complexity_score: 4,
            max_dashboard_complexity_score: 20,
            fail_on_warnings: true,
            forbid_select_star: true,
            require_sql_time_filter: true,
            forbid_broad_loki_regex: true,
        },
    );
    assert_builtin_policy_profile(
        "balanced",
        BuiltinPolicyExpectations {
            max_queries_per_dashboard: 60,
            max_queries_per_panel: 6,
            max_query_complexity_score: 5,
            max_dashboard_complexity_score: 30,
            fail_on_warnings: false,
            forbid_select_star: true,
            require_sql_time_filter: true,
            forbid_broad_loki_regex: true,
        },
    );
    assert_builtin_policy_profile(
        "lenient",
        BuiltinPolicyExpectations {
            max_queries_per_dashboard: 120,
            max_queries_per_panel: 12,
            max_query_complexity_score: 8,
            max_dashboard_complexity_score: 60,
            fail_on_warnings: false,
            forbid_select_star: false,
            require_sql_time_filter: false,
            forbid_broad_loki_regex: false,
        },
    );
}

#[test]
fn load_governance_policy_source_reports_unknown_builtin_policy_names() {
    let error = test_support::load_governance_policy_source(
        GovernancePolicySource::Builtin,
        None,
        Some("custom"),
    )
    .unwrap_err();
    let message = error.to_string();

    assert!(message.contains("Unknown built-in governance policy \"custom\""));
    assert!(message.contains("Supported values: default, strict, balanced, lenient."));
    assert!(message.contains("Alias: example -> default."));
}
