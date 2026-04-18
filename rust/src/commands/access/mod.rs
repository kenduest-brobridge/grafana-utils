//! Access-management domain orchestrator.
//!
//! Owns access command dispatch, argument normalization, and the shared parser/model re-exports
//! used by the CLI and tests.
use reqwest::Method;
use serde_json::{Map, Value};
use std::path::Path;

use crate::common::{message, tool_version, value_as_object, Result};

pub(crate) const ACCESS_IMPORT_DRY_RUN_KIND: &str = "grafana-utils-access-import-dry-run";
pub(crate) const ACCESS_IMPORT_DRY_RUN_SCHEMA_VERSION: i64 = 1;

// Internal modules stay split by resource kind so user/org/team/service-account
// workflows can evolve independently while this file keeps only domain routing.
#[path = "access_plan.rs"]
mod access_plan;
#[path = "access_plan_org.rs"]
mod access_plan_org;
#[path = "access_plan_team.rs"]
mod access_plan_team;
#[path = "auth_materialize.rs"]
mod auth_materialize;
#[path = "browse_support.rs"]
mod browse_support;
#[path = "browse_terminal.rs"]
mod browse_terminal;
#[path = "cli_defs.rs"]
mod cli_defs;
#[path = "dispatch.rs"]
mod dispatch;
#[path = "facade_support.rs"]
mod facade_support;
#[path = "live_project_status.rs"]
mod live_project_status;
#[path = "org.rs"]
mod org;
#[path = "pending_delete.rs"]
mod pending_delete;
#[path = "project_status.rs"]
mod project_status;
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
    CommonCliArgsNoOrgId, DryRunOutputFormat, OrgAddArgs, OrgCommand, OrgDeleteArgs, OrgDiffArgs,
    OrgExportArgs, OrgImportArgs, OrgListArgs, OrgModifyArgs, Scope, ServiceAccountAddArgs,
    ServiceAccountCommand, ServiceAccountDiffArgs, ServiceAccountExportArgs,
    ServiceAccountImportArgs, ServiceAccountListArgs, ServiceAccountTokenAddArgs,
    ServiceAccountTokenCommand, TeamAddArgs, TeamBrowseArgs, TeamCommand, TeamDiffArgs,
    TeamExportArgs, TeamImportArgs, TeamListArgs, TeamModifyArgs, UserAddArgs, UserBrowseArgs,
    UserCommand, UserDeleteArgs, UserDiffArgs, UserExportArgs, UserImportArgs, UserListArgs,
    UserModifyArgs, ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS,
    ACCESS_EXPORT_KIND_TEAMS, ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME,
    ACCESS_EXPORT_VERSION, ACCESS_ORG_EXPORT_FILENAME, ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
    ACCESS_TEAM_EXPORT_FILENAME, ACCESS_USER_EXPORT_FILENAME, DEFAULT_ACCESS_ORG_EXPORT_DIR,
    DEFAULT_ACCESS_SERVICE_ACCOUNT_EXPORT_DIR, DEFAULT_ACCESS_TEAM_EXPORT_DIR,
    DEFAULT_ACCESS_USER_EXPORT_DIR, DEFAULT_PAGE_SIZE, DEFAULT_TIMEOUT, DEFAULT_URL,
};
#[allow(unused_imports)]
pub(crate) use facade_support::{
    build_access_live_domain_status, build_access_live_domain_status_with_request,
};
pub use pending_delete::{
    GroupCommandStage, ServiceAccountDeleteArgs, ServiceAccountTokenDeleteArgs, TeamDeleteArgs,
};
pub(crate) use project_status::{build_access_domain_status, AccessDomainStatusInputs};

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

pub(crate) fn build_access_import_dry_run_document(
    resource_kind: &str,
    rows: &[Map<String, Value>],
    processed: usize,
    created: usize,
    updated: usize,
    skipped: usize,
    source: &Path,
) -> Value {
    let blocked = rows
        .iter()
        .filter(|row| {
            matches!(row.get("blocked"), Some(Value::Bool(true)))
                || matches!(row.get("status"), Some(Value::String(status)) if status == "blocked")
        })
        .count();
    Value::Object(Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(ACCESS_IMPORT_DRY_RUN_KIND.to_string()),
        ),
        (
            "schemaVersion".to_string(),
            Value::Number(ACCESS_IMPORT_DRY_RUN_SCHEMA_VERSION.into()),
        ),
        (
            "toolVersion".to_string(),
            Value::String(tool_version().to_string()),
        ),
        ("reviewRequired".to_string(), Value::Bool(true)),
        ("reviewed".to_string(), Value::Bool(false)),
        (
            "resourceKind".to_string(),
            Value::String(resource_kind.to_string()),
        ),
        (
            "rows".to_string(),
            Value::Array(rows.iter().cloned().map(Value::Object).collect()),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "processed".to_string(),
                    Value::Number((processed as i64).into()),
                ),
                (
                    "created".to_string(),
                    Value::Number((created as i64).into()),
                ),
                (
                    "updated".to_string(),
                    Value::Number((updated as i64).into()),
                ),
                (
                    "skipped".to_string(),
                    Value::Number((skipped as i64).into()),
                ),
                (
                    "blocked".to_string(),
                    Value::Number((blocked as i64).into()),
                ),
                (
                    "source".to_string(),
                    Value::String(source.to_string_lossy().to_string()),
                ),
            ])),
        ),
    ]))
}

pub fn run_access_cli(args: AccessCliArgs) -> Result<()> {
    // Access CLI boundary:
    // normalize and materialize auth/headers once, then dispatch to user/org/team/service-account handlers.
    let args = normalize_access_cli_args(args);
    if let AccessCommand::Plan(plan_args) = &args.command {
        if plan_args.list_columns {
            access_plan::print_access_plan_columns();
            return Ok(());
        }
    }
    match &args.command {
        AccessCommand::User {
            command: UserCommand::List(inner),
        } if inner.list_columns => {
            let _ = user::list_users_from_input_dir(inner)?;
            return Ok(());
        }
        AccessCommand::Team {
            command: TeamCommand::List(inner),
        } if inner.list_columns => {
            let _ = team::list_teams_from_input_dir(inner)?;
            return Ok(());
        }
        AccessCommand::ServiceAccount {
            command: ServiceAccountCommand::List(inner),
        } if inner.list_columns => {
            let _ = service_account::list_service_accounts_from_input_dir(inner)?;
            return Ok(());
        }
        _ => {}
    }
    let args = auth_materialize::materialize_access_command_auth(args)?;
    dispatch::run_access_cli_with_materialized_args(&args)
}

pub use dispatch::{run_access_cli_with_client, run_access_cli_with_request};

#[cfg(test)]
#[path = "rust_tests.rs"]
mod access_rust_tests;
