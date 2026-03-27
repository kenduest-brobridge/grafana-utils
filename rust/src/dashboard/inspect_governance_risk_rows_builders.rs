use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use crate::dashboard::inspect_report::{ExportInspectionQueryReport, ExportInspectionQueryRow};
use crate::dashboard::ExportInspectionSummary;

use super::super::super::{
    build_dashboard_dependency_rows, build_inventory_lookup, resolve_datasource_identity,
    DashboardAuditRow, GovernanceRiskRow, QueryAuditRow,
};
use super::super::inspect_governance_risk_spec::{
    lookup_governance_risk_spec, ordered_push, parse_duration_seconds, severity_for_score,
    GOVERNANCE_RISK_KIND_BROAD_LOKI_SELECTOR, GOVERNANCE_RISK_KIND_BROAD_PROMETHEUS_SELECTOR,
    GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE, GOVERNANCE_RISK_KIND_DASHBOARD_REFRESH_PRESSURE,
    GOVERNANCE_RISK_KIND_EMPTY_QUERY_ANALYSIS, GOVERNANCE_RISK_KIND_LARGE_PROMETHEUS_RANGE,
    GOVERNANCE_RISK_KIND_MIXED_DASHBOARD, GOVERNANCE_RISK_KIND_ORPHANED_DATASOURCE,
    GOVERNANCE_RISK_KIND_PROMETHEUS_DEEP_AGGREGATION,
    GOVERNANCE_RISK_KIND_PROMETHEUS_HIGH_CARDINALITY_REGEX,
    GOVERNANCE_RISK_KIND_PROMETHEUS_REGEX_HEAVY, GOVERNANCE_RISK_KIND_UNKNOWN_DATASOURCE_FAMILY,
    GOVERNANCE_RISK_KIND_UNSCOPED_LOKI_SEARCH,
};
use super::inspect_governance_risk_rows_query_helpers::{
    extract_loki_stream_selectors, find_broad_loki_selector, largest_bucket_seconds,
    prometheus_aggregation_depth, prometheus_estimated_series_risk, prometheus_regex_matcher_count,
    query_uses_broad_prometheus_selector, query_uses_high_cardinality_prometheus_regex,
    query_uses_prometheus_regex, query_uses_unscoped_loki_search,
};
use crate::dashboard::inspect_family::normalize_family_name;

fn build_query_audit_row(
    row: &ExportInspectionQueryRow,
    datasource_name: String,
    datasource_uid: String,
    datasource_family: String,
) -> QueryAuditRow {
    let mut score = 0usize;
    let mut reasons = Vec::new();
    let mut recommendations = Vec::new();
    let broad_prometheus_selector = query_uses_broad_prometheus_selector(row);
    let prometheus_regex = query_uses_prometheus_regex(row);
    let regex_matcher_count = prometheus_regex_matcher_count(row);
    let high_cardinality_regex = query_uses_high_cardinality_prometheus_regex(row);
    let aggregation_depth = prometheus_aggregation_depth(row);
    let max_bucket_seconds = largest_bucket_seconds(row);
    if broad_prometheus_selector {
        score += 2;
        ordered_push(&mut reasons, GOVERNANCE_RISK_KIND_BROAD_PROMETHEUS_SELECTOR);
        ordered_push(
            &mut recommendations,
            lookup_governance_risk_spec(GOVERNANCE_RISK_KIND_BROAD_PROMETHEUS_SELECTOR)
                .recommendation,
        );
    }
    if prometheus_regex {
        score += 1;
        ordered_push(&mut reasons, GOVERNANCE_RISK_KIND_PROMETHEUS_REGEX_HEAVY);
        ordered_push(
            &mut recommendations,
            lookup_governance_risk_spec(GOVERNANCE_RISK_KIND_PROMETHEUS_REGEX_HEAVY).recommendation,
        );
    }
    if high_cardinality_regex {
        score += 2;
        ordered_push(
            &mut reasons,
            GOVERNANCE_RISK_KIND_PROMETHEUS_HIGH_CARDINALITY_REGEX,
        );
        ordered_push(
            &mut recommendations,
            lookup_governance_risk_spec(GOVERNANCE_RISK_KIND_PROMETHEUS_HIGH_CARDINALITY_REGEX)
                .recommendation,
        );
    }
    if aggregation_depth >= 2 {
        score += 1;
        ordered_push(
            &mut reasons,
            GOVERNANCE_RISK_KIND_PROMETHEUS_DEEP_AGGREGATION,
        );
        ordered_push(
            &mut recommendations,
            lookup_governance_risk_spec(GOVERNANCE_RISK_KIND_PROMETHEUS_DEEP_AGGREGATION)
                .recommendation,
        );
    }
    if max_bucket_seconds.unwrap_or(0) >= 60 * 60
        && normalize_family_name(&row.datasource_type) == "prometheus"
    {
        score += 2;
        ordered_push(&mut reasons, GOVERNANCE_RISK_KIND_LARGE_PROMETHEUS_RANGE);
        ordered_push(
            &mut recommendations,
            lookup_governance_risk_spec(GOVERNANCE_RISK_KIND_LARGE_PROMETHEUS_RANGE).recommendation,
        );
    }
    if query_uses_unscoped_loki_search(row) {
        score += 3;
        ordered_push(&mut reasons, GOVERNANCE_RISK_KIND_UNSCOPED_LOKI_SEARCH);
        ordered_push(
            &mut recommendations,
            lookup_governance_risk_spec(GOVERNANCE_RISK_KIND_UNSCOPED_LOKI_SEARCH).recommendation,
        );
    }
    let query_cost_score = score;
    let estimated_series_risk = prometheus_estimated_series_risk(
        broad_prometheus_selector,
        regex_matcher_count,
        high_cardinality_regex,
        aggregation_depth,
        max_bucket_seconds,
    );
    QueryAuditRow {
        dashboard_uid: row.dashboard_uid.clone(),
        dashboard_title: row.dashboard_title.clone(),
        folder_path: row.folder_path.clone(),
        panel_id: row.panel_id.clone(),
        panel_title: row.panel_title.clone(),
        ref_id: row.ref_id.clone(),
        datasource: datasource_name,
        datasource_uid,
        datasource_family,
        aggregation_depth,
        regex_matcher_count,
        estimated_series_risk,
        query_cost_score,
        score,
        severity: severity_for_score(score),
        reasons,
        recommendations,
    }
}

pub(crate) fn build_dashboard_audit_rows(
    report: &ExportInspectionQueryReport,
) -> Vec<DashboardAuditRow> {
    let refresh_by_dashboard = load_dashboard_refresh_by_uid(report);
    build_dashboard_dependency_rows(report)
        .into_iter()
        .map(|dashboard| {
            let mut score = 0usize;
            let mut reasons = Vec::new();
            let mut recommendations = Vec::new();
            if dashboard.panel_count > 30 {
                score += 2;
                ordered_push(&mut reasons, GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE);
                ordered_push(
                    &mut recommendations,
                    lookup_governance_risk_spec(GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE)
                        .recommendation,
                );
            }
            let refresh_interval_seconds = refresh_by_dashboard
                .get(&dashboard.dashboard_uid)
                .and_then(|value| parse_duration_seconds(value));
            if refresh_interval_seconds.unwrap_or(0) != 0
                && refresh_interval_seconds.unwrap_or(0) < 30
            {
                score += if refresh_interval_seconds.unwrap_or(0) < 10 {
                    3
                } else {
                    2
                };
                ordered_push(
                    &mut reasons,
                    GOVERNANCE_RISK_KIND_DASHBOARD_REFRESH_PRESSURE,
                );
                ordered_push(
                    &mut recommendations,
                    lookup_governance_risk_spec(GOVERNANCE_RISK_KIND_DASHBOARD_REFRESH_PRESSURE)
                        .recommendation,
                );
            }
            DashboardAuditRow {
                dashboard_uid: dashboard.dashboard_uid,
                dashboard_title: dashboard.dashboard_title,
                folder_path: dashboard.folder_path,
                panel_count: dashboard.panel_count,
                query_count: dashboard.query_count,
                refresh_interval_seconds,
                score,
                severity: severity_for_score(score),
                reasons,
                recommendations,
            }
        })
        .collect()
}

fn load_dashboard_refresh_by_uid(report: &ExportInspectionQueryReport) -> BTreeMap<String, String> {
    let mut refresh_by_uid = BTreeMap::new();
    let mut file_by_uid = BTreeMap::new();
    for row in &report.queries {
        if !row.dashboard_uid.trim().is_empty() && !row.file_path.trim().is_empty() {
            file_by_uid
                .entry(row.dashboard_uid.clone())
                .or_insert(row.file_path.clone());
        }
    }
    for (dashboard_uid, file_path) in file_by_uid {
        let Ok(raw) = fs::read_to_string(&file_path) else {
            continue;
        };
        let Ok(document) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let refresh = document
            .get("dashboard")
            .and_then(|dashboard| dashboard.get("refresh"))
            .or_else(|| document.get("refresh"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if let Some(refresh) = refresh {
            refresh_by_uid.insert(dashboard_uid, refresh);
        }
    }
    refresh_by_uid
}

fn build_governance_risk_row(
    kind: &str,
    dashboard_uid: String,
    panel_id: String,
    datasource: String,
    detail: String,
) -> GovernanceRiskRow {
    let spec = lookup_governance_risk_spec(kind);
    GovernanceRiskRow {
        kind: kind.to_string(),
        severity: spec.severity.to_string(),
        category: spec.category.to_string(),
        dashboard_uid,
        panel_id,
        datasource,
        detail,
        recommendation: spec.recommendation.to_string(),
    }
}

pub(crate) fn build_governance_risk_rows(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> Vec<GovernanceRiskRow> {
    let (inventory_by_uid, inventory_by_name) = build_inventory_lookup(summary);
    let refresh_by_dashboard = load_dashboard_refresh_by_uid(report);
    let mut seen = BTreeSet::new();
    let mut risks = Vec::new();

    for dashboard in &summary.mixed_dashboards {
        let risk = build_governance_risk_row(
            GOVERNANCE_RISK_KIND_MIXED_DASHBOARD,
            dashboard.uid.clone(),
            String::new(),
            dashboard.datasources.join(","),
            dashboard.title.clone(),
        );
        if seen.insert(risk.clone()) {
            risks.push(risk);
        }
    }
    for datasource in &summary.datasource_inventory {
        if datasource.reference_count != 0 || datasource.dashboard_count != 0 {
            continue;
        }
        let risk = build_governance_risk_row(
            GOVERNANCE_RISK_KIND_ORPHANED_DATASOURCE,
            String::new(),
            String::new(),
            if datasource.uid.trim().is_empty() {
                datasource.name.clone()
            } else {
                datasource.uid.clone()
            },
            datasource.datasource_type.clone(),
        );
        if seen.insert(risk.clone()) {
            risks.push(risk);
        }
    }
    for row in &report.queries {
        let identity = resolve_datasource_identity(row, &inventory_by_uid, &inventory_by_name);
        if normalize_family_name(&identity.datasource_type) == "unknown" {
            let risk = build_governance_risk_row(
                GOVERNANCE_RISK_KIND_UNKNOWN_DATASOURCE_FAMILY,
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                identity.name.clone(),
                row.query_field.clone(),
            );
            if seen.insert(risk.clone()) {
                risks.push(risk);
            }
        }
        if normalize_family_name(&identity.datasource_type) == "loki" {
            if let Some(selector) = find_broad_loki_selector(&row.query_text) {
                let risk = build_governance_risk_row(
                    GOVERNANCE_RISK_KIND_BROAD_LOKI_SELECTOR,
                    row.dashboard_uid.clone(),
                    row.panel_id.clone(),
                    identity.name.clone(),
                    selector,
                );
                if seen.insert(risk.clone()) {
                    risks.push(risk);
                }
            }
        }
        if query_uses_broad_prometheus_selector(row) {
            let risk = build_governance_risk_row(
                GOVERNANCE_RISK_KIND_BROAD_PROMETHEUS_SELECTOR,
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                identity.name.clone(),
                row.query_text.clone(),
            );
            if seen.insert(risk.clone()) {
                risks.push(risk);
            }
        }
        if query_uses_prometheus_regex(row) {
            let risk = build_governance_risk_row(
                GOVERNANCE_RISK_KIND_PROMETHEUS_REGEX_HEAVY,
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                identity.name.clone(),
                row.query_text.clone(),
            );
            if seen.insert(risk.clone()) {
                risks.push(risk);
            }
        }
        if query_uses_high_cardinality_prometheus_regex(row) {
            let risk = build_governance_risk_row(
                GOVERNANCE_RISK_KIND_PROMETHEUS_HIGH_CARDINALITY_REGEX,
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                identity.name.clone(),
                row.query_text.clone(),
            );
            if seen.insert(risk.clone()) {
                risks.push(risk);
            }
        }
        if prometheus_aggregation_depth(row) >= 2 {
            let risk = build_governance_risk_row(
                GOVERNANCE_RISK_KIND_PROMETHEUS_DEEP_AGGREGATION,
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                identity.name.clone(),
                row.query_text.clone(),
            );
            if seen.insert(risk.clone()) {
                risks.push(risk);
            }
        }
        if let Some(bucket_seconds) = largest_bucket_seconds(row) {
            if normalize_family_name(&identity.datasource_type) == "prometheus"
                && bucket_seconds >= 60 * 60
            {
                let risk = build_governance_risk_row(
                    GOVERNANCE_RISK_KIND_LARGE_PROMETHEUS_RANGE,
                    row.dashboard_uid.clone(),
                    row.panel_id.clone(),
                    identity.name.clone(),
                    row.buckets.join(","),
                );
                if seen.insert(risk.clone()) {
                    risks.push(risk);
                }
            }
        }
        if query_uses_unscoped_loki_search(row) {
            let risk = build_governance_risk_row(
                GOVERNANCE_RISK_KIND_UNSCOPED_LOKI_SEARCH,
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                identity.name.clone(),
                extract_loki_stream_selectors(&row.query_text).join(","),
            );
            if seen.insert(risk.clone()) {
                risks.push(risk);
            }
        }
        if row.metrics.is_empty()
            && row.functions.is_empty()
            && row.measurements.is_empty()
            && row.buckets.is_empty()
        {
            let risk = build_governance_risk_row(
                GOVERNANCE_RISK_KIND_EMPTY_QUERY_ANALYSIS,
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                identity.name,
                row.query_field.clone(),
            );
            if seen.insert(risk.clone()) {
                risks.push(risk);
            }
        }
    }
    for dashboard in build_dashboard_dependency_rows(report) {
        if dashboard.panel_count > 30 {
            let risk = build_governance_risk_row(
                GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE,
                dashboard.dashboard_uid.clone(),
                String::new(),
                dashboard.dashboard_title.clone(),
                dashboard.panel_count.to_string(),
            );
            if seen.insert(risk.clone()) {
                risks.push(risk);
            }
        }
        if let Some(refresh) = refresh_by_dashboard.get(&dashboard.dashboard_uid) {
            if let Some(refresh_seconds) = parse_duration_seconds(refresh) {
                if refresh_seconds != 0 && refresh_seconds < 30 {
                    let risk = build_governance_risk_row(
                        GOVERNANCE_RISK_KIND_DASHBOARD_REFRESH_PRESSURE,
                        dashboard.dashboard_uid.clone(),
                        String::new(),
                        dashboard.dashboard_title.clone(),
                        refresh.clone(),
                    );
                    if seen.insert(risk.clone()) {
                        risks.push(risk);
                    }
                }
            }
        }
    }
    risks.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then(left.kind.cmp(&right.kind))
            .then(left.dashboard_uid.cmp(&right.dashboard_uid))
            .then(left.panel_id.cmp(&right.panel_id))
            .then(left.datasource.cmp(&right.datasource))
    });
    risks
}

pub(crate) fn build_query_audit_rows(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> Vec<QueryAuditRow> {
    let (inventory_by_uid, inventory_by_name) = build_inventory_lookup(summary);
    let mut rows = Vec::new();
    for row in &report.queries {
        let identity = resolve_datasource_identity(row, &inventory_by_uid, &inventory_by_name);
        let datasource_family = normalize_family_name(&identity.datasource_type);
        let audit = build_query_audit_row(row, identity.name, identity.uid, datasource_family);
        if audit.score != 0 {
            rows.push(audit);
        }
    }
    rows.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.dashboard_uid.cmp(&right.dashboard_uid))
            .then(left.panel_id.cmp(&right.panel_id))
            .then(left.ref_id.cmp(&right.ref_id))
    });
    rows
}
