//! Access-management domain orchestrator.
//!
//! Purpose:
//! - Own access command taxonomy (`user`, `team`, `service-account`) and argument
//!   normalization.
//! - Centralize dispatch between repository-owned handlers and injectable request backends.
//! - Re-export shared access parser/model types for CLI and test call sites.
//!
//! Flow:
//! - Parse CLI args via `access_cli_defs`.
//! - For each subcommand, normalize args, build HTTP client(s), and delegate handler calls.
//! - Allow `run_access_cli_with_request` to receive a mockable request function for tests.
//!
//! Caveats:
//! - Do not implement request semantics in handler branches; keep transport concerns inside
//!   `http` or per-handler client code.
//! - Keep this module focused on orchestration, not resource-specific JSON shape details.
use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, value_as_object, Result};
use crate::http::JsonHttpClient;

#[path = "access_cli_defs.rs"]
mod access_cli_defs;
#[path = "access_org.rs"]
mod access_org;
#[path = "access_pending_delete.rs"]
mod access_pending_delete;
#[path = "access_render.rs"]
mod access_render;
#[path = "access_service_account.rs"]
mod access_service_account;
#[path = "access_team.rs"]
mod access_team;
#[path = "access_user.rs"]
mod access_user;

pub use access_cli_defs::{
    build_auth_context, build_http_client, build_http_client_no_org_id, normalize_access_cli_args,
    parse_cli_from, root_command, AccessAuthContext, AccessCliArgs, AccessCommand, CommonCliArgs,
    DryRunOutputFormat, OrgAddArgs, OrgCommand, OrgDeleteArgs, OrgExportArgs, OrgImportArgs,
    OrgListArgs, OrgModifyArgs, Scope, ServiceAccountAddArgs, ServiceAccountCommand,
    ServiceAccountDiffArgs, ServiceAccountExportArgs, ServiceAccountImportArgs,
    ServiceAccountListArgs, ServiceAccountTokenAddArgs, ServiceAccountTokenCommand, TeamAddArgs,
    TeamCommand, TeamDiffArgs, TeamExportArgs, TeamImportArgs, TeamListArgs, TeamModifyArgs,
    UserAddArgs, UserCommand, UserDeleteArgs, UserDiffArgs, UserExportArgs, UserImportArgs,
    UserListArgs, UserModifyArgs, ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
    ACCESS_EXPORT_KIND_TEAMS, ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME,
    ACCESS_EXPORT_VERSION, ACCESS_ORG_EXPORT_FILENAME, ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
    ACCESS_TEAM_EXPORT_FILENAME, ACCESS_USER_EXPORT_FILENAME, DEFAULT_PAGE_SIZE, DEFAULT_TIMEOUT,
    DEFAULT_URL,
};
pub use access_pending_delete::{
    GroupCommandStage, ServiceAccountDeleteArgs, ServiceAccountTokenDeleteArgs, TeamDeleteArgs,
};

#[cfg(test)]
pub(crate) use access_org::{
    delete_org_with_request, list_orgs_with_request, modify_org_with_request,
};
#[cfg(test)]
pub(crate) use access_pending_delete::{
    delete_service_account_token_with_request, delete_service_account_with_request,
    delete_team_with_request,
};
#[cfg(test)]
pub(crate) use access_service_account::{
    add_service_account_token_with_request, add_service_account_with_request,
    diff_service_accounts_with_request, export_service_accounts_with_request,
    import_service_accounts_with_request, list_service_accounts_command_with_request,
};
#[cfg(test)]
pub(crate) use access_team::{
    add_team_with_request, diff_teams_with_request, import_teams_with_request,
    list_teams_command_with_request, modify_team_with_request,
};
#[cfg(test)]
pub(crate) use access_user::{
    add_user_with_request, delete_user_with_request, diff_users_with_request,
    list_users_with_request, modify_user_with_request,
};

fn request_object<F>(
    mut request_json: F,
    method: Method,
    path: &str,
    params: &[(String, String)],
    payload: Option<&Value>,
    error_message: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let value = request_json(method, path, params, payload)?
        .ok_or_else(|| message(error_message.to_string()))?;
    Ok(value_as_object(&value, error_message)?.clone())
}

fn request_array<F>(
    mut request_json: F,
    method: Method,
    path: &str,
    params: &[(String, String)],
    payload: Option<&Value>,
    error_message: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match request_json(method, path, params, payload)? {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| Ok(value_as_object(&item, error_message)?.clone()))
            .collect(),
        Some(_) => Err(message(error_message.to_string())),
        None => Ok(Vec::new()),
    }
}

/// Access execution path for callers that already own a configured `JsonHttpClient`.
/// Delegates to the request-injection path to keep side effects explicit and testable.
pub fn run_access_cli_with_client(client: &JsonHttpClient, args: AccessCliArgs) -> Result<()> {
    run_access_cli_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

fn user_command_common(command: &UserCommand) -> &CommonCliArgs {
    match command {
        UserCommand::List(inner) => &inner.common,
        UserCommand::Add(inner) => &inner.common,
        UserCommand::Modify(inner) => &inner.common,
        UserCommand::Export(inner) => &inner.common,
        UserCommand::Import(inner) => &inner.common,
        UserCommand::Diff(inner) => &inner.common,
        UserCommand::Delete(inner) => &inner.common,
    }
}

fn org_command_common(command: &OrgCommand) -> &access_cli_defs::CommonCliArgsNoOrgId {
    match command {
        OrgCommand::List(inner) => &inner.common,
        OrgCommand::Add(inner) => &inner.common,
        OrgCommand::Modify(inner) => &inner.common,
        OrgCommand::Export(inner) => &inner.common,
        OrgCommand::Import(inner) => &inner.common,
        OrgCommand::Delete(inner) => &inner.common,
    }
}

fn team_command_common(command: &TeamCommand) -> &CommonCliArgs {
    match command {
        TeamCommand::List(inner) => &inner.common,
        TeamCommand::Add(inner) => &inner.common,
        TeamCommand::Modify(inner) => &inner.common,
        TeamCommand::Export(inner) => &inner.common,
        TeamCommand::Import(inner) => &inner.common,
        TeamCommand::Diff(inner) => &inner.common,
        TeamCommand::Delete(inner) => &inner.common,
    }
}

fn service_account_command_common(command: &ServiceAccountCommand) -> &CommonCliArgs {
    match command {
        ServiceAccountCommand::List(inner) => &inner.common,
        ServiceAccountCommand::Add(inner) => &inner.common,
        ServiceAccountCommand::Export(inner) => &inner.common,
        ServiceAccountCommand::Import(inner) => &inner.common,
        ServiceAccountCommand::Diff(inner) => &inner.common,
        ServiceAccountCommand::Delete(inner) => &inner.common,
        ServiceAccountCommand::Token { command } => match command {
            ServiceAccountTokenCommand::Add(inner) => &inner.common,
            ServiceAccountTokenCommand::Delete(inner) => &inner.common,
        },
    }
}

fn build_client_for_access_command(command: &AccessCommand) -> Result<JsonHttpClient> {
    match command {
        AccessCommand::User { command } => build_http_client(user_command_common(command)),
        AccessCommand::Org { command } => build_http_client_no_org_id(org_command_common(command)),
        AccessCommand::Team { command } => build_http_client(team_command_common(command)),
        AccessCommand::ServiceAccount { command } => {
            build_http_client(service_account_command_common(command))
        }
    }
}

/// Access execution path with request-function injection.
///
/// Receives fully parsed CLI args and routes each command branch to matching handler
/// functions that perform request execution.
pub fn run_access_cli_with_request<F>(mut request_json: F, args: AccessCliArgs) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    match args.command {
        AccessCommand::User { command } => match command {
            UserCommand::List(args) => {
                let _ = access_user::list_users_with_request(&mut request_json, &args)?;
            }
            UserCommand::Add(args) => {
                let _ = access_user::add_user_with_request(&mut request_json, &args)?;
            }
            UserCommand::Modify(args) => {
                let _ = access_user::modify_user_with_request(&mut request_json, &args)?;
            }
            UserCommand::Export(args) => {
                let _ = access_user::export_users_with_request(&mut request_json, &args)?;
            }
            UserCommand::Import(args) => {
                let _ = access_user::import_users_with_request(&mut request_json, &args)?;
            }
            UserCommand::Diff(args) => {
                let _ = access_user::diff_users_with_request(&mut request_json, &args)?;
            }
            UserCommand::Delete(args) => {
                let _ = access_user::delete_user_with_request(&mut request_json, &args)?;
            }
        },
        AccessCommand::Org { command } => match command {
            OrgCommand::List(args) => {
                let _ = access_org::list_orgs_with_request(&mut request_json, &args)?;
            }
            OrgCommand::Add(args) => {
                let _ = access_org::add_org_with_request(&mut request_json, &args)?;
            }
            OrgCommand::Modify(args) => {
                let _ = access_org::modify_org_with_request(&mut request_json, &args)?;
            }
            OrgCommand::Export(args) => {
                let _ = access_org::export_orgs_with_request(&mut request_json, &args)?;
            }
            OrgCommand::Import(args) => {
                let _ = access_org::import_orgs_with_request(&mut request_json, &args)?;
            }
            OrgCommand::Delete(args) => {
                let _ = access_org::delete_org_with_request(&mut request_json, &args)?;
            }
        },
        AccessCommand::Team { command } => match command {
            TeamCommand::List(args) => {
                let _ = access_team::list_teams_command_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Add(args) => {
                let _ = access_team::add_team_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Modify(args) => {
                let _ = access_team::modify_team_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Export(args) => {
                let _ = access_team::export_teams_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Import(args) => {
                let _ = access_team::import_teams_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Diff(args) => {
                let _ = access_team::diff_teams_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Delete(args) => {
                let _ = access_pending_delete::delete_team_with_request(&mut request_json, &args)?;
            }
        },
        AccessCommand::ServiceAccount { command } => match command {
            ServiceAccountCommand::List(args) => {
                let _ = access_service_account::list_service_accounts_command_with_request(
                    &mut request_json,
                    &args,
                )?;
            }
            ServiceAccountCommand::Add(args) => {
                let _ = access_service_account::add_service_account_with_request(
                    &mut request_json,
                    &args,
                )?;
            }
            ServiceAccountCommand::Export(args) => {
                let _ = access_service_account::export_service_accounts_with_request(
                    &mut request_json,
                    &args,
                )?;
            }
            ServiceAccountCommand::Import(args) => {
                let _ = access_service_account::import_service_accounts_with_request(
                    &mut request_json,
                    &args,
                )?;
            }
            ServiceAccountCommand::Diff(args) => {
                let _ = access_service_account::diff_service_accounts_with_request(
                    &mut request_json,
                    &args,
                )?;
            }
            ServiceAccountCommand::Delete(args) => {
                let _ = access_pending_delete::delete_service_account_with_request(
                    &mut request_json,
                    &args,
                )?;
            }
            ServiceAccountCommand::Token { command } => match command {
                ServiceAccountTokenCommand::Add(args) => {
                    let _ = access_service_account::add_service_account_token_with_request(
                        &mut request_json,
                        &args,
                    )?;
                }
                ServiceAccountTokenCommand::Delete(args) => {
                    let _ = access_pending_delete::delete_service_account_token_with_request(
                        &mut request_json,
                        &args,
                    )?;
                }
            },
        },
    }
    Ok(())
}

/// Access binary entrypoint.
///
/// Normalizes arguments and builds one HTTP client per concrete subcommand branch before
/// delegating to the request-injection runner.
pub fn run_access_cli(args: AccessCliArgs) -> Result<()> {
    let args = normalize_access_cli_args(args);
    let client = build_client_for_access_command(&args.command)?;
    run_access_cli_with_client(&client, args)
}

#[cfg(test)]
#[path = "access_rust_tests.rs"]
mod access_rust_tests;
