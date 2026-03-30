//! Overview document assembly and text rendering.

use super::{
    overview_kind::{parse_overview_artifact_kind, OverviewArtifactKind},
    overview_sections::build_overview_summary_item,
    overview_summary_projection::build_overview_summary,
    OverviewArtifact, OverviewDocument, OverviewProjectStatus, OverviewProjectStatusDomain,
    OverviewProjectStatusFreshness, OVERVIEW_KIND, OVERVIEW_SCHEMA_VERSION,
};
use crate::access::{build_access_domain_status, AccessDomainStatusInputs};
use crate::alert::build_alert_project_status_domain as build_alert_domain_status;
use crate::common::{message, tool_version, Result};
use crate::dashboard::build_dashboard_domain_status;
use crate::datasource_project_status::build_datasource_domain_status;
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
    kinds: &[OverviewArtifactKind],
) -> OverviewProjectStatusFreshness {
    let mut ages = Vec::new();
    for artifact in artifacts {
        let Ok(kind) = parse_overview_artifact_kind(&artifact.kind) else {
            continue;
        };
        if kinds.contains(&kind) {
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

fn find_artifact(
    artifacts: &[OverviewArtifact],
    kind: OverviewArtifactKind,
) -> Option<&OverviewArtifact> {
    artifacts
        .iter()
        .find(|artifact| parse_overview_artifact_kind(&artifact.kind).ok() == Some(kind))
}

fn build_dashboard_project_status_domain(
    artifacts: &[OverviewArtifact],
) -> Option<OverviewProjectStatusDomain> {
    build_dashboard_domain_status(
        find_artifact(artifacts, OverviewArtifactKind::DashboardExport)
            .map(|artifact| &artifact.document),
    )
    .map(|domain| {
        attach_domain_freshness(
            domain,
            domain_freshness(artifacts, &[OverviewArtifactKind::DashboardExport]),
        )
    })
}

fn build_datasource_project_status_domain(
    artifacts: &[OverviewArtifact],
) -> Option<OverviewProjectStatusDomain> {
    build_datasource_domain_status(
        find_artifact(artifacts, OverviewArtifactKind::DatasourceExport)
            .map(|artifact| &artifact.document),
    )
    .map(|domain| {
        attach_domain_freshness(
            domain,
            domain_freshness(artifacts, &[OverviewArtifactKind::DatasourceExport]),
        )
    })
}

fn build_alert_project_status_domain(
    artifacts: &[OverviewArtifact],
) -> Option<OverviewProjectStatusDomain> {
    build_alert_domain_status(
        find_artifact(artifacts, OverviewArtifactKind::AlertExport)
            .map(|artifact| &artifact.document),
    )
    .map(|domain| {
        attach_domain_freshness(
            domain,
            domain_freshness(artifacts, &[OverviewArtifactKind::AlertExport]),
        )
    })
}

fn build_access_project_status_domain(
    artifacts: &[OverviewArtifact],
) -> Option<OverviewProjectStatusDomain> {
    build_access_domain_status(AccessDomainStatusInputs {
        user_export_document: find_artifact(artifacts, OverviewArtifactKind::AccessUserExport)
            .map(|artifact| &artifact.document),
        team_export_document: find_artifact(artifacts, OverviewArtifactKind::AccessTeamExport)
            .map(|artifact| &artifact.document),
        org_export_document: find_artifact(artifacts, OverviewArtifactKind::AccessOrgExport)
            .map(|artifact| &artifact.document),
        service_account_export_document: find_artifact(
            artifacts,
            OverviewArtifactKind::AccessServiceAccountExport,
        )
        .map(|artifact| &artifact.document),
    })
    .map(|domain| {
        attach_domain_freshness(
            domain,
            domain_freshness(
                artifacts,
                &[
                    OverviewArtifactKind::AccessUserExport,
                    OverviewArtifactKind::AccessTeamExport,
                    OverviewArtifactKind::AccessOrgExport,
                    OverviewArtifactKind::AccessServiceAccountExport,
                ],
            ),
        )
    })
}

fn build_sync_project_status_domain(
    artifacts: &[OverviewArtifact],
) -> Option<OverviewProjectStatusDomain> {
    build_sync_domain_status(SyncDomainStatusInputs {
        summary_document: find_artifact(artifacts, OverviewArtifactKind::SyncSummary)
            .map(|artifact| &artifact.document),
        bundle_preflight_document: find_artifact(artifacts, OverviewArtifactKind::BundlePreflight)
            .map(|artifact| &artifact.document),
    })
    .map(|domain| {
        attach_domain_freshness(
            domain,
            domain_freshness(
                artifacts,
                &[
                    OverviewArtifactKind::SyncSummary,
                    OverviewArtifactKind::BundlePreflight,
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
            find_artifact(artifacts, OverviewArtifactKind::PromotionPreflight)
                .map(|artifact| &artifact.document),
        )
        .map(|domain| {
            attach_domain_freshness(
                domain,
                domain_freshness(artifacts, &[OverviewArtifactKind::PromotionPreflight]),
            )
        }),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn build_project_status(artifacts: &[OverviewArtifact]) -> OverviewProjectStatus {
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

pub(crate) fn build_overview_document(
    artifacts: Vec<OverviewArtifact>,
) -> Result<OverviewDocument> {
    if artifacts.is_empty() {
        return Err(message("Overview requires at least one input artifact."));
    }
    for artifact in &artifacts {
        if artifact.title.trim().is_empty() {
            return Err(message("Overview artifacts require a title."));
        }
        parse_overview_artifact_kind(&artifact.kind)?;
    }
    let sections = super::overview_sections::build_overview_sections(&artifacts)?;
    Ok(OverviewDocument {
        kind: OVERVIEW_KIND.to_string(),
        schema_version: OVERVIEW_SCHEMA_VERSION,
        tool_version: tool_version().to_string(),
        summary: build_overview_summary(&artifacts)?,
        project_status: build_project_status(&artifacts),
        artifacts,
        selected_section_index: 0,
        sections,
    })
}

pub(crate) fn render_overview_text(document: &OverviewDocument) -> Result<Vec<String>> {
    if document.kind != OVERVIEW_KIND {
        return Err(message("Overview document kind is not supported."));
    }
    let mut lines = vec![
        "Project overview".to_string(),
        format!(
            "Status: {} domains={} present={} blocked={} blockers={} warnings={} freshness={} oldestAge={}s",
            document.project_status.overall.status,
            document.project_status.overall.domain_count,
            document.project_status.overall.present_count,
            document.project_status.overall.blocked_count,
            document.project_status.overall.blocker_count,
            document.project_status.overall.warning_count,
            document.project_status.overall.freshness.status,
            document
                .project_status
                .overall
                .freshness
                .oldest_age_seconds
                .unwrap_or(0),
        ),
        format!(
            "Artifacts: {} total, {} dashboard export, {} datasource export, {} alert export, {} access user export, {} access team export, {} access org export, {} access service-account export, {} sync summary, {} bundle preflight, {} promotion preflight",
            document.summary.artifact_count,
            document.summary.dashboard_export_count,
            document.summary.datasource_export_count,
            document.summary.alert_export_count,
            document.summary.access_user_export_count,
            document.summary.access_team_export_count,
            document.summary.access_org_export_count,
            document.summary.access_service_account_export_count,
            document.summary.sync_summary_count,
            document.summary.bundle_preflight_count,
            document.summary.promotion_preflight_count,
        ),
    ];
    if !document.project_status.domains.is_empty() {
        lines.push("Domain status:".to_string());
        for domain in &document.project_status.domains {
            let mut line = format!(
                "- {} status={} reason={} primary={} blockers={} warnings={} freshness={}",
                domain.id,
                domain.status,
                domain.reason_code,
                domain.primary_count,
                domain.blocker_count,
                domain.warning_count,
                domain.freshness.status,
            );
            if let Some(action) = domain.next_actions.first() {
                line.push_str(&format!(" next={action}"));
            }
            lines.push(line);
        }
    }
    if !document.project_status.top_blockers.is_empty() {
        lines.push("Top blockers:".to_string());
        for blocker in document.project_status.top_blockers.iter().take(5) {
            lines.push(format!(
                "- {} {} count={} source={}",
                blocker.domain, blocker.kind, blocker.count, blocker.source
            ));
        }
    }
    if !document.project_status.next_actions.is_empty() {
        lines.push("Next actions:".to_string());
        for action in document.project_status.next_actions.iter().take(5) {
            lines.push(format!(
                "- {} reason={} action={}",
                action.domain, action.reason_code, action.action
            ));
        }
    }
    for artifact in &document.artifacts {
        let item = build_overview_summary_item(artifact)?;
        lines.push(String::new());
        lines.push(format!("# {}", artifact.title));
        lines.extend(item.details);
    }
    Ok(lines)
}
