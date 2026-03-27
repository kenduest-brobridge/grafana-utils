//! User export workflow helpers.

use reqwest::Method;
use serde_json::{Map, Value};
use std::path::Path;

use crate::access::render::{map_get_text, normalize_org_role, normalize_user_row, scalar_text};
use crate::common::{message, string_field, write_json_file, Result};

use super::super::{
    build_auth_context, iter_global_users_with_request, list_org_users_with_request,
    list_user_teams_with_request, validate_user_scope_auth, Scope, UserExportArgs,
    ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME, ACCESS_EXPORT_VERSION,
    ACCESS_USER_EXPORT_FILENAME, DEFAULT_PAGE_SIZE,
};

fn assert_not_overwrite(path: &Path, dry_run: bool, overwrite: bool) -> Result<()> {
    if dry_run || !path.exists() || overwrite {
        return Ok(());
    }
    Err(message(format!(
        "Refusing to overwrite existing file: {}. Use --overwrite.",
        path.display()
    )))
}

fn build_access_export_metadata(
    source_url: &str,
    source_dir: &Path,
    record_count: usize,
) -> Map<String, Value> {
    Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(ACCESS_EXPORT_KIND_USERS.to_string()),
        ),
        (
            "version".to_string(),
            Value::Number((ACCESS_EXPORT_VERSION).into()),
        ),
        (
            "sourceUrl".to_string(),
            Value::String(source_url.to_string()),
        ),
        (
            "recordCount".to_string(),
            Value::Number((record_count as i64).into()),
        ),
        (
            "sourceDir".to_string(),
            Value::String(source_dir.to_string_lossy().to_string()),
        ),
    ])
}

fn build_user_export_records<F>(
    mut request_json: F,
    args: &UserExportArgs,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut rows = match args.scope {
        Scope::Org => list_org_users_with_request(&mut request_json)?
            .into_iter()
            .map(|item| normalize_user_row(&item, &Scope::Org))
            .collect::<Vec<Map<String, Value>>>(),
        Scope::Global => iter_global_users_with_request(&mut request_json, DEFAULT_PAGE_SIZE)?
            .into_iter()
            .map(|item| normalize_user_row(&item, &Scope::Global))
            .collect::<Vec<Map<String, Value>>>(),
    };

    if args.with_teams {
        for row in &mut rows {
            let user_id = map_get_text(row, "id");
            let mut team_names = list_user_teams_with_request(&mut request_json, &user_id)?
                .into_iter()
                .map(|team| string_field(&team, "name", ""))
                .filter(|name| !name.is_empty())
                .collect::<Vec<String>>();
            team_names.sort();
            team_names.dedup();
            row.insert(
                "teams".to_string(),
                Value::Array(team_names.into_iter().map(Value::String).collect()),
            );
        }
    }

    Ok(rows)
}

pub(crate) fn export_users_with_request<F>(
    mut request_json: F,
    args: &UserExportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_scope_auth(&args.scope, args.with_teams, &auth_mode)?;
    let records = build_user_export_records(&mut request_json, args)?;

    let users_path = args.export_dir.join(ACCESS_USER_EXPORT_FILENAME);
    let metadata_path = args.export_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
    assert_not_overwrite(&users_path, args.dry_run, args.overwrite)?;
    assert_not_overwrite(&metadata_path, args.dry_run, args.overwrite)?;

    if !args.dry_run {
        let payload = Value::Object(Map::from_iter(vec![
            (
                "kind".to_string(),
                Value::String(ACCESS_EXPORT_KIND_USERS.to_string()),
            ),
            (
                "version".to_string(),
                Value::Number((ACCESS_EXPORT_VERSION).into()),
            ),
            (
                "records".to_string(),
                Value::Array(records.iter().cloned().map(Value::Object).collect()),
            ),
        ]));
        write_json_file(&users_path, &payload, args.overwrite)?;
        write_json_file(
            &metadata_path,
            &Value::Object(build_access_export_metadata(
                &args.common.url,
                &args.export_dir,
                records.len(),
            )),
            args.overwrite,
        )?;
    }

    let action = if args.dry_run {
        "Would export"
    } else {
        "Exported"
    };
    println!(
        "{} {} user(s) from {} -> {} and {}",
        action,
        records.len(),
        args.common.url,
        users_path.display(),
        metadata_path.display()
    );

    Ok(records.len())
}
