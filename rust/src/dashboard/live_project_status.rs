//! Live dashboard domain-status producer.
//!
//! Maintainer note:
//! - This module derives one dashboard-owned domain-status row directly from
//!   live dashboard read surfaces.
//! - Keep the producer conservative and cheap: dashboard summaries and
//!   datasource inventory are enough for readiness and drift signals, and the
//!   dashboard search summaries already carry the folder/title metadata needed
//!   for conservative import-readiness evidence without adding more live
//!   requests.

use std::collections::BTreeSet;

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{string_field, Result};
use crate::project_status::{
    status_finding, ProjectDomainStatus, ProjectStatusFinding, PROJECT_STATUS_PARTIAL,
    PROJECT_STATUS_READY,
};

use super::{
    list_dashboard_summaries_with_request, list_datasources_with_request, DEFAULT_PAGE_SIZE,
};

const DASHBOARD_DOMAIN_ID: &str = "dashboard";
const DASHBOARD_SCOPE: &str = "live";
const DASHBOARD_MODE: &str = "live-dashboard-read";
const DASHBOARD_REASON_READY: &str = PROJECT_STATUS_READY;
const DASHBOARD_REASON_PARTIAL_NO_DATA: &str = "partial-no-data";
const DASHBOARD_REASON_PARTIAL_NO_DATASOURCES: &str = "partial-no-datasources";

const DASHBOARD_SOURCE_KINDS: &[&str] = &["live-dashboard-search", "live-datasource-list"];
const DASHBOARD_SIGNAL_KEYS: &[&str] = &[
    "live.dashboardCount",
    "live.datasourceCount",
    "live.folderCount",
    "live.dashboardTitleGapCount",
    "live.folderTitleGapCount",
    "live.importReadyDashboardCount",
];
const DASHBOARD_DATASOURCE_EMPTY_SIGNAL_KEY: &str = "live.datasourceInventoryEmpty";
const DASHBOARD_DATASOURCE_DRIFT_SIGNAL_KEY: &str = "live.datasourceDriftCount";
const DASHBOARD_FOLDER_SPREAD_SIGNAL_KEY: &str = "live.folderSpreadCount";
const DASHBOARD_ROOT_SCOPE_SIGNAL_KEY: &str = "live.rootDashboardCount";
const DASHBOARD_TITLE_GAP_SIGNAL_KEY: &str = "live.dashboardTitleGapCount";
const DASHBOARD_FOLDER_TITLE_GAP_SIGNAL_KEY: &str = "live.folderTitleGapCount";
const DASHBOARD_IMPORT_READY_SIGNAL_KEY: &str = "live.importReadyDashboardCount";

const DASHBOARD_READY_NEXT_ACTIONS: &[&str] =
    &["re-run live dashboard read after dashboard, folder, or datasource changes"];
const DASHBOARD_NO_DATA_NEXT_ACTIONS: &[&str] =
    &["create or import at least one dashboard, then re-run live dashboard read"];
const DASHBOARD_NO_DATASOURCES_NEXT_ACTIONS: &[&str] =
    &["create or import at least one datasource, then re-run live dashboard read"];
const DASHBOARD_REVIEW_WARNING_NEXT_ACTIONS: &[&str] = &[
    "review live dashboard governance and import-readiness warnings before re-running live dashboard read",
];
const DASHBOARD_WARNING_KIND_EMPTY_DATASOURCE_INVENTORY: &str = "live-datasource-inventory-empty";
const DASHBOARD_WARNING_KIND_DATASOURCE_DRIFT: &str = "live-datasource-inventory-drift";
const DASHBOARD_WARNING_KIND_FOLDER_SPREAD: &str = "live-dashboard-folder-spread";
const DASHBOARD_WARNING_KIND_ROOT_SCOPE: &str = "live-dashboard-root-scope";
const DASHBOARD_WARNING_KIND_TITLE_GAP: &str = "live-dashboard-title-gap";
const DASHBOARD_WARNING_KIND_FOLDER_TITLE_GAP: &str = "live-dashboard-folder-title-gap";

fn count_unique_dashboard_uids(dashboard_summaries: &[Map<String, Value>]) -> usize {
    let mut seen = BTreeSet::new();
    for summary in dashboard_summaries {
        let uid = string_field(summary, "uid", "");
        if !uid.is_empty() {
            seen.insert(uid);
        }
    }
    seen.len()
}

fn count_unique_folder_uids(dashboard_summaries: &[Map<String, Value>]) -> usize {
    let mut seen = BTreeSet::new();
    for summary in dashboard_summaries {
        let folder_uid = string_field(summary, "folderUid", "");
        if !folder_uid.is_empty() {
            seen.insert(folder_uid);
        }
    }
    seen.len()
}

fn count_root_scoped_dashboards(dashboard_summaries: &[Map<String, Value>]) -> usize {
    dashboard_summaries
        .iter()
        .filter(|summary| string_field(summary, "folderUid", "").is_empty())
        .count()
}

fn count_dashboard_title_gaps(dashboard_summaries: &[Map<String, Value>]) -> usize {
    dashboard_summaries
        .iter()
        .filter(|summary| string_field(summary, "title", "").trim().is_empty())
        .count()
}

fn count_folder_title_gaps(dashboard_summaries: &[Map<String, Value>]) -> usize {
    dashboard_summaries
        .iter()
        .filter(|summary| {
            !string_field(summary, "folderUid", "").trim().is_empty()
                && string_field(summary, "folderTitle", "").trim().is_empty()
        })
        .count()
}

fn count_import_ready_dashboards(dashboard_summaries: &[Map<String, Value>]) -> usize {
    let mut seen = BTreeSet::new();
    for summary in dashboard_summaries {
        let uid = string_field(summary, "uid", "");
        if uid.is_empty() {
            continue;
        }

        let title = string_field(summary, "title", "");
        let folder_uid = string_field(summary, "folderUid", "");
        let folder_title = string_field(summary, "folderTitle", "");
        let folder_is_ready = folder_uid.trim().is_empty() || !folder_title.trim().is_empty();
        if !title.trim().is_empty() && folder_is_ready {
            seen.insert(uid);
        }
    }
    seen.len()
}

fn count_unique_datasource_uids(datasources: &[Map<String, Value>]) -> usize {
    let mut seen = BTreeSet::new();
    for datasource in datasources {
        let uid = string_field(datasource, "uid", "");
        if !uid.is_empty() {
            seen.insert(uid);
        }
    }
    seen.len()
}

fn push_warning(
    warnings: &mut Vec<ProjectStatusFinding>,
    signal_keys: &mut Vec<String>,
    kind: &str,
    count: usize,
    source: &str,
) {
    if count == 0 {
        return;
    }
    warnings.push(status_finding(kind, count, source));
    if !signal_keys.iter().any(|item| item == source) {
        signal_keys.push(source.to_string());
    }
}

#[allow(clippy::too_many_arguments)]
fn build_live_dashboard_warnings(
    dashboard_count: usize,
    folder_count: usize,
    root_scoped_dashboard_count: usize,
    dashboard_title_gap_count: usize,
    folder_title_gap_count: usize,
    import_ready_dashboard_count: usize,
    datasource_count: usize,
    raw_datasource_count: usize,
    signal_keys: &mut Vec<String>,
) -> Vec<ProjectStatusFinding> {
    let mut warnings = Vec::new();
    if dashboard_count > 0 && datasource_count == 0 {
        push_warning(
            &mut warnings,
            signal_keys,
            DASHBOARD_WARNING_KIND_EMPTY_DATASOURCE_INVENTORY,
            1,
            DASHBOARD_DATASOURCE_EMPTY_SIGNAL_KEY,
        );
        return warnings;
    }

    let drift_count = raw_datasource_count.saturating_sub(datasource_count);
    push_warning(
        &mut warnings,
        signal_keys,
        DASHBOARD_WARNING_KIND_DATASOURCE_DRIFT,
        drift_count,
        DASHBOARD_DATASOURCE_DRIFT_SIGNAL_KEY,
    );
    push_warning(
        &mut warnings,
        signal_keys,
        DASHBOARD_WARNING_KIND_FOLDER_SPREAD,
        folder_count.saturating_sub(1),
        DASHBOARD_FOLDER_SPREAD_SIGNAL_KEY,
    );
    if folder_count > 0 {
        push_warning(
            &mut warnings,
            signal_keys,
            DASHBOARD_WARNING_KIND_ROOT_SCOPE,
            root_scoped_dashboard_count,
            DASHBOARD_ROOT_SCOPE_SIGNAL_KEY,
        );
    }
    push_warning(
        &mut warnings,
        signal_keys,
        DASHBOARD_WARNING_KIND_TITLE_GAP,
        dashboard_title_gap_count,
        DASHBOARD_TITLE_GAP_SIGNAL_KEY,
    );
    push_warning(
        &mut warnings,
        signal_keys,
        DASHBOARD_WARNING_KIND_FOLDER_TITLE_GAP,
        folder_title_gap_count,
        DASHBOARD_FOLDER_TITLE_GAP_SIGNAL_KEY,
    );
    push_warning(
        &mut warnings,
        signal_keys,
        "live-dashboard-import-ready-gap",
        dashboard_count.saturating_sub(import_ready_dashboard_count),
        DASHBOARD_IMPORT_READY_SIGNAL_KEY,
    );
    warnings
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LiveDashboardProjectStatusInputs {
    pub dashboard_summaries: Vec<Map<String, Value>>,
    pub datasources: Vec<Map<String, Value>>,
}

pub(crate) fn collect_live_dashboard_project_status_inputs_with_request<F>(
    request_json: &mut F,
) -> Result<LiveDashboardProjectStatusInputs>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let dashboard_summaries =
        list_dashboard_summaries_with_request(&mut *request_json, DEFAULT_PAGE_SIZE)?;
    let datasources = list_datasources_with_request(&mut *request_json)?;
    Ok(LiveDashboardProjectStatusInputs {
        dashboard_summaries,
        datasources,
    })
}

pub(crate) fn build_live_dashboard_domain_status(
    dashboard_summaries: &[Map<String, Value>],
    datasources: &[Map<String, Value>],
) -> ProjectDomainStatus {
    let dashboard_count = count_unique_dashboard_uids(dashboard_summaries);
    let root_scoped_dashboard_count = count_root_scoped_dashboards(dashboard_summaries);
    let folder_count = count_unique_folder_uids(dashboard_summaries);
    let dashboard_title_gap_count = count_dashboard_title_gaps(dashboard_summaries);
    let folder_title_gap_count = count_folder_title_gaps(dashboard_summaries);
    let import_ready_dashboard_count = count_import_ready_dashboards(dashboard_summaries);
    let datasource_count = count_unique_datasource_uids(datasources);
    let raw_datasource_count = datasources.len();

    let mut signal_keys = DASHBOARD_SIGNAL_KEYS
        .iter()
        .map(|item| (*item).to_string())
        .collect::<Vec<String>>();
    let warnings = build_live_dashboard_warnings(
        dashboard_count,
        folder_count,
        root_scoped_dashboard_count,
        dashboard_title_gap_count,
        folder_title_gap_count,
        import_ready_dashboard_count,
        datasource_count,
        raw_datasource_count,
        &mut signal_keys,
    );

    let (status, reason_code, mut next_actions) = if dashboard_count == 0 {
        (
            PROJECT_STATUS_PARTIAL,
            DASHBOARD_REASON_PARTIAL_NO_DATA,
            DASHBOARD_NO_DATA_NEXT_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>(),
        )
    } else if datasource_count == 0 {
        (
            PROJECT_STATUS_PARTIAL,
            DASHBOARD_REASON_PARTIAL_NO_DATASOURCES,
            DASHBOARD_NO_DATASOURCES_NEXT_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>(),
        )
    } else {
        (
            PROJECT_STATUS_READY,
            DASHBOARD_REASON_READY,
            DASHBOARD_READY_NEXT_ACTIONS
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<String>>(),
        )
    };
    if !warnings.is_empty() && status == PROJECT_STATUS_READY {
        next_actions.extend(
            DASHBOARD_REVIEW_WARNING_NEXT_ACTIONS
                .iter()
                .map(|item| (*item).to_string()),
        );
    }

    ProjectDomainStatus {
        id: DASHBOARD_DOMAIN_ID.to_string(),
        scope: DASHBOARD_SCOPE.to_string(),
        mode: DASHBOARD_MODE.to_string(),
        status: status.to_string(),
        reason_code: reason_code.to_string(),
        primary_count: dashboard_count,
        blocker_count: 0,
        warning_count: warnings.iter().map(|item| item.count).sum(),
        source_kinds: DASHBOARD_SOURCE_KINDS
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        signal_keys,
        blockers: Vec::new(),
        warnings,
        next_actions,
        freshness: Default::default(),
    }
}

pub(crate) fn build_live_dashboard_domain_status_from_inputs(
    inputs: &LiveDashboardProjectStatusInputs,
) -> ProjectDomainStatus {
    build_live_dashboard_domain_status(&inputs.dashboard_summaries, &inputs.datasources)
}

#[allow(dead_code)]
pub(crate) fn build_live_dashboard_domain_status_with_request<F>(
    request_json: F,
) -> Result<ProjectDomainStatus>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut request_json = request_json;
    let inputs = collect_live_dashboard_project_status_inputs_with_request(&mut request_json)?;
    Ok(build_live_dashboard_domain_status_from_inputs(&inputs))
}

#[cfg(test)]
mod live_project_status_rust_tests {
    use super::build_live_dashboard_domain_status_with_request;
    use super::{
        build_live_dashboard_domain_status_from_inputs,
        collect_live_dashboard_project_status_inputs_with_request,
    };
    use crate::project_status::{status_finding, PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY};
    use serde_json::json;
    use serde_json::Value;

    type TestRequestResult = crate::common::Result<Option<Value>>;

    fn request_fixture(
    ) -> impl FnMut(reqwest::Method, &str, &[(String, String)], Option<&Value>) -> TestRequestResult
    {
        move |method, path, _params, _payload| {
            let method_name = method.to_string();
            match (method, path) {
                (reqwest::Method::GET, "/api/search") => Ok(Some(json!([
                    {
                        "uid": "cpu-main",
                        "title": "CPU Main",
                        "type": "dash-db",
                        "folderUid": "infra",
                        "folderTitle": "Infra"
                    },
                    {
                        "uid": "logs-main",
                        "title": "Logs Main",
                        "type": "dash-db",
                        "folderUid": "platform",
                        "folderTitle": "Platform"
                    },
                    {
                        "uid": "cpu-main",
                        "title": "CPU Main",
                        "type": "dash-db",
                        "folderUid": "infra",
                        "folderTitle": "Infra"
                    }
                ]))),
                (reqwest::Method::GET, "/api/datasources") => Ok(Some(json!([
                    {"uid": "prom-main", "name": "Prometheus Main"},
                    {"uid": "loki-main", "name": "Loki Main"},
                    {"uid": "tempo-main", "name": "Tempo Main"}
                ]))),
                _ => Err(crate::common::message(format!(
                    "unexpected request {:?} {path}",
                    method_name
                ))),
            }
        }
    }

    #[test]
    fn build_live_dashboard_domain_status_tracks_live_summary_and_datasource_reads() {
        let mut request = request_fixture();
        let inputs =
            collect_live_dashboard_project_status_inputs_with_request(&mut request).unwrap();
        let domain = build_live_dashboard_domain_status_from_inputs(&inputs);

        assert_eq!(domain.id, "dashboard");
        assert_eq!(domain.scope, "live");
        assert_eq!(domain.mode, "live-dashboard-read");
        assert_eq!(domain.status, PROJECT_STATUS_READY);
        assert_eq!(domain.reason_code, "ready");
        assert_eq!(domain.primary_count, 2);
        assert_eq!(domain.blocker_count, 0);
        assert_eq!(domain.warning_count, 1);
        assert_eq!(
            domain.source_kinds,
            vec![
                "live-dashboard-search".to_string(),
                "live-datasource-list".to_string(),
            ]
        );
        assert_eq!(
            domain.signal_keys,
            vec![
                "live.dashboardCount".to_string(),
                "live.datasourceCount".to_string(),
                "live.folderCount".to_string(),
                "live.dashboardTitleGapCount".to_string(),
                "live.folderTitleGapCount".to_string(),
                "live.importReadyDashboardCount".to_string(),
                "live.folderSpreadCount".to_string(),
            ]
        );
        assert!(domain.blockers.is_empty());
        assert_eq!(
            domain.warnings,
            vec![status_finding(
                "live-dashboard-folder-spread",
                1,
                "live.folderSpreadCount",
            )]
        );
        assert_eq!(
            domain.next_actions,
            vec![
                "re-run live dashboard read after dashboard, folder, or datasource changes"
                    .to_string(),
                "review live dashboard governance and import-readiness warnings before re-running live dashboard read"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn collect_live_dashboard_project_status_inputs_with_request_reads_dashboard_and_datasource_surfaces(
    ) {
        let mut request = request_fixture();
        let inputs =
            collect_live_dashboard_project_status_inputs_with_request(&mut request).unwrap();

        assert_eq!(inputs.dashboard_summaries.len(), 2);
        assert_eq!(inputs.datasources.len(), 3);
        assert_eq!(
            inputs
                .dashboard_summaries
                .first()
                .and_then(|summary| summary.get("uid"))
                .and_then(Value::as_str),
            Some("cpu-main")
        );
        assert_eq!(
            inputs
                .datasources
                .first()
                .and_then(|datasource| datasource.get("uid"))
                .and_then(Value::as_str),
            Some("prom-main")
        );
    }

    #[test]
    fn build_live_dashboard_domain_status_reports_root_scope_warning_for_general_dashboards() {
        let domain = build_live_dashboard_domain_status_with_request(
            move |method, path, _params, _payload| {
                let method_name = method.to_string();
                match (method, path) {
                    (reqwest::Method::GET, "/api/search") => Ok(Some(json!([
                        {
                            "uid": "cpu-main",
                            "title": "CPU Main",
                            "type": "dash-db",
                            "folderUid": "",
                            "folderTitle": "General"
                        },
                        {
                            "uid": "logs-main",
                            "title": "Logs Main",
                            "type": "dash-db",
                            "folderUid": "platform",
                            "folderTitle": "Platform"
                        }
                    ]))),
                    (reqwest::Method::GET, "/api/datasources") => Ok(Some(json!([
                        {"uid": "prom-main", "name": "Prometheus Main"}
                    ]))),
                    _ => Err(crate::common::message(format!(
                        "unexpected request {:?} {path}",
                        method_name
                    ))),
                }
            },
        )
        .unwrap();

        assert_eq!(domain.status, PROJECT_STATUS_READY);
        assert_eq!(domain.warning_count, 1);
        assert_eq!(
            domain.warnings,
            vec![status_finding(
                "live-dashboard-root-scope",
                1,
                "live.rootDashboardCount",
            )]
        );
        assert_eq!(
            domain.signal_keys,
            vec![
                "live.dashboardCount".to_string(),
                "live.datasourceCount".to_string(),
                "live.folderCount".to_string(),
                "live.dashboardTitleGapCount".to_string(),
                "live.folderTitleGapCount".to_string(),
                "live.importReadyDashboardCount".to_string(),
                "live.rootDashboardCount".to_string(),
            ]
        );
        assert_eq!(
            domain.next_actions,
            vec![
                "re-run live dashboard read after dashboard, folder, or datasource changes"
                    .to_string(),
                "review live dashboard governance and import-readiness warnings before re-running live dashboard read"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn build_live_dashboard_domain_status_reports_partial_when_no_dashboards_exist() {
        let domain = build_live_dashboard_domain_status_with_request(
            move |method, path, _params, _payload| {
                let method_name = method.to_string();
                match (method, path) {
                    (reqwest::Method::GET, "/api/search") => Ok(Some(json!([]))),
                    (reqwest::Method::GET, "/api/datasources") => Ok(Some(json!([
                        {"uid": "prom-main", "name": "Prometheus Main"}
                    ]))),
                    _ => Err(crate::common::message(format!(
                        "unexpected request {:?} {path}",
                        method_name
                    ))),
                }
            },
        )
        .unwrap();

        assert_eq!(domain.status, PROJECT_STATUS_PARTIAL);
        assert_eq!(domain.reason_code, "partial-no-data");
        assert_eq!(domain.primary_count, 0);
        assert_eq!(
            domain.next_actions,
            vec![
                "create or import at least one dashboard, then re-run live dashboard read"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn build_live_dashboard_domain_status_reports_import_readiness_gaps_for_live_dashboards() {
        let domain = build_live_dashboard_domain_status_with_request(
            move |method, path, _params, _payload| {
                let method_name = method.to_string();
                match (method, path) {
                    (reqwest::Method::GET, "/api/search") => Ok(Some(json!([
                        {
                            "uid": "cpu-main",
                            "title": "",
                            "type": "dash-db",
                            "folderUid": "infra",
                            "folderTitle": ""
                        },
                        {
                            "uid": "ops-main",
                            "title": "Ops Main",
                            "type": "dash-db",
                            "folderUid": "",
                            "folderTitle": "General"
                        }
                    ]))),
                    (reqwest::Method::GET, "/api/datasources") => Ok(Some(json!([
                        {"uid": "prom-main", "name": "Prometheus Main"}
                    ]))),
                    _ => Err(crate::common::message(format!(
                        "unexpected request {:?} {path}",
                        method_name
                    ))),
                }
            },
        )
        .unwrap();

        assert_eq!(domain.status, PROJECT_STATUS_READY);
        assert_eq!(domain.reason_code, "ready");
        assert_eq!(domain.primary_count, 2);
        assert_eq!(domain.warning_count, 4);
        assert_eq!(
            domain.warnings,
            vec![
                status_finding("live-dashboard-root-scope", 1, "live.rootDashboardCount"),
                status_finding("live-dashboard-title-gap", 1, "live.dashboardTitleGapCount"),
                status_finding(
                    "live-dashboard-folder-title-gap",
                    1,
                    "live.folderTitleGapCount"
                ),
                status_finding(
                    "live-dashboard-import-ready-gap",
                    1,
                    "live.importReadyDashboardCount"
                ),
            ]
        );
        assert_eq!(
            domain.signal_keys,
            vec![
                "live.dashboardCount".to_string(),
                "live.datasourceCount".to_string(),
                "live.folderCount".to_string(),
                "live.dashboardTitleGapCount".to_string(),
                "live.folderTitleGapCount".to_string(),
                "live.importReadyDashboardCount".to_string(),
                "live.rootDashboardCount".to_string(),
            ]
        );
        assert_eq!(
            domain.next_actions,
            vec![
                "re-run live dashboard read after dashboard, folder, or datasource changes"
                    .to_string(),
                "review live dashboard governance and import-readiness warnings before re-running live dashboard read"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn build_live_dashboard_domain_status_reports_partial_when_dashboard_has_no_datasources() {
        let domain = build_live_dashboard_domain_status_with_request(
            move |method, path, _params, _payload| {
                let method_name = method.to_string();
                match (method, path) {
                    (reqwest::Method::GET, "/api/search") => Ok(Some(json!([
                        {
                            "uid": "cpu-main",
                            "title": "CPU Main",
                            "type": "dash-db",
                            "folderUid": "infra",
                            "folderTitle": "Infra"
                        }
                    ]))),
                    (reqwest::Method::GET, "/api/datasources") => Ok(Some(json!([]))),
                    _ => Err(crate::common::message(format!(
                        "unexpected request {:?} {path}",
                        method_name
                    ))),
                }
            },
        )
        .unwrap();

        assert_eq!(domain.status, PROJECT_STATUS_PARTIAL);
        assert_eq!(domain.reason_code, "partial-no-datasources");
        assert_eq!(domain.primary_count, 1);
        assert_eq!(domain.warning_count, 1);
        assert_eq!(
            domain.warnings,
            vec![status_finding(
                "live-datasource-inventory-empty",
                1,
                "live.datasourceInventoryEmpty",
            )]
        );
        assert_eq!(
            domain.signal_keys,
            vec![
                "live.dashboardCount".to_string(),
                "live.datasourceCount".to_string(),
                "live.folderCount".to_string(),
                "live.dashboardTitleGapCount".to_string(),
                "live.folderTitleGapCount".to_string(),
                "live.importReadyDashboardCount".to_string(),
                "live.datasourceInventoryEmpty".to_string(),
            ]
        );
        assert_eq!(
            domain.next_actions,
            vec![
                "create or import at least one datasource, then re-run live dashboard read"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn build_live_dashboard_domain_status_reports_datasource_inventory_drift() {
        let domain = build_live_dashboard_domain_status_with_request(
            move |method, path, _params, _payload| {
                let method_name = method.to_string();
                match (method, path) {
                    (reqwest::Method::GET, "/api/search") => Ok(Some(json!([
                        {
                            "uid": "cpu-main",
                            "title": "CPU Main",
                            "type": "dash-db",
                            "folderUid": "infra",
                            "folderTitle": "Infra"
                        }
                    ]))),
                    (reqwest::Method::GET, "/api/datasources") => Ok(Some(json!([
                        {"uid": "prom-main", "name": "Prometheus Main"},
                        {"uid": "prom-main", "name": "Prometheus Main"},
                        {"uid": "loki-main", "name": "Loki Main"}
                    ]))),
                    _ => Err(crate::common::message(format!(
                        "unexpected request {:?} {path}",
                        method_name
                    ))),
                }
            },
        )
        .unwrap();

        assert_eq!(domain.status, PROJECT_STATUS_READY);
        assert_eq!(domain.reason_code, "ready");
        assert_eq!(domain.primary_count, 1);
        assert_eq!(domain.warning_count, 1);
        assert_eq!(
            domain.warnings,
            vec![status_finding(
                "live-datasource-inventory-drift",
                1,
                "live.datasourceDriftCount",
            )]
        );
        assert_eq!(
            domain.signal_keys,
            vec![
                "live.dashboardCount".to_string(),
                "live.datasourceCount".to_string(),
                "live.folderCount".to_string(),
                "live.dashboardTitleGapCount".to_string(),
                "live.folderTitleGapCount".to_string(),
                "live.importReadyDashboardCount".to_string(),
                "live.datasourceDriftCount".to_string(),
            ]
        );
        assert_eq!(
            domain.next_actions,
            vec![
                "re-run live dashboard read after dashboard, folder, or datasource changes"
                    .to_string(),
                "review live dashboard governance and import-readiness warnings before re-running live dashboard read"
                    .to_string(),
            ]
        );
    }
}
