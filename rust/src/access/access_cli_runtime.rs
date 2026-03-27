use clap::{Command, CommandFactory, Parser};
use std::path::PathBuf;

use crate::common::{resolve_auth_headers, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

use super::{
    AccessCliArgs, AccessCliRoot, AccessCommand, DryRunOutputFormat, ListOutputFormat, Scope,
};

pub fn parse_cli_from<I, T>(iter: I) -> AccessCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    normalize_access_cli_args(AccessCliRoot::parse_from(iter).args)
}

fn apply_list_output_format(
    table: &mut bool,
    csv: &mut bool,
    json: &mut bool,
    output_format: &Option<ListOutputFormat>,
) {
    match output_format {
        Some(ListOutputFormat::Text) => {}
        Some(ListOutputFormat::Table) => *table = true,
        Some(ListOutputFormat::Csv) => *csv = true,
        Some(ListOutputFormat::Json) => *json = true,
        None => {}
    }
}

fn apply_dry_run_output_format(
    table: &mut bool,
    json: &mut bool,
    output_format: &DryRunOutputFormat,
) {
    match output_format {
        DryRunOutputFormat::Text => {}
        DryRunOutputFormat::Table => *table = true,
        DryRunOutputFormat::Json => *json = true,
    }
}

pub fn normalize_access_cli_args(mut args: AccessCliArgs) -> AccessCliArgs {
    match &mut args.command {
        AccessCommand::User { command } => match command {
            super::UserCommand::List(list_args) => {
                if list_args.all_orgs {
                    list_args.scope = Scope::Global;
                }
                apply_list_output_format(
                    &mut list_args.table,
                    &mut list_args.csv,
                    &mut list_args.json,
                    &list_args.output_format,
                );
            }
            super::UserCommand::Browse(browse_args) => {
                if browse_args.current_org {
                    browse_args.scope = Scope::Org;
                } else if browse_args.all_orgs {
                    browse_args.scope = Scope::Global;
                }
            }
            super::UserCommand::Import(import_args) => {
                apply_dry_run_output_format(
                    &mut import_args.table,
                    &mut import_args.json,
                    &import_args.output_format,
                );
            }
            _ => {}
        },
        AccessCommand::Org { command } => {
            if let super::OrgCommand::List(list_args) = command {
                apply_list_output_format(
                    &mut list_args.table,
                    &mut list_args.csv,
                    &mut list_args.json,
                    &list_args.output_format,
                );
            }
        }
        AccessCommand::Team { command } => {
            if let super::TeamCommand::List(list_args) = command {
                apply_list_output_format(
                    &mut list_args.table,
                    &mut list_args.csv,
                    &mut list_args.json,
                    &list_args.output_format,
                );
            }
            if let super::TeamCommand::Import(import_args) = command {
                apply_dry_run_output_format(
                    &mut import_args.table,
                    &mut import_args.json,
                    &import_args.output_format,
                );
            }
        }
        AccessCommand::ServiceAccount { command } => {
            if let super::ServiceAccountCommand::List(list_args) = command {
                apply_list_output_format(
                    &mut list_args.table,
                    &mut list_args.csv,
                    &mut list_args.json,
                    &list_args.output_format,
                );
            }
            if let super::ServiceAccountCommand::Import(import_args) = command {
                apply_dry_run_output_format(
                    &mut import_args.table,
                    &mut import_args.json,
                    &import_args.output_format,
                );
            }
        }
    }
    args
}

pub fn root_command() -> Command {
    AccessCliRoot::command()
}

#[derive(Debug, Clone)]
pub struct AccessAuthContext {
    pub url: String,
    pub timeout: u64,
    pub verify_ssl: bool,
    pub ca_cert: Option<PathBuf>,
    pub auth_mode: String,
    pub headers: Vec<(String, String)>,
}

pub fn build_auth_context(common: &super::CommonCliArgs) -> Result<AccessAuthContext> {
    let mut headers = resolve_auth_headers(
        common.api_token.as_deref(),
        common.username.as_deref(),
        common.password.as_deref(),
        common.prompt_password,
        common.prompt_token,
    )?;
    if let Some(org_id) = common.org_id {
        headers.push(("X-Grafana-Org-Id".to_string(), org_id.to_string()));
    }
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
    Ok(AccessAuthContext {
        url: common.url.clone(),
        timeout: common.timeout,
        verify_ssl: common.verify_ssl || common.ca_cert.is_some(),
        ca_cert: common.ca_cert.clone(),
        auth_mode,
        headers,
    })
}

pub fn build_auth_context_no_org_id(
    common: &super::CommonCliArgsNoOrgId,
) -> Result<AccessAuthContext> {
    let headers = resolve_auth_headers(
        common.api_token.as_deref(),
        common.username.as_deref(),
        common.password.as_deref(),
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
    Ok(AccessAuthContext {
        url: common.url.clone(),
        timeout: common.timeout,
        verify_ssl: common.verify_ssl || common.ca_cert.is_some(),
        ca_cert: common.ca_cert.clone(),
        auth_mode,
        headers,
    })
}

pub fn build_http_client(common: &super::CommonCliArgs) -> Result<JsonHttpClient> {
    let context = build_auth_context(common)?;
    JsonHttpClient::new_with_ca_cert(
        JsonHttpClientConfig {
            base_url: context.url,
            headers: context.headers,
            timeout_secs: context.timeout,
            verify_ssl: context.verify_ssl,
        },
        context.ca_cert.as_deref(),
    )
}

pub fn build_http_client_no_org_id(common: &super::CommonCliArgsNoOrgId) -> Result<JsonHttpClient> {
    let context = build_auth_context_no_org_id(common)?;
    JsonHttpClient::new_with_ca_cert(
        JsonHttpClientConfig {
            base_url: context.url,
            headers: context.headers,
            timeout_secs: context.timeout,
            verify_ssl: context.verify_ssl,
        },
        context.ca_cert.as_deref(),
    )
}
