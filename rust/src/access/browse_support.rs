//! Shared support for access browse switching.
//!
//! This keeps the TUI-only browse transition helpers out of the access facade.

use super::cli_defs::{TeamBrowseArgs, UserBrowseArgs, DEFAULT_PAGE_SIZE, Scope};

#[cfg_attr(not(feature = "tui"), allow(dead_code))]
#[derive(Clone, Debug)]
pub(crate) enum BrowseSwitch {
    Exit,
    ToUser(UserBrowseArgs),
    ToTeam(TeamBrowseArgs),
}

#[cfg_attr(not(feature = "tui"), allow(dead_code))]
pub(crate) fn default_team_browse_args_from_user(args: &UserBrowseArgs) -> TeamBrowseArgs {
    TeamBrowseArgs {
        common: args.common.clone(),
        query: None,
        name: None,
        with_members: true,
        page: 1,
        per_page: DEFAULT_PAGE_SIZE,
    }
}

#[cfg_attr(not(feature = "tui"), allow(dead_code))]
pub(crate) fn default_user_browse_args_from_team(args: &TeamBrowseArgs) -> UserBrowseArgs {
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
