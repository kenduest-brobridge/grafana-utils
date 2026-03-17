//! Access-management domain orchestrator.
//!
//! Purpose:
//! - Own access command taxonomy (`user`, `team`, `service-account`) and argument
//!   normalization.
//! - Centralize dispatch between repository-owned handlers and injectable request backends.
//! - Re-export shared access parser/model types for CLI and test call sites.
//!
//! Flow:
//! - Parse CLI args via `cli_defs`.
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

#[path = "cli_defs.rs"]
mod cli_defs;
#[path = "org.rs"]
mod org;
#[path = "pending_delete.rs"]
mod pending_delete;
#[path = "render.rs"]
mod render;
#[path = "service_account.rs"]
mod service_account;
#[path = "team.rs"]
mod team;
#[path = "user.rs"]
mod user;

pub use cli_defs::{
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
pub use pending_delete::{
    GroupCommandStage, ServiceAccountDeleteArgs, ServiceAccountTokenDeleteArgs, TeamDeleteArgs,
};

#[cfg(test)]
pub(crate) use org::{delete_org_with_request, list_orgs_with_request, modify_org_with_request};
#[cfg(test)]
pub(crate) use pending_delete::{
    delete_service_account_token_with_request, delete_service_account_with_request,
    delete_team_with_request,
};
#[cfg(test)]
pub(crate) use service_account::{
    add_service_account_token_with_request, add_service_account_with_request,
    diff_service_accounts_with_request, export_service_accounts_with_request,
    import_service_accounts_with_request, list_service_accounts_command_with_request,
};
#[cfg(test)]
pub(crate) use team::{
    add_team_with_request, diff_teams_with_request, import_teams_with_request,
    list_teams_command_with_request, modify_team_with_request,
};
#[cfg(test)]
pub(crate) use user::{
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
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: access.rs:run_access_cli
    // Downstream callees: access.rs:run_access_cli_with_request

    run_access_cli_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

/// Access execution path with request-function injection.
///
/// Receives fully parsed CLI args and routes each command branch to matching handler
/// functions that perform request execution.
pub fn run_access_cli_with_request<F>(mut request_json: F, args: AccessCliArgs) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: access.rs:run_access_cli_with_client, rust_tests.rs:run_access_cli_with_request_routes_org_export, rust_tests.rs:run_access_cli_with_request_routes_org_import, rust_tests.rs:run_access_cli_with_request_routes_team_diff, rust_tests.rs:run_access_cli_with_request_routes_team_export, rust_tests.rs:run_access_cli_with_request_routes_team_import, rust_tests.rs:run_access_cli_with_request_routes_user_diff, rust_tests.rs:run_access_cli_with_request_routes_user_export, rust_tests.rs:run_access_cli_with_request_routes_user_list
    // Downstream callees: 無

    match args.command {
        AccessCommand::User { command } => match command {
            UserCommand::List(args) => {
                let _ = user::list_users_with_request(&mut request_json, &args)?;
            }
            UserCommand::Add(args) => {
                let _ = user::add_user_with_request(&mut request_json, &args)?;
            }
            UserCommand::Modify(args) => {
                let _ = user::modify_user_with_request(&mut request_json, &args)?;
            }
            UserCommand::Export(args) => {
                let _ = user::export_users_with_request(&mut request_json, &args)?;
            }
            UserCommand::Import(args) => {
                let _ = user::import_users_with_request(&mut request_json, &args)?;
            }
            UserCommand::Diff(args) => {
                let _ = user::diff_users_with_request(&mut request_json, &args)?;
            }
            UserCommand::Delete(args) => {
                let _ = user::delete_user_with_request(&mut request_json, &args)?;
            }
        },
        AccessCommand::Org { command } => match command {
            OrgCommand::List(args) => {
                let _ = org::list_orgs_with_request(&mut request_json, &args)?;
            }
            OrgCommand::Add(args) => {
                let _ = org::add_org_with_request(&mut request_json, &args)?;
            }
            OrgCommand::Modify(args) => {
                let _ = org::modify_org_with_request(&mut request_json, &args)?;
            }
            OrgCommand::Export(args) => {
                let _ = org::export_orgs_with_request(&mut request_json, &args)?;
            }
            OrgCommand::Import(args) => {
                let _ = org::import_orgs_with_request(&mut request_json, &args)?;
            }
            OrgCommand::Delete(args) => {
                let _ = org::delete_org_with_request(&mut request_json, &args)?;
            }
        },
        AccessCommand::Team { command } => match command {
            TeamCommand::List(args) => {
                let _ = team::list_teams_command_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Add(args) => {
                let _ = team::add_team_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Modify(args) => {
                let _ = team::modify_team_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Export(args) => {
                let _ = team::export_teams_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Import(args) => {
                let _ = team::import_teams_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Diff(args) => {
                let _ = team::diff_teams_with_request(&mut request_json, &args)?;
            }
            TeamCommand::Delete(args) => {
                let _ = pending_delete::delete_team_with_request(&mut request_json, &args)?;
            }
        },
        AccessCommand::ServiceAccount { command } => match command {
            ServiceAccountCommand::List(args) => {
                let _ = service_account::list_service_accounts_command_with_request(
                    &mut request_json,
                    &args,
                )?;
            }
            ServiceAccountCommand::Add(args) => {
                let _ =
                    service_account::add_service_account_with_request(&mut request_json, &args)?;
            }
            ServiceAccountCommand::Export(args) => {
                let _ = service_account::export_service_accounts_with_request(
                    &mut request_json,
                    &args,
                )?;
            }
            ServiceAccountCommand::Import(args) => {
                let _ = service_account::import_service_accounts_with_request(
                    &mut request_json,
                    &args,
                )?;
            }
            ServiceAccountCommand::Diff(args) => {
                let _ =
                    service_account::diff_service_accounts_with_request(&mut request_json, &args)?;
            }
            ServiceAccountCommand::Delete(args) => {
                let _ =
                    pending_delete::delete_service_account_with_request(&mut request_json, &args)?;
            }
            ServiceAccountCommand::Token { command } => match command {
                ServiceAccountTokenCommand::Add(args) => {
                    let _ = service_account::add_service_account_token_with_request(
                        &mut request_json,
                        &args,
                    )?;
                }
                ServiceAccountTokenCommand::Delete(args) => {
                    let _ = pending_delete::delete_service_account_token_with_request(
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
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: 無
    // Downstream callees: access.rs:run_access_cli_with_client, cli_defs.rs:build_http_client_no_org_id, cli_defs.rs:normalize_access_cli_args

    let args = normalize_access_cli_args(args);
    match &args.command {
        AccessCommand::User { command } => match command {
            UserCommand::List(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            UserCommand::Add(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            UserCommand::Modify(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            UserCommand::Export(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            UserCommand::Import(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            UserCommand::Diff(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            UserCommand::Delete(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
        },
        AccessCommand::Org { command } => match command {
            OrgCommand::List(inner) => {
                let client = build_http_client_no_org_id(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            OrgCommand::Add(inner) => {
                let client = build_http_client_no_org_id(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            OrgCommand::Modify(inner) => {
                let client = build_http_client_no_org_id(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            OrgCommand::Export(inner) => {
                let client = build_http_client_no_org_id(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            OrgCommand::Import(inner) => {
                let client = build_http_client_no_org_id(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            OrgCommand::Delete(inner) => {
                let client = build_http_client_no_org_id(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
        },
        AccessCommand::Team { command } => match command {
            TeamCommand::List(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            TeamCommand::Add(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            TeamCommand::Modify(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            TeamCommand::Export(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            TeamCommand::Import(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            TeamCommand::Diff(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            TeamCommand::Delete(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
        },
        AccessCommand::ServiceAccount { command } => match command {
            ServiceAccountCommand::List(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            ServiceAccountCommand::Add(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            ServiceAccountCommand::Export(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            ServiceAccountCommand::Import(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            ServiceAccountCommand::Diff(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            ServiceAccountCommand::Delete(inner) => {
                let client = build_http_client(&inner.common)?;
                run_access_cli_with_client(&client, args)
            }
            ServiceAccountCommand::Token { command } => match command {
                ServiceAccountTokenCommand::Add(inner) => {
                    let client = build_http_client(&inner.common)?;
                    run_access_cli_with_client(&client, args)
                }
                ServiceAccountTokenCommand::Delete(inner) => {
                    let client = build_http_client(&inner.common)?;
                    run_access_cli_with_client(&client, args)
                }
            },
        },
    }
}

#[cfg(test)]
#[path = "rust_tests.rs"]
mod access_rust_tests;
