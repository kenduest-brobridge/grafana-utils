use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

use crate::common::{message, Result};
use crate::http::JsonHttpClient;
use crate::project_status::{
    ProjectDomainStatus, ProjectStatusFinding, PROJECT_STATUS_BLOCKED, PROJECT_STATUS_PARTIAL,
    PROJECT_STATUS_READY,
};
use crate::project_status_model::{StatusReading, StatusRecordCount};
use crate::project_status_support::build_live_project_status_client_from_api;

use super::{
    build_live_overall_freshness, PROJECT_STATUS_LIVE_ALL_ORGS_AGGREGATE,
    PROJECT_STATUS_LIVE_ALL_ORGS_MODE_SUFFIX,
};

fn project_status_severity_rank(status: &str) -> usize {
    match status {
        PROJECT_STATUS_BLOCKED => 0,
        PROJECT_STATUS_PARTIAL => 1,
        PROJECT_STATUS_READY => 2,
        _ => 3,
    }
}

fn org_id_from_record(org: &Map<String, Value>) -> Result<i64> {
    org.get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Grafana org payload did not include a usable numeric id."))
}

fn merge_project_status_findings(findings: &[ProjectStatusFinding]) -> Vec<ProjectStatusFinding> {
    let mut merged = BTreeMap::<(String, String), usize>::new();
    for finding in findings {
        *merged
            .entry((finding.kind.clone(), finding.source.clone()))
            .or_default() += finding.count;
    }
    merged
        .into_iter()
        .map(|((kind, source), count)| ProjectStatusFinding {
            kind,
            count,
            source,
        })
        .collect()
}

fn merge_live_domain_statuses(statuses: Vec<ProjectDomainStatus>) -> Result<ProjectDomainStatus> {
    let aggregate = statuses
        .iter()
        .min_by_key(|status| {
            (
                project_status_severity_rank(&status.status),
                usize::MAX - status.blocker_count,
                usize::MAX - status.warning_count,
            )
        })
        .ok_or_else(|| message("Expected at least one per-org domain status to aggregate."))?;
    let blockers = merge_project_status_findings(
        &statuses
            .iter()
            .flat_map(|status| status.blockers.iter().cloned())
            .collect::<Vec<_>>(),
    );
    let warnings = merge_project_status_findings(
        &statuses
            .iter()
            .flat_map(|status| status.warnings.iter().cloned())
            .collect::<Vec<_>>(),
    );
    let freshness = build_live_overall_freshness(&statuses);
    let reason_code = if statuses
        .iter()
        .all(|status| status.reason_code == aggregate.reason_code)
    {
        aggregate.reason_code.clone()
    } else {
        PROJECT_STATUS_LIVE_ALL_ORGS_AGGREGATE.to_string()
    };
    let mode = if statuses.iter().all(|status| status.mode == aggregate.mode) {
        format!(
            "{}{}",
            aggregate.mode, PROJECT_STATUS_LIVE_ALL_ORGS_MODE_SUFFIX
        )
    } else {
        PROJECT_STATUS_LIVE_ALL_ORGS_AGGREGATE.to_string()
    };
    let source_kinds = statuses
        .iter()
        .flat_map(|status| status.source_kinds.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let signal_keys = statuses
        .iter()
        .flat_map(|status| status.signal_keys.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let next_actions = statuses
        .iter()
        .flat_map(|status| status.next_actions.iter().cloned())
        .fold(Vec::<String>::new(), |mut acc, item| {
            if !acc.iter().any(|existing| existing == &item) {
                acc.push(item);
            }
            acc
        });

    Ok(StatusReading {
        id: aggregate.id.clone(),
        scope: aggregate.scope.clone(),
        mode,
        status: aggregate.status.clone(),
        reason_code,
        primary_count: statuses.iter().map(|status| status.primary_count).sum(),
        source_kinds,
        signal_keys,
        blockers: blockers.into_iter().map(StatusRecordCount::from).collect(),
        warnings: warnings.into_iter().map(StatusRecordCount::from).collect(),
        next_actions,
        freshness,
    }
    .into_project_domain_status())
}

pub(super) fn build_live_multi_org_domain_status_with_orgs<F>(
    orgs: &[Map<String, Value>],
    mut build_org_status: F,
) -> Result<ProjectDomainStatus>
where
    F: FnMut(i64) -> Result<ProjectDomainStatus>,
{
    let mut statuses = Vec::new();
    for org in orgs {
        statuses.push(build_org_status(org_id_from_record(org)?)?);
    }
    merge_live_domain_statuses(statuses)
}

pub(super) fn build_live_multi_org_domain_status<F>(
    api: &crate::grafana_api::GrafanaApiClient,
    orgs: &[Map<String, Value>],
    mut build_status: F,
) -> Result<ProjectDomainStatus>
where
    F: FnMut(&JsonHttpClient) -> ProjectDomainStatus,
{
    build_live_multi_org_domain_status_with_orgs(orgs, |org_id| {
        let client = build_live_project_status_client_from_api(api, Some(org_id))?;
        Ok(build_status(&client))
    })
}
