//! Parser/runtime helpers for dashboard CLI commands.
use clap::Parser;

use crate::common::{resolve_auth_headers, Result};
use crate::dashboard::{DEFAULT_TIMEOUT, DEFAULT_URL};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};
use crate::profile_config::{
    load_selected_profile, resolve_connection_settings, ConnectionMergeInput,
};

use super::{
    CommonCliArgs, DashboardCliArgs, DashboardCommand, DryRunOutputFormat, SimpleOutputFormat,
};

/// Shared Grafana connection/authentication runtime state for dashboard commands.
#[derive(Debug, Clone)]
pub struct DashboardAuthContext {
    pub url: String,
    pub timeout: u64,
    pub verify_ssl: bool,
    pub auth_mode: String,
    pub headers: Vec<(String, String)>,
}

/// Parse dashboard CLI argv and normalize output-format aliases to keep
/// downstream handlers deterministic.
pub fn parse_cli_from<I, T>(iter: I) -> DashboardCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    normalize_dashboard_cli_args(DashboardCliArgs::parse_from(iter))
}

pub(super) fn parse_dashboard_import_output_column(
    value: &str,
) -> std::result::Result<String, String> {
    match value {
        "uid" => Ok("uid".to_string()),
        "destination" => Ok("destination".to_string()),
        "action" => Ok("action".to_string()),
        "folder_path" | "folderPath" => Ok("folder_path".to_string()),
        "source_folder_path" | "sourceFolderPath" => Ok("source_folder_path".to_string()),
        "destination_folder_path" | "destinationFolderPath" => {
            Ok("destination_folder_path".to_string())
        }
        "reason" => Ok("reason".to_string()),
        "file" => Ok("file".to_string()),
        _ => Err(format!(
            "Unsupported --output-columns value '{value}'. Supported values: uid, destination, action, folder_path, source_folder_path, destination_folder_path, reason, file."
        )),
    }
}

pub(super) fn parse_dashboard_list_output_column(
    value: &str,
) -> std::result::Result<String, String> {
    match value {
        "uid" => Ok("uid".to_string()),
        "name" => Ok("name".to_string()),
        "folder" => Ok("folder".to_string()),
        "folder_uid" | "folderUid" => Ok("folder_uid".to_string()),
        "path" => Ok("path".to_string()),
        "org" => Ok("org".to_string()),
        "org_id" | "orgId" => Ok("org_id".to_string()),
        "sources" => Ok("sources".to_string()),
        "source_uids" | "sourceUids" => Ok("source_uids".to_string()),
        _ => Err(format!(
            "Unsupported --output-columns value '{value}'. Supported values: uid, name, folder, folder_uid, path, org, org_id, sources, source_uids."
        )),
    }
}

pub(super) fn parse_inspect_report_column(value: &str) -> std::result::Result<String, String> {
    match value {
        "all" => Ok("all".to_string()),
        "org" => Ok("org".to_string()),
        "org_id" | "orgId" => Ok("org_id".to_string()),
        "dashboard_uid" | "dashboardUid" => Ok("dashboard_uid".to_string()),
        "dashboard_title" | "dashboardTitle" => Ok("dashboard_title".to_string()),
        "dashboard_tags" | "dashboardTags" => Ok("dashboard_tags".to_string()),
        "folder_path" | "folderPath" => Ok("folder_path".to_string()),
        "folder_full_path" | "folderFullPath" => Ok("folder_full_path".to_string()),
        "folder_level" | "folderLevel" => Ok("folder_level".to_string()),
        "folder_uid" | "folderUid" => Ok("folder_uid".to_string()),
        "parent_folder_uid" | "parentFolderUid" => Ok("parent_folder_uid".to_string()),
        "panel_id" | "panelId" => Ok("panel_id".to_string()),
        "panel_title" | "panelTitle" => Ok("panel_title".to_string()),
        "panel_type" | "panelType" => Ok("panel_type".to_string()),
        "panel_target_count" | "panelTargetCount" => Ok("panel_target_count".to_string()),
        "panel_query_count" | "panelQueryCount" => Ok("panel_query_count".to_string()),
        "panel_datasource_count" | "panelDatasourceCount" => {
            Ok("panel_datasource_count".to_string())
        }
        "panel_variables" | "panelVariables" => Ok("panel_variables".to_string()),
        "ref_id" | "refId" => Ok("ref_id".to_string()),
        "datasource" => Ok("datasource".to_string()),
        "datasource_name" | "datasourceName" => Ok("datasource_name".to_string()),
        "datasource_uid" | "datasourceUid" => Ok("datasource_uid".to_string()),
        "datasource_org" | "datasourceOrg" => Ok("datasource_org".to_string()),
        "datasource_org_id" | "datasourceOrgId" => Ok("datasource_org_id".to_string()),
        "datasource_database" | "datasourceDatabase" => Ok("datasource_database".to_string()),
        "datasource_bucket" | "datasourceBucket" => Ok("datasource_bucket".to_string()),
        "datasource_organization" | "datasourceOrganization" => {
            Ok("datasource_organization".to_string())
        }
        "datasource_index_pattern" | "datasourceIndexPattern" => {
            Ok("datasource_index_pattern".to_string())
        }
        "datasource_type" | "datasourceType" => Ok("datasource_type".to_string()),
        "datasource_family" | "datasourceFamily" => Ok("datasource_family".to_string()),
        "query_field" | "queryField" => Ok("query_field".to_string()),
        "target_hidden" | "targetHidden" => Ok("target_hidden".to_string()),
        "target_disabled" | "targetDisabled" => Ok("target_disabled".to_string()),
        "query_variables" | "queryVariables" => Ok("query_variables".to_string()),
        "metrics" => Ok("metrics".to_string()),
        "functions" => Ok("functions".to_string()),
        "measurements" => Ok("measurements".to_string()),
        "buckets" => Ok("buckets".to_string()),
        "query" => Ok("query".to_string()),
        "file" => Ok("file".to_string()),
        _ => Err(format!(
            "Unsupported --report-columns value '{value}'. Supported values: all, org, org_id, dashboard_uid, dashboard_title, dashboard_tags, folder_path, folder_full_path, folder_level, folder_uid, parent_folder_uid, panel_id, panel_title, panel_type, panel_target_count, panel_query_count, panel_datasource_count, panel_variables, ref_id, datasource, datasource_name, datasource_uid, datasource_org, datasource_org_id, datasource_database, datasource_bucket, datasource_organization, datasource_index_pattern, datasource_type, datasource_family, query_field, target_hidden, target_disabled, query_variables, metrics, functions, measurements, buckets, query, file."
        )),
    }
}

fn normalize_simple_output_format(
    text: &mut bool,
    table: &mut bool,
    csv: &mut bool,
    json: &mut bool,
    yaml: &mut bool,
    output_format: Option<SimpleOutputFormat>,
) {
    match output_format {
        Some(SimpleOutputFormat::Text) => *text = true,
        Some(SimpleOutputFormat::Table) => *table = true,
        Some(SimpleOutputFormat::Csv) => *csv = true,
        Some(SimpleOutputFormat::Json) => *json = true,
        Some(SimpleOutputFormat::Yaml) => *yaml = true,
        None => {}
    }
}

fn normalize_dry_run_output_format(
    table: &mut bool,
    json: &mut bool,
    output_format: Option<DryRunOutputFormat>,
) {
    match output_format {
        Some(DryRunOutputFormat::Table) => *table = true,
        Some(DryRunOutputFormat::Json) => *json = true,
        Some(DryRunOutputFormat::Text) | None => {}
    }
}

/// Normalize dashboard subcommand variants so legacy and explicit flags end up with
/// the same boolean state contract for command handlers.
pub fn normalize_dashboard_cli_args(mut args: DashboardCliArgs) -> DashboardCliArgs {
    match &mut args.command {
        DashboardCommand::List(list_args) => normalize_simple_output_format(
            &mut list_args.text,
            &mut list_args.table,
            &mut list_args.csv,
            &mut list_args.json,
            &mut list_args.yaml,
            list_args.output_format,
        ),
        DashboardCommand::Import(import_args) => normalize_dry_run_output_format(
            &mut import_args.table,
            &mut import_args.json,
            import_args.output_format,
        ),
        DashboardCommand::Delete(delete_args) => normalize_dry_run_output_format(
            &mut delete_args.table,
            &mut delete_args.json,
            delete_args.output_format,
        ),
        _ => {}
    }
    args
}

pub fn build_auth_context(common: &CommonCliArgs) -> Result<DashboardAuthContext> {
    let selected_profile = load_selected_profile(common.profile.as_deref())?;
    let resolved = resolve_connection_settings(
        ConnectionMergeInput {
            url: &common.url,
            url_default: DEFAULT_URL,
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            org_id: None,
            timeout: common.timeout,
            timeout_default: DEFAULT_TIMEOUT,
            verify_ssl: common.verify_ssl,
            insecure: false,
            ca_cert: None,
        },
        selected_profile.as_ref(),
    )?;
    let token = if common.prompt_token && common.api_token.is_none() {
        None
    } else {
        resolved.api_token.as_deref()
    };
    let username = if common.prompt_password {
        common.username.as_deref().or(resolved.username.as_deref())
    } else {
        resolved.username.as_deref()
    };
    let password = if common.prompt_password && common.password.is_none() {
        None
    } else {
        resolved.password.as_deref()
    };
    let headers = resolve_auth_headers(
        token,
        username,
        password,
        common.prompt_password,
        common.prompt_token,
    )?;
    let auth_mode = headers
        .iter()
        .find(|(name, _)| name == "Authorization")
        .map(|(_, value)| {
            if value.starts_with("Basic ") {
                "basic".to_string()
            } else {
                "token".to_string()
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    Ok(DashboardAuthContext {
        url: resolved.url,
        timeout: resolved.timeout,
        verify_ssl: resolved.verify_ssl,
        auth_mode,
        headers,
    })
}

pub fn build_http_client(common: &CommonCliArgs) -> Result<JsonHttpClient> {
    let context = build_auth_context(common)?;
    JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url,
        headers: context.headers,
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })
}

pub fn build_http_client_for_org(common: &CommonCliArgs, org_id: i64) -> Result<JsonHttpClient> {
    let mut context = build_auth_context(common)?;
    context
        .headers
        .push(("X-Grafana-Org-Id".to_string(), org_id.to_string()));
    JsonHttpClient::new(JsonHttpClientConfig {
        base_url: context.url,
        headers: context.headers,
        timeout_secs: context.timeout,
        verify_ssl: context.verify_ssl,
    })
}
