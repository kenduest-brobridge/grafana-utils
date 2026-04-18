use crate::common::Result;

use super::cli_defs::{materialize_access_common_auth, materialize_access_common_auth_no_org_id};
use super::{
    AccessCliArgs, AccessCommand, OrgCommand, ServiceAccountCommand, ServiceAccountTokenCommand,
    TeamCommand, UserCommand,
};

pub(crate) fn materialize_access_command_auth(mut args: AccessCliArgs) -> Result<AccessCliArgs> {
    match &mut args.command {
        AccessCommand::User { command } => match command {
            UserCommand::List(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            UserCommand::Browse(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            UserCommand::Add(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            UserCommand::Modify(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            UserCommand::Export(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            UserCommand::Import(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            UserCommand::Diff(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            UserCommand::Delete(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
        },
        AccessCommand::Plan(inner) => {
            inner.common = materialize_access_common_auth(inner.common.clone())?
        }
        AccessCommand::Org { command } => match command {
            OrgCommand::List(inner) => {
                inner.common = materialize_access_common_auth_no_org_id(inner.common.clone())?
            }
            OrgCommand::Add(inner) => {
                inner.common = materialize_access_common_auth_no_org_id(inner.common.clone())?
            }
            OrgCommand::Modify(inner) => {
                inner.common = materialize_access_common_auth_no_org_id(inner.common.clone())?
            }
            OrgCommand::Export(inner) => {
                inner.common = materialize_access_common_auth_no_org_id(inner.common.clone())?
            }
            OrgCommand::Import(inner) => {
                inner.common = materialize_access_common_auth_no_org_id(inner.common.clone())?
            }
            OrgCommand::Diff(inner) => {
                inner.common = materialize_access_common_auth_no_org_id(inner.common.clone())?
            }
            OrgCommand::Delete(inner) => {
                inner.common = materialize_access_common_auth_no_org_id(inner.common.clone())?
            }
        },
        AccessCommand::Team { command } => match command {
            TeamCommand::List(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            TeamCommand::Browse(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            TeamCommand::Add(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            TeamCommand::Modify(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            TeamCommand::Export(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            TeamCommand::Import(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            TeamCommand::Diff(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            TeamCommand::Delete(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
        },
        AccessCommand::ServiceAccount { command } => match command {
            ServiceAccountCommand::List(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            ServiceAccountCommand::Add(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            ServiceAccountCommand::Export(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            ServiceAccountCommand::Import(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            ServiceAccountCommand::Diff(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            ServiceAccountCommand::Delete(inner) => {
                inner.common = materialize_access_common_auth(inner.common.clone())?
            }
            ServiceAccountCommand::Token { command } => match command {
                ServiceAccountTokenCommand::Add(inner) => {
                    inner.common = materialize_access_common_auth(inner.common.clone())?
                }
                ServiceAccountTokenCommand::Delete(inner) => {
                    inner.common = materialize_access_common_auth(inner.common.clone())?
                }
            },
        },
    }
    Ok(args)
}
