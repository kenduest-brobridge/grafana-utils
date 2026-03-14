use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, value_as_object, Result};
use crate::http::JsonHttpClient;

#[path = "access_cli_defs.rs"]
mod access_cli_defs;
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
    build_auth_context, build_http_client, normalize_access_cli_args, parse_cli_from, root_command,
    AccessAuthContext, AccessCliArgs, AccessCommand, CommonCliArgs, Scope, ServiceAccountAddArgs,
    ServiceAccountCommand, ServiceAccountListArgs, ServiceAccountTokenAddArgs,
    ServiceAccountTokenCommand, TeamAddArgs, TeamCommand, TeamListArgs, TeamModifyArgs,
    UserAddArgs, UserCommand, UserDeleteArgs, UserListArgs, UserModifyArgs, DEFAULT_PAGE_SIZE,
    DEFAULT_TIMEOUT, DEFAULT_URL,
};
pub use access_pending_delete::{
    GroupCommandStage, ServiceAccountDeleteArgs, ServiceAccountTokenDeleteArgs, TeamDeleteArgs,
};

#[cfg(test)]
pub(crate) use access_pending_delete::{
    delete_service_account_token_with_request, delete_service_account_with_request,
    delete_team_with_request,
};
#[cfg(test)]
pub(crate) use access_service_account::{
    add_service_account_token_with_request, add_service_account_with_request,
    list_service_accounts_command_with_request,
};
#[cfg(test)]
pub(crate) use access_team::{
    add_team_with_request, list_teams_command_with_request, modify_team_with_request,
};
#[cfg(test)]
pub(crate) use access_user::{
    add_user_with_request, delete_user_with_request, list_users_with_request,
    modify_user_with_request,
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

pub fn run_access_cli_with_client(client: &JsonHttpClient, args: AccessCliArgs) -> Result<()> {
    run_access_cli_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

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
            UserCommand::Delete(args) => {
                let _ = access_user::delete_user_with_request(&mut request_json, &args)?;
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

pub fn run_access_cli(args: AccessCliArgs) -> Result<()> {
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
            UserCommand::Delete(inner) => {
                let client = build_http_client(&inner.common)?;
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
#[path = "access_rust_tests.rs"]
mod access_rust_tests;
