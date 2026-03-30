//! Shared status command surface.
//!
//! Maintainer note:
//! - This module owns the top-level `grafana-util status ...` command.
//! - It should stay focused on command args, shared rendering, and high-level
//!   staged/live aggregation handoff.
//! - Domain-specific staged/live producer logic belongs in the owning domain
//!   modules, not here.

use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::Metadata;
use std::path::PathBuf;

use crate::access::build_access_live_domain_status;
use crate::alert::{build_alert_live_project_status_domain, AlertLiveProjectStatusInputs};
use crate::common::{
    load_json_object_file, message, resolve_auth_headers, value_as_object, Result,
};
use crate::dashboard::{
    build_live_dashboard_domain_status, list_dashboard_summaries_with_request,
    list_datasources_with_request, DEFAULT_PAGE_SIZE,
};
use crate::datasource_live_project_status::{
    build_datasource_live_project_status, DatasourceLiveProjectStatusInputs,
};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};
use crate::overview::{self, OverviewArgs, OverviewOutputFormat};
use crate::project_status::{
    build_project_status, status_finding, ProjectDomainStatus, ProjectStatus, ProjectStatusFinding,
    ProjectStatusFreshness, PROJECT_STATUS_BLOCKED, PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY,
};
use crate::project_status_freshness::{
    build_live_project_status_freshness, build_live_project_status_freshness_from_samples,
    build_live_project_status_freshness_from_source_count, ProjectStatusFreshnessSample,
};
use crate::sync::{
    build_live_promotion_domain_status_transport, build_live_promotion_project_status,
    build_live_sync_domain_status, build_live_sync_domain_status_transport,
    LivePromotionProjectStatusInputs, SyncLiveProjectStatusInputs,
};

const PROJECT_STATUS_DOMAIN_COUNT: usize = 6;
const PROJECT_STATUS_LIVE_SCOPE: &str = "live";
const PROJECT_STATUS_LIVE_ALL_ORGS_MODE_SUFFIX: &str = "-all-orgs";
const PROJECT_STATUS_LIVE_READ_FAILED: &str = "live-read-failed";
const PROJECT_STATUS_LIVE_ALL_ORGS_AGGREGATE: &str = "multi-org-aggregate";
const PROJECT_STATUS_TIMESTAMP_FIELDS: &[&str] =
    &["updated", "updatedAt", "modified", "createdAt", "created"];
const PROJECT_STATUS_HELP_TEXT: &str = "Examples:\n\n  Render staged project status as JSON:\n    grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output json\n\n  Render live project status with staged sync context:\n    grafana-util status live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --sync-summary-file ./sync-summary.json --bundle-preflight-file ./bundle-preflight.json --output json";
const PROJECT_STATUS_STAGED_HELP_TEXT: &str = "Examples:\n\n  Render staged project status as JSON:\n    grafana-util status staged --dashboard-export-dir ./dashboards/raw --desired-file ./desired.json --output json\n\n  Render staged project status in the interactive workbench:\n    grafana-util status staged --dashboard-export-dir ./dashboards/raw --alert-export-dir ./alerts --output interactive";
const PROJECT_STATUS_LIVE_HELP_TEXT: &str = "Examples:\n\n  Render live project status as JSON:\n    grafana-util status live --url http://localhost:3000 --token \"$GRAFANA_API_TOKEN\" --output json\n\n  Render live status across visible orgs while layering staged sync context:\n    grafana-util status live --url http://localhost:3000 --basic-user admin --basic-password admin --all-orgs --sync-summary-file ./sync-summary.json --output interactive";

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ProjectStatusOutputFormat {
    Text,
    Json,
    #[cfg(feature = "tui")]
    Interactive,
}

#[derive(Debug, Clone, Args)]
pub struct ProjectStatusStagedArgs {
    #[arg(
        long,
        help = "Dashboard export directory to summarize from staged artifacts."
    )]
    pub dashboard_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Datasource export directory to summarize from staged artifacts."
    )]
    pub datasource_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access user export directory to summarize from staged artifacts."
    )]
    pub access_user_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access team export directory to summarize from staged artifacts."
    )]
    pub access_team_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access org export directory to summarize from staged artifacts."
    )]
    pub access_org_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Access service-account export directory to summarize from staged artifacts."
    )]
    pub access_service_account_export_dir: Option<PathBuf>,
    #[arg(long, help = "Desired change file to summarize from staged artifacts.")]
    pub desired_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Source bundle JSON file used by staged bundle/promotion checks."
    )]
    pub source_bundle: Option<PathBuf>,
    #[arg(
        long,
        help = "Target inventory JSON file used by staged bundle/promotion checks."
    )]
    pub target_inventory: Option<PathBuf>,
    #[arg(
        long,
        help = "Alert export directory to summarize from staged artifacts."
    )]
    pub alert_export_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional availability JSON reused by staged preflight builders."
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional mapping JSON reused by staged promotion builders."
    )]
    pub mapping_file: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = ProjectStatusOutputFormat::Text)]
    pub output: ProjectStatusOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct ProjectStatusLiveArgs {
    #[arg(
        long,
        default_value = "http://localhost:3000",
        help = "Grafana base URL."
    )]
    pub url: String,
    #[arg(
        long = "token",
        visible_alias = "api-token",
        help = "Grafana API token. Preferred flag: --token. Falls back to GRAFANA_API_TOKEN."
    )]
    pub api_token: Option<String>,
    #[arg(
        long = "basic-user",
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME."
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        help = "Grafana Basic auth password. Preferred flag: --basic-password. Falls back to GRAFANA_PASSWORD."
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana Basic auth password."
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana API token."
    )]
    pub prompt_token: bool,
    #[arg(long, default_value_t = 30, help = "HTTP timeout in seconds.")]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable TLS certificate verification. Verification is disabled by default."
    )]
    pub verify_ssl: bool,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["verify_ssl", "ca_cert"],
        help = "Disable TLS certificate verification explicitly."
    )]
    pub insecure: bool,
    #[arg(
        long = "ca-cert",
        value_name = "PATH",
        help = "PEM bundle file to trust for Grafana TLS verification."
    )]
    pub ca_cert: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Query live status across all Grafana organizations where the domain supports it."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        help = "Grafana organization id to scope live reads where supported."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        help = "Optional staged change-summary JSON used to deepen live change status."
    )]
    pub sync_summary_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional staged bundle-preflight JSON used to deepen live change status."
    )]
    pub bundle_preflight_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional staged promotion-preflight JSON used to deepen live promotion status."
    )]
    pub promotion_summary_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional staged promotion mapping JSON used to deepen live promotion status."
    )]
    pub mapping_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional staged availability JSON used to deepen live promotion status."
    )]
    pub availability_file: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = ProjectStatusOutputFormat::Text)]
    pub output: ProjectStatusOutputFormat,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ProjectStatusSubcommand {
    #[command(
        about = "Render project status from staged artifacts. Use exported project inputs.",
        after_help = PROJECT_STATUS_STAGED_HELP_TEXT
    )]
    Staged(ProjectStatusStagedArgs),
    #[command(
        about = "Render project status from live Grafana read surfaces. Use current Grafana state plus optional staged context files.",
        after_help = PROJECT_STATUS_LIVE_HELP_TEXT
    )]
    Live(ProjectStatusLiveArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util status",
    about = "Render project-wide staged or live status through the shared status contract. Staged subcommands read exports; live subcommands query Grafana.",
    after_help = PROJECT_STATUS_HELP_TEXT
)]
pub struct ProjectStatusCliArgs {
    #[command(subcommand)]
    pub command: ProjectStatusSubcommand,
}

fn staged_args_to_overview_args(args: &ProjectStatusStagedArgs) -> OverviewArgs {
    OverviewArgs {
        dashboard_export_dir: args.dashboard_export_dir.clone(),
        datasource_export_dir: args.datasource_export_dir.clone(),
        access_user_export_dir: args.access_user_export_dir.clone(),
        access_team_export_dir: args.access_team_export_dir.clone(),
        access_org_export_dir: args.access_org_export_dir.clone(),
        access_service_account_export_dir: args.access_service_account_export_dir.clone(),
        desired_file: args.desired_file.clone(),
        source_bundle: args.source_bundle.clone(),
        target_inventory: args.target_inventory.clone(),
        alert_export_dir: args.alert_export_dir.clone(),
        availability_file: args.availability_file.clone(),
        mapping_file: args.mapping_file.clone(),
        output: OverviewOutputFormat::Text,
    }
}

fn request_json_best_effort(
    client: &JsonHttpClient,
    path: &str,
    params: &[(String, String)],
) -> Option<Value> {
    client
        .request_json(Method::GET, path, params, None)
        .ok()
        .flatten()
}

fn request_object_list(
    client: &JsonHttpClient,
    path: &str,
    params: &[(String, String)],
    error_message: &str,
) -> Result<Vec<Map<String, Value>>> {
    match client.request_json(Method::GET, path, params, None)? {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| Ok(value_as_object(item, error_message)?.clone()))
            .collect(),
        Some(_) => Err(message(error_message)),
        None => Ok(Vec::new()),
    }
}

fn request_object_optional(client: &JsonHttpClient, path: &str) -> Option<Map<String, Value>> {
    client
        .request_json(Method::GET, path, &[], None)
        .ok()
        .flatten()
        .and_then(|value| value.as_object().cloned())
}

fn build_live_read_failed_domain_status(
    id: &str,
    mode: &str,
    source_kind: &str,
    signal_key: &str,
    action: &str,
) -> ProjectDomainStatus {
    ProjectDomainStatus {
        id: id.to_string(),
        scope: PROJECT_STATUS_LIVE_SCOPE.to_string(),
        mode: mode.to_string(),
        status: PROJECT_STATUS_PARTIAL.to_string(),
        reason_code: PROJECT_STATUS_LIVE_READ_FAILED.to_string(),
        primary_count: 0,
        blocker_count: 1,
        warning_count: 0,
        source_kinds: vec![source_kind.to_string()],
        signal_keys: vec![signal_key.to_string()],
        blockers: vec![status_finding(
            PROJECT_STATUS_LIVE_READ_FAILED,
            1,
            signal_key,
        )],
        warnings: Vec::new(),
        next_actions: vec![action.to_string()],
        freshness: ProjectStatusFreshness::default(),
    }
}

fn load_optional_project_status_document_with_metadata(
    path: Option<&PathBuf>,
    label: &str,
) -> Result<Option<(Value, Metadata)>> {
    path.map(|path| {
        let document = load_json_object_file(path, label)?;
        let metadata = std::fs::metadata(path)
            .map_err(|error| message(&format!("Failed to stat {}: {}", path.display(), error)))?;
        Ok((document, metadata))
    })
    .transpose()
}

fn project_status_timestamp_from_object<'a>(object: &'a Map<String, Value>) -> Option<&'a str> {
    for key in PROJECT_STATUS_TIMESTAMP_FIELDS {
        if let Some(observed_at) = object.get(*key).and_then(Value::as_str) {
            let observed_at = observed_at.trim();
            if !observed_at.is_empty() {
                return Some(observed_at);
            }
        }
    }
    None
}

fn project_status_freshness_samples_from_value<'a>(
    source: &'static str,
    value: &'a Value,
) -> Vec<ProjectStatusFreshnessSample<'a>> {
    match value {
        Value::Array(items) => items
            .iter()
            .flat_map(|item| project_status_freshness_samples_from_value(source, item))
            .collect(),
        Value::Object(object) => project_status_timestamp_from_object(object)
            .map(|observed_at| {
                vec![ProjectStatusFreshnessSample::ObservedAtRfc3339 {
                    source,
                    observed_at,
                }]
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn project_status_freshness_samples_from_records<'a>(
    source: &'static str,
    records: &'a [Map<String, Value>],
) -> Vec<ProjectStatusFreshnessSample<'a>> {
    records
        .iter()
        .filter_map(|record| {
            project_status_timestamp_from_object(record).map(|observed_at| {
                ProjectStatusFreshnessSample::ObservedAtRfc3339 {
                    source,
                    observed_at,
                }
            })
        })
        .collect()
}

fn first_dashboard_uid(dashboard_summaries: &[Map<String, Value>]) -> Option<&str> {
    dashboard_summaries.iter().find_map(|summary| {
        summary
            .get("uid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

fn latest_dashboard_version_timestamp<F>(
    request_json: &mut F,
    dashboard_summaries: &[Map<String, Value>],
) -> Option<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let uid = first_dashboard_uid(dashboard_summaries)?;
    let path = format!("/api/dashboards/uid/{uid}/versions");
    let params = vec![("limit".to_string(), "1".to_string())];
    let response = request_json(Method::GET, &path, &params, None)
        .ok()
        .flatten()?;
    let versions = match response {
        Value::Array(items) => items,
        Value::Object(object) => object
            .get("versions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    versions
        .first()
        .and_then(Value::as_object)
        .and_then(project_status_timestamp_from_object)
        .map(str::to_string)
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

fn build_live_dashboard_status_with_request<F>(mut request_json: F) -> ProjectDomainStatus
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match list_dashboard_summaries_with_request(&mut request_json, DEFAULT_PAGE_SIZE) {
        Ok(dashboard_summaries) => match list_datasources_with_request(&mut request_json) {
            Ok(datasources) => {
                let status = build_live_dashboard_domain_status(&dashboard_summaries, &datasources);
                let mut freshness_samples = project_status_freshness_samples_from_records(
                    "dashboard-search",
                    &dashboard_summaries,
                );
                freshness_samples.extend(project_status_freshness_samples_from_records(
                    "datasource-list",
                    &datasources,
                ));
                let dashboard_version_timestamp = if freshness_samples.is_empty() {
                    latest_dashboard_version_timestamp(&mut request_json, &dashboard_summaries)
                } else {
                    None
                };
                if let Some(observed_at) = dashboard_version_timestamp.as_deref() {
                    freshness_samples.push(ProjectStatusFreshnessSample::ObservedAtRfc3339 {
                        source: "dashboard-version-history",
                        observed_at,
                    });
                }
                stamp_live_domain_freshness(status, &freshness_samples)
            }
            Err(_) => build_live_read_failed_domain_status(
                "dashboard",
                "live-dashboard-read",
                "live-datasource-list",
                "live.datasourceCount",
                "restore datasource read access, then re-run live status",
            ),
        },
        Err(_) => build_live_read_failed_domain_status(
            "dashboard",
            "live-dashboard-read",
            "live-dashboard-search",
            "live.dashboardCount",
            "restore dashboard search access, then re-run live status",
        ),
    }
}

fn build_live_dashboard_status(client: &JsonHttpClient) -> ProjectDomainStatus {
    build_live_dashboard_status_with_request(|method, path, params, payload| {
        client.request_json(method, path, params, payload)
    })
}

fn build_live_datasource_status(client: &JsonHttpClient) -> ProjectDomainStatus {
    let status = match request_object_list(
        client,
        "/api/datasources",
        &[],
        "Unexpected datasource list response from Grafana.",
    ) {
        Ok(datasource_list) => {
            let org_list = request_object_list(
                client,
                "/api/orgs",
                &[],
                "Unexpected /api/orgs payload from Grafana.",
            )
            .ok();
            let current_org = request_object_optional(client, "/api/org");
            build_datasource_live_project_status(DatasourceLiveProjectStatusInputs {
                datasource_list: Some(&datasource_list),
                datasource_read: None,
                org_list: org_list.as_deref(),
                current_org: current_org.as_ref(),
            })
            .unwrap_or_else(|| {
                build_live_read_failed_domain_status(
                    "datasource",
                    "live-inventory",
                    "live-datasource-list",
                    "live.datasourceCount",
                    "restore datasource inventory access, then re-run live status",
                )
            })
        }
        Err(_) => build_live_read_failed_domain_status(
            "datasource",
            "live-inventory",
            "live-datasource-list",
            "live.datasourceCount",
            "restore datasource inventory access, then re-run live status",
        ),
    };
    stamp_live_domain_freshness(status, &[])
}

fn build_live_alert_status(client: &JsonHttpClient) -> ProjectDomainStatus {
    let rules_document = request_json_best_effort(client, "/api/v1/provisioning/alert-rules", &[]);
    let contact_points_document =
        request_json_best_effort(client, "/api/v1/provisioning/contact-points", &[]);
    let mute_timings_document =
        request_json_best_effort(client, "/api/v1/provisioning/mute-timings", &[]);
    let policies_document = request_json_best_effort(client, "/api/v1/provisioning/policies", &[]);
    let templates_document =
        request_json_best_effort(client, "/api/v1/provisioning/templates", &[]);
    let status = build_alert_live_project_status_domain(AlertLiveProjectStatusInputs {
        rules_document: rules_document.as_ref(),
        contact_points_document: contact_points_document.as_ref(),
        mute_timings_document: mute_timings_document.as_ref(),
        policies_document: policies_document.as_ref(),
        templates_document: templates_document.as_ref(),
    })
    .unwrap_or_else(|| {
        build_live_read_failed_domain_status(
            "alert",
            "live-alert-surfaces",
            "alert",
            "live.alertRuleCount",
            "restore alert read access, then re-run live status",
        )
    });
    let mut freshness_samples = Vec::new();
    if let Some(document) = rules_document.as_ref() {
        freshness_samples.extend(project_status_freshness_samples_from_value(
            "alert-rules",
            document,
        ));
    }
    if let Some(document) = contact_points_document.as_ref() {
        freshness_samples.extend(project_status_freshness_samples_from_value(
            "alert-contact-points",
            document,
        ));
    }
    if let Some(document) = mute_timings_document.as_ref() {
        freshness_samples.extend(project_status_freshness_samples_from_value(
            "alert-mute-timings",
            document,
        ));
    }
    if let Some(document) = policies_document.as_ref() {
        freshness_samples.extend(project_status_freshness_samples_from_value(
            "alert-policies",
            document,
        ));
    }
    if let Some(document) = templates_document.as_ref() {
        freshness_samples.extend(project_status_freshness_samples_from_value(
            "alert-templates",
            document,
        ));
    }
    stamp_live_domain_freshness(status, &freshness_samples)
}

fn project_status_severity_rank(status: &str) -> usize {
    match status {
        PROJECT_STATUS_BLOCKED => 0,
        PROJECT_STATUS_PARTIAL => 1,
        PROJECT_STATUS_READY => 2,
        _ => 3,
    }
}

fn list_visible_orgs(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    request_object_list(
        client,
        "/api/orgs",
        &[],
        "Unexpected /api/orgs payload from Grafana.",
    )
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
    let freshness = build_live_overall_freshness(
        &statuses
            .iter()
            .cloned()
            .collect::<Vec<ProjectDomainStatus>>(),
    );
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

    Ok(ProjectDomainStatus {
        id: aggregate.id.clone(),
        scope: aggregate.scope.clone(),
        mode,
        status: aggregate.status.clone(),
        reason_code,
        primary_count: statuses.iter().map(|status| status.primary_count).sum(),
        blocker_count: statuses.iter().map(|status| status.blocker_count).sum(),
        warning_count: statuses.iter().map(|status| status.warning_count).sum(),
        source_kinds,
        signal_keys,
        blockers,
        warnings,
        next_actions,
        freshness,
    })
}

fn build_live_multi_org_domain_status_with_orgs<F>(
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

fn build_live_multi_org_domain_status<F>(
    args: &ProjectStatusLiveArgs,
    orgs: &[Map<String, Value>],
    mut build_status: F,
) -> Result<ProjectDomainStatus>
where
    F: FnMut(&JsonHttpClient) -> ProjectDomainStatus,
{
    build_live_multi_org_domain_status_with_orgs(orgs, |org_id| {
        let client = build_live_project_status_client_for_org(args, Some(org_id))?;
        Ok(build_status(&client))
    })
}

fn build_live_access_status(client: &JsonHttpClient) -> ProjectDomainStatus {
    let status = build_access_live_domain_status(client).unwrap_or_else(|| {
        build_live_read_failed_domain_status(
            "access",
            "live-list-surfaces",
            "grafana-utils-access-live-org-users",
            "live.users.count",
            "restore access read scopes, then re-run live status",
        )
    });
    stamp_live_domain_freshness(status, &[])
}

fn build_live_sync_status(
    sync_summary_document: Option<&Value>,
    bundle_preflight_document: Option<&Value>,
    sync_summary_metadata: Option<&Metadata>,
    bundle_preflight_metadata: Option<&Metadata>,
) -> ProjectDomainStatus {
    let status = build_live_sync_domain_status(SyncLiveProjectStatusInputs {
        summary_document: sync_summary_document,
        bundle_preflight_document,
    })
    .unwrap_or_else(build_live_sync_domain_status_transport);
    let mut samples = Vec::new();
    if let Some(metadata) = sync_summary_metadata {
        samples.push(ProjectStatusFreshnessSample::ObservedAtMetadata {
            source: "sync-summary",
            metadata,
        });
    }
    if let Some(metadata) = bundle_preflight_metadata {
        samples.push(ProjectStatusFreshnessSample::ObservedAtMetadata {
            source: "bundle-preflight",
            metadata,
        });
    }
    stamp_live_domain_freshness(status, &samples)
}

fn build_live_promotion_status(
    promotion_summary_document: Option<&Value>,
    promotion_mapping_document: Option<&Value>,
    availability_document: Option<&Value>,
    promotion_summary_metadata: Option<&Metadata>,
    promotion_mapping_metadata: Option<&Metadata>,
    availability_metadata: Option<&Metadata>,
) -> ProjectDomainStatus {
    let status = build_live_promotion_project_status(LivePromotionProjectStatusInputs {
        promotion_summary_document,
        promotion_mapping_document,
        availability_document,
    })
    .unwrap_or_else(build_live_promotion_domain_status_transport);
    let mut samples = Vec::new();
    if let Some(metadata) = promotion_summary_metadata {
        samples.push(ProjectStatusFreshnessSample::ObservedAtMetadata {
            source: "promotion-summary",
            metadata,
        });
    }
    if let Some(metadata) = promotion_mapping_metadata {
        samples.push(ProjectStatusFreshnessSample::ObservedAtMetadata {
            source: "promotion-mapping",
            metadata,
        });
    }
    if let Some(metadata) = availability_metadata {
        samples.push(ProjectStatusFreshnessSample::ObservedAtMetadata {
            source: "availability",
            metadata,
        });
    }
    stamp_live_domain_freshness(status, &samples)
}

fn build_live_project_status(args: &ProjectStatusLiveArgs) -> Result<ProjectStatus> {
    let client = build_live_project_status_client(args)?;
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
        Some(list_visible_orgs(&client))
    } else {
        None
    };
    let dashboard_status = if let Some(orgs_result) = all_org_domain_statuses.as_ref() {
        match orgs_result {
            Ok(orgs) if !orgs.is_empty() => {
                build_live_multi_org_domain_status(args, orgs, build_live_dashboard_status)
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
            Ok(orgs) if !orgs.is_empty() => build_live_multi_org_domain_status(
                args,
                orgs,
                build_live_datasource_status,
            )
            .unwrap_or_else(|_| {
                build_live_read_failed_domain_status(
                    "datasource",
                    "live-inventory",
                    "live-datasource-list",
                    "live.datasourceCount",
                    "restore datasource/org read access, then re-run live status --all-orgs",
                )
            }),
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
                build_live_multi_org_domain_status(args, orgs, build_live_alert_status)
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
                build_live_multi_org_domain_status(args, orgs, build_live_access_status)
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
    Ok(build_project_status(
        PROJECT_STATUS_LIVE_SCOPE,
        PROJECT_STATUS_DOMAIN_COUNT,
        overall_freshness,
        domains,
    ))
}

#[cfg(feature = "tui")]
fn run_project_status_interactive(status: ProjectStatus) -> Result<()> {
    crate::project_status_tui::run_project_status_interactive(status)
}

#[cfg(not(feature = "tui"))]
fn run_project_status_interactive(_status: ProjectStatus) -> Result<()> {
    Err(crate::common::tui(
        "Project-status interactive mode requires the `tui` feature.",
    ))
}

pub(crate) fn render_project_status_text(status: &ProjectStatus) -> Vec<String> {
    let mut lines = vec![
        "Project status".to_string(),
        format!(
            "Overall: status={} scope={} domains={} present={} blocked={} blockers={} warnings={} freshness={}",
            status.overall.status,
            status.scope,
            status.overall.domain_count,
            status.overall.present_count,
            status.overall.blocked_count,
            status.overall.blocker_count,
            status.overall.warning_count,
            status.overall.freshness.status,
        ),
    ];
    if !status.domains.is_empty() {
        lines.push("Domains:".to_string());
        for domain in &status.domains {
            let mut line = format!(
                "- {} status={} mode={} primary={} blockers={} warnings={} freshness={}",
                domain.id,
                domain.status,
                domain.mode,
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
    if !status.top_blockers.is_empty() {
        lines.push("Top blockers:".to_string());
        for blocker in status.top_blockers.iter().take(5) {
            lines.push(format!(
                "- {} {} count={} source={}",
                blocker.domain, blocker.kind, blocker.count, blocker.source
            ));
        }
    }
    if !status.next_actions.is_empty() {
        lines.push("Next actions:".to_string());
        for action in status.next_actions.iter().take(5) {
            lines.push(format!(
                "- {} reason={} action={}",
                action.domain, action.reason_code, action.action
            ));
        }
    }
    lines
}

pub(crate) fn build_live_project_status_client(
    args: &ProjectStatusLiveArgs,
) -> Result<JsonHttpClient> {
    build_live_project_status_client_for_org(args, args.org_id)
}

fn resolve_live_project_status_headers(
    args: &ProjectStatusLiveArgs,
    org_id: Option<i64>,
) -> Result<Vec<(String, String)>> {
    let mut headers = resolve_auth_headers(
        args.api_token.as_deref(),
        args.username.as_deref(),
        args.password.as_deref(),
        args.prompt_password,
        args.prompt_token,
    )?;
    if let Some(org_id) = org_id {
        headers.push(("X-Grafana-Org-Id".to_string(), org_id.to_string()));
    }
    Ok(headers)
}

fn build_live_project_status_client_for_org(
    args: &ProjectStatusLiveArgs,
    org_id: Option<i64>,
) -> Result<JsonHttpClient> {
    let headers = resolve_live_project_status_headers(args, org_id)?;
    JsonHttpClient::new_with_ca_cert(
        JsonHttpClientConfig {
            base_url: args.url.clone(),
            headers,
            timeout_secs: args.timeout,
            verify_ssl: args.verify_ssl && !args.insecure,
        },
        args.ca_cert.as_deref(),
    )
}

/// Build the staged status document without rendering it.
pub fn execute_project_status_staged(args: &ProjectStatusStagedArgs) -> Result<ProjectStatus> {
    let overview_args = staged_args_to_overview_args(args);
    let artifacts = overview::build_overview_artifacts(&overview_args)?;
    let document = overview::build_overview_document(artifacts)?;
    Ok(document.project_status)
}

/// Build the live status document without rendering it.
pub fn execute_project_status_live(args: &ProjectStatusLiveArgs) -> Result<ProjectStatus> {
    build_live_project_status(args)
}

pub fn run_project_status_staged(args: ProjectStatusStagedArgs) -> Result<()> {
    let status = execute_project_status_staged(&args)?;
    match args.output {
        ProjectStatusOutputFormat::Text => {
            for line in render_project_status_text(&status) {
                println!("{line}");
            }
            Ok(())
        }
        ProjectStatusOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        #[cfg(feature = "tui")]
        ProjectStatusOutputFormat::Interactive => run_project_status_interactive(status),
    }
}

pub fn run_project_status_live(args: ProjectStatusLiveArgs) -> Result<()> {
    let status = execute_project_status_live(&args)?;
    match args.output {
        ProjectStatusOutputFormat::Text => {
            for line in render_project_status_text(&status) {
                println!("{line}");
            }
            Ok(())
        }
        ProjectStatusOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }
        #[cfg(feature = "tui")]
        ProjectStatusOutputFormat::Interactive => run_project_status_interactive(status),
    }
}

pub fn run_project_status_cli(args: ProjectStatusCliArgs) -> Result<()> {
    match args.command {
        ProjectStatusSubcommand::Staged(inner) => run_project_status_staged(inner),
        ProjectStatusSubcommand::Live(inner) => run_project_status_live(inner),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_live_dashboard_status_with_request, build_live_multi_org_domain_status_with_orgs,
        build_live_promotion_status, build_live_sync_status,
        project_status_freshness_samples_from_value, resolve_live_project_status_headers,
    };
    use crate::project_status::{
        status_finding, ProjectDomainStatus, ProjectStatusFreshness, PROJECT_STATUS_BLOCKED,
        PROJECT_STATUS_PARTIAL, PROJECT_STATUS_READY, PROJECT_STATUS_UNKNOWN,
    };
    use crate::project_status_freshness::build_live_project_status_freshness_from_samples;
    use chrono::{DateTime, Utc};
    use reqwest::Method;
    use serde_json::json;
    use std::fs;
    use std::time::{Duration, SystemTime};
    use tempfile::tempdir;

    const TEST_DASHBOARD_LIMIT: &str = "500";

    #[test]
    fn build_live_sync_status_uses_staged_input_metadata_for_freshness() {
        let dir = tempdir().unwrap();
        let summary_path = dir.path().join("sync-summary.json");
        let bundle_path = dir.path().join("bundle-preflight.json");
        fs::write(&summary_path, "{}").unwrap();
        fs::write(
            &bundle_path,
            r#"{"summary":{"resourceCount":1,"syncBlockingCount":0}}"#,
        )
        .unwrap();
        let summary_metadata = fs::metadata(&summary_path).unwrap();
        let bundle_metadata = fs::metadata(&bundle_path).unwrap();
        let summary_document = json!({"summary":{"resourceCount":1}});
        let bundle_document = json!({"summary":{"resourceCount":1,"syncBlockingCount":0}});

        let status = build_live_sync_status(
            Some(&summary_document),
            Some(&bundle_document),
            Some(&summary_metadata),
            Some(&bundle_metadata),
        );

        assert_eq!(status.freshness.status, "current");
        assert_eq!(status.freshness.source_count, 2);
        assert!(status.freshness.newest_age_seconds.is_some());
        assert!(status.freshness.oldest_age_seconds.is_some());
    }

    #[test]
    fn build_live_promotion_status_uses_staged_input_metadata_for_freshness() {
        let dir = tempdir().unwrap();
        let summary_path = dir.path().join("promotion-summary.json");
        let mapping_path = dir.path().join("mapping.json");
        let availability_path = dir.path().join("availability.json");
        fs::write(
            &summary_path,
            r#"{"summary":{"resourceCount":1,"blockingCount":0},"handoffSummary":{"readyForReview":false}}"#,
        )
        .unwrap();
        fs::write(&mapping_path, "{}").unwrap();
        fs::write(&availability_path, "{}").unwrap();
        let summary_metadata = fs::metadata(&summary_path).unwrap();
        let mapping_metadata = fs::metadata(&mapping_path).unwrap();
        let availability_metadata = fs::metadata(&availability_path).unwrap();
        let summary_document = json!({"summary":{"resourceCount":1,"blockingCount":0},"handoffSummary":{"readyForReview":false}});
        let mapping_document = json!({});
        let availability_document = json!({});

        let status = build_live_promotion_status(
            Some(&summary_document),
            Some(&mapping_document),
            Some(&availability_document),
            Some(&summary_metadata),
            Some(&mapping_metadata),
            Some(&availability_metadata),
        );

        assert_eq!(status.status, PROJECT_STATUS_PARTIAL);
        assert_eq!(status.freshness.status, "current");
        assert_eq!(status.freshness.source_count, 3);
        assert!(status.freshness.newest_age_seconds.is_some());
        assert!(status.freshness.oldest_age_seconds.is_some());
    }

    #[test]
    fn build_live_dashboard_status_uses_dashboard_version_history_for_freshness() {
        let status = build_live_dashboard_status_with_request(|method, path, params, _payload| {
            match (method, path) {
                (Method::GET, "/api/search") => {
                    assert!(params
                        .iter()
                        .any(|(key, value)| key == "type" && value == "dash-db"));
                    assert!(params
                        .iter()
                        .any(|(key, value)| key == "limit" && value == TEST_DASHBOARD_LIMIT));
                    Ok(Some(json!([
                        {
                            "uid": "cpu-main",
                            "title": "CPU Main",
                            "type": "dash-db",
                            "folderUid": "infra",
                            "folderTitle": "Infra"
                        }
                    ])))
                }
                (Method::GET, "/api/datasources") => Ok(Some(json!([
                    {
                        "uid": "prom-main",
                        "name": "Prometheus Main",
                        "type": "prometheus"
                    }
                ]))),
                (Method::GET, "/api/dashboards/uid/cpu-main/versions") => {
                    assert_eq!(params, &vec![("limit".to_string(), "1".to_string())]);
                    Ok(Some(json!([
                        {
                            "version": 7,
                            "created": "2026-03-26T10:00:00Z",
                            "createdBy": "admin"
                        }
                    ])))
                }
                _ => Err(crate::common::message(format!("unexpected request {path}"))),
            }
        });

        assert_eq!(status.status, "ready");
        assert_eq!(status.freshness.status, "current");
        assert_eq!(status.freshness.source_count, 1);
        assert!(status.freshness.newest_age_seconds.is_some());
        assert!(status.freshness.oldest_age_seconds.is_some());
    }

    #[test]
    fn project_status_freshness_samples_from_value_uses_timestamp_fields_from_arrays_and_objects() {
        let now = SystemTime::now();
        let updated_at = DateTime::<Utc>::from(now - Duration::from_secs(60)).to_rfc3339();
        let created_at = DateTime::<Utc>::from(now - Duration::from_secs(120)).to_rfc3339();
        let document = json!([
            {
                "uid": "rule-1",
                "updated": updated_at
            },
            {
                "uid": "rule-2",
                "created": created_at
            }
        ]);

        let samples = project_status_freshness_samples_from_value("alert-rules", &document);
        let freshness = build_live_project_status_freshness_from_samples(&samples);

        assert_eq!(samples.len(), 2);
        assert_eq!(freshness.status, "current");
        assert_eq!(freshness.source_count, 1);
        assert!(freshness.newest_age_seconds.is_some());
        assert!(freshness.oldest_age_seconds.is_some());
    }

    #[test]
    fn resolve_live_project_status_headers_adds_org_scope_when_requested() {
        let args = super::ProjectStatusLiveArgs {
            url: "http://localhost:3000".to_string(),
            api_token: Some("token-123".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
            insecure: false,
            ca_cert: None,
            all_orgs: false,
            org_id: Some(7),
            sync_summary_file: None,
            bundle_preflight_file: None,
            promotion_summary_file: None,
            mapping_file: None,
            availability_file: None,
            output: super::ProjectStatusOutputFormat::Text,
        };

        let headers = resolve_live_project_status_headers(&args, args.org_id).unwrap();

        assert!(headers
            .iter()
            .any(|(name, value)| { name == "X-Grafana-Org-Id" && value == "7" }));
    }

    #[test]
    fn build_live_multi_org_domain_status_with_orgs_fans_out_and_aggregates_counts() {
        let orgs = vec![
            json!({"id": 11, "name": "Core"})
                .as_object()
                .unwrap()
                .clone(),
            json!({"id": 22, "name": "Edge"})
                .as_object()
                .unwrap()
                .clone(),
        ];
        let mut seen_org_ids = Vec::new();

        let aggregated = build_live_multi_org_domain_status_with_orgs(&orgs, |org_id| {
            seen_org_ids.push(org_id);
            Ok(ProjectDomainStatus {
                id: "alert".to_string(),
                scope: "live".to_string(),
                mode: "live-alert-surfaces".to_string(),
                status: if org_id == 11 {
                    PROJECT_STATUS_READY.to_string()
                } else {
                    PROJECT_STATUS_BLOCKED.to_string()
                },
                reason_code: if org_id == 11 {
                    PROJECT_STATUS_READY.to_string()
                } else {
                    "blocked-by-blockers".to_string()
                },
                primary_count: if org_id == 11 { 3 } else { 5 },
                blocker_count: if org_id == 11 { 0 } else { 2 },
                warning_count: if org_id == 11 { 1 } else { 4 },
                source_kinds: vec!["alert".to_string()],
                signal_keys: vec![
                    "live.alertRuleCount".to_string(),
                    "live.policyCount".to_string(),
                ],
                blockers: if org_id == 11 {
                    Vec::new()
                } else {
                    vec![status_finding(
                        "missing-alert-policy",
                        2,
                        "live.policyCount",
                    )]
                },
                warnings: vec![status_finding(
                    "missing-panel-links",
                    if org_id == 11 { 1 } else { 4 },
                    "live.rulePanelMissingCount",
                )],
                next_actions: vec!["re-run alert checks".to_string()],
                freshness: ProjectStatusFreshness {
                    status: "current".to_string(),
                    source_count: 1,
                    newest_age_seconds: Some(if org_id == 11 { 15 } else { 40 }),
                    oldest_age_seconds: Some(if org_id == 11 { 30 } else { 55 }),
                },
            })
        })
        .unwrap();

        assert_eq!(seen_org_ids, vec![11, 22]);
        assert_eq!(aggregated.id, "alert");
        assert_eq!(aggregated.status, PROJECT_STATUS_BLOCKED);
        assert_eq!(aggregated.reason_code, "multi-org-aggregate");
        assert_eq!(aggregated.primary_count, 8);
        assert_eq!(aggregated.blocker_count, 2);
        assert_eq!(aggregated.warning_count, 5);
        assert_eq!(
            aggregated.blockers,
            vec![status_finding(
                "missing-alert-policy",
                2,
                "live.policyCount"
            )]
        );
        assert_eq!(
            aggregated.warnings,
            vec![status_finding(
                "missing-panel-links",
                5,
                "live.rulePanelMissingCount"
            )]
        );
        assert_eq!(
            aggregated.next_actions,
            vec!["re-run alert checks".to_string()]
        );
        assert_eq!(aggregated.freshness.status, "current");
        assert_eq!(aggregated.freshness.source_count, 2);
        assert_eq!(aggregated.freshness.newest_age_seconds, Some(15));
        assert_eq!(aggregated.freshness.oldest_age_seconds, Some(55));
    }

    #[test]
    fn build_live_multi_org_domain_status_with_orgs_rejects_empty_org_lists() {
        let error = build_live_multi_org_domain_status_with_orgs(&[], |_org_id| {
            Ok(ProjectDomainStatus {
                id: "dashboard".to_string(),
                scope: "live".to_string(),
                mode: "live-dashboard-read".to_string(),
                status: PROJECT_STATUS_UNKNOWN.to_string(),
                reason_code: "unknown".to_string(),
                primary_count: 0,
                blocker_count: 0,
                warning_count: 0,
                source_kinds: Vec::new(),
                signal_keys: Vec::new(),
                blockers: Vec::new(),
                warnings: Vec::new(),
                next_actions: Vec::new(),
                freshness: ProjectStatusFreshness::default(),
            })
        })
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("at least one per-org domain status"));
    }
}
