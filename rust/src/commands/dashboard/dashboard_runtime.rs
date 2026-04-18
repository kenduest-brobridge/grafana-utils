//! Parser/runtime helpers for dashboard CLI commands.
use clap::Parser;
use rpassword::prompt_password;

use crate::common::{GrafanaCliError, Result};
use crate::dashboard::DEFAULT_TIMEOUT;
use crate::grafana_api::{AuthInputs, GrafanaApiClient, GrafanaConnection};
use crate::http::JsonHttpClient;
use crate::profile_config::ConnectionMergeInput;

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
        "all" => Ok("all".to_string()),
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
            "Unsupported --output-columns value '{value}'. Supported values: all, uid, destination, action, folder_path, source_folder_path, destination_folder_path, reason, file."
        )),
    }
}

pub(super) fn parse_dashboard_list_output_column(
    value: &str,
) -> std::result::Result<String, String> {
    match value {
        "all" => Ok("all".to_string()),
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
            "Unsupported --output-columns value '{value}'. Supported values: all, uid, name, folder, folder_uid, path, org, org_id, sources, source_uids."
        )),
    }
}

pub(super) fn parse_dashboard_plan_output_column(
    value: &str,
) -> std::result::Result<String, String> {
    match value {
        "all" => Ok("all".to_string()),
        "action_id" | "actionId" => Ok("action_id".to_string()),
        "action" => Ok("action".to_string()),
        "status" => Ok("status".to_string()),
        "uid" | "dashboard_uid" | "dashboardUid" => Ok("dashboard_uid".to_string()),
        "title" | "dashboard_title" | "dashboardTitle" => Ok("dashboard_title".to_string()),
        "folder_uid" | "folderUid" => Ok("folder_uid".to_string()),
        "folder_path" | "folderPath" => Ok("folder_path".to_string()),
        "source_org_id" | "sourceOrgId" => Ok("source_org_id".to_string()),
        "source_org_name" | "sourceOrgName" => Ok("source_org_name".to_string()),
        "target_org_id" | "targetOrgId" => Ok("target_org_id".to_string()),
        "target_org_name" | "targetOrgName" => Ok("target_org_name".to_string()),
        "match_basis" | "matchBasis" => Ok("match_basis".to_string()),
        "changed_fields" | "changedFields" => Ok("changed_fields".to_string()),
        "blocked_reason" | "blockedReason" => Ok("blocked_reason".to_string()),
        "source_file" | "sourceFile" => Ok("source_file".to_string()),
        _ => Err(format!(
            "Unsupported --output-columns value '{value}'. Supported values: all, action_id, action, status, dashboard_uid, dashboard_title, folder_uid, folder_path, source_org_id, source_org_name, target_org_id, target_org_name, match_basis, changed_fields, blocked_reason, source_file."
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
    // Auth context is the single contract between parsing and transport:
    // profile/env defaults are resolved here and then reused by every dashboard client call.
    let connection = GrafanaConnection::resolve(
        common.profile.as_deref(),
        ConnectionMergeInput {
            url: &common.url,
            url_default: "",
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
        AuthInputs {
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            prompt_password: common.prompt_password,
            prompt_token: common.prompt_token,
        },
        false,
    )?;
    Ok(DashboardAuthContext {
        url: connection.base_url,
        timeout: connection.timeout_secs,
        verify_ssl: connection.verify_ssl,
        auth_mode: connection.auth_mode,
        headers: connection.headers,
    })
}

pub(crate) fn materialize_dashboard_common_auth_with_prompt<F, G>(
    mut common: CommonCliArgs,
    mut prompt_password_reader: F,
    mut prompt_token_reader: G,
) -> Result<CommonCliArgs>
where
    F: FnMut() -> Result<String>,
    G: FnMut() -> Result<String>,
{
    if common.prompt_password && common.password.is_none() {
        common.password = Some(prompt_password_reader()?);
    }
    if common.prompt_token && common.api_token.is_none() {
        common.api_token = Some(prompt_token_reader()?);
    }
    common.prompt_password = false;
    common.prompt_token = false;
    Ok(common)
}

pub(crate) fn materialize_dashboard_common_auth(common: CommonCliArgs) -> Result<CommonCliArgs> {
    materialize_dashboard_common_auth_with_prompt(
        common,
        || prompt_password("Grafana Basic auth password: ").map_err(GrafanaCliError::from),
        || prompt_password("Grafana API token: ").map_err(GrafanaCliError::from),
    )
}

pub fn build_http_client(common: &CommonCliArgs) -> Result<JsonHttpClient> {
    Ok(build_api_client(common)?.into_http_client())
}

pub fn build_http_client_for_org(common: &CommonCliArgs, org_id: i64) -> Result<JsonHttpClient> {
    Ok(build_api_client(common)?
        .scoped_to_org(org_id)?
        .into_http_client())
}

pub(crate) fn build_api_client(common: &CommonCliArgs) -> Result<GrafanaApiClient> {
    let connection = build_connection(common)?;
    GrafanaApiClient::from_connection(connection)
}

pub(crate) fn build_http_client_for_org_from_api(
    api: &GrafanaApiClient,
    org_id: i64,
) -> Result<JsonHttpClient> {
    Ok(api.scoped_to_org(org_id)?.into_http_client())
}

fn build_connection(common: &CommonCliArgs) -> Result<GrafanaConnection> {
    // Internal client boundary: keep connection/profile merge rules in one place
    // so dashboard commands stay focused on command semantics.
    GrafanaConnection::resolve(
        common.profile.as_deref(),
        ConnectionMergeInput {
            url: &common.url,
            url_default: "",
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
        AuthInputs {
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            prompt_password: common.prompt_password,
            prompt_token: common.prompt_token,
        },
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_common() -> CommonCliArgs {
        CommonCliArgs {
            color: crate::common::CliColorChoice::Auto,
            profile: None,
            url: "http://127.0.0.1:3000".to_string(),
            api_token: None,
            username: Some("admin".to_string()),
            password: None,
            prompt_password: true,
            prompt_token: false,
            timeout: DEFAULT_TIMEOUT,
            verify_ssl: false,
        }
    }

    #[test]
    fn materialize_dashboard_common_auth_prompts_password_once_and_clears_prompt_flags() {
        let mut prompts = 0usize;
        let resolved = materialize_dashboard_common_auth_with_prompt(
            make_common(),
            || {
                prompts += 1;
                Ok("prompted-password".to_string())
            },
            || panic!("token prompt should not be used"),
        )
        .unwrap();

        assert_eq!(prompts, 1);
        assert_eq!(resolved.password.as_deref(), Some("prompted-password"));
        assert!(!resolved.prompt_password);
        assert!(!resolved.prompt_token);
    }
}
