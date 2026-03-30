//! Shared project-status contract types.
//!
//! Maintainer note:
//! - Keep this module limited to reusable status document shapes and small
//!   shared constants/helpers.
//! - Domain-specific producer logic belongs in the owning domain modules.

use serde::Serialize;
use std::cmp::Reverse;

use crate::common::tool_version;

pub const PROJECT_STATUS_READY: &str = "ready";
pub const PROJECT_STATUS_PARTIAL: &str = "partial";
pub const PROJECT_STATUS_BLOCKED: &str = "blocked";
pub const PROJECT_STATUS_UNKNOWN: &str = "unknown";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatusFreshness {
    pub status: String,
    pub source_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest_age_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_age_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatusFinding {
    pub kind: String,
    pub count: usize,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDomainStatus {
    pub id: String,
    pub scope: String,
    pub mode: String,
    pub status: String,
    pub reason_code: String,
    pub primary_count: usize,
    pub blocker_count: usize,
    pub warning_count: usize,
    pub source_kinds: Vec<String>,
    pub signal_keys: Vec<String>,
    pub blockers: Vec<ProjectStatusFinding>,
    pub warnings: Vec<ProjectStatusFinding>,
    pub next_actions: Vec<String>,
    pub freshness: ProjectStatusFreshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatusOverall {
    pub status: String,
    pub domain_count: usize,
    pub present_count: usize,
    pub blocked_count: usize,
    pub blocker_count: usize,
    pub warning_count: usize,
    pub freshness: ProjectStatusFreshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatusRankedFinding {
    pub domain: String,
    pub kind: String,
    pub count: usize,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatusAction {
    pub domain: String,
    pub reason_code: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStatus {
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "toolVersion")]
    pub tool_version: String,
    pub scope: String,
    pub overall: ProjectStatusOverall,
    pub domains: Vec<ProjectDomainStatus>,
    pub top_blockers: Vec<ProjectStatusRankedFinding>,
    pub next_actions: Vec<ProjectStatusAction>,
}

pub(crate) fn status_finding(kind: &str, count: usize, source: &str) -> ProjectStatusFinding {
    ProjectStatusFinding {
        kind: kind.to_string(),
        count,
        source: source.to_string(),
    }
}

fn domain_status_rank(status: &str) -> usize {
    match status {
        PROJECT_STATUS_BLOCKED => 0,
        PROJECT_STATUS_PARTIAL => 1,
        PROJECT_STATUS_READY => 2,
        _ => 3,
    }
}

pub(crate) fn build_project_top_blockers(
    domains: &[ProjectDomainStatus],
) -> Vec<ProjectStatusRankedFinding> {
    let mut blockers = domains
        .iter()
        .flat_map(|domain| {
            domain
                .blockers
                .iter()
                .map(|blocker| ProjectStatusRankedFinding {
                    domain: domain.id.clone(),
                    kind: blocker.kind.clone(),
                    count: blocker.count,
                    source: blocker.source.clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    blockers.sort_by_key(|item| {
        (
            Reverse(item.count),
            item.domain.clone(),
            item.kind.clone(),
            item.source.clone(),
        )
    });
    blockers
}

pub(crate) fn build_project_next_actions(
    domains: &[ProjectDomainStatus],
) -> Vec<ProjectStatusAction> {
    let mut actions = domains
        .iter()
        .flat_map(|domain| {
            domain
                .next_actions
                .iter()
                .map(|action| ProjectStatusAction {
                    domain: domain.id.clone(),
                    reason_code: domain.reason_code.clone(),
                    action: action.clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    actions.sort_by_key(|item| {
        let domain = domains
            .iter()
            .find(|domain| domain.id == item.domain)
            .expect("action domain must exist");
        (
            domain_status_rank(&domain.status),
            Reverse(domain.blocker_count),
            Reverse(domain.warning_count),
            item.domain.clone(),
            item.action.clone(),
        )
    });
    actions
}

pub(crate) fn build_project_status(
    scope: &str,
    domain_count: usize,
    freshness: ProjectStatusFreshness,
    domains: Vec<ProjectDomainStatus>,
) -> ProjectStatus {
    let present_count = domains.len();
    let blocked_count = domains
        .iter()
        .filter(|domain| domain.status == PROJECT_STATUS_BLOCKED)
        .count();
    let blocker_count = domains
        .iter()
        .map(|domain| domain.blocker_count)
        .sum::<usize>();
    let warning_count = domains
        .iter()
        .map(|domain| domain.warning_count)
        .sum::<usize>();
    let overall_status = if blocked_count > 0 {
        PROJECT_STATUS_BLOCKED
    } else if present_count < domain_count
        || domains.iter().any(|domain| {
            domain.status == PROJECT_STATUS_PARTIAL || domain.status == PROJECT_STATUS_UNKNOWN
        })
    {
        PROJECT_STATUS_PARTIAL
    } else {
        PROJECT_STATUS_READY
    };

    ProjectStatus {
        schema_version: 1,
        tool_version: tool_version().to_string(),
        scope: scope.to_string(),
        overall: ProjectStatusOverall {
            status: overall_status.to_string(),
            domain_count,
            present_count,
            blocked_count,
            blocker_count,
            warning_count,
            freshness,
        },
        top_blockers: build_project_top_blockers(&domains),
        next_actions: build_project_next_actions(&domains),
        domains,
    }
}
