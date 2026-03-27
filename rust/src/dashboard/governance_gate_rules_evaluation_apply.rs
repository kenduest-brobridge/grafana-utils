use serde_json::Value;
use std::collections::BTreeMap;

use super::super::{string_field, QueryThresholdPolicy};
use crate::common::{message, Result};
use crate::dashboard::governance_gate::DashboardGovernanceGateFinding;

use super::governance_gate_rules_evaluation_findings::{
    array_of_objects, build_dashboard_violation, build_dashboard_violation_from_fields,
    build_query_violation,
};
use super::governance_gate_rules_evaluation_policy::{
    is_sql_family, loki_query_is_broad, parse_duration_seconds, prometheus_query_is_broad,
    query_dashboard_refresh_seconds, query_uses_regex_matchers, query_uses_time_filter,
    query_uses_unscoped_loki_search, score_query_complexity,
};

pub(crate) fn evaluate_dashboard_governance_gate_violations(
    policy: &QueryThresholdPolicy,
    governance_document: &Value,
    queries: &[Value],
) -> Result<Vec<DashboardGovernanceGateFinding>> {
    let mut dashboard_counts = BTreeMap::<String, (String, usize)>::new();
    let mut dashboard_refresh_seconds = BTreeMap::<String, (String, usize)>::new();
    let mut dashboard_complexity_scores = BTreeMap::<(String, String), usize>::new();
    let mut panel_counts = BTreeMap::<(String, String), (String, String, usize)>::new();
    let mut violations = Vec::new();
    for query in queries {
        let dashboard_uid = string_field(query, "dashboardUid");
        let dashboard_title = string_field(query, "dashboardTitle");
        let panel_id = string_field(query, "panelId");
        let panel_title = string_field(query, "panelTitle");
        let datasource = string_field(query, "datasource");
        let datasource_uid = string_field(query, "datasourceUid");
        let datasource_family = string_field(query, "datasourceFamily");
        let folder_path = string_field(query, "folderPath");
        let query_text = string_field(query, "query");
        let complexity_score = score_query_complexity(query);
        let dashboard_entry = dashboard_counts
            .entry(dashboard_uid.clone())
            .or_insert((dashboard_title.clone(), 0usize));
        dashboard_entry.1 += 1;
        *dashboard_complexity_scores
            .entry((dashboard_uid.clone(), dashboard_title.clone()))
            .or_insert(0usize) += complexity_score;
        if let Some(refresh_seconds) = query_dashboard_refresh_seconds(query) {
            let entry = dashboard_refresh_seconds
                .entry(dashboard_uid.clone())
                .or_insert((dashboard_title.clone(), refresh_seconds));
            if refresh_seconds != 0 {
                entry.1 = entry.1.min(refresh_seconds);
            }
        }
        let panel_entry = panel_counts.entry((dashboard_uid, panel_id)).or_insert((
            dashboard_title,
            panel_title,
            0usize,
        ));
        panel_entry.2 += 1;

        if policy.forbid_unknown
            && (datasource_family.is_empty()
                || datasource_family.eq_ignore_ascii_case("unknown")
                || datasource.is_empty())
        {
            violations.push(build_query_violation(
                "datasource-unknown",
                "Datasource identity could not be resolved for this query row.".to_string(),
                query,
            ));
        }
        if !policy.allowed_families.is_empty()
            && !policy.allowed_families.contains(&datasource_family)
        {
            let family = if datasource_family.is_empty() {
                "unknown".to_string()
            } else {
                datasource_family.clone()
            };
            violations.push(build_query_violation(
                "datasource-family-not-allowed",
                format!("Datasource family {family} is not allowed by policy."),
                query,
            ));
        }
        if !policy.allowed_uids.is_empty()
            && !datasource_uid.is_empty()
            && !policy.allowed_uids.contains(&datasource_uid)
        {
            violations.push(build_query_violation(
                "datasource-uid-not-allowed",
                format!("Datasource uid {datasource_uid} is not allowed by policy."),
                query,
            ));
        }
        if !policy.allowed_folder_prefixes.is_empty()
            && !policy.allowed_folder_prefixes.iter().any(|prefix| {
                folder_path == *prefix || folder_path.starts_with(&format!("{prefix} /"))
            })
        {
            violations.push(build_query_violation(
                "routing-folder-not-allowed",
                format!(
                    "Dashboard folderPath {} is not allowed by policy.",
                    if folder_path.is_empty() {
                        "unknown".to_string()
                    } else {
                        folder_path.clone()
                    }
                ),
                query,
            ));
        }
        if policy.forbid_select_star
            && is_sql_family(&datasource_family)
            && query_text.to_ascii_lowercase().contains("select *")
        {
            violations.push(build_query_violation(
                "sql-select-star",
                "SQL query uses SELECT * and violates the policy.".to_string(),
                query,
            ));
        }
        if policy.require_sql_time_filter
            && is_sql_family(&datasource_family)
            && !query_uses_time_filter(&query_text)
        {
            violations.push(build_query_violation(
                "sql-missing-time-filter",
                "SQL query does not include a Grafana time filter macro.".to_string(),
                query,
            ));
        }
        if policy.forbid_broad_loki_regex
            && datasource_family.eq_ignore_ascii_case("loki")
            && loki_query_is_broad(&query_text)
        {
            violations.push(build_query_violation(
                "loki-broad-regex",
                "Loki query contains a broad match or empty selector.".to_string(),
                query,
            ));
        }
        if policy.forbid_broad_prometheus_selectors && prometheus_query_is_broad(query) {
            violations.push(build_query_violation(
                "prometheus-broad-selector",
                "Prometheus query uses a broad selector without label filters.".to_string(),
                query,
            ));
        }
        if policy.forbid_regex_heavy_prometheus
            && datasource_family.eq_ignore_ascii_case("prometheus")
            && query_uses_regex_matchers(&query_text)
        {
            violations.push(build_query_violation(
                "prometheus-regex-heavy",
                "Prometheus query uses regex label matchers and violates the policy.".to_string(),
                query,
            ));
        }
        if let Some(limit) = policy.max_prometheus_range_window_seconds {
            if datasource_family.eq_ignore_ascii_case("prometheus") {
                let max_bucket = query
                    .get("buckets")
                    .and_then(Value::as_array)
                    .map(|values: &Vec<Value>| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .filter_map(parse_duration_seconds)
                            .max()
                            .unwrap_or(0)
                    })
                    .unwrap_or(0);
                if max_bucket > limit {
                    violations.push(build_query_violation(
                        "prometheus-range-window-too-large",
                        format!(
                            "Prometheus range window {max_bucket}s exceeds policy maxPrometheusRangeWindowSeconds={limit}."
                        ),
                        query,
                    ));
                }
            }
        }
        if policy.forbid_unscoped_loki_search && query_uses_unscoped_loki_search(query) {
            violations.push(build_query_violation(
                "loki-unscoped-search",
                "Loki query performs line filtering without concrete label scoping.".to_string(),
                query,
            ));
        }
        if let Some(limit) = policy.max_query_complexity_score {
            if complexity_score > limit {
                violations.push(build_query_violation(
                    "query-complexity-too-high",
                    format!(
                        "Query complexity score {complexity_score} exceeds policy maxQueryComplexityScore={limit}."
                    ),
                    query,
                ));
            }
        }
    }

    if let Some(limit) = policy.max_queries_per_dashboard {
        for (dashboard_uid, (dashboard_title, query_count)) in &dashboard_counts {
            if *query_count > limit {
                violations.push(DashboardGovernanceGateFinding {
                    severity: "error".to_string(),
                    code: "max-queries-per-dashboard".to_string(),
                    message: format!(
                        "Dashboard query count {query_count} exceeds policy maxQueriesPerDashboard={limit}."
                    ),
                    dashboard_uid: dashboard_uid.clone(),
                    dashboard_title: dashboard_title.clone(),
                    panel_id: String::new(),
                    panel_title: String::new(),
                    ref_id: String::new(),
                    datasource: String::new(),
                    datasource_uid: String::new(),
                    datasource_family: String::new(),
                    risk_kind: String::new(),
                });
            }
        }
    }
    if let Some(limit) = policy.max_queries_per_panel {
        for ((dashboard_uid, panel_id), (dashboard_title, panel_title, query_count)) in
            &panel_counts
        {
            if *query_count > limit {
                violations.push(DashboardGovernanceGateFinding {
                    severity: "error".to_string(),
                    code: "max-queries-per-panel".to_string(),
                    message: format!(
                        "Panel query count {query_count} exceeds policy maxQueriesPerPanel={limit}."
                    ),
                    dashboard_uid: dashboard_uid.clone(),
                    dashboard_title: dashboard_title.clone(),
                    panel_id: panel_id.clone(),
                    panel_title: panel_title.clone(),
                    ref_id: String::new(),
                    datasource: String::new(),
                    datasource_uid: String::new(),
                    datasource_family: String::new(),
                    risk_kind: String::new(),
                });
            }
        }
    }
    if let Some(limit) = policy.max_dashboard_complexity_score {
        for ((dashboard_uid, dashboard_title), complexity_score) in &dashboard_complexity_scores {
            if *complexity_score > limit {
                violations.push(DashboardGovernanceGateFinding {
                    severity: "error".to_string(),
                    code: "dashboard-complexity-too-high".to_string(),
                    message: format!(
                        "Dashboard complexity score {complexity_score} exceeds policy maxDashboardComplexityScore={limit}."
                    ),
                    dashboard_uid: dashboard_uid.clone(),
                    dashboard_title: dashboard_title.clone(),
                    panel_id: String::new(),
                    panel_title: String::new(),
                    ref_id: String::new(),
                    datasource: String::new(),
                    datasource_uid: String::new(),
                    datasource_family: String::new(),
                    risk_kind: String::new(),
                });
            }
        }
    }

    if policy.forbid_mixed_families {
        let dashboards = governance_document
            .get("dashboardGovernance")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                message("Dashboard governance JSON must contain a dashboardGovernance array.")
            })?;
        for dashboard in dashboards {
            let mixed = dashboard
                .get("mixedDatasource")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if mixed {
                let families = dashboard
                    .get("datasourceFamilies")
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<&str>>()
                            .join(",")
                    })
                    .unwrap_or_default();
                violations.push(build_dashboard_violation(
                    "mixed-datasource-families-not-allowed",
                    format!(
                        "Dashboard uses mixed datasource families{}{}.",
                        if families.is_empty() { "" } else { ": " },
                        families
                    ),
                    dashboard,
                ));
            }
        }
    }
    if policy.max_panels_per_dashboard.is_some() || policy.min_refresh_interval_seconds.is_some() {
        let dashboards = array_of_objects(governance_document, "dashboardGovernance")?;
        if let Some(limit) = policy.max_panels_per_dashboard {
            for dashboard in dashboards {
                let panel_count = dashboard
                    .get("panelCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
                if panel_count > limit {
                    violations.push(build_dashboard_violation(
                        "max-panels-per-dashboard",
                        format!(
                            "Dashboard panel count {panel_count} exceeds policy maxPanelsPerDashboard={limit}."
                        ),
                        dashboard,
                    ));
                }
            }
        }
        if let Some(limit) = policy.min_refresh_interval_seconds {
            let mut refresh_by_dashboard = BTreeMap::<String, (String, usize)>::new();
            for query in queries {
                let Some(refresh_seconds) = query_dashboard_refresh_seconds(query) else {
                    continue;
                };
                let dashboard_uid = string_field(query, "dashboardUid");
                if dashboard_uid.is_empty() {
                    continue;
                }
                let dashboard_title = string_field(query, "dashboardTitle");
                let entry = refresh_by_dashboard
                    .entry(dashboard_uid)
                    .or_insert((dashboard_title, refresh_seconds));
                if refresh_seconds != 0 {
                    entry.1 = entry.1.min(refresh_seconds);
                }
            }
            for (dashboard_uid, (dashboard_title, refresh_seconds)) in refresh_by_dashboard {
                if refresh_seconds != 0 && refresh_seconds < limit {
                    violations.push(build_dashboard_violation_from_fields(
                        "min-refresh-interval-seconds",
                        format!(
                            "Dashboard refresh interval {refresh_seconds}s is below policy minRefreshIntervalSeconds={limit}."
                        ),
                        dashboard_uid,
                        dashboard_title,
                    ));
                }
            }
        }
    }

    if policy.max_audit_score.is_some()
        || policy.max_reason_count.is_some()
        || !policy.block_reasons.is_empty()
        || policy.max_dashboard_load_score.is_some()
        || policy.forbid_high_cardinality_regex
        || policy.max_prometheus_aggregation_depth.is_some()
        || policy.max_prometheus_cost_score.is_some()
    {
        let query_audits = array_of_objects(governance_document, "queryAudits")?;
        for audit in query_audits {
            let score = audit.get("score").and_then(Value::as_u64).unwrap_or(0) as usize;
            let query_cost_score = audit
                .get("queryCostScore")
                .and_then(Value::as_u64)
                .unwrap_or(score as u64) as usize;
            let aggregation_depth = audit
                .get("aggregationDepth")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            let reasons = audit
                .get("reasons")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default();
            if let Some(limit) = policy.max_audit_score {
                if score > limit {
                    violations.push(build_query_violation(
                        "query-audit-score-too-high",
                        format!(
                            "Query audit score {score} exceeds policy maxAuditScore={limit}. reasons={}",
                            reasons.join(",")
                        ),
                        audit,
                    ));
                }
            }
            if let Some(limit) = policy.max_reason_count {
                if reasons.len() > limit {
                    violations.push(build_query_violation(
                        "query-audit-reason-count-too-high",
                        format!(
                            "Query audit reason count {} exceeds policy maxReasonCount={limit}. reasons={}",
                            reasons.len(),
                            reasons.join(",")
                        ),
                        audit,
                    ));
                }
            }
            if !policy.block_reasons.is_empty()
                && reasons
                    .iter()
                    .any(|reason| policy.block_reasons.contains(reason))
            {
                violations.push(build_query_violation(
                    "query-audit-blocked-reason",
                    format!(
                        "Query audit contains blocked reasons: {}",
                        reasons.join(",")
                    ),
                    audit,
                ));
            }
            if policy.forbid_high_cardinality_regex
                && reasons
                    .iter()
                    .any(|reason| reason == "prometheus-high-cardinality-regex")
            {
                violations.push(build_query_violation(
                    "prometheus-high-cardinality-regex",
                    "Prometheus query uses regex matchers on likely high-cardinality labels."
                        .to_string(),
                    audit,
                ));
            }
            if let Some(limit) = policy.max_prometheus_aggregation_depth {
                if aggregation_depth > limit {
                    violations.push(build_query_violation(
                        "prometheus-aggregation-depth-too-high",
                        format!(
                            "Prometheus aggregation depth {aggregation_depth} exceeds policy maxPrometheusAggregationDepth={limit}."
                        ),
                        audit,
                    ));
                }
            }
            if let Some(limit) = policy.max_prometheus_cost_score {
                if query_cost_score > limit {
                    violations.push(build_query_violation(
                        "prometheus-cost-score-too-high",
                        format!(
                            "Prometheus query cost score {query_cost_score} exceeds policy maxPrometheusCostScore={limit}."
                        ),
                        audit,
                    ));
                }
            }
        }
        let dashboard_audits = array_of_objects(governance_document, "dashboardAudits")?;
        if let Some(limit) = policy.max_dashboard_load_score {
            for audit in dashboard_audits {
                let score = audit.get("score").and_then(Value::as_u64).unwrap_or(0) as usize;
                if score > limit {
                    violations.push(build_dashboard_violation(
                        "dashboard-load-score-too-high",
                        format!(
                            "Dashboard load score {score} exceeds policy maxDashboardLoadScore={limit}."
                        ),
                        audit,
                    ));
                }
            }
        }
    }

    Ok(violations)
}
