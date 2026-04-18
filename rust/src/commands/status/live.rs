//! Live project status runtime orchestration.
//!
//! Responsibilities:
//! - Build per-domain collectors for dashboard, datasource, alert, access, and sync status.
//! - Aggregate live findings across orgs and score combined freshness/severity.
//! - Emit a stable status document for `project-status` reporting.

#[path = "live_discovery.rs"]
mod live_discovery;
#[path = "live_domains.rs"]
mod live_domains;
#[path = "live_multi_org.rs"]
mod live_multi_org;
#[cfg(test)]
#[path = "live_tests.rs"]
mod tests;

use serde_json::Value;
use std::fs::Metadata;
use std::path::PathBuf;

use crate::common::{load_json_object_file, message, Result};
use crate::project_status::{
    build_project_status, ProjectDomainStatus, ProjectStatus, ProjectStatusFreshness,
    PROJECT_STATUS_PARTIAL,
};
use crate::project_status_command::{ProjectStatusLiveArgs, PROJECT_STATUS_DOMAIN_COUNT};
use crate::project_status_freshness::{
    build_live_project_status_freshness, build_live_project_status_freshness_from_samples,
    build_live_project_status_freshness_from_source_count, ProjectStatusFreshnessSample,
};
use crate::project_status_model::{StatusReading, StatusRecordCount};
use crate::project_status_support::{build_live_project_status_api_client, project_status_live};

use self::live_discovery::build_live_status_discovery;
use self::live_domains::{
    build_live_access_status, build_live_alert_status, build_live_dashboard_status,
    build_live_datasource_status, build_live_promotion_status, build_live_sync_status,
};
use self::live_multi_org::build_live_multi_org_domain_status;

const PROJECT_STATUS_LIVE_SCOPE: &str = "live";
const PROJECT_STATUS_LIVE_ALL_ORGS_MODE_SUFFIX: &str = "-all-orgs";
const PROJECT_STATUS_LIVE_READ_FAILED: &str = "live-read-failed";
const PROJECT_STATUS_LIVE_ALL_ORGS_AGGREGATE: &str = "multi-org-aggregate";
const PROJECT_STATUS_LIVE_INSTANCE_SOURCE: &str = "api-health";

fn build_live_read_failed_domain_status(
    id: &str,
    mode: &str,
    source_kind: &str,
    signal_key: &str,
    action: &str,
) -> ProjectDomainStatus {
    StatusReading {
        id: id.to_string(),
        scope: PROJECT_STATUS_LIVE_SCOPE.to_string(),
        mode: mode.to_string(),
        status: PROJECT_STATUS_PARTIAL.to_string(),
        reason_code: PROJECT_STATUS_LIVE_READ_FAILED.to_string(),
        primary_count: 0,
        source_kinds: vec![source_kind.to_string()],
        signal_keys: vec![signal_key.to_string()],
        blockers: vec![StatusRecordCount::new(
            PROJECT_STATUS_LIVE_READ_FAILED,
            1,
            signal_key,
        )],
        warnings: Vec::new(),
        next_actions: vec![action.to_string()],
        freshness: ProjectStatusFreshness::default(),
    }
    .into_project_domain_status()
}

fn load_optional_project_status_document_with_metadata(
    path: Option<&PathBuf>,
    label: &str,
) -> Result<Option<(Value, Metadata)>> {
    path.map(|path| {
        let document = load_json_object_file(path, label)?;
        let metadata = std::fs::metadata(path)
            .map_err(|error| message(format!("Failed to stat {}: {}", path.display(), error)))?;
        Ok((document, metadata))
    })
    .transpose()
}

fn stamp_live_domain_freshness(
    mut domain: ProjectDomainStatus,
    samples: &[ProjectStatusFreshnessSample<'_>],
) -> ProjectDomainStatus {
    domain.freshness = if samples.is_empty() {
        build_live_project_status_freshness_from_source_count(domain.source_kinds.len())
    } else {
        build_live_project_status_freshness_from_samples(samples)
    };
    domain
}

fn build_live_overall_freshness(domains: &[ProjectDomainStatus]) -> ProjectStatusFreshness {
    let mut ages = Vec::new();
    let mut source_count = 0usize;
    for domain in domains {
        source_count += domain.freshness.source_count;
        if let Some(age) = domain.freshness.newest_age_seconds {
            ages.push(age);
        }
        if let Some(age) = domain.freshness.oldest_age_seconds {
            ages.push(age);
        }
    }
    build_live_project_status_freshness(source_count, &ages)
}

pub(crate) fn build_live_project_status(args: &ProjectStatusLiveArgs) -> Result<ProjectStatus> {
    let api = build_live_project_status_api_client(args)?;
    let client = api.http_client().clone();
    let sync_summary_document = load_optional_project_status_document_with_metadata(
        args.sync_summary_file.as_ref(),
        "Project status sync summary input",
    )?;
    let bundle_preflight_document = load_optional_project_status_document_with_metadata(
        args.bundle_preflight_file.as_ref(),
        "Project status bundle preflight input",
    )?;
    let promotion_summary_document = load_optional_project_status_document_with_metadata(
        args.promotion_summary_file.as_ref(),
        "Project status promotion summary input",
    )?;
    let promotion_mapping_document = load_optional_project_status_document_with_metadata(
        args.mapping_file.as_ref(),
        "Project status mapping input",
    )?;
    let availability_document = load_optional_project_status_document_with_metadata(
        args.availability_file.as_ref(),
        "Project status availability input",
    )?;
    let all_org_domain_statuses = if args.all_orgs {
        Some(project_status_live::list_visible_orgs(&client))
    } else {
        None
    };
    let dashboard_status = if let Some(orgs_result) = all_org_domain_statuses.as_ref() {
        match orgs_result {
            Ok(orgs) if !orgs.is_empty() => {
                build_live_multi_org_domain_status(&api, orgs, build_live_dashboard_status)
                    .unwrap_or_else(|_| {
                        build_live_read_failed_domain_status(
                            "dashboard",
                            "live-dashboard-read",
                            "live-dashboard-search",
                            "live.dashboardCount",
                            "restore dashboard/org read access, then re-run live status --all-orgs",
                        )
                    })
            }
            Ok(_) => build_live_dashboard_status(&client),
            Err(_) => build_live_read_failed_domain_status(
                "dashboard",
                "live-dashboard-read",
                "live-org-list",
                "live.dashboardCount",
                "restore org list access, then re-run live status --all-orgs",
            ),
        }
    } else {
        build_live_dashboard_status(&client)
    };
    let datasource_status = if let Some(orgs_result) = all_org_domain_statuses.as_ref() {
        match orgs_result {
            Ok(orgs) if !orgs.is_empty() => {
                build_live_multi_org_domain_status(&api, orgs, build_live_datasource_status)
                    .unwrap_or_else(|_| {
                        build_live_read_failed_domain_status(
                    "datasource",
                    "live-inventory",
                    "live-datasource-list",
                    "live.datasourceCount",
                    "restore datasource/org read access, then re-run live status --all-orgs",
                )
                    })
            }
            Ok(_) => build_live_datasource_status(&client),
            Err(_) => build_live_read_failed_domain_status(
                "datasource",
                "live-inventory",
                "live-org-list",
                "live.datasourceCount",
                "restore org list access, then re-run live status --all-orgs",
            ),
        }
    } else {
        build_live_datasource_status(&client)
    };
    let alert_status = if let Some(orgs_result) = all_org_domain_statuses.as_ref() {
        match orgs_result {
            Ok(orgs) if !orgs.is_empty() => {
                build_live_multi_org_domain_status(&api, orgs, build_live_alert_status)
                    .unwrap_or_else(|_| {
                        build_live_read_failed_domain_status(
                            "alert",
                            "live-alert-surfaces",
                            "alert",
                            "live.alertRuleCount",
                            "restore alert/org read access, then re-run live status --all-orgs",
                        )
                    })
            }
            Ok(_) => build_live_alert_status(&client),
            Err(_) => build_live_read_failed_domain_status(
                "alert",
                "live-alert-surfaces",
                "live-org-list",
                "live.alertRuleCount",
                "restore org list access, then re-run live status --all-orgs",
            ),
        }
    } else {
        build_live_alert_status(&client)
    };
    let access_status = if let Some(orgs_result) = all_org_domain_statuses.as_ref() {
        match orgs_result {
            Ok(orgs) if !orgs.is_empty() => {
                build_live_multi_org_domain_status(&api, orgs, build_live_access_status)
                    .unwrap_or_else(|_| {
                        build_live_read_failed_domain_status(
                            "access",
                            "live-list-surfaces",
                            "grafana-utils-access-live-org-users",
                            "live.users.count",
                            "restore access/org read access, then re-run live status --all-orgs",
                        )
                    })
            }
            Ok(_) => build_live_access_status(&client),
            Err(_) => build_live_read_failed_domain_status(
                "access",
                "live-list-surfaces",
                "live-org-list",
                "live.users.count",
                "restore org list access, then re-run live status --all-orgs",
            ),
        }
    } else {
        build_live_access_status(&client)
    };
    let domains = vec![
        dashboard_status,
        datasource_status,
        alert_status,
        access_status,
        build_live_sync_status(
            sync_summary_document.as_ref().map(|(document, _)| document),
            bundle_preflight_document
                .as_ref()
                .map(|(document, _)| document),
            sync_summary_document.as_ref().map(|(_, metadata)| metadata),
            bundle_preflight_document
                .as_ref()
                .map(|(_, metadata)| metadata),
        ),
        build_live_promotion_status(
            promotion_summary_document
                .as_ref()
                .map(|(document, _)| document),
            promotion_mapping_document
                .as_ref()
                .map(|(document, _)| document),
            availability_document.as_ref().map(|(document, _)| document),
            promotion_summary_document
                .as_ref()
                .map(|(_, metadata)| metadata),
            promotion_mapping_document
                .as_ref()
                .map(|(_, metadata)| metadata),
            availability_document.as_ref().map(|(_, metadata)| metadata),
        ),
    ];
    let overall_freshness = build_live_overall_freshness(&domains);
    let mut status = build_project_status(
        PROJECT_STATUS_LIVE_SCOPE,
        PROJECT_STATUS_DOMAIN_COUNT,
        overall_freshness,
        domains,
    );
    status.discovery = Some(build_live_status_discovery(&client));
    Ok(status)
}
