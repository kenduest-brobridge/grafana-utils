use reqwest::Method;
use serde_json::Value;

use crate::common::{message, Result};
use crate::http::JsonHttpClient;

use super::cli_defs::{build_http_client, build_http_client_no_org_id};
use super::{
    browse_support, browse_terminal, org, pending_delete, service_account, team, team_browse, user,
    user_browse, AccessCliArgs, AccessCommand, OrgCommand, ServiceAccountCommand,
    ServiceAccountTokenCommand, TeamCommand, UserCommand,
};

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
    let client = build_client(common)?;
    run_access_cli_with_client(&client, args)
}

fn run_user_access_cli(command: &UserCommand, args: &AccessCliArgs) -> Result<()> {
    match command {
        UserCommand::List(inner) => {
            if inner.input_dir.is_some() {
                user::list_users_from_input_dir(inner)?;
                Ok(())
            } else {
                run_access_cli_with_common(&inner.common, args, build_http_client)
            }
        }
        UserCommand::Browse(inner) => {
            if inner.input_dir.is_some() {
                run_access_cli_with_request(
                    |_method, _path, _params, _payload| {
                        Err(message(
                            "Local access user browse should not call the live request layer.",
                        ))
                    },
                    args,
                )
            } else {
                run_access_cli_with_common(&inner.common, args, build_http_client)
            }
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
    match command {
        OrgCommand::List(inner) => {
            if inner.input_dir.is_some() {
                org::list_orgs_from_input_dir(inner)?;
                Ok(())
            } else {
                run_access_cli_with_common(&inner.common, args, build_http_client_no_org_id)
            }
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
    match command {
        TeamCommand::List(inner) => {
            if inner.input_dir.is_some() {
                team::list_teams_from_input_dir(inner)?;
                Ok(())
            } else {
                run_access_cli_with_common(&inner.common, args, build_http_client)
            }
        }
        TeamCommand::Browse(inner) => {
            if inner.input_dir.is_some() {
                run_access_cli_with_request(
                    |_method, _path, _params, _payload| {
                        Err(message(
                            "Local access team browse should not call the live request layer.",
                        ))
                    },
                    args,
                )
            } else {
                run_access_cli_with_common(&inner.common, args, build_http_client)
            }
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
    match command {
        ServiceAccountCommand::List(inner) => {
            if inner.input_dir.is_some() {
                service_account::list_service_accounts_from_input_dir(inner)?;
                Ok(())
            } else {
                run_access_cli_with_common(&inner.common, args, build_http_client)
            }
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

pub(crate) fn run_access_cli_with_materialized_args(args: &AccessCliArgs) -> Result<()> {
    match &args.command {
        AccessCommand::User { command } => run_user_access_cli(command, args),
        AccessCommand::Org { command } => run_org_access_cli(command, args),
        AccessCommand::Team { command } => run_team_access_cli(command, args),
        AccessCommand::ServiceAccount { command } => run_service_account_access_cli(command, args),
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
    match &args.command {
        AccessCommand::User { command } => match command {
            UserCommand::List(args) => {
                if args.input_dir.is_some() {
                    let _ = user::list_users_from_input_dir(args)?;
                } else {
                    let _ = user::list_users_with_request(&mut request_json, args)?;
                }
            }
            UserCommand::Browse(args) => {
                #[cfg(feature = "tui")]
                {
                    let mut session = browse_terminal::TerminalSession::enter()?;
                    let mut next = browse_support::BrowseSwitch::ToUser(args.clone());
                    loop {
                        next = match next {
                            browse_support::BrowseSwitch::Exit => break,
                            browse_support::BrowseSwitch::ToUser(inner) => {
                                user_browse::browse_users_in_session(
                                    &mut session,
                                    &mut request_json,
                                    &inner,
                                )?
                            }
                            browse_support::BrowseSwitch::ToTeam(inner) => {
                                team_browse::browse_teams_in_session(
                                    &mut session,
                                    &mut request_json,
                                    &inner,
                                )?
                            }
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
                if args.input_dir.is_some() {
                    let _ = org::list_orgs_from_input_dir(args)?;
                } else {
                    let _ = org::list_orgs_with_request(&mut request_json, args)?;
                }
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
                if args.input_dir.is_some() {
                    let _ = team::list_teams_from_input_dir(args)?;
                } else {
                    let _ = team::list_teams_command_with_request(&mut request_json, args)?;
                }
            }
            TeamCommand::Browse(args) => {
                #[cfg(feature = "tui")]
                {
                    let mut session = browse_terminal::TerminalSession::enter()?;
                    let mut next = browse_support::BrowseSwitch::ToTeam(args.clone());
                    loop {
                        next = match next {
                            browse_support::BrowseSwitch::Exit => break,
                            browse_support::BrowseSwitch::ToUser(inner) => {
                                user_browse::browse_users_in_session(
                                    &mut session,
                                    &mut request_json,
                                    &inner,
                                )?
                            }
                            browse_support::BrowseSwitch::ToTeam(inner) => {
                                team_browse::browse_teams_in_session(
                                    &mut session,
                                    &mut request_json,
                                    &inner,
                                )?
                            }
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
                if args.input_dir.is_some() {
                    let _ = service_account::list_service_accounts_from_input_dir(args)?;
                } else {
                    let _ = service_account::list_service_accounts_command_with_request(
                        &mut request_json,
                        args,
                    )?;
                }
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
