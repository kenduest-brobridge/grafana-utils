//! Shared project-status command surface.
//!
//! Maintainer note:
//! - This module owns the top-level `grafana-util project-status ...` command.
//! - It should stay focused on command args, shared rendering, and high-level
//!   staged/live aggregation handoff.
//! - Domain-specific staged/live producer logic belongs in the owning domain
//!   modules, not here.

use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Method;
use serde_json::{Map, Value};
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
    build_project_status, status_finding, ProjectDomainStatus, ProjectStatus,
    ProjectStatusFreshness, PROJECT_STATUS_PARTIAL,
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
const PROJECT_STATUS_LIVE_READ_FAILED: &str = "live-read-failed";
const PROJECT_STATUS_TIMESTAMP_FIELDS: &[&str] =
    &["updated", "updatedAt", "modified", "createdAt", "created"];

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
    #[arg(long, help = "Desired sync file to summarize from staged artifacts.")]
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
        help = "Optional staged sync-summary JSON used to deepen live sync status."
    )]
    pub sync_summary_file: Option<PathBuf>,
    #[arg(
        long,
        help = "Optional staged bundle-preflight JSON used to deepen live sync status."
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
    #[command(about = "Render project status from staged artifacts. Use exported project inputs.")]
    Staged(ProjectStatusStagedArgs),
    #[command(
        about = "Render project status from live Grafana read surfaces. Use current Grafana state plus optional staged context files."
    )]
    Live(ProjectStatusLiveArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util project-status",
    about = "Render project-wide staged or live status through the shared project-status contract. Staged subcommands read exports; live subcommands query Grafana."
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
                "restore datasource read access, then re-run live project-status",
            ),
        },
        Err(_) => build_live_read_failed_domain_status(
            "dashboard",
            "live-dashboard-read",
            "live-dashboard-search",
            "live.dashboardCount",
            "restore dashboard search access, then re-run live project-status",
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
                    "restore datasource inventory access, then re-run live project-status",
                )
            })
        }
        Err(_) => build_live_read_failed_domain_status(
            "datasource",
            "live-inventory",
            "live-datasource-list",
            "live.datasourceCount",
            "restore datasource inventory access, then re-run live project-status",
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
            "restore alert read access, then re-run live project-status",
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

fn build_live_access_status(client: &JsonHttpClient) -> ProjectDomainStatus {
    let status = build_access_live_domain_status(client).unwrap_or_else(|| {
        build_live_read_failed_domain_status(
            "access",
            "live-list-surfaces",
            "grafana-utils-access-live-org-users",
            "live.users.count",
            "restore access read scopes, then re-run live project-status",
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
    let domains = vec![
        build_live_dashboard_status(&client),
        build_live_datasource_status(&client),
        build_live_alert_status(&client),
        build_live_access_status(&client),
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
    let headers = resolve_auth_headers(
        args.api_token.as_deref(),
        args.username.as_deref(),
        args.password.as_deref(),
        args.prompt_password,
        args.prompt_token,
    )?;
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

pub fn run_project_status_staged(args: ProjectStatusStagedArgs) -> Result<()> {
    let overview_args = staged_args_to_overview_args(&args);
    let artifacts = overview::build_overview_artifacts(&overview_args)?;
    let document = overview::build_overview_document(artifacts)?;
    match args.output {
        ProjectStatusOutputFormat::Text => {
            for line in render_project_status_text(&document.project_status) {
                println!("{line}");
            }
            Ok(())
        }
        ProjectStatusOutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&document.project_status)?
            );
            Ok(())
        }
        #[cfg(feature = "tui")]
        ProjectStatusOutputFormat::Interactive => {
            run_project_status_interactive(document.project_status)
        }
    }
}

pub fn run_project_status_live(args: ProjectStatusLiveArgs) -> Result<()> {
    let status = build_live_project_status(&args)?;
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
        build_live_dashboard_status_with_request, build_live_promotion_status,
        build_live_sync_status, project_status_freshness_samples_from_value,
    };
    use crate::project_status::PROJECT_STATUS_PARTIAL;
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
}
