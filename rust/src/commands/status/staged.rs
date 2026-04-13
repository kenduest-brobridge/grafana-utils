//! Shared staged project-status assembly reused by overview and status flows.

use crate::access::{build_access_domain_status, AccessDomainStatusInputs};
use crate::alert::build_alert_project_status_domain as build_alert_domain_status;
use crate::dashboard::build_dashboard_domain_status;
use crate::datasource_project_status::build_datasource_domain_status;
use crate::overview::{
    OverviewArtifact, OverviewProjectStatus, OverviewProjectStatusDomain,
    OverviewProjectStatusFreshness, OVERVIEW_ARTIFACT_ACCESS_ORG_EXPORT_KIND,
    OVERVIEW_ARTIFACT_ACCESS_SERVICE_ACCOUNT_EXPORT_KIND,
    OVERVIEW_ARTIFACT_ACCESS_TEAM_EXPORT_KIND, OVERVIEW_ARTIFACT_ACCESS_USER_EXPORT_KIND,
    OVERVIEW_ARTIFACT_ALERT_EXPORT_KIND, OVERVIEW_ARTIFACT_BUNDLE_PREFLIGHT_KIND,
    OVERVIEW_ARTIFACT_DASHBOARD_EXPORT_KIND, OVERVIEW_ARTIFACT_DATASOURCE_EXPORT_KIND,
    OVERVIEW_ARTIFACT_PROMOTION_PREFLIGHT_KIND, OVERVIEW_ARTIFACT_SYNC_SUMMARY_KIND,
};
use crate::project_status::build_project_status as build_shared_project_status;
use crate::sync::{
    build_promotion_domain_status, build_sync_domain_status, SyncDomainStatusInputs,
};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

const OVERVIEW_PROJECT_DOMAIN_COUNT: usize = 6;
const PROJECT_STATUS_FRESHNESS_CURRENT: &str = "current";
const PROJECT_STATUS_FRESHNESS_STALE: &str = "stale";
const PROJECT_STATUS_FRESHNESS_UNKNOWN: &str = "unknown";
const PROJECT_STATUS_STALE_AGE_SECONDS: u64 = 7 * 24 * 60 * 60;

fn artifact_input_ages_seconds(artifact: &OverviewArtifact) -> Vec<u64> {
    let now = SystemTime::now();
    artifact
        .inputs
        .iter()
        .filter_map(|input| {
            let path = Path::new(&input.value);
            let metadata = fs::metadata(path).ok()?;
            let modified = metadata.modified().ok()?;
            now.duration_since(modified).ok().map(|age| age.as_secs())
        })
        .collect()
}

fn freshness_from_ages(ages: &[u64]) -> OverviewProjectStatusFreshness {
    if ages.is_empty() {
        return OverviewProjectStatusFreshness {
            status: PROJECT_STATUS_FRESHNESS_UNKNOWN.to_string(),
            source_count: 0,
            newest_age_seconds: None,
            oldest_age_seconds: None,
        };
    }
    let newest_age_seconds = ages.iter().min().copied();
    let oldest_age_seconds = ages.iter().max().copied();
    let status = if oldest_age_seconds.unwrap_or(0) > PROJECT_STATUS_STALE_AGE_SECONDS {
        PROJECT_STATUS_FRESHNESS_STALE
    } else {
        PROJECT_STATUS_FRESHNESS_CURRENT
    };
    OverviewProjectStatusFreshness {
        status: status.to_string(),
        source_count: ages.len(),
        newest_age_seconds,
        oldest_age_seconds,
    }
}

fn domain_freshness(
    artifacts: &[OverviewArtifact],
    supported_kinds: &[&str],
) -> OverviewProjectStatusFreshness {
    let mut ages = Vec::new();
    for artifact in artifacts {
        if supported_kinds.contains(&artifact.kind.as_str()) {
            ages.extend(artifact_input_ages_seconds(artifact));
        }
    }
    freshness_from_ages(&ages)
}

fn attach_domain_freshness(
    mut domain: OverviewProjectStatusDomain,
    freshness: OverviewProjectStatusFreshness,
) -> OverviewProjectStatusDomain {
    domain.freshness = freshness;
    domain
}

fn find_artifact<'a>(
    artifacts: &'a [OverviewArtifact],
    kind: &str,
) -> Option<&'a OverviewArtifact> {
    artifacts.iter().find(|artifact| artifact.kind == kind)
}

fn build_dashboard_project_status_domain(
    artifacts: &[OverviewArtifact],
) -> Option<OverviewProjectStatusDomain> {
    build_dashboard_domain_status(
        find_artifact(artifacts, OVERVIEW_ARTIFACT_DASHBOARD_EXPORT_KIND)
            .map(|artifact| &artifact.document),
    )
    .map(|domain| {
        attach_domain_freshness(
            domain,
            domain_freshness(artifacts, &[OVERVIEW_ARTIFACT_DASHBOARD_EXPORT_KIND]),
        )
    })
}

fn build_datasource_project_status_domain(
    artifacts: &[OverviewArtifact],
) -> Option<OverviewProjectStatusDomain> {
    build_datasource_domain_status(
        find_artifact(artifacts, OVERVIEW_ARTIFACT_DATASOURCE_EXPORT_KIND)
            .map(|artifact| &artifact.document),
    )
    .map(|domain| {
        attach_domain_freshness(
            domain,
            domain_freshness(artifacts, &[OVERVIEW_ARTIFACT_DATASOURCE_EXPORT_KIND]),
        )
    })
}

fn build_alert_project_status_domain(
    artifacts: &[OverviewArtifact],
) -> Option<OverviewProjectStatusDomain> {
    build_alert_domain_status(
        find_artifact(artifacts, OVERVIEW_ARTIFACT_ALERT_EXPORT_KIND)
            .map(|artifact| &artifact.document),
    )
    .map(|domain| {
        attach_domain_freshness(
            domain,
            domain_freshness(artifacts, &[OVERVIEW_ARTIFACT_ALERT_EXPORT_KIND]),
        )
    })
}

fn build_access_project_status_domain(
    artifacts: &[OverviewArtifact],
) -> Option<OverviewProjectStatusDomain> {
    build_access_domain_status(AccessDomainStatusInputs {
        user_export_document: find_artifact(artifacts, OVERVIEW_ARTIFACT_ACCESS_USER_EXPORT_KIND)
            .map(|artifact| &artifact.document),
        team_export_document: find_artifact(artifacts, OVERVIEW_ARTIFACT_ACCESS_TEAM_EXPORT_KIND)
            .map(|artifact| &artifact.document),
        org_export_document: find_artifact(artifacts, OVERVIEW_ARTIFACT_ACCESS_ORG_EXPORT_KIND)
            .map(|artifact| &artifact.document),
        service_account_export_document: find_artifact(
            artifacts,
            OVERVIEW_ARTIFACT_ACCESS_SERVICE_ACCOUNT_EXPORT_KIND,
        )
        .map(|artifact| &artifact.document),
    })
    .map(|domain| {
        attach_domain_freshness(
            domain,
            domain_freshness(
                artifacts,
                &[
                    OVERVIEW_ARTIFACT_ACCESS_USER_EXPORT_KIND,
                    OVERVIEW_ARTIFACT_ACCESS_TEAM_EXPORT_KIND,
                    OVERVIEW_ARTIFACT_ACCESS_ORG_EXPORT_KIND,
                    OVERVIEW_ARTIFACT_ACCESS_SERVICE_ACCOUNT_EXPORT_KIND,
                ],
            ),
        )
    })
}

fn build_sync_project_status_domain(
    artifacts: &[OverviewArtifact],
) -> Option<OverviewProjectStatusDomain> {
    build_sync_domain_status(SyncDomainStatusInputs {
        summary_document: find_artifact(artifacts, OVERVIEW_ARTIFACT_SYNC_SUMMARY_KIND)
            .map(|artifact| &artifact.document),
        bundle_preflight_document: find_artifact(
            artifacts,
            OVERVIEW_ARTIFACT_BUNDLE_PREFLIGHT_KIND,
        )
        .map(|artifact| &artifact.document),
    })
    .map(|domain| {
        attach_domain_freshness(
            domain,
            domain_freshness(
                artifacts,
                &[
                    OVERVIEW_ARTIFACT_SYNC_SUMMARY_KIND,
                    OVERVIEW_ARTIFACT_BUNDLE_PREFLIGHT_KIND,
                ],
            ),
        )
    })
}

fn build_project_status_domains(
    artifacts: &[OverviewArtifact],
) -> Vec<OverviewProjectStatusDomain> {
    [
        build_dashboard_project_status_domain(artifacts),
        build_datasource_project_status_domain(artifacts),
        build_alert_project_status_domain(artifacts),
        build_access_project_status_domain(artifacts),
        build_sync_project_status_domain(artifacts),
        build_promotion_domain_status(
            find_artifact(artifacts, OVERVIEW_ARTIFACT_PROMOTION_PREFLIGHT_KIND)
                .map(|artifact| &artifact.document),
        )
        .map(|domain| {
            attach_domain_freshness(
                domain,
                domain_freshness(artifacts, &[OVERVIEW_ARTIFACT_PROMOTION_PREFLIGHT_KIND]),
            )
        }),
    ]
    .into_iter()
    .flatten()
    .collect()
}

pub(crate) fn build_staged_project_status(artifacts: &[OverviewArtifact]) -> OverviewProjectStatus {
    build_shared_project_status(
        "staged-only",
        OVERVIEW_PROJECT_DOMAIN_COUNT,
        freshness_from_ages(
            &artifacts
                .iter()
                .flat_map(artifact_input_ages_seconds)
                .collect::<Vec<_>>(),
        ),
        build_project_status_domains(artifacts),
    )
}
