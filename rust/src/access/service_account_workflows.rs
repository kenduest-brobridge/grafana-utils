//! Service-account workflow dispatcher.
//! Keeps the public access-service-account entrypoints stable while the
//! command implementations live in focused sibling modules.

#[path = "service_account_workflows_mutation.rs"]
mod service_account_workflows_mutation;
#[path = "service_account_workflows_support.rs"]
mod service_account_workflows_support;
#[path = "service_account_workflows_sync.rs"]
mod service_account_workflows_sync;

pub(crate) use service_account_workflows_mutation::{
    add_service_account_token_with_request, add_service_account_with_request,
    list_service_accounts_command_with_request,
};
pub(crate) use service_account_workflows_sync::{
    diff_service_accounts_with_request, export_service_accounts_with_request,
    import_service_accounts_with_request,
};
