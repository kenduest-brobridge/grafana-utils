//! Access CLI runtime glue layer.
//!
//! Responsibilities:
//! - Build parser entrypoints for `access` subcommands and normalize shared
//!   auth options.
//! - Resolve execution settings (including output format + dry-run intent).
//! - Route to Access domain handlers with a prepared HTTP client and auth headers.

use clap::{Command, CommandFactory, Parser};
use std::path::PathBuf;

use crate::common::{set_json_color_choice, Result};
use crate::grafana_api::{AuthInputs, GrafanaApiClient, GrafanaConnection};
use crate::http::JsonHttpClient;
use crate::profile_config::ConnectionMergeInput;

use super::{
    AccessCliArgs, AccessCliRoot, AccessCommand, DryRunOutputFormat, ListOutputFormat, Scope,
};

pub fn parse_cli_from<I, T>(iter: I) -> AccessCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let root = AccessCliRoot::parse_from(iter);
    set_json_color_choice(root.color);
    normalize_access_cli_args(root.args)
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
    let connection = GrafanaConnection::resolve(
        common.profile.as_deref(),
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
        AuthInputs {
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            prompt_password: common.prompt_password,
            prompt_token: common.prompt_token,
        },
        true,
    )?;
    Ok(AccessAuthContext {
        url: connection.base_url,
        timeout: connection.timeout_secs,
        verify_ssl: connection.verify_ssl,
        ca_cert: connection.ca_cert,
        auth_mode: connection.auth_mode,
        headers: connection.headers,
    })
}

pub fn build_auth_context_no_org_id(
    common: &super::CommonCliArgsNoOrgId,
) -> Result<AccessAuthContext> {
    let connection = GrafanaConnection::resolve(
        common.profile.as_deref(),
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
        AuthInputs {
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            prompt_password: common.prompt_password,
            prompt_token: common.prompt_token,
        },
        false,
    )?;
    Ok(AccessAuthContext {
        url: connection.base_url,
        timeout: connection.timeout_secs,
        verify_ssl: connection.verify_ssl,
        ca_cert: connection.ca_cert,
        auth_mode: connection.auth_mode,
        headers: connection.headers,
    })
}

pub fn build_http_client(common: &super::CommonCliArgs) -> Result<JsonHttpClient> {
    let connection = build_connection(common)?;
    Ok(GrafanaApiClient::from_connection(connection)?.into_http_client())
}

pub fn build_http_client_no_org_id(common: &super::CommonCliArgsNoOrgId) -> Result<JsonHttpClient> {
    let connection = build_connection_no_org_id(common)?;
    Ok(GrafanaApiClient::from_connection(connection)?.into_http_client())
}

fn build_connection(common: &super::CommonCliArgs) -> Result<GrafanaConnection> {
    GrafanaConnection::resolve(
        common.profile.as_deref(),
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
        AuthInputs {
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            prompt_password: common.prompt_password,
            prompt_token: common.prompt_token,
        },
        true,
    )
}

fn build_connection_no_org_id(common: &super::CommonCliArgsNoOrgId) -> Result<GrafanaConnection> {
    GrafanaConnection::resolve(
        common.profile.as_deref(),
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
