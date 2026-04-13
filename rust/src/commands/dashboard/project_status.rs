//! Shared dashboard domain-status producer.
//!
//! Maintainer note:
//! - This module derives one dashboard-owned domain-status row from the staged
//!   inspect summary document.
//! - Keep the producer document-driven and conservative; this module may
//!   surface staged governance warnings from summary fields and the existing
//!   detail arrays, but deeper heuristics should still layer on top of this
//!   base instead of being re-parsed inside overview.

use serde_json::Value;

use crate::project_status::{
    status_finding, ProjectDomainStatus, PROJECT_STATUS_BLOCKED, PROJECT_STATUS_PARTIAL,
    PROJECT_STATUS_READY,
};

const DASHBOARD_DOMAIN_ID: &str = "dashboard";
const DASHBOARD_SCOPE: &str = "staged";
const DASHBOARD_MODE: &str = "inspect-summary";
const DASHBOARD_REASON_READY: &str = PROJECT_STATUS_READY;
const DASHBOARD_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";
const DASHBOARD_REASON_BLOCKED_BY_BLOCKERS: &str = "blocked-by-blockers";

const DASHBOARD_SOURCE_KINDS: &[&str] = &["dashboard-export"];
const DASHBOARD_SIGNAL_KEYS: &[&str] = &[
    "summary.dashboardCount",
    "summary.queryCount",
    "summary.orphanedDatasourceCount",
    "summary.mixedDatasourceDashboardCount",
];
const DASHBOARD_ORPHANED_DATASOURCES_KEY: &str = "orphanedDatasources";
const DASHBOARD_MIXED_DASHBOARDS_KEY: &str = "mixedDatasourceDashboards";
const DASHBOARD_DASHBOARD_GOVERNANCE_KEY: &str = "dashboardGovernance";
const DASHBOARD_DATASOURCE_GOVERNANCE_KEY: &str = "datasourceGovernance";
const DASHBOARD_DASHBOARD_DEPENDENCIES_KEY: &str = "dashboardDependencies";

const DASHBOARD_BLOCKER_ORPHANED_DATASOURCES: &str = "orphaned-datasources";
const DASHBOARD_BLOCKER_MIXED_DASHBOARDS: &str = "mixed-dashboards";
const DASHBOARD_WARNING_RISK_RECORDS: &str = "risk-records";
const DASHBOARD_WARNING_HIGH_BLAST_RADIUS_DATASOURCES: &str = "high-blast-radius-datasources";
const DASHBOARD_WARNING_QUERY_AUDITS: &str = "query-audits";
const DASHBOARD_WARNING_DASHBOARD_AUDITS: &str = "dashboard-audits";
const DASHBOARD_WARNING_IMPORT_READINESS_GAPS: &str = "import-readiness-gaps";
const DASHBOARD_WARNING_IMPORT_READINESS_DETAIL_GAPS: &str = "import-readiness-detail-gaps";

const DASHBOARD_EXPORT_AT_LEAST_ONE_ACTIONS: &[&str] = &["export at least one dashboard"];
const DASHBOARD_REVIEW_GOVERNANCE_WARNINGS_ACTIONS: &[&str] =
    &["review dashboard governance warnings before promotion or apply"];
const DASHBOARD_RESOLVE_BLOCKERS_ACTIONS: &[&str] =
    &["resolve orphaned datasources, then mixed dashboards"];

fn summary_number(document: &Value, key: &str) -> usize {
    document
        .get("summary")
        .and_then(|value| value.get(key))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize
}

fn top_level_array<'a>(document: &'a Value, key: &str) -> Option<&'a [Value]> {
    document
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
}

fn push_warning(
    warnings: &mut Vec<crate::project_status::ProjectStatusFinding>,
    signal_keys: &mut Vec<String>,
    kind: &str,
    count: usize,
    source: &str,
) {
    if count > 0 {
        warnings.push(status_finding(kind, count, source));
        signal_keys.push(source.to_string());
    }
}

fn push_signal_key(signal_keys: &mut Vec<String>, source: &str) {
    if !signal_keys.iter().any(|item| item == source) {
        signal_keys.push(source.to_string());
    }
}

fn count_import_ready_dashboard_dependencies(items: Option<&[Value]>) -> usize {
    items
        .map(|items| {
            items
                .iter()
                .filter(|item| {
                    item.get("file")
                        .and_then(Value::as_str)
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

fn count_import_ready_dashboard_dependency_details(items: Option<&[Value]>) -> usize {
    items
        .map(|items| {
            items
                .iter()
                .filter(|item| {
                    item.get("file")
                        .and_then(Value::as_str)
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false)
                        && item
                            .get("panelIds")
                            .and_then(Value::as_array)
                            .map(|value| !value.is_empty())
                            .unwrap_or(false)
                        && item
                            .get("queryFields")
                            .and_then(Value::as_array)
                            .map(|value| !value.is_empty())
                            .unwrap_or(false)
                        && item
                            .get("datasources")
                            .and_then(Value::as_array)
                            .map(|value| !value.is_empty())
                            .unwrap_or(false)
                        && item
                            .get("datasourceFamilies")
                            .and_then(Value::as_array)
                            .map(|value| !value.is_empty())
                            .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

fn push_detail_signal_keys(
    signal_keys: &mut Vec<String>,
    document_key: &str,
    items: Option<&[Value]>,
    fields: &[&str],
) {
    if let Some(items) = items {
        if !items.is_empty() {
            push_signal_key(signal_keys, document_key);
            for (index, item) in items.iter().enumerate() {
                let source = detail_source_key(document_key, item, index, fields);
                push_signal_key(signal_keys, &source);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_detail_findings_with_count<F>(
    findings: &mut Vec<crate::project_status::ProjectStatusFinding>,
    signal_keys: &mut Vec<String>,
    kind: &str,
    document_key: &str,
    items: Option<&[Value]>,
    fields: &[&str],
    fallback_count: usize,
    fallback_source: &str,
    mut count_for_item: F,
) where
    F: FnMut(&Value) -> usize,
{
    if let Some(items) = items {
        if !items.is_empty() {
            push_signal_key(signal_keys, document_key);
            let mut used_detail_rows = false;
            for (index, item) in items.iter().enumerate() {
                let count = count_for_item(item);
                if count == 0 {
                    continue;
                }
                let source = detail_source_key(document_key, item, index, fields);
                findings.push(status_finding(kind, count, &source));
                push_signal_key(signal_keys, &source);
                used_detail_rows = true;
            }
            if used_detail_rows {
                return;
            }
        }
    }

    if fallback_count > 0 {
        findings.push(status_finding(kind, fallback_count, fallback_source));
        push_signal_key(signal_keys, fallback_source);
    }
}

fn detail_source_key(document_key: &str, item: &Value, index: usize, fields: &[&str]) -> String {
    for field in fields {
        if let Some(value) = item.get(field).and_then(Value::as_str) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return format!("{document_key}.{trimmed}");
            }
        }
    }
    format!("{document_key}[{index}]")
}

#[allow(clippy::too_many_arguments)]
fn push_detail_findings(
    findings: &mut Vec<crate::project_status::ProjectStatusFinding>,
    signal_keys: &mut Vec<String>,
    kind: &str,
    document_key: &str,
    items: Option<&[Value]>,
    fields: &[&str],
    fallback_count: usize,
    fallback_source: &str,
) {
    if let Some(items) = items {
        if !items.is_empty() {
            push_signal_key(signal_keys, document_key);
            for (index, item) in items.iter().enumerate() {
                let source = detail_source_key(document_key, item, index, fields);
                findings.push(status_finding(kind, 1, &source));
                push_signal_key(signal_keys, &source);
            }
            return;
        }
    }

    if fallback_count > 0 {
        findings.push(status_finding(kind, fallback_count, fallback_source));
        push_signal_key(signal_keys, fallback_source);
    }
}

pub(crate) fn build_dashboard_domain_status(
    summary_document: Option<&Value>,
) -> Option<ProjectDomainStatus> {
    let document = summary_document?;
    let dashboards = summary_number(document, "dashboardCount");
    let queries = summary_number(document, "queryCount");
    let orphaned = summary_number(document, "orphanedDatasourceCount");
    let mixed = summary_number(document, "mixedDatasourceDashboardCount");
    let risk_records = summary_number(document, "riskRecordCount");
    let high_blast_radius_datasources = summary_number(document, "highBlastRadiusDatasourceCount");
    let query_audits = summary_number(document, "queryAuditCount");
    let dashboard_audits = summary_number(document, "dashboardAuditCount");
    let orphaned_datasource_items = document
        .get(DASHBOARD_ORPHANED_DATASOURCES_KEY)
        .and_then(Value::as_array)
        .map(Vec::as_slice);
    let mixed_dashboard_items = top_level_array(document, DASHBOARD_MIXED_DASHBOARDS_KEY);
    let dashboard_governance_items = top_level_array(document, DASHBOARD_DASHBOARD_GOVERNANCE_KEY);
    let datasource_governance_items =
        top_level_array(document, DASHBOARD_DATASOURCE_GOVERNANCE_KEY);
    let dashboard_dependency_items =
        top_level_array(document, DASHBOARD_DASHBOARD_DEPENDENCIES_KEY);

    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut signal_keys = DASHBOARD_SIGNAL_KEYS
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<String>>();
    push_detail_findings(
        &mut blockers,
        &mut signal_keys,
        DASHBOARD_BLOCKER_ORPHANED_DATASOURCES,
        DASHBOARD_ORPHANED_DATASOURCES_KEY,
        orphaned_datasource_items,
        &["uid", "name"],
        orphaned,
        "summary.orphanedDatasourceCount",
    );
    push_detail_findings(
        &mut blockers,
        &mut signal_keys,
        DASHBOARD_BLOCKER_MIXED_DASHBOARDS,
        DASHBOARD_MIXED_DASHBOARDS_KEY,
        mixed_dashboard_items,
        &["uid", "title"],
        mixed,
        "summary.mixedDatasourceDashboardCount",
    );
    push_detail_findings_with_count(
        &mut warnings,
        &mut signal_keys,
        DASHBOARD_WARNING_RISK_RECORDS,
        DASHBOARD_DASHBOARD_GOVERNANCE_KEY,
        dashboard_governance_items,
        &["dashboardUid", "dashboardTitle"],
        risk_records,
        "summary.riskRecordCount",
        |item| item.get("riskCount").and_then(Value::as_u64).unwrap_or(0) as usize,
    );
    push_detail_findings_with_count(
        &mut warnings,
        &mut signal_keys,
        DASHBOARD_WARNING_HIGH_BLAST_RADIUS_DATASOURCES,
        DASHBOARD_DATASOURCE_GOVERNANCE_KEY,
        datasource_governance_items,
        &["datasourceUid", "datasource"],
        high_blast_radius_datasources,
        "summary.highBlastRadiusDatasourceCount",
        |item| {
            item.get("highBlastRadius")
                .and_then(Value::as_bool)
                .unwrap_or(false) as usize
        },
    );
    push_warning(
        &mut warnings,
        &mut signal_keys,
        DASHBOARD_WARNING_QUERY_AUDITS,
        query_audits,
        "summary.queryAuditCount",
    );
    push_warning(
        &mut warnings,
        &mut signal_keys,
        DASHBOARD_WARNING_DASHBOARD_AUDITS,
        dashboard_audits,
        "summary.dashboardAuditCount",
    );
    if dashboard_dependency_items.is_some() {
        let import_ready_dependency_count =
            count_import_ready_dashboard_dependencies(dashboard_dependency_items);
        let import_readiness_gap = dashboards.saturating_sub(import_ready_dependency_count);
        if import_readiness_gap > 0 {
            warnings.push(status_finding(
                DASHBOARD_WARNING_IMPORT_READINESS_GAPS,
                import_readiness_gap,
                DASHBOARD_DASHBOARD_DEPENDENCIES_KEY,
            ));
        }

        let import_ready_detail_count =
            count_import_ready_dashboard_dependency_details(dashboard_dependency_items);
        let import_readiness_detail_gap =
            import_ready_dependency_count.saturating_sub(import_ready_detail_count);
        if import_readiness_detail_gap > 0 {
            warnings.push(status_finding(
                DASHBOARD_WARNING_IMPORT_READINESS_DETAIL_GAPS,
                import_readiness_detail_gap,
                DASHBOARD_DASHBOARD_DEPENDENCIES_KEY,
            ));
        }
    }

    let (status, reason_code, mut next_actions) = if !blockers.is_empty() {
        (
            PROJECT_STATUS_BLOCKED,
            DASHBOARD_REASON_BLOCKED_BY_BLOCKERS,
            DASHBOARD_RESOLVE_BLOCKERS_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>(),
        )
    } else if dashboards == 0 {
        (
            PROJECT_STATUS_PARTIAL,
            DASHBOARD_REASON_PARTIAL_NO_DATA,
            DASHBOARD_EXPORT_AT_LEAST_ONE_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>(),
        )
    } else {
        (PROJECT_STATUS_READY, DASHBOARD_REASON_READY, Vec::new())
    };
    if !warnings.is_empty() && blockers.is_empty() {
        next_actions.extend(
            DASHBOARD_REVIEW_GOVERNANCE_WARNINGS_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }

    push_detail_signal_keys(
        &mut signal_keys,
        DASHBOARD_DASHBOARD_DEPENDENCIES_KEY,
        dashboard_dependency_items,
        &["dashboardUid", "dashboardTitle"],
    );

    Some(ProjectDomainStatus {
        id: DASHBOARD_DOMAIN_ID.to_string(),
        scope: DASHBOARD_SCOPE.to_string(),
        mode: DASHBOARD_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: dashboards.max(queries),
        blocker_count: blockers.iter().map(|item| item.count).sum(),
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds: DASHBOARD_SOURCE_KINDS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        signal_keys,
        blockers,
        warnings,
        next_actions,
        freshness: Default::default(),
    })
}

#[cfg(test)]
mod dashboard_project_status_rust_tests {
    use super::build_dashboard_domain_status;
    use crate::project_status::{status_finding, PROJECT_STATUS_BLOCKED};
    use serde_json::json;

    #[test]
    fn build_dashboard_domain_status_uses_detail_rows_for_blockers_when_available() {
        let document = json!({
            "summary": {
                "dashboardCount": 2,
                "queryCount": 5,
                "orphanedDatasourceCount": 2,
                "mixedDatasourceDashboardCount": 2,
            },
            "orphanedDatasources": [
                {
                    "uid": "unused-main",
                    "name": "Unused Main",
                },
                {
                    "uid": "unused-ops",
                    "name": "Unused Ops",
                }
            ],
            "mixedDatasourceDashboards": [
                {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                },
                {
                    "uid": "logs-main",
                    "title": "Logs Main",
                }
            ]
        });

        let domain = build_dashboard_domain_status(Some(&document)).unwrap();

        assert_eq!(domain.status, PROJECT_STATUS_BLOCKED);
        assert_eq!(domain.reason_code, "blocked-by-blockers");
        assert_eq!(domain.primary_count, 5);
        assert_eq!(domain.blocker_count, 4);
        assert_eq!(domain.warning_count, 0);
        assert_eq!(
            domain.signal_keys,
            vec![
                "summary.dashboardCount".to_string(),
                "summary.queryCount".to_string(),
                "summary.orphanedDatasourceCount".to_string(),
                "summary.mixedDatasourceDashboardCount".to_string(),
                "orphanedDatasources".to_string(),
                "orphanedDatasources.unused-main".to_string(),
                "orphanedDatasources.unused-ops".to_string(),
                "mixedDatasourceDashboards".to_string(),
                "mixedDatasourceDashboards.cpu-main".to_string(),
                "mixedDatasourceDashboards.logs-main".to_string(),
            ]
        );
        assert_eq!(
            domain.blockers,
            vec![
                status_finding("orphaned-datasources", 1, "orphanedDatasources.unused-main",),
                status_finding("orphaned-datasources", 1, "orphanedDatasources.unused-ops",),
                status_finding("mixed-dashboards", 1, "mixedDatasourceDashboards.cpu-main"),
                status_finding("mixed-dashboards", 1, "mixedDatasourceDashboards.logs-main"),
            ]
        );
        assert_eq!(
            domain.next_actions,
            vec!["resolve orphaned datasources, then mixed dashboards".to_string()]
        );
    }

    #[test]
    fn build_dashboard_domain_status_prefers_detail_governance_rows_when_available() {
        let document = json!({
            "summary": {
                "dashboardCount": 2,
                "queryCount": 5,
                "orphanedDatasourceCount": 0,
                "mixedDatasourceDashboardCount": 0,
                "riskRecordCount": 0,
                "highBlastRadiusDatasourceCount": 0,
                "queryAuditCount": 0,
                "dashboardAuditCount": 0,
            },
            "dashboardGovernance": [
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "folderPath": "General",
                    "panelCount": 3,
                    "queryCount": 4,
                    "datasourceCount": 2,
                    "datasourceFamilyCount": 2,
                    "datasources": ["Prometheus Main", "Loki Main"],
                    "datasourceFamilies": ["prometheus", "loki"],
                    "mixedDatasource": true,
                    "riskCount": 2,
                    "riskKinds": ["mixed-datasource-dashboard", "dashboard-panel-pressure"],
                },
                {
                    "dashboardUid": "ops-main",
                    "dashboardTitle": "Ops Main",
                    "folderPath": "Platform",
                    "panelCount": 2,
                    "queryCount": 1,
                    "datasourceCount": 1,
                    "datasourceFamilyCount": 1,
                    "datasources": ["Prometheus Main"],
                    "datasourceFamilies": ["prometheus"],
                    "mixedDatasource": false,
                    "riskCount": 1,
                    "riskKinds": ["empty-query-analysis"],
                }
            ],
            "datasourceGovernance": [
                {
                    "datasourceUid": "prom-main",
                    "datasource": "Prometheus Main",
                    "family": "prometheus",
                    "queryCount": 4,
                    "dashboardCount": 2,
                    "panelCount": 5,
                    "mixedDashboardCount": 1,
                    "riskCount": 1,
                    "riskKinds": ["datasource-high-blast-radius", "mixed-datasource-dashboard"],
                    "folderCount": 2,
                    "highBlastRadius": true,
                    "crossFolder": true,
                    "folderPaths": ["General", "Platform"],
                    "dashboardUids": ["core-main", "ops-main"],
                    "dashboardTitles": ["Core Main", "Ops Main"],
                    "orphaned": false,
                }
            ],
            "dashboardDependencies": [
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "folderPath": "General",
                    "file": "/tmp/raw/core-main.json",
                    "panelCount": 3,
                    "queryCount": 4,
                    "datasourceCount": 2,
                    "datasourceFamilyCount": 2,
                    "panelIds": ["1", "2", "3"],
                    "datasources": ["Prometheus Main", "Loki Main"],
                    "datasourceFamilies": ["prometheus", "loki"],
                    "queryFields": ["expr", "query"],
                    "panelVariables": [],
                    "queryVariables": [],
                    "metrics": ["http_requests_total"],
                    "functions": ["rate"],
                    "measurements": ["job=\"grafana\""],
                    "buckets": ["5m"],
                }
            ]
        });

        let domain = build_dashboard_domain_status(Some(&document)).unwrap();

        assert_eq!(domain.status, "ready");
        assert_eq!(domain.reason_code, "ready");
        assert_eq!(domain.primary_count, 5);
        assert_eq!(domain.blocker_count, 0);
        assert_eq!(domain.warning_count, 5);
        assert_eq!(
            domain.signal_keys,
            vec![
                "summary.dashboardCount".to_string(),
                "summary.queryCount".to_string(),
                "summary.orphanedDatasourceCount".to_string(),
                "summary.mixedDatasourceDashboardCount".to_string(),
                "dashboardGovernance".to_string(),
                "dashboardGovernance.core-main".to_string(),
                "dashboardGovernance.ops-main".to_string(),
                "datasourceGovernance".to_string(),
                "datasourceGovernance.prom-main".to_string(),
                "dashboardDependencies".to_string(),
                "dashboardDependencies.core-main".to_string(),
            ]
        );
        assert_eq!(
            domain.warnings,
            vec![
                status_finding("risk-records", 2, "dashboardGovernance.core-main"),
                status_finding("risk-records", 1, "dashboardGovernance.ops-main"),
                status_finding(
                    "high-blast-radius-datasources",
                    1,
                    "datasourceGovernance.prom-main"
                ),
                status_finding("import-readiness-gaps", 1, "dashboardDependencies"),
            ]
        );
        assert_eq!(
            domain.next_actions,
            vec!["review dashboard governance warnings before promotion or apply".to_string()]
        );
    }

    #[test]
    fn build_dashboard_domain_status_reports_import_readiness_gaps_from_dependency_coverage() {
        let document = json!({
            "summary": {
                "dashboardCount": 2,
                "queryCount": 5,
                "orphanedDatasourceCount": 0,
                "mixedDatasourceDashboardCount": 0,
                "riskRecordCount": 0,
                "highBlastRadiusDatasourceCount": 0,
                "queryAuditCount": 0,
                "dashboardAuditCount": 0,
            },
            "dashboardDependencies": [
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "folderPath": "General",
                    "file": "/tmp/raw/core-main.json",
                    "panelCount": 3,
                    "queryCount": 4,
                    "datasourceCount": 2,
                    "datasourceFamilyCount": 2,
                    "panelIds": ["1", "2", "3"],
                    "datasources": ["Prometheus Main", "Loki Main"],
                    "datasourceFamilies": ["prometheus", "loki"],
                    "queryFields": ["expr", "query"],
                    "panelVariables": [],
                    "queryVariables": [],
                    "metrics": ["http_requests_total"],
                    "functions": ["rate"],
                    "measurements": ["job=\"grafana\""],
                    "buckets": ["5m"],
                }
            ]
        });

        let domain = build_dashboard_domain_status(Some(&document)).unwrap();

        assert_eq!(domain.status, "ready");
        assert_eq!(domain.reason_code, "ready");
        assert_eq!(domain.primary_count, 5);
        assert_eq!(domain.blocker_count, 0);
        assert_eq!(domain.warning_count, 1);
        assert_eq!(
            domain.warnings,
            vec![status_finding(
                "import-readiness-gaps",
                1,
                "dashboardDependencies"
            )]
        );
        assert_eq!(
            domain.signal_keys,
            vec![
                "summary.dashboardCount".to_string(),
                "summary.queryCount".to_string(),
                "summary.orphanedDatasourceCount".to_string(),
                "summary.mixedDatasourceDashboardCount".to_string(),
                "dashboardDependencies".to_string(),
                "dashboardDependencies.core-main".to_string(),
            ]
        );
        assert_eq!(
            domain.next_actions,
            vec!["review dashboard governance warnings before promotion or apply".to_string()]
        );
    }

    #[test]
    fn build_dashboard_domain_status_reports_import_readiness_detail_gaps_from_richer_dependency_rows(
    ) {
        let document = json!({
            "summary": {
                "dashboardCount": 2,
                "queryCount": 5,
                "orphanedDatasourceCount": 0,
                "mixedDatasourceDashboardCount": 0,
                "riskRecordCount": 0,
                "highBlastRadiusDatasourceCount": 0,
                "queryAuditCount": 0,
                "dashboardAuditCount": 0,
            },
            "dashboardDependencies": [
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "folderPath": "General",
                    "file": "/tmp/raw/core-main.json",
                    "panelCount": 3,
                    "queryCount": 4,
                    "datasourceCount": 2,
                    "datasourceFamilyCount": 2,
                    "panelIds": ["1", "2", "3"],
                    "datasources": ["Prometheus Main", "Loki Main"],
                    "datasourceFamilies": ["prometheus", "loki"],
                    "queryFields": ["expr", "query"],
                    "panelVariables": [],
                    "queryVariables": [],
                    "metrics": ["http_requests_total"],
                    "functions": ["rate"],
                    "measurements": ["job=\"grafana\""],
                    "buckets": ["5m"],
                },
                {
                    "dashboardUid": "ops-main",
                    "dashboardTitle": "Ops Main",
                    "folderPath": "Platform",
                    "file": "/tmp/raw/ops-main.json",
                    "panelCount": 2,
                    "queryCount": 1,
                    "datasourceCount": 1,
                    "datasourceFamilyCount": 1,
                    "panelIds": [],
                    "datasources": ["Prometheus Main"],
                    "datasourceFamilies": ["prometheus"],
                    "queryFields": [],
                    "panelVariables": [],
                    "queryVariables": [],
                    "metrics": ["up"],
                    "functions": ["rate"],
                    "measurements": ["service.name"],
                    "buckets": ["5m"],
                }
            ]
        });

        let domain = build_dashboard_domain_status(Some(&document)).unwrap();

        assert_eq!(domain.status, "ready");
        assert_eq!(domain.reason_code, "ready");
        assert_eq!(domain.primary_count, 5);
        assert_eq!(domain.blocker_count, 0);
        assert_eq!(domain.warning_count, 1);
        assert_eq!(
            domain.warnings,
            vec![status_finding(
                "import-readiness-detail-gaps",
                1,
                "dashboardDependencies"
            )]
        );
        assert_eq!(
            domain.signal_keys,
            vec![
                "summary.dashboardCount".to_string(),
                "summary.queryCount".to_string(),
                "summary.orphanedDatasourceCount".to_string(),
                "summary.mixedDatasourceDashboardCount".to_string(),
                "dashboardDependencies".to_string(),
                "dashboardDependencies.core-main".to_string(),
                "dashboardDependencies.ops-main".to_string(),
            ]
        );
        assert_eq!(
            domain.next_actions,
            vec!["review dashboard governance warnings before promotion or apply".to_string()]
        );
    }

    #[test]
    fn build_dashboard_domain_status_reports_import_readiness_detail_gaps_when_datasource_coverage_is_missing(
    ) {
        let document = json!({
            "summary": {
                "dashboardCount": 1,
                "queryCount": 4,
                "orphanedDatasourceCount": 0,
                "mixedDatasourceDashboardCount": 0,
                "riskRecordCount": 0,
                "highBlastRadiusDatasourceCount": 0,
                "queryAuditCount": 0,
                "dashboardAuditCount": 0,
            },
            "dashboardDependencies": [
                {
                    "dashboardUid": "core-main",
                    "dashboardTitle": "Core Main",
                    "folderPath": "General",
                    "file": "/tmp/raw/core-main.json",
                    "panelCount": 3,
                    "queryCount": 4,
                    "datasourceCount": 2,
                    "datasourceFamilyCount": 2,
                    "panelIds": ["1", "2", "3"],
                    "datasources": [],
                    "datasourceFamilies": [],
                    "queryFields": ["expr", "query"],
                    "panelVariables": [],
                    "queryVariables": [],
                    "metrics": ["http_requests_total"],
                    "functions": ["rate"],
                    "measurements": ["job=\"grafana\""],
                    "buckets": ["5m"],
                }
            ]
        });

        let domain = build_dashboard_domain_status(Some(&document)).unwrap();

        assert_eq!(domain.status, "ready");
        assert_eq!(domain.reason_code, "ready");
        assert_eq!(domain.primary_count, 4);
        assert_eq!(domain.blocker_count, 0);
        assert_eq!(domain.warning_count, 1);
        assert_eq!(
            domain.warnings,
            vec![status_finding(
                "import-readiness-detail-gaps",
                1,
                "dashboardDependencies"
            )]
        );
        assert_eq!(
            domain.signal_keys,
            vec![
                "summary.dashboardCount".to_string(),
                "summary.queryCount".to_string(),
                "summary.orphanedDatasourceCount".to_string(),
                "summary.mixedDatasourceDashboardCount".to_string(),
                "dashboardDependencies".to_string(),
                "dashboardDependencies.core-main".to_string(),
            ]
        );
    }

    #[test]
    fn build_dashboard_domain_status_falls_back_to_summary_counts_without_detail_rows() {
        let document = json!({
            "summary": {
                "dashboardCount": 2,
                "queryCount": 5,
                "orphanedDatasourceCount": 1,
                "mixedDatasourceDashboardCount": 2,
                "riskRecordCount": 2,
                "highBlastRadiusDatasourceCount": 1,
                "queryAuditCount": 3,
                "dashboardAuditCount": 1,
            }
        });

        let domain = build_dashboard_domain_status(Some(&document)).unwrap();

        assert_eq!(domain.status, PROJECT_STATUS_BLOCKED);
        assert_eq!(domain.reason_code, "blocked-by-blockers");
        assert_eq!(domain.blocker_count, 3);
        assert_eq!(domain.warning_count, 7);
        assert_eq!(
            domain.blockers,
            vec![
                status_finding("orphaned-datasources", 1, "summary.orphanedDatasourceCount",),
                status_finding(
                    "mixed-dashboards",
                    2,
                    "summary.mixedDatasourceDashboardCount"
                ),
            ]
        );
        assert_eq!(
            domain.signal_keys,
            vec![
                "summary.dashboardCount".to_string(),
                "summary.queryCount".to_string(),
                "summary.orphanedDatasourceCount".to_string(),
                "summary.mixedDatasourceDashboardCount".to_string(),
                "summary.riskRecordCount".to_string(),
                "summary.highBlastRadiusDatasourceCount".to_string(),
                "summary.queryAuditCount".to_string(),
                "summary.dashboardAuditCount".to_string(),
            ]
        );
    }
}
