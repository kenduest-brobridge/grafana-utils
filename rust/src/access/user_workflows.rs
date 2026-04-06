//! User export/import/diff workflow helpers.
#![allow(unused_imports)]

use reqwest::Method;
use rpassword::prompt_password;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::common::{
    load_json_object_file, message, string_field, value_as_object, write_json_file, Result,
};

use super::super::render::{
    bool_label, format_table, map_get_text, normalize_org_role, normalize_user_row, paginate_rows,
    render_csv, render_objects_json, scalar_text, user_account_scope_text, user_matches,
    user_scope_text, user_summary_line, user_table_rows, value_bool,
};
use super::{
    build_auth_context, create_user_with_request, delete_global_user_with_request,
    delete_org_user_with_request, get_user_with_request, iter_global_users_with_request,
    list_org_users_with_request, list_user_teams_with_request, lookup_global_user_by_identity,
    lookup_org_user_by_identity, request_array, request_object, request_object_list_field,
    update_user_org_role_with_request, update_user_permissions_with_request,
    update_user_with_request, validate_user_scope_auth, Scope, UserAddArgs, UserDeleteArgs,
    UserDiffArgs, UserExportArgs, UserImportArgs, UserListArgs, UserModifyArgs,
    ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME, ACCESS_EXPORT_VERSION,
    ACCESS_USER_EXPORT_FILENAME, DEFAULT_PAGE_SIZE,
};

fn user_id_json_value(user_id: &str) -> Value {
    match user_id.trim().parse::<u64>() {
        Ok(value) => Value::Number(value.into()),
        Err(_) => Value::String(user_id.to_string()),
    }
}

fn build_access_import_dry_run_row(
    index: usize,
    identity: &str,
    action: &str,
    detail: &str,
) -> Map<String, Value> {
    Map::from_iter(vec![
        ("index".to_string(), Value::String(index.to_string())),
        ("identity".to_string(), Value::String(identity.to_string())),
        ("action".to_string(), Value::String(action.to_string())),
        ("detail".to_string(), Value::String(detail.to_string())),
    ])
}

fn build_access_import_dry_run_rows(rows: &[Map<String, Value>]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            vec![
                map_get_text(row, "index"),
                map_get_text(row, "identity"),
                map_get_text(row, "action"),
                map_get_text(row, "detail"),
            ]
        })
        .collect()
}

pub(crate) fn build_user_import_dry_run_document(
    rows: &[Map<String, Value>],
    processed: usize,
    created: usize,
    updated: usize,
    skipped: usize,
    source: &Path,
) -> Value {
    Value::Object(Map::from_iter(vec![
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
                    "source".to_string(),
                    Value::String(source.to_string_lossy().to_string()),
                ),
            ])),
        ),
    ]))
}

fn validate_user_import_dry_run_output(args: &UserImportArgs) -> Result<()> {
    if (args.table || args.json) && !args.dry_run {
        return Err(message(
            "--table/--json for user import are only supported with --dry-run.",
        ));
    }
    if args.table && args.json {
        return Err(message(
            "--table and --json cannot be used together for user import.",
        ));
    }
    Ok(())
}

fn normalize_access_identity(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

type DiffPayload = (String, Map<String, Value>);
type DiffPayloadMap = BTreeMap<String, DiffPayload>;

fn parse_access_identity_list(value: &Value) -> Vec<String> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: user.rs:normalize_user_for_diff
    // Downstream callees: user.rs:normalize_access_identity

    value
        .as_array()
        .map(|values| {
            let mut seen = BTreeSet::new();
            values
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|identity| !identity.is_empty())
                .filter_map(|identity| {
                    let lowered = normalize_access_identity(identity);
                    if seen.insert(lowered.clone()) {
                        Some(identity.to_string())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[path = "user_workflows_diff.rs"]
mod user_workflows_diff;
#[path = "user_workflows_import_export.rs"]
mod user_workflows_import_export;

pub(crate) use user_workflows_diff::diff_users_with_request;
pub(crate) use user_workflows_import_export::{
    export_users_with_request, import_users_with_request, load_access_import_records,
};
