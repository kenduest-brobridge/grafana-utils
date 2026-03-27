use serde_json::{json, Value};
use std::collections::BTreeSet;

use crate::common::{message, Result};

use super::QueryThresholdPolicy;

fn value_to_usize(value: Option<&Value>) -> Result<Option<usize>> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number
            .as_u64()
            .map(|value| Some(value as usize))
            .ok_or_else(|| message("Expected a non-negative integer in governance policy.")),
        Some(other) => Err(message(format!(
            "Expected a non-negative integer in governance policy, got {other}."
        ))),
    }
}

fn value_to_bool(value: Option<&Value>, default: bool) -> Result<bool> {
    match value {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(other) => Err(message(format!(
            "Expected a boolean in governance policy, got {other}."
        ))),
    }
}

fn value_to_string_set(value: Option<&Value>) -> Result<BTreeSet<String>> {
    match value {
        None | Some(Value::Null) => Ok(BTreeSet::new()),
        Some(Value::Array(values)) => Ok(values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect()),
        Some(other) => Err(message(format!(
            "Expected an array of strings in governance policy, got {other}."
        ))),
    }
}

pub(crate) fn parse_query_threshold_policy(policy: &Value) -> Result<QueryThresholdPolicy> {
    let Some(policy_object) = policy.as_object() else {
        return Err(message("Governance policy JSON must be an object."));
    };
    if let Some(version) = policy_object.get("version") {
        match version {
            Value::Number(number) if number.as_i64() == Some(1) => {}
            _ => {
                return Err(message(
                    "Governance policy version is not supported. Expected version 1.",
                ))
            }
        }
    }
    let queries = policy_object
        .get("queries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let datasources = policy_object
        .get("datasources")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let enforcement = policy_object
        .get("enforcement")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let routing = policy_object
        .get("routing")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let dashboards = policy_object
        .get("dashboards")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    Ok(QueryThresholdPolicy {
        allowed_families: value_to_string_set(datasources.get("allowedFamilies"))?,
        allowed_uids: value_to_string_set(datasources.get("allowedUids"))?,
        allowed_folder_prefixes: value_to_string_set(routing.get("allowedFolderPrefixes"))?
            .into_iter()
            .collect(),
        forbid_unknown: value_to_bool(datasources.get("forbidUnknown"), false)?,
        forbid_mixed_families: value_to_bool(datasources.get("forbidMixedFamilies"), false)?,
        forbid_select_star: value_to_bool(queries.get("forbidSelectStar"), false)?,
        require_sql_time_filter: value_to_bool(queries.get("requireSqlTimeFilter"), false)?,
        forbid_broad_loki_regex: value_to_bool(queries.get("forbidBroadLokiRegex"), false)?,
        forbid_broad_prometheus_selectors: value_to_bool(
            queries.get("forbidBroadPrometheusSelectors"),
            false,
        )?,
        forbid_regex_heavy_prometheus: value_to_bool(
            queries.get("forbidRegexHeavyPrometheus"),
            false,
        )?,
        forbid_high_cardinality_regex: value_to_bool(
            queries.get("forbidHighCardinalityRegex"),
            false,
        )?,
        max_prometheus_range_window_seconds: value_to_usize(
            queries.get("maxPrometheusRangeWindowSeconds"),
        )?,
        max_prometheus_aggregation_depth: value_to_usize(
            queries.get("maxPrometheusAggregationDepth"),
        )?,
        max_prometheus_cost_score: value_to_usize(queries.get("maxPrometheusCostScore"))?,
        forbid_unscoped_loki_search: value_to_bool(queries.get("forbidUnscopedLokiSearch"), false)?,
        max_panels_per_dashboard: value_to_usize(dashboards.get("maxPanelsPerDashboard"))?,
        min_refresh_interval_seconds: value_to_usize(dashboards.get("minRefreshIntervalSeconds"))?,
        max_audit_score: value_to_usize(queries.get("maxAuditScore"))?,
        max_reason_count: value_to_usize(queries.get("maxReasonCount"))?,
        block_reasons: value_to_string_set(queries.get("blockReasons"))?,
        max_dashboard_load_score: value_to_usize(dashboards.get("maxDashboardLoadScore"))?,
        max_query_complexity_score: value_to_usize(queries.get("maxQueryComplexityScore"))?,
        max_dashboard_complexity_score: value_to_usize(queries.get("maxDashboardComplexityScore"))?,
        max_queries_per_dashboard: value_to_usize(queries.get("maxQueriesPerDashboard"))?,
        max_queries_per_panel: value_to_usize(queries.get("maxQueriesPerPanel"))?,
        fail_on_warnings: value_to_bool(enforcement.get("failOnWarnings"), false)?,
    })
}

pub(crate) fn build_checked_rules(policy: &QueryThresholdPolicy) -> Value {
    json!({
        "datasourceAllowedFamilies": policy.allowed_families,
        "datasourceAllowedUids": policy.allowed_uids,
        "allowedFolderPrefixes": policy.allowed_folder_prefixes,
        "forbidUnknown": policy.forbid_unknown,
        "forbidMixedFamilies": policy.forbid_mixed_families,
        "forbidSelectStar": policy.forbid_select_star,
        "requireSqlTimeFilter": policy.require_sql_time_filter,
        "forbidBroadLokiRegex": policy.forbid_broad_loki_regex,
        "forbidBroadPrometheusSelectors": policy.forbid_broad_prometheus_selectors,
        "forbidRegexHeavyPrometheus": policy.forbid_regex_heavy_prometheus,
        "forbidHighCardinalityRegex": policy.forbid_high_cardinality_regex,
        "maxPrometheusRangeWindowSeconds": policy.max_prometheus_range_window_seconds,
        "maxPrometheusAggregationDepth": policy.max_prometheus_aggregation_depth,
        "maxPrometheusCostScore": policy.max_prometheus_cost_score,
        "forbidUnscopedLokiSearch": policy.forbid_unscoped_loki_search,
        "maxPanelsPerDashboard": policy.max_panels_per_dashboard,
        "minRefreshIntervalSeconds": policy.min_refresh_interval_seconds,
        "maxAuditScore": policy.max_audit_score,
        "maxReasonCount": policy.max_reason_count,
        "blockReasons": policy.block_reasons,
        "maxDashboardLoadScore": policy.max_dashboard_load_score,
        "maxQueryComplexityScore": policy.max_query_complexity_score,
        "maxDashboardComplexityScore": policy.max_dashboard_complexity_score,
        "maxQueriesPerDashboard": policy.max_queries_per_dashboard,
        "maxQueriesPerPanel": policy.max_queries_per_panel,
        "failOnWarnings": policy.fail_on_warnings,
    })
}
