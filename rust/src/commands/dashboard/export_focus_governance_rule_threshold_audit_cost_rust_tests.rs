//! Dashboard governance rules for query thresholds, audits, and cost checks.
use super::test_support;
use serde_json::json;

#[test]
fn evaluate_dashboard_governance_gate_enforces_query_thresholds_and_warning_policy() {
    let policy = json!({
        "version": 1,
        "queries": {
            "maxQueriesPerDashboard": 1,
            "maxQueriesPerPanel": 1
        },
        "enforcement": {
            "failOnWarnings": true
        }
    });
    let governance = json!({
        "summary": {
            "dashboardCount": 1,
            "queryRecordCount": 2
        },
        "riskRecords": [
            {
                "kind": "broad-loki-selector",
                "dashboardUid": "core-main",
                "panelId": "7",
                "datasource": "Logs Main",
                "detail": "{}",
                "recommendation": "Narrow the Loki stream selector."
            }
        ]
    });
    let queries = json!({
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
                "datasource": "Logs Main",
                "datasourceUid": "logs-main",
                "datasourceFamily": "loki"
            },
            {
                "dashboardUid": "core-main",
                "dashboardTitle": "Core Main",
                "panelId": "7",
                "panelTitle": "Errors",
                "refId": "B",
                "datasource": "Logs Main",
                "datasourceUid": "logs-main",
                "datasourceFamily": "loki"
            }
        ]
    });

    let result =
        test_support::evaluate_dashboard_governance_gate(&policy, &governance, &queries).unwrap();

    assert!(!result.ok);
    assert_eq!(result.summary.dashboard_count, 1);
    assert_eq!(result.summary.query_record_count, 2);
    assert_eq!(result.summary.violation_count, 2);
    assert_eq!(result.summary.warning_count, 1);
    assert_eq!(
        result.summary.checked_rules,
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
            "maxQueriesPerDashboard": 1,
            "maxQueriesPerPanel": 1,
            "failOnWarnings": true
        })
    );
    assert_eq!(result.violations[0].code, "max-queries-per-dashboard");
    assert_eq!(result.violations[1].code, "max-queries-per-panel");
    assert_eq!(result.warnings[0].risk_kind, "broad-loki-selector");
}

#[test]
fn evaluate_dashboard_governance_gate_enforces_query_audit_contract_rules() {
    let policy = json!({
        "version": 1,
        "queries": {
            "maxAuditScore": 2,
            "maxReasonCount": 1,
            "blockReasons": ["unscoped-loki-search"]
        },
        "dashboards": {
            "maxDashboardLoadScore": 2
        }
    });
    let governance = json!({
        "summary": {
            "dashboardCount": 1,
            "queryRecordCount": 2
        },
        "dashboardGovernance": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "panelCount": 31,
                "queryCount": 2
            }
        ],
        "riskRecords": [],
        "queryAudits": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "folderPath": "Platform",
                "panelId": "7",
                "panelTitle": "CPU",
                "refId": "A",
                "datasource": "Prometheus Main",
                "datasourceUid": "prom-main",
                "datasourceFamily": "prometheus",
                "aggregationDepth": 1,
                "regexMatcherCount": 1,
                "estimatedSeriesRisk": "medium",
                "queryCostScore": 3,
                "score": 3,
                "severity": "medium",
                "reasons": ["broad-prometheus-selector", "prometheus-regex-heavy"],
                "recommendations": ["scope it"]
            },
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "folderPath": "Platform",
                "panelId": "8",
                "panelTitle": "Logs",
                "refId": "B",
                "datasource": "Logs Main",
                "datasourceUid": "logs-main",
                "datasourceFamily": "loki",
                "aggregationDepth": 0,
                "regexMatcherCount": 0,
                "estimatedSeriesRisk": "low",
                "queryCostScore": 4,
                "score": 4,
                "severity": "high",
                "reasons": ["unscoped-loki-search"],
                "recommendations": ["scope labels"]
            }
        ],
        "dashboardAudits": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "folderPath": "Platform",
                "panelCount": 31,
                "queryCount": 2,
                "refreshIntervalSeconds": 5,
                "score": 4,
                "severity": "high",
                "reasons": ["dashboard-panel-pressure", "dashboard-refresh-pressure"],
                "recommendations": ["split it"]
            }
        ]
    });
    let queries = json!({
        "summary": {
            "dashboardCount": 1,
            "queryRecordCount": 2
        },
        "queries": []
    });

    let result =
        test_support::evaluate_dashboard_governance_gate(&policy, &governance, &queries).unwrap();
    let codes = result
        .violations
        .iter()
        .map(|item| item.code.as_str())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"query-audit-score-too-high"));
    assert!(codes.contains(&"query-audit-reason-count-too-high"));
    assert!(codes.contains(&"query-audit-blocked-reason"));
    assert!(codes.contains(&"dashboard-load-score-too-high"));
}

#[test]
fn evaluate_dashboard_governance_gate_enforces_prometheus_cost_policy_rules() {
    let policy = json!({
        "version": 1,
        "queries": {
            "forbidHighCardinalityRegex": true,
            "maxPrometheusAggregationDepth": 1,
            "maxPrometheusCostScore": 3
        }
    });
    let governance = json!({
        "summary": {
            "dashboardCount": 1,
            "queryRecordCount": 1
        },
        "riskRecords": [],
        "queryAudits": [
            {
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "folderPath": "Platform",
                "panelId": "7",
                "panelTitle": "CPU",
                "refId": "A",
                "datasource": "Prometheus Main",
                "datasourceUid": "prom-main",
                "datasourceFamily": "prometheus",
                "aggregationDepth": 2,
                "regexMatcherCount": 2,
                "estimatedSeriesRisk": "high",
                "queryCostScore": 5,
                "score": 5,
                "severity": "high",
                "reasons": ["prometheus-high-cardinality-regex", "prometheus-deep-aggregation"],
                "recommendations": ["scope it"]
            }
        ],
        "dashboardAudits": []
    });
    let queries = json!({
        "summary": {
            "dashboardCount": 1,
            "queryRecordCount": 1
        },
        "queries": []
    });

    let result =
        test_support::evaluate_dashboard_governance_gate(&policy, &governance, &queries).unwrap();
    let codes = result
        .violations
        .iter()
        .map(|item| item.code.as_str())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"prometheus-high-cardinality-regex"));
    assert!(codes.contains(&"prometheus-aggregation-depth-too-high"));
    assert!(codes.contains(&"prometheus-cost-score-too-high"));
}
