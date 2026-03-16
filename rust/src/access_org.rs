//! Access organization command handlers.
//! Handles org CRUD plus snapshot export/import behind shared access-request wrappers.
use reqwest::Method;
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

use crate::common::{message, string_field, value_as_object, write_json_file, Result};

use super::access_render::{format_table, render_csv, render_objects_json, scalar_text};
use super::{
    request_array, request_object, OrgAddArgs, OrgDeleteArgs, OrgExportArgs, OrgImportArgs,
    OrgListArgs, OrgModifyArgs, ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_METADATA_FILENAME,
    ACCESS_EXPORT_VERSION, ACCESS_ORG_EXPORT_FILENAME,
};
use crate::access::access_cli_defs::{build_auth_context_no_org_id, CommonCliArgsNoOrgId};

fn validate_basic_auth_only(common: &CommonCliArgsNoOrgId) -> Result<()> {
    let auth_mode = build_auth_context_no_org_id(common)?.auth_mode;
    if auth_mode != "basic" {
        Err(message(
            "Organization commands require Basic auth (--basic-user / --basic-password).",
        ))
    } else {
        Ok(())
    }
}

fn list_organizations_with_request<F>(mut request_json: F) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_array(
        &mut request_json,
        Method::GET,
        "/api/orgs",
        &[],
        None,
        "Unexpected organization list response from Grafana.",
    )
}

fn create_organization_with_request<F>(
    mut request_json: F,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        "/api/orgs",
        &[],
        Some(payload),
        "Unexpected organization create response from Grafana.",
    )
}

fn update_organization_with_request<F>(
    mut request_json: F,
    org_id: &str,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PUT,
        &format!("/api/orgs/{org_id}"),
        &[],
        Some(payload),
        &format!("Unexpected organization update response for Grafana org {org_id}."),
    )
}

fn delete_organization_with_request<F>(
    mut request_json: F,
    org_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/orgs/{org_id}"),
        &[],
        None,
        &format!("Unexpected organization delete response for Grafana org {org_id}."),
    )
}

fn list_org_users_with_request<F>(
    mut request_json: F,
    org_id: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_array(
        &mut request_json,
        Method::GET,
        &format!("/api/orgs/{org_id}/users"),
        &[],
        None,
        &format!("Unexpected organization user list response for Grafana org {org_id}."),
    )
}

fn add_user_to_org_with_request<F>(
    mut request_json: F,
    org_id: &str,
    login_or_email: &str,
    role: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        &format!("/api/orgs/{org_id}/users"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![
            (
                "loginOrEmail".to_string(),
                Value::String(login_or_email.to_string()),
            ),
            ("role".to_string(), Value::String(role.to_string())),
        ]))),
        &format!("Unexpected organization add-user response for Grafana org {org_id}."),
    )
}

fn update_org_user_role_with_request<F>(
    mut request_json: F,
    org_id: &str,
    user_id: &str,
    role: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PATCH,
        &format!("/api/orgs/{org_id}/users/{user_id}"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "role".to_string(),
            Value::String(role.to_string()),
        )]))),
        &format!(
            "Unexpected organization user role update response for Grafana org {org_id} user {user_id}."
        ),
    )
}

fn normalize_org_user_row(user: &Map<String, Value>) -> Map<String, Value> {
    Map::from_iter(vec![
        (
            "userId".to_string(),
            Value::String({
                let user_id = scalar_text(user.get("userId"));
                if user_id.is_empty() {
                    scalar_text(user.get("id"))
                } else {
                    user_id
                }
            }),
        ),
        (
            "login".to_string(),
            Value::String(string_field(user, "login", "")),
        ),
        (
            "email".to_string(),
            Value::String(string_field(user, "email", "")),
        ),
        (
            "name".to_string(),
            Value::String(string_field(user, "name", "")),
        ),
        (
            "orgRole".to_string(),
            Value::String(
                string_field(user, "role", "")
                    .trim()
                    .replace("NoBasicRole", "None"),
            ),
        ),
    ])
}

fn normalize_org_row(org: &Map<String, Value>) -> Map<String, Value> {
    let users = match org.get("users") {
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(|item| value_as_object(item, "Unexpected org user record.").ok())
            .map(normalize_org_user_row)
            .map(Value::Object)
            .collect::<Vec<Value>>(),
        _ => Vec::new(),
    };
    let user_count = match org.get("userCount") {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        _ => users.len().to_string(),
    };
    Map::from_iter(vec![
        (
            "id".to_string(),
            Value::String({
                let id = scalar_text(org.get("id"));
                if id.is_empty() {
                    scalar_text(org.get("orgId"))
                } else {
                    id
                }
            }),
        ),
        (
            "name".to_string(),
            Value::String(string_field(org, "name", "")),
        ),
        ("userCount".to_string(), Value::String(user_count)),
        ("users".to_string(), Value::Array(users)),
    ])
}

fn org_matches(org: &Map<String, Value>, args: &OrgListArgs) -> bool {
    if let Some(org_id) = args.org_id {
        if scalar_text(org.get("id")) != org_id.to_string() {
            return false;
        }
    }
    if let Some(name) = &args.name {
        if string_field(org, "name", "") != *name {
            return false;
        }
    }
    if let Some(query) = &args.query {
        if !string_field(org, "name", "")
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase())
        {
            return false;
        }
    }
    true
}

fn org_table_rows(rows: &[Map<String, Value>]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            vec![
                scalar_text(row.get("id")),
                string_field(row, "name", ""),
                scalar_text(row.get("userCount")),
            ]
        })
        .collect()
}

fn org_summary_line(row: &Map<String, Value>) -> String {
    format!(
        "id={} name={} userCount={}",
        scalar_text(row.get("id")),
        string_field(row, "name", ""),
        scalar_text(row.get("userCount"))
    )
}

fn assert_not_overwrite(path: &Path, dry_run: bool, overwrite: bool) -> Result<()> {
    if dry_run || !path.exists() || overwrite {
        return Ok(());
    }
    Err(message(format!(
        "Refusing to overwrite existing file: {}. Use --overwrite.",
        path.display()
    )))
}

fn build_org_export_metadata(source_url: &str, source_dir: &Path, record_count: usize) -> Map<String, Value> {
    Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(ACCESS_EXPORT_KIND_ORGS.to_string()),
        ),
        (
            "version".to_string(),
            Value::Number(ACCESS_EXPORT_VERSION.into()),
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

fn load_org_import_records(import_dir: &Path) -> Result<Vec<Map<String, Value>>> {
    let path = import_dir.join(ACCESS_ORG_EXPORT_FILENAME);
    if !path.is_file() {
        return Err(message(format!(
            "Access import file not found: {}",
            path.display()
        )));
    }
    let raw = fs::read_to_string(&path)?;
    let payload: Value = serde_json::from_str(&raw)?;
    let records = match payload {
        Value::Array(values) => values,
        Value::Object(object) => {
            if let Some(kind) = object.get("kind").and_then(Value::as_str) {
                if kind != ACCESS_EXPORT_KIND_ORGS {
                    return Err(message(format!(
                        "Access import kind mismatch in {}: expected {}, got {}",
                        path.display(),
                        ACCESS_EXPORT_KIND_ORGS,
                        kind
                    )));
                }
            }
            if let Some(version) = object.get("version").and_then(Value::as_i64) {
                if version > ACCESS_EXPORT_VERSION {
                    return Err(message(format!(
                        "Unsupported access import version {} in {}. Supported <= {}.",
                        version,
                        path.display(),
                        ACCESS_EXPORT_VERSION
                    )));
                }
            }
            object
                .get("records")
                .cloned()
                .ok_or_else(|| {
                    message(format!(
                        "Access import bundle is missing a records list: {}",
                        path.display()
                    ))
                })?
                .as_array()
                .cloned()
                .ok_or_else(|| {
                    message(format!(
                        "Access import records must be a list in {}.",
                        path.display()
                    ))
                })?
        }
        _ => {
            return Err(message(format!(
                "Unsupported access import payload in {}.",
                path.display()
            )));
        }
    };
    records
        .into_iter()
        .map(|value| {
            value_as_object(&value, "Access import entry must be an object.").map(|object| object.clone())
        })
        .collect()
}

fn lookup_org_by_identity<F>(
    mut request_json: F,
    org_id: Option<i64>,
    name: Option<&str>,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let orgs = list_organizations_with_request(&mut request_json)?;
    orgs.into_iter()
        .find(|org| {
            org_id.is_some_and(|value| scalar_text(org.get("id")) == value.to_string())
                || name.is_some_and(|value| string_field(org, "name", "") == value)
        })
        .map(|org| normalize_org_row(&org))
        .ok_or_else(|| message("Grafana org lookup did not find a matching organization."))
}

pub(crate) fn list_orgs_with_request<F>(mut request_json: F, args: &OrgListArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_basic_auth_only(&args.common)?;
    let mut rows = list_organizations_with_request(&mut request_json)?
        .into_iter()
        .map(|item| normalize_org_row(&item))
        .collect::<Vec<Map<String, Value>>>();
    rows.retain(|row| org_matches(row, args));
    if args.with_users {
        for row in rows.iter_mut() {
            let org_id = scalar_text(row.get("id"));
            let users = list_org_users_with_request(&mut request_json, &org_id)?
                .into_iter()
                .map(|user| normalize_org_user_row(&user))
                .map(Value::Object)
                .collect::<Vec<Value>>();
            row.insert(
                "userCount".to_string(),
                Value::String(users.len().to_string()),
            );
            row.insert("users".to_string(), Value::Array(users));
        }
    }
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.table {
        for line in format_table(&["ID", "NAME", "USER_COUNT"], &org_table_rows(&rows)) {
            println!("{line}");
        }
        println!();
        println!("Listed {} org(s) at {}", rows.len(), args.common.url);
    } else if args.csv {
        for line in render_csv(&["id", "name", "userCount"], &org_table_rows(&rows)) {
            println!("{line}");
        }
    } else {
        for row in &rows {
            println!("{}", org_summary_line(row));
        }
        println!();
        println!("Listed {} org(s) at {}", rows.len(), args.common.url);
    }
    Ok(rows.len())
}

pub(crate) fn add_org_with_request<F>(mut request_json: F, args: &OrgAddArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_basic_auth_only(&args.common)?;
    let payload = Value::Object(Map::from_iter(vec![(
        "name".to_string(),
        Value::String(args.name.clone()),
    )]));
    let created = create_organization_with_request(&mut request_json, &payload)?;
    let row = Map::from_iter(vec![
        (
            "id".to_string(),
            Value::String({
                let id = scalar_text(created.get("orgId"));
                if id.is_empty() {
                    scalar_text(created.get("id"))
                } else {
                    id
                }
            }),
        ),
        ("name".to_string(), Value::String(args.name.clone())),
        ("userCount".to_string(), Value::String("0".to_string())),
        ("users".to_string(), Value::Array(Vec::new())),
    ]);
    if args.json {
        println!("{}", render_objects_json(&[row])?);
    } else {
        println!(
            "Created org {} -> id={}",
            args.name,
            scalar_text(created.get("orgId"))
        );
    }
    Ok(0)
}

pub(crate) fn modify_org_with_request<F>(mut request_json: F, args: &OrgModifyArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_basic_auth_only(&args.common)?;
    let org = lookup_org_by_identity(
        &mut request_json,
        args.org_id,
        args.name.as_deref(),
    )?;
    let org_id = scalar_text(org.get("id"));
    let payload = Value::Object(Map::from_iter(vec![(
        "name".to_string(),
        Value::String(args.set_name.clone()),
    )]));
    let _ = update_organization_with_request(&mut request_json, &org_id, &payload)?;
    let row = Map::from_iter(vec![
        ("id".to_string(), Value::String(org_id.clone())),
        ("name".to_string(), Value::String(args.set_name.clone())),
        (
            "previousName".to_string(),
            Value::String(string_field(&org, "name", "")),
        ),
    ]);
    if args.json {
        println!("{}", render_objects_json(&[row])?);
    } else {
        println!(
            "Modified org {} -> id={} name={}",
            string_field(&org, "name", ""),
            org_id,
            args.set_name
        );
    }
    Ok(0)
}

pub(crate) fn delete_org_with_request<F>(mut request_json: F, args: &OrgDeleteArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_basic_auth_only(&args.common)?;
    if !args.yes {
        return Err(message("Org delete requires --yes."));
    }
    let org = lookup_org_by_identity(
        &mut request_json,
        args.org_id,
        args.name.as_deref(),
    )?;
    let org_id = scalar_text(org.get("id"));
    let delete_payload = delete_organization_with_request(&mut request_json, &org_id)?;
    let row = Map::from_iter(vec![
        ("id".to_string(), Value::String(org_id.clone())),
        (
            "name".to_string(),
            Value::String(string_field(&org, "name", "")),
        ),
        (
            "message".to_string(),
            Value::String(string_field(&delete_payload, "message", "")),
        ),
    ]);
    if args.json {
        println!("{}", render_objects_json(&[row])?);
    } else {
        println!(
            "Deleted org {} -> id={}",
            string_field(&org, "name", ""),
            org_id
        );
    }
    Ok(0)
}

pub(crate) fn export_orgs_with_request<F>(mut request_json: F, args: &OrgExportArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_basic_auth_only(&args.common)?;
    let payload_path = args.export_dir.join(ACCESS_ORG_EXPORT_FILENAME);
    let metadata_path = args.export_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
    assert_not_overwrite(&payload_path, args.dry_run, args.overwrite)?;
    assert_not_overwrite(&metadata_path, args.dry_run, args.overwrite)?;
    let mut records = list_organizations_with_request(&mut request_json)?
        .into_iter()
        .map(|item| normalize_org_row(&item))
        .collect::<Vec<Map<String, Value>>>();
    records.retain(|row| {
        if let Some(org_id) = args.org_id {
            if scalar_text(row.get("id")) != org_id.to_string() {
                return false;
            }
        }
        if let Some(name) = &args.name {
            return string_field(row, "name", "") == *name;
        }
        true
    });
    if args.with_users {
        for row in records.iter_mut() {
            let org_id = scalar_text(row.get("id"));
            let users = list_org_users_with_request(&mut request_json, &org_id)?
                .into_iter()
                .map(|user| normalize_org_user_row(&user))
                .map(Value::Object)
                .collect::<Vec<Value>>();
            row.insert(
                "userCount".to_string(),
                Value::String(users.len().to_string()),
            );
            row.insert("users".to_string(), Value::Array(users));
        }
    }
    if !args.dry_run {
        write_json_file(
            &payload_path,
            &Value::Object(Map::from_iter(vec![
                (
                    "kind".to_string(),
                    Value::String(ACCESS_EXPORT_KIND_ORGS.to_string()),
                ),
                (
                    "version".to_string(),
                    Value::Number(ACCESS_EXPORT_VERSION.into()),
                ),
                (
                    "records".to_string(),
                    Value::Array(records.iter().cloned().map(Value::Object).collect()),
                ),
            ])),
            args.overwrite,
        )?;
        write_json_file(
            &metadata_path,
            &Value::Object(build_org_export_metadata(
                &args.common.url,
                &args.export_dir,
                records.len(),
            )),
            args.overwrite,
        )?;
    }
    let action = if args.dry_run { "Would export" } else { "Exported" };
    println!(
        "{action} {} org(s) from {} -> {} and {}",
        records.len(),
        args.common.url,
        payload_path.display(),
        metadata_path.display()
    );
    Ok(records.len())
}

pub(crate) fn import_orgs_with_request<F>(mut request_json: F, args: &OrgImportArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_basic_auth_only(&args.common)?;
    let records = load_org_import_records(&args.import_dir)?
        .into_iter()
        .map(|record| normalize_org_row(&record))
        .collect::<Vec<Map<String, Value>>>();
    let mut processed = 0usize;
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut live_orgs = list_organizations_with_request(&mut request_json)?;

    for record in records {
        processed += 1;
        let desired_name = string_field(&record, "name", "");
        if desired_name.is_empty() {
            return Err(message("Organization import record is missing name."));
        }
        let exported_id = scalar_text(record.get("id"));
        let existing = live_orgs.iter().find(|org| {
            string_field(org, "name", "") == desired_name
                || (!exported_id.is_empty() && scalar_text(org.get("id")) == exported_id)
        }).cloned();
        let existing_found = existing.is_some();

        let org_id = if let Some(existing) = existing.as_ref() {
            let existing_id = scalar_text(existing.get("id"));
            if !args.replace_existing {
                skipped += 1;
                println!("Skipped existing org {}", desired_name);
                continue;
            }
            if !exported_id.is_empty()
                && exported_id == existing_id
                && string_field(&existing, "name", "") != desired_name
            {
                if args.dry_run {
                    println!(
                        "Would rename org {} -> {}",
                        string_field(&existing, "name", ""),
                        desired_name
                    );
                } else {
                    let payload = Value::Object(Map::from_iter(vec![(
                        "name".to_string(),
                        Value::String(desired_name.clone()),
                    )]));
                    let _ = update_organization_with_request(&mut request_json, &existing_id, &payload)?;
                }
            }
            existing_id
        } else {
            if !args.replace_existing {
                skipped += 1;
                println!(
                    "Skipped org {}: missing and --replace-existing was not set.",
                    desired_name
                );
                continue;
            }
            if args.dry_run {
                created += 1;
                println!("Would create org {}", desired_name);
                exported_id.clone()
            } else {
                let payload = Value::Object(Map::from_iter(vec![(
                    "name".to_string(),
                    Value::String(desired_name.clone()),
                )]));
                let created_payload = create_organization_with_request(&mut request_json, &payload)?;
                created += 1;
                let created_id = scalar_text(created_payload.get("orgId"));
                live_orgs.push(Map::from_iter(vec![
                    ("id".to_string(), Value::String(created_id.clone())),
                    ("name".to_string(), Value::String(desired_name.clone())),
                ]));
                created_id
            }
        };

        if !org_id.is_empty() {
            let live_users = if args.dry_run {
                Vec::new()
            } else {
                list_org_users_with_request(&mut request_json, &org_id)?
            };
            let desired_users = match record.get("users") {
                Some(Value::Array(values)) => values
                    .iter()
                    .filter_map(|item| value_as_object(item, "Unexpected org user record.").ok())
                    .map(|item| normalize_org_user_row(item))
                    .collect::<Vec<Map<String, Value>>>(),
                _ => Vec::new(),
            };
            for desired_user in desired_users {
                let login = string_field(&desired_user, "login", "");
                let email = string_field(&desired_user, "email", "");
                let identity = if !login.is_empty() { login } else { email };
                if identity.is_empty() {
                    continue;
                }
                let desired_role = {
                    let role = string_field(&desired_user, "orgRole", "");
                    if role.is_empty() {
                        "Viewer".to_string()
                    } else {
                        role
                    }
                };
                let existing_user = live_users.iter().find(|user| {
                    string_field(user, "login", "") == identity
                        || string_field(user, "email", "") == identity
                });
                match existing_user {
                    Some(user) => {
                        let current_role = string_field(user, "role", "");
                        if current_role != desired_role {
                            if args.dry_run {
                                println!(
                                    "Would update org user role {} -> {} in org {}",
                                    identity, desired_role, desired_name
                                );
                            } else {
                                let user_id = {
                                    let user_id = scalar_text(user.get("userId"));
                                    if user_id.is_empty() {
                                        scalar_text(user.get("id"))
                                    } else {
                                        user_id
                                    }
                                };
                                let _ = update_org_user_role_with_request(
                                    &mut request_json,
                                    &org_id,
                                    &user_id,
                                    &desired_role,
                                )?;
                            }
                        }
                    }
                    None => {
                        if args.dry_run {
                            println!("Would add org user {} -> {} in org {}", identity, desired_role, desired_name);
                        } else {
                            let _ = add_user_to_org_with_request(
                                &mut request_json,
                                &org_id,
                                &identity,
                                &desired_role,
                            )?;
                        }
                    }
                }
            }
        }
        if existing_found {
            updated += 1;
            println!("Updated org {}", desired_name);
        }
    }

    println!(
        "Import summary: processed={} created={} updated={} skipped={} source={}",
        processed,
        created,
        updated,
        skipped,
        args.import_dir.display()
    );
    Ok(0)
}
