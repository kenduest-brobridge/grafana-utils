//! Access-management domain orchestrator.
//!
//! Owns access command dispatch, argument normalization, and the shared parser/model re-exports
//! used by the CLI and tests.
use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, value_as_object, Result};
use crate::http::JsonHttpClient;

// Internal modules stay split by resource kind so user/org/team/service-account
// workflows can evolve independently while this file keeps only domain routing.
#[path = "browse_terminal.rs"]
mod browse_terminal;
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
#[path = "team_browse.rs"]
mod team_browse;
#[path = "team_import_export_diff.rs"]
mod team_import_export_diff;
#[path = "team_runtime.rs"]
mod team_runtime;
#[path = "user.rs"]
mod user;
#[path = "user_browse.rs"]
mod user_browse;

pub use cli_defs::{
    build_auth_context, build_http_client, build_http_client_no_org_id, normalize_access_cli_args,
    parse_cli_from, root_command, AccessAuthContext, AccessCliArgs, AccessCommand, CommonCliArgs,
    DryRunOutputFormat, OrgAddArgs, OrgCommand, OrgDeleteArgs, OrgDiffArgs, OrgExportArgs,
    OrgImportArgs, OrgListArgs, OrgModifyArgs, Scope, ServiceAccountAddArgs, ServiceAccountCommand,
    ServiceAccountDiffArgs, ServiceAccountExportArgs, ServiceAccountImportArgs,
    ServiceAccountListArgs, ServiceAccountTokenAddArgs, ServiceAccountTokenCommand, TeamAddArgs,
    TeamBrowseArgs, TeamCommand, TeamDiffArgs, TeamExportArgs, TeamImportArgs, TeamListArgs,
    TeamModifyArgs, UserAddArgs, UserBrowseArgs, UserCommand, UserDeleteArgs, UserDiffArgs,
    UserExportArgs, UserImportArgs, UserListArgs, UserModifyArgs, ACCESS_EXPORT_KIND_ORGS,
    ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS, ACCESS_EXPORT_KIND_TEAMS, ACCESS_EXPORT_KIND_USERS,
    ACCESS_EXPORT_METADATA_FILENAME, ACCESS_EXPORT_VERSION, ACCESS_ORG_EXPORT_FILENAME,
    ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME, ACCESS_TEAM_EXPORT_FILENAME,
    ACCESS_USER_EXPORT_FILENAME, DEFAULT_PAGE_SIZE, DEFAULT_TIMEOUT, DEFAULT_URL,
};
pub use pending_delete::{
    GroupCommandStage, ServiceAccountDeleteArgs, ServiceAccountTokenDeleteArgs, TeamDeleteArgs,
};

#[derive(Clone, Debug)]
enum BrowseSwitch {
    Exit,
    ToUser(UserBrowseArgs),
    ToTeam(TeamBrowseArgs),
}

fn default_team_browse_args_from_user(args: &UserBrowseArgs) -> TeamBrowseArgs {
    TeamBrowseArgs {
        common: args.common.clone(),
        query: None,
        name: None,
        with_members: true,
        page: 1,
        per_page: DEFAULT_PAGE_SIZE,
    }
}

fn default_user_browse_args_from_team(args: &TeamBrowseArgs) -> UserBrowseArgs {
    UserBrowseArgs {
        common: args.common.clone(),
        scope: Scope::Global,
        all_orgs: false,
        current_org: false,
        query: None,
        login: None,
        email: None,
        org_role: None,
        grafana_admin: None,
        with_teams: false,
        page: 1,
        per_page: DEFAULT_PAGE_SIZE,
    }
}

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
    // Normalize "one object expected" API responses into a single helper so
    // callers can focus on workflow logic instead of response-shape validation.
    let value =
        request_json(method, path, params, payload)?.ok_or_else(|| message(error_message))?;
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
    // Treat missing list payloads as empty collections to match Grafana list-style
    // endpoints that can legitimately return no body or an empty array.
    match request_json(method, path, params, payload)? {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| Ok(value_as_object(&item, error_message)?.clone()))
            .collect(),
        Some(_) => Err(message(error_message)),
        None => Ok(Vec::new()),
    }
}

fn request_object_list_field<F>(
    mut request_json: F,
    method: Method,
    path: &str,
    params: &[(String, String)],
    payload: Option<&Value>,
    field: &str,
    error_messages: (&str, &str),
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let (object_error_message, list_error_message) = error_messages;
    let object = request_object(
        &mut request_json,
        method,
        path,
        params,
        payload,
        object_error_message,
    )?;
    match object.get(field) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| Ok(value_as_object(item, list_error_message)?.clone()))
            .collect(),
        _ => Err(message(list_error_message)),
    }
}

/// Access execution path for callers that already own a configured `JsonHttpClient`.
/// Delegates to the request-injection path to keep side effects explicit and testable.
pub fn run_access_cli_with_client(client: &JsonHttpClient, args: &AccessCliArgs) -> Result<()> {
    run_access_cli_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        args,
    )
}

fn run_access_cli_with_common<C, F>(common: &C, args: &AccessCliArgs, build_client: F) -> Result<()>
where
    F: FnOnce(&C) -> Result<JsonHttpClient>,
{
    // Client construction is centralized here so each command branch can pick the
    // correct auth/org-id rules without duplicating the handoff code.
    let client = build_client(common)?;
    run_access_cli_with_client(&client, args)
}

fn run_user_access_cli(command: &UserCommand, args: &AccessCliArgs) -> Result<()> {
    // User operations require the standard access client, including org-scoped
    // auth behavior where applicable.
    match command {
        UserCommand::List(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        UserCommand::Browse(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        UserCommand::Add(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        UserCommand::Modify(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        UserCommand::Export(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        UserCommand::Import(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        UserCommand::Diff(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        UserCommand::Delete(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
    }
}

fn run_org_access_cli(command: &OrgCommand, args: &AccessCliArgs) -> Result<()> {
    // Org operations intentionally use the "no org id" client path because org
    // management targets Grafana's global admin surface rather than one org.
    match command {
        OrgCommand::List(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client_no_org_id)
        }
        OrgCommand::Add(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client_no_org_id)
        }
        OrgCommand::Modify(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client_no_org_id)
        }
        OrgCommand::Export(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client_no_org_id)
        }
        OrgCommand::Import(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client_no_org_id)
        }
        OrgCommand::Diff(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client_no_org_id)
        }
        OrgCommand::Delete(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client_no_org_id)
        }
    }
}

fn run_team_access_cli(command: &TeamCommand, args: &AccessCliArgs) -> Result<()> {
    // Team workflows reuse the standard access client because team APIs are
    // resolved within the selected Grafana org scope.
    match command {
        TeamCommand::List(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        TeamCommand::Browse(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        TeamCommand::Add(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        TeamCommand::Modify(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        TeamCommand::Export(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        TeamCommand::Import(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        TeamCommand::Diff(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        TeamCommand::Delete(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
    }
}

fn run_service_account_access_cli(
    command: &ServiceAccountCommand,
    args: &AccessCliArgs,
) -> Result<()> {
    // Service-account token subcommands stay nested here so the outer dispatcher
    // can treat service-account management as one domain branch.
    match command {
        ServiceAccountCommand::List(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        ServiceAccountCommand::Add(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        ServiceAccountCommand::Export(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        ServiceAccountCommand::Import(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        ServiceAccountCommand::Diff(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        ServiceAccountCommand::Delete(inner) => {
            run_access_cli_with_common(&inner.common, args, build_http_client)
        }
        ServiceAccountCommand::Token { command } => match command {
            ServiceAccountTokenCommand::Add(inner) => {
                run_access_cli_with_common(&inner.common, args, build_http_client)
            }
            ServiceAccountTokenCommand::Delete(inner) => {
                run_access_cli_with_common(&inner.common, args, build_http_client)
            }
        },
    }
}

/// Access execution path with request-function injection.
///
/// Receives fully parsed CLI args and routes each command branch to matching handler
/// functions that perform request execution.
pub fn run_access_cli_with_request<F>(mut request_json: F, args: &AccessCliArgs) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    // This branch fan-out is the testable core of the access domain: once callers
    // inject a request function, all remaining behavior is pure command routing.
    match &args.command {
        AccessCommand::User { command } => match command {
            UserCommand::List(args) => {
                let _ = user::list_users_with_request(&mut request_json, args)?;
            }
            UserCommand::Browse(args) => {
                #[cfg(feature = "tui")]
                {
                    let mut session = browse_terminal::TerminalSession::enter()?;
                    let mut next = BrowseSwitch::ToUser(args.clone());
                    loop {
                        next = match next {
                            BrowseSwitch::Exit => break,
                            BrowseSwitch::ToUser(inner) => user_browse::browse_users_in_session(
                                &mut session,
                                &mut request_json,
                                &inner,
                            )?,
                            BrowseSwitch::ToTeam(inner) => team_browse::browse_teams_in_session(
                                &mut session,
                                &mut request_json,
                                &inner,
                            )?,
                        };
                    }
                }
                #[cfg(not(feature = "tui"))]
                {
                    let _ = user_browse::browse_users_with_request(&mut request_json, args)?;
                }
            }
            UserCommand::Add(args) => {
                let _ = user::add_user_with_request(&mut request_json, args)?;
            }
            UserCommand::Modify(args) => {
                let _ = user::modify_user_with_request(&mut request_json, args)?;
            }
            UserCommand::Export(args) => {
                let _ = user::export_users_with_request(&mut request_json, args)?;
            }
            UserCommand::Import(args) => {
                let _ = user::import_users_with_request(&mut request_json, args)?;
            }
            UserCommand::Diff(args) => {
                let _ = user::diff_users_with_request(&mut request_json, args)?;
            }
            UserCommand::Delete(args) => {
                let _ = user::delete_user_with_request(&mut request_json, args)?;
            }
        },
        AccessCommand::Org { command } => match command {
            OrgCommand::List(args) => {
                let _ = org::list_orgs_with_request(&mut request_json, args)?;
            }
            OrgCommand::Add(args) => {
                let _ = org::add_org_with_request(&mut request_json, args)?;
            }
            OrgCommand::Modify(args) => {
                let _ = org::modify_org_with_request(&mut request_json, args)?;
            }
            OrgCommand::Export(args) => {
                let _ = org::export_orgs_with_request(&mut request_json, args)?;
            }
            OrgCommand::Import(args) => {
                let _ = org::import_orgs_with_request(&mut request_json, args)?;
            }
            OrgCommand::Diff(args) => {
                let _ = org::diff_orgs_with_request(&mut request_json, args)?;
            }
            OrgCommand::Delete(args) => {
                let _ = org::delete_org_with_request(&mut request_json, args)?;
            }
        },
        AccessCommand::Team { command } => match command {
            TeamCommand::List(args) => {
                let _ = team::list_teams_command_with_request(&mut request_json, args)?;
            }
            TeamCommand::Browse(args) => {
                #[cfg(feature = "tui")]
                {
                    let mut session = browse_terminal::TerminalSession::enter()?;
                    let mut next = BrowseSwitch::ToTeam(args.clone());
                    loop {
                        next = match next {
                            BrowseSwitch::Exit => break,
                            BrowseSwitch::ToUser(inner) => user_browse::browse_users_in_session(
                                &mut session,
                                &mut request_json,
                                &inner,
                            )?,
                            BrowseSwitch::ToTeam(inner) => team_browse::browse_teams_in_session(
                                &mut session,
                                &mut request_json,
                                &inner,
                            )?,
                        };
                    }
                }
                #[cfg(not(feature = "tui"))]
                {
                    let _ = team_browse::browse_teams_with_request(&mut request_json, args)?;
                }
            }
            TeamCommand::Add(args) => {
                let _ = team::add_team_with_request(&mut request_json, args)?;
            }
            TeamCommand::Modify(args) => {
                let _ = team::modify_team_with_request(&mut request_json, args)?;
            }
            TeamCommand::Export(args) => {
                let _ = team::export_teams_with_request(&mut request_json, args)?;
            }
            TeamCommand::Import(args) => {
                let _ = team::import_teams_with_request(&mut request_json, args)?;
            }
            TeamCommand::Diff(args) => {
                let _ = team::diff_teams_with_request(&mut request_json, args)?;
            }
            TeamCommand::Delete(args) => {
                let _ = pending_delete::delete_team_with_request(&mut request_json, args)?;
            }
        },
        AccessCommand::ServiceAccount { command } => match command {
            ServiceAccountCommand::List(args) => {
                let _ = service_account::list_service_accounts_command_with_request(
                    &mut request_json,
                    args,
                )?;
            }
            ServiceAccountCommand::Add(args) => {
                let _ = service_account::add_service_account_with_request(&mut request_json, args)?;
            }
            ServiceAccountCommand::Export(args) => {
                let _ =
                    service_account::export_service_accounts_with_request(&mut request_json, args)?;
            }
            ServiceAccountCommand::Import(args) => {
                let _ =
                    service_account::import_service_accounts_with_request(&mut request_json, args)?;
            }
            ServiceAccountCommand::Diff(args) => {
                let _ =
                    service_account::diff_service_accounts_with_request(&mut request_json, args)?;
            }
            ServiceAccountCommand::Delete(args) => {
                let _ =
                    pending_delete::delete_service_account_with_request(&mut request_json, args)?;
            }
            ServiceAccountCommand::Token { command } => match command {
                ServiceAccountTokenCommand::Add(args) => {
                    let _ = service_account::add_service_account_token_with_request(
                        &mut request_json,
                        args,
                    )?;
                }
                ServiceAccountTokenCommand::Delete(args) => {
                    let _ = pending_delete::delete_service_account_token_with_request(
                        &mut request_json,
                        args,
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
    match &args.command {
        AccessCommand::User { command } => run_user_access_cli(command, &args),
        AccessCommand::Org { command } => run_org_access_cli(command, &args),
        AccessCommand::Team { command } => run_team_access_cli(command, &args),
        AccessCommand::ServiceAccount { command } => run_service_account_access_cli(command, &args),
    }
}

#[cfg(test)]
#[path = "rust_tests.rs"]
mod access_rust_tests;
