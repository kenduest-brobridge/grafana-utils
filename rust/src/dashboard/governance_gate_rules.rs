use serde_json::Value;
use std::collections::BTreeSet;

#[path = "governance_gate_rules_evaluation.rs"]
mod governance_gate_rules_evaluation;
#[path = "governance_gate_rules_findings.rs"]
mod governance_gate_rules_findings;
#[path = "governance_gate_rules_policy.rs"]
mod governance_gate_rules_policy;

pub(crate) use governance_gate_rules_evaluation::evaluate_dashboard_governance_gate_violations;
pub(crate) use governance_gate_rules_findings::build_governance_warning_findings;
pub(crate) use governance_gate_rules_policy::{build_checked_rules, parse_query_threshold_policy};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct QueryThresholdPolicy {
    allowed_families: BTreeSet<String>,
    allowed_uids: BTreeSet<String>,
    allowed_folder_prefixes: Vec<String>,
    forbid_unknown: bool,
    forbid_high_blast_radius: bool,
    forbid_mixed_families: bool,
    forbid_select_star: bool,
    require_sql_time_filter: bool,
    forbid_broad_loki_regex: bool,
    forbid_broad_prometheus_selectors: bool,
    forbid_regex_heavy_prometheus: bool,
    forbid_high_cardinality_regex: bool,
    max_prometheus_range_window_seconds: Option<usize>,
    max_prometheus_aggregation_depth: Option<usize>,
    max_prometheus_cost_score: Option<usize>,
    forbid_unscoped_loki_search: bool,
    max_panels_per_dashboard: Option<usize>,
    min_refresh_interval_seconds: Option<usize>,
    max_audit_score: Option<usize>,
    max_reason_count: Option<usize>,
    block_reasons: BTreeSet<String>,
    max_dashboard_load_score: Option<usize>,
    max_query_complexity_score: Option<usize>,
    max_dashboard_complexity_score: Option<usize>,
    max_queries_per_dashboard: Option<usize>,
    max_queries_per_panel: Option<usize>,
    pub(crate) fail_on_warnings: bool,
}

fn string_field(record: &Value, key: &str) -> String {
    record
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_policy() -> QueryThresholdPolicy {
        QueryThresholdPolicy {
            max_queries_per_dashboard: Some(1),
            max_queries_per_panel: Some(1),
            min_refresh_interval_seconds: Some(30),
            forbid_mixed_families: true,
            ..QueryThresholdPolicy::default()
        }
    }

    #[test]
    fn evaluate_dashboard_governance_gate_violations_keeps_aggregate_rules_intact() {
        let policy = sample_policy();
        let governance_document = json!({
            "dashboardGovernance": [
                {
                    "dashboardUid": "cpu-main",
                    "dashboardTitle": "CPU Main",
                    "panelCount": 2,
                    "mixedDatasource": true,
                    "datasourceFamilies": ["prometheus", "loki"]
                }
            ],
            "queryAudits": [],
            "dashboardAudits": [],
            "riskRecords": []
        });
        let queries = vec![
            json!({
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "panelId": "7",
                "panelTitle": "CPU",
                "datasource": "Prometheus Main",
                "datasourceUid": "prom-main",
                "datasourceFamily": "prometheus",
                "query": "up",
                "metrics": ["up"],
                "functions": [],
                "measurements": [],
                "buckets": [],
                "refresh": "5s"
            }),
            json!({
                "dashboardUid": "cpu-main",
                "dashboardTitle": "CPU Main",
                "panelId": "7",
                "panelTitle": "CPU",
                "datasource": "Prometheus Main",
                "datasourceUid": "prom-main",
                "datasourceFamily": "prometheus",
                "query": "sum(rate(http_requests_total[5m]))",
                "metrics": ["http_requests_total"],
                "functions": ["sum", "rate"],
                "measurements": [],
                "buckets": ["5m"],
                "refresh": "5s"
            }),
        ];

        let violations =
            evaluate_dashboard_governance_gate_violations(&policy, &governance_document, &queries)
                .unwrap();
        let codes = violations
            .iter()
            .map(|finding| finding.code.as_str())
            .collect::<Vec<&str>>();

        assert!(codes.contains(&"max-queries-per-dashboard"));
        assert!(codes.contains(&"max-queries-per-panel"));
        assert!(codes.contains(&"min-refresh-interval-seconds"));
        assert!(codes.contains(&"mixed-datasource-families-not-allowed"));
    }

    #[test]
    fn build_governance_warning_findings_uses_recommendation_then_detail_fallback() {
        let governance_document = json!({
            "riskRecords": [
                {
                    "kind": "dashboard-refresh-pressure",
                    "dashboardUid": "cpu-main",
                    "panelId": "7",
                    "datasource": "Prometheus Main",
                    "detail": "5s",
                    "recommendation": "Increase the refresh interval."
                },
                {
                    "kind": "custom-risk",
                    "dashboardUid": "cpu-main",
                    "panelId": "8",
                    "datasource": "Logs Main",
                    "detail": "Review this dashboard"
                }
            ]
        });

        let warnings = build_governance_warning_findings(&governance_document).unwrap();

        assert_eq!(warnings[0].message, "Increase the refresh interval.");
        assert_eq!(warnings[0].risk_kind, "dashboard-refresh-pressure");
        assert_eq!(warnings[1].message, "Review this dashboard");
        assert_eq!(warnings[1].risk_kind, "custom-risk");
    }
}
