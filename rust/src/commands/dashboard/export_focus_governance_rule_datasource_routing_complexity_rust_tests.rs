//! Dashboard governance rules for datasource, routing, and complexity checks.
use super::test_support;
use serde_json::json;

#[test]
fn evaluate_dashboard_governance_gate_enforces_datasource_policy_rules() {
    let policy = json!({
        "version": 1,
        "datasources": {
            "allowedFamilies": ["prometheus"],
            "allowedUids": ["prom-main"],
            "forbidUnknown": true,
            "forbidHighBlastRadius": false,
            "forbidMixedFamilies": true
        },
        "enforcement": {
            "failOnWarnings": false
        }
    });
    let governance = json!({
        "summary": {
            "dashboardCount": 1,
            "queryRecordCount": 2
        },
        "dashboardGovernance": [
            {
                "dashboardUid": "mixed-main",
                "dashboardTitle": "Mixed Main",
                "datasourceFamilies": ["prometheus", "unknown"],
                "mixedDatasource": true
            }
        ],
        "riskRecords": []
    });
    let queries = json!({
        "summary": {
            "dashboardCount": 1,
            "queryRecordCount": 2
        },
        "queries": [
            {
                "dashboardUid": "mixed-main",
                "dashboardTitle": "Mixed Main",
                "panelId": "7",
                "panelTitle": "CPU",
                "refId": "A",
                "datasource": "Prometheus Main",
                "datasourceUid": "prom-main",
                "datasourceFamily": "prometheus"
            },
            {
                "dashboardUid": "mixed-main",
                "dashboardTitle": "Mixed Main",
                "panelId": "8",
                "panelTitle": "Custom",
                "refId": "B",
                "datasource": "",
                "datasourceUid": "custom-main",
                "datasourceFamily": "unknown"
            }
        ]
    });

    let result =
        test_support::evaluate_dashboard_governance_gate(&policy, &governance, &queries).unwrap();
    let codes = result
        .violations
        .iter()
        .map(|item| item.code.as_str())
        .collect::<Vec<&str>>();

    assert!(!result.ok);
    assert_eq!(result.summary.violation_count, 4);
    assert!(codes.contains(&"datasource-unknown"));
    assert!(codes.contains(&"datasource-family-not-allowed"));
    assert!(codes.contains(&"datasource-uid-not-allowed"));
    assert!(codes.contains(&"mixed-datasource-families-not-allowed"));
    assert_eq!(
        result.summary.checked_rules,
        json!({
            "datasourceAllowedFamilies": ["prometheus"],
            "datasourceAllowedUids": ["prom-main"],
            "allowedFolderPrefixes": [],
            "forbidUnknown": true,
            "forbidHighBlastRadius": false,
            "forbidMixedFamilies": true,
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
            "maxQueriesPerDashboard": null,
            "maxQueriesPerPanel": null,
            "failOnWarnings": false
        })
    );
}

#[test]
fn evaluate_dashboard_governance_gate_enforces_high_blast_radius_datasource_policy() {
    let policy = json!({
        "version": 1,
        "datasources": {
            "forbidHighBlastRadius": true
        }
    });
    let governance = json!({
        "summary": {
            "dashboardCount": 2,
            "queryRecordCount": 2,
            "highBlastRadiusDatasourceCount": 1
        },
        "dashboardGovernance": [],
        "datasourceGovernance": [
            {
                "datasourceUid": "prom-main",
                "datasource": "Prometheus Main",
                "family": "prometheus",
                "dashboardCount": 2,
                "folderCount": 2,
                "highBlastRadius": true,
                "dashboardTitles": ["Core Main", "Ops Main"]
            }
        ],
        "riskRecords": []
    });
    let queries = json!({
        "summary": {
            "dashboardCount": 2,
            "queryRecordCount": 2
        },
        "queries": [
            {
                "dashboardUid": "core-main",
                "dashboardTitle": "Core Main",
                "folderPath": "General",
                "panelId": "7",
                "panelTitle": "CPU",
                "refId": "A",
                "datasource": "Prometheus Main",
                "datasourceUid": "prom-main",
                "datasourceFamily": "prometheus",
                "query": "sum(rate(http_requests_total[5m]))"
            },
            {
                "dashboardUid": "ops-main",
                "dashboardTitle": "Ops Main",
                "folderPath": "Platform",
                "panelId": "8",
                "panelTitle": "Latency",
                "refId": "B",
                "datasource": "Prometheus Main",
                "datasourceUid": "prom-main",
                "datasourceFamily": "prometheus",
                "query": "sum(rate(process_cpu_seconds_total[5m]))"
            }
        ]
    });

    let result =
        test_support::evaluate_dashboard_governance_gate(&policy, &governance, &queries).unwrap();

    assert!(!result.ok);
    assert_eq!(result.summary.violation_count, 1);
    assert_eq!(
        result.violations[0].code,
        "datasource-high-blast-radius-not-allowed"
    );
    assert_eq!(result.violations[0].datasource_uid, "prom-main");
    assert_eq!(
        result.summary.checked_rules,
        json!({
            "datasourceAllowedFamilies": [],
            "datasourceAllowedUids": [],
            "allowedFolderPrefixes": [],
            "forbidUnknown": false,
            "forbidHighBlastRadius": true,
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
            "maxQueriesPerDashboard": null,
            "maxQueriesPerPanel": null,
            "failOnWarnings": false
        })
    );
}

#[test]
fn evaluate_dashboard_governance_gate_enforces_routing_sql_and_loki_policy_rules() {
    let policy = json!({
        "version": 1,
        "routing": {
            "allowedFolderPrefixes": ["Platform"]
        },
        "queries": {
            "forbidSelectStar": true,
            "requireSqlTimeFilter": true,
            "forbidBroadLokiRegex": true
        }
    });
    let governance = json!({
        "summary": {
            "dashboardCount": 2,
            "queryRecordCount": 3
        },
        "dashboardGovernance": [],
        "riskRecords": []
    });
    let queries = json!({
        "summary": {
            "dashboardCount": 2,
            "queryRecordCount": 3
        },
        "queries": [
            {
                "dashboardUid": "sql-main",
                "dashboardTitle": "SQL Main",
                "folderPath": "Operations",
                "panelId": "7",
                "panelTitle": "Rows",
                "refId": "A",
                "datasource": "Warehouse",
                "datasourceUid": "sql-main",
                "datasourceFamily": "sql",
                "query": "SELECT * FROM metrics"
            },
            {
                "dashboardUid": "sql-main",
                "dashboardTitle": "SQL Main",
                "folderPath": "Operations",
                "panelId": "8",
                "panelTitle": "Latency",
                "refId": "B",
                "datasource": "Warehouse",
                "datasourceUid": "sql-main",
                "datasourceFamily": "sql",
                "query": "SELECT count(*) FROM metrics"
            },
            {
                "dashboardUid": "logs-main",
                "dashboardTitle": "Logs Main",
                "folderPath": "Platform / Logs",
                "panelId": "9",
                "panelTitle": "Errors",
                "refId": "C",
                "datasource": "Logs Main",
                "datasourceUid": "logs-main",
                "datasourceFamily": "loki",
                "query": "{namespace=~\".*\"} |~ \".*\""
            }
        ]
    });

    let result =
        test_support::evaluate_dashboard_governance_gate(&policy, &governance, &queries).unwrap();
    let codes = result
        .violations
        .iter()
        .map(|item| item.code.as_str())
        .collect::<Vec<&str>>();

    assert!(!result.ok);
    assert_eq!(result.summary.violation_count, 6);
    assert!(codes.contains(&"routing-folder-not-allowed"));
    assert!(codes.contains(&"sql-select-star"));
    assert!(codes.contains(&"sql-missing-time-filter"));
    assert!(codes.contains(&"loki-broad-regex"));
    assert_eq!(
        result.summary.checked_rules,
        json!({
            "datasourceAllowedFamilies": [],
            "datasourceAllowedUids": [],
            "allowedFolderPrefixes": ["Platform"],
            "forbidUnknown": false,
            "forbidHighBlastRadius": false,
            "forbidMixedFamilies": false,
            "forbidSelectStar": true,
            "requireSqlTimeFilter": true,
            "forbidBroadLokiRegex": true,
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
            "maxQueriesPerDashboard": null,
            "maxQueriesPerPanel": null,
            "failOnWarnings": false
        })
    );
}

#[test]
fn evaluate_dashboard_governance_gate_enforces_query_and_dashboard_complexity_rules() {
    let policy = json!({
        "version": 1,
        "queries": {
            "maxQueryComplexityScore": 3,
            "maxDashboardComplexityScore": 6
        }
    });
    let governance = json!({
        "summary": {
            "dashboardCount": 1,
            "queryRecordCount": 2
        },
        "dashboardGovernance": [],
        "riskRecords": []
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
                "folderPath": "Platform",
                "panelId": "7",
                "panelTitle": "CPU",
                "refId": "A",
                "datasource": "Prometheus Main",
                "datasourceUid": "prom-main",
                "datasourceFamily": "prometheus",
                "metrics": ["http_requests_total", "process_cpu_seconds_total"],
                "measurements": ["job=\"grafana\""],
                "buckets": ["5m"],
                "query": "sum(rate(http_requests_total{job=~\"grafana\"}[5m]))"
            },
            {
                "dashboardUid": "core-main",
                "dashboardTitle": "Core Main",
                "folderPath": "Platform",
                "panelId": "8",
                "panelTitle": "Memory",
                "refId": "B",
                "datasource": "Prometheus Main",
                "datasourceUid": "prom-main",
                "datasourceFamily": "prometheus",
                "metrics": ["node_memory_MemAvailable_bytes"],
                "measurements": [],
                "buckets": [],
                "query": "max(node_memory_MemAvailable_bytes)"
            }
        ]
    });

    let result =
        test_support::evaluate_dashboard_governance_gate(&policy, &governance, &queries).unwrap();
    let codes = result
        .violations
        .iter()
        .map(|item| item.code.as_str())
        .collect::<Vec<&str>>();

    assert!(!result.ok);
    assert!(codes.contains(&"query-complexity-too-high"));
    assert!(codes.contains(&"dashboard-complexity-too-high"));
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
            "maxQueryComplexityScore": 3,
            "maxDashboardComplexityScore": 6,
            "maxQueriesPerDashboard": null,
            "maxQueriesPerPanel": null,
            "failOnWarnings": false
        })
    );
}
