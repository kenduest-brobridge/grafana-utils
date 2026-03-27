//! Access user command handlers.
//! Supports user listing/lookup and CRUD operations with org/user scope-aware rendering paths.
#![allow(unused_imports)]

use super::render;
use super::{
    build_auth_context, request_array, request_object, request_object_list_field, Scope,
    UserAddArgs, UserDeleteArgs, UserDiffArgs, UserExportArgs, UserImportArgs, UserListArgs,
    UserModifyArgs, ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME,
    ACCESS_EXPORT_VERSION, ACCESS_USER_EXPORT_FILENAME, DEFAULT_PAGE_SIZE,
};

#[path = "user_mutation.rs"]
mod user_mutation;
#[path = "user_read.rs"]
mod user_read;
#[path = "user_workflows.rs"]
mod user_workflows;

pub(crate) use user_mutation::{
    add_user_with_request, create_user_with_request, delete_global_user_with_request,
    delete_org_user_with_request, delete_user_with_request, get_user_with_request,
    modify_user_with_request, update_user_org_role_with_request, update_user_password_with_request,
    update_user_permissions_with_request, update_user_with_request,
};
pub(crate) use user_read::{
    annotate_user_account_scope, iter_global_users_with_request, list_org_users_with_request,
    list_user_teams_with_request, list_users_with_request, lookup_global_user_by_identity,
    lookup_org_user_by_identity, validate_user_scope_auth,
};
#[cfg(test)]
pub(crate) use user_workflows::build_user_import_dry_run_document;
pub(crate) use user_workflows::{
    diff_users_with_request, export_users_with_request, import_users_with_request,
};
