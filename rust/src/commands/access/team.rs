//! Access team command handlers.
//! Facade over list, modify, and import/export workflows.

#[path = "team_import_export.rs"]
mod team_import_export;
#[path = "team_list.rs"]
mod team_list;
#[path = "team_modify.rs"]
mod team_modify;

#[allow(unused_imports)]
pub(crate) use super::team_import_export_diff::build_team_import_dry_run_document;
#[allow(unused_imports)]
pub(crate) use super::team_import_export_diff::load_team_import_records;
pub(crate) use super::team_runtime::iter_teams_with_request;
#[allow(unused_imports)]
pub(crate) use super::team_runtime::{list_team_members_with_request, team_member_identity};
pub(crate) use team_import_export::{export_teams_with_request, import_teams_with_request};
pub(crate) use team_list::{
    diff_teams_with_request, list_teams_command_with_request, list_teams_from_input_dir,
};
pub(crate) use team_modify::{add_team_with_request, modify_team_with_request};
