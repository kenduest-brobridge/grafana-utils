//! Access delete handlers for teams, service accounts, and service-account tokens.
//!
//! Keeps the public delete workflow API stable while the concrete validation and
//! per-resource handlers live in focused sibling modules.

#[path = "pending_delete_service_account.rs"]
mod pending_delete_service_account;
#[path = "pending_delete_support.rs"]
mod pending_delete_support;
#[path = "pending_delete_team.rs"]
mod pending_delete_team;

pub(crate) use pending_delete_service_account::{
    delete_service_account_token_with_request, delete_service_account_with_request,
};
pub(crate) use pending_delete_support::{
    format_prompt_row, print_delete_confirmation_summary, prompt_confirm_delete,
    prompt_select_index, prompt_select_indexes, validate_delete_prompt,
};
pub use pending_delete_support::{
    GroupCommandStage, ServiceAccountDeleteArgs, ServiceAccountTokenDeleteArgs, TeamDeleteArgs,
};
pub(crate) use pending_delete_team::delete_team_with_request;
