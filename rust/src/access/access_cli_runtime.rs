//! Access CLI runtime glue layer.
//!
//! Responsibilities:
//! - Build parser entrypoints for `access` subcommands and normalize shared
//!   auth options.
//! - Resolve execution settings (including output format + dry-run intent).
//! - Route to Access domain handlers with a prepared HTTP client and auth headers.

use clap::{Command, CommandFactory, Parser};
use std::path::PathBuf;

use crate::common::{resolve_auth_headers, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};
use crate::profile_config::{
    load_selected_profile, resolve_connection_settings, ConnectionMergeInput,
};

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
    yaml: &mut bool,
    output_format: &Option<ListOutputFormat>,
) {
    match output_format {
        Some(ListOutputFormat::Text) => {}
        Some(ListOutputFormat::Table) => *table = true,
        Some(ListOutputFormat::Csv) => *csv = true,
        Some(ListOutputFormat::Json) => *json = true,
        Some(ListOutputFormat::Yaml) => *yaml = true,
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
                    &mut list_args.yaml,
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
                    &mut list_args.yaml,
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
                    &mut list_args.yaml,
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
                    &mut list_args.yaml,
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
    let selected_profile = load_selected_profile(common.profile.as_deref())?;
    let resolved = resolve_connection_settings(
        ConnectionMergeInput {
            url: &common.url,
            url_default: super::DEFAULT_URL,
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            org_id: common.org_id,
            timeout: common.timeout,
            timeout_default: super::DEFAULT_TIMEOUT,
            verify_ssl: common.verify_ssl,
            insecure: common.insecure,
            ca_cert: common.ca_cert.as_deref(),
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
    let mut headers = resolve_auth_headers(
        token,
        username,
        password,
        common.prompt_password,
        common.prompt_token,
    )?;
    if let Some(org_id) = resolved.org_id {
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
        url: resolved.url,
        timeout: resolved.timeout,
        verify_ssl: resolved.verify_ssl,
        ca_cert: resolved.ca_cert,
        auth_mode,
        headers,
    })
}

pub fn build_auth_context_no_org_id(
    common: &super::CommonCliArgsNoOrgId,
) -> Result<AccessAuthContext> {
    let selected_profile = load_selected_profile(common.profile.as_deref())?;
    let resolved = resolve_connection_settings(
        ConnectionMergeInput {
            url: &common.url,
            url_default: super::DEFAULT_URL,
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            org_id: None,
            timeout: common.timeout,
            timeout_default: super::DEFAULT_TIMEOUT,
            verify_ssl: common.verify_ssl,
            insecure: common.insecure,
            ca_cert: common.ca_cert.as_deref(),
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
    Ok(AccessAuthContext {
        url: resolved.url,
        timeout: resolved.timeout,
        verify_ssl: resolved.verify_ssl,
        ca_cert: resolved.ca_cert,
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
