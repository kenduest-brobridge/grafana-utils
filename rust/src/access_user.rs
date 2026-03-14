//! Access user command handlers.
//! Supports user listing/lookup and CRUD operations with org/user scope-aware rendering paths.
use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::common::{
    load_json_object_file, message, string_field, value_as_object, write_json_file, Result,
};

use super::access_render::{
    bool_label, format_table, map_get_text, normalize_org_role, normalize_user_row, paginate_rows,
    render_csv, render_objects_json, scalar_text, user_matches, user_scope_text, user_summary_line,
    user_table_rows, value_bool,
};
use super::{
    build_auth_context, request_array, request_object, Scope, UserAddArgs, UserDeleteArgs,
    UserDiffArgs, UserExportArgs, UserImportArgs, UserListArgs, UserModifyArgs,
    ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME, ACCESS_EXPORT_VERSION,
    ACCESS_USER_EXPORT_FILENAME, DEFAULT_PAGE_SIZE,
};

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

fn parse_access_identity_list(value: &Value) -> Vec<String> {
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

fn load_access_import_records(
    import_dir: &Path,
    expected_kind: &str,
) -> Result<Vec<Map<String, Value>>> {
    let path = import_dir.join(ACCESS_USER_EXPORT_FILENAME);
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
                if kind != expected_kind {
                    return Err(message(format!(
                        "Access import kind mismatch in {}: expected {}, got {}",
                        path.display(),
                        expected_kind,
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
                        "Access import bundle is missing records list: {}",
                        path.display()
                    ))
                })?
                .as_array()
                .ok_or_else(|| {
                    message(format!(
                        "Access import records must be a list in {}",
                        path.display()
                    ))
                })?
                .to_vec()
        }
        _ => {
            return Err(message(format!(
                "Unsupported access import payload in {}",
                path.display()
            )))
        }
    };

    let metadata_path = import_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
    if metadata_path.is_file() {
        let _metadata = load_json_object_file(&metadata_path, "Access import metadata")?;
    }

    let mut normalized = Vec::new();
    for value in records {
        normalized.push(
            value_as_object(
                &value,
                &format!("Access import entry in {}", path.display()),
            )?
            .clone(),
        );
    }
    Ok(normalized)
}

fn lookup_team_with_request<F>(mut request_json: F, team_name: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let params = vec![
        ("query".to_string(), team_name.to_string()),
        ("page".to_string(), "1".to_string()),
        ("perpage".to_string(), DEFAULT_PAGE_SIZE.to_string()),
    ];
    let object = request_object(
        &mut request_json,
        Method::GET,
        "/api/teams/search",
        &params,
        None,
        "Unexpected team list response for user import.",
    )?;
    let teams = object
        .get("teams")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Unexpected team lookup response from Grafana."))?;
    teams
        .iter()
        .find_map(|team| {
            team.as_object().and_then(|record| {
                let name = string_field(record, "name", "");
                if name == team_name {
                    Some(record.clone())
                } else {
                    None
                }
            })
        })
        .ok_or_else(|| message(format!("Team not found by name: {}", team_name)))
}

fn add_team_member_for_user_with_request<F>(
    mut request_json: F,
    team_id: &str,
    user_id: &str,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let _ = request_object(
        &mut request_json,
        Method::POST,
        &format!("/api/teams/{team_id}/members"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "userId".to_string(),
            Value::String(user_id.to_string()),
        )]))),
        &format!("Unexpected team-member add response for team {team_id} user {user_id}"),
    )?;
    Ok(())
}

fn remove_team_member_for_user_with_request<F>(
    mut request_json: F,
    team_id: &str,
    user_id: &str,
) -> Result<()>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let _ = request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/teams/{team_id}/members/{user_id}"),
        &[],
        None,
        &format!("Unexpected team-member remove response for team {team_id} user {user_id}"),
    )?;
    Ok(())
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

pub(crate) fn import_users_with_request<F>(
    mut request_json: F,
    args: &UserImportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let _auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_import_dry_run_output(args)?;
    let records = load_access_import_records(&args.import_dir, ACCESS_EXPORT_KIND_USERS)?;

    let mut created = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut processed = 0usize;
    let mut dry_run_rows: Vec<Map<String, Value>> = Vec::new();
    let is_dry_run_table_or_json = args.dry_run && (args.table || args.json);

    for (index, record) in records.iter().enumerate() {
        processed += 1;
        let login = string_field(record, "login", "");
        let email = string_field(record, "email", "");
        let identity = if !login.is_empty() {
            login.clone()
        } else {
            email.clone()
        };
        if identity.is_empty() {
            return Err(message(format!(
                "Access user import record {} in {} lacks login or email.",
                index + 1,
                args.import_dir.display()
            )));
        }

        let existing = match args.scope {
            Scope::Global => lookup_global_user_by_identity(
                &mut request_json,
                if !login.is_empty() {
                    Some(&login)
                } else {
                    None
                },
                if !email.is_empty() {
                    Some(&email)
                } else {
                    None
                },
            )
            .ok(),
            Scope::Org => lookup_org_user_by_identity(
                &mut request_json,
                if !login.is_empty() { &login } else { &email },
            )
            .ok(),
        };

        let target_teams = parse_access_identity_list(record.get("teams").unwrap_or(&Value::Null));

        if existing.is_none() {
            if !args.replace_existing {
                skipped += 1;
                if is_dry_run_table_or_json {
                    dry_run_rows.push(build_access_import_dry_run_row(
                        index + 1,
                        &identity,
                        "skip",
                        "missing and --replace-existing was not set.",
                    ));
                } else {
                    println!("Skipped user {} ({})", identity, index + 1);
                }
                continue;
            }
            if args.scope == Scope::Org {
                return Err(message(format!(
                    "User import cannot create missing org users by login/email: {}",
                    identity
                )));
            }
            let password = string_field(record, "password", "");
            if password.is_empty() {
                return Err(message(format!(
                    "Missing password for new user import entry {}: {}",
                    index + 1,
                    identity
                )));
            }
            if args.dry_run {
                if is_dry_run_table_or_json {
                    dry_run_rows.push(build_access_import_dry_run_row(
                        index + 1,
                        &identity,
                        "create",
                        "would create user",
                    ));
                } else {
                    println!("Would create user {}", identity);
                }
                created += 1;
            } else {
                let payload = Map::from_iter(vec![
                    ("login".to_string(), Value::String(login.clone())),
                    ("email".to_string(), Value::String(email.clone())),
                    (
                        "name".to_string(),
                        Value::String(string_field(record, "name", "")),
                    ),
                    ("password".to_string(), Value::String(password)),
                ]);
                let created_user =
                    create_user_with_request(&mut request_json, &Value::Object(payload))?;
                let user_id = scalar_text(created_user.get("id"));
                if !user_id.is_empty() {
                    if let Some(org_role) = record.get("orgRole") {
                        let normalized_role = normalize_org_role(Some(org_role));
                        if !normalized_role.is_empty() {
                            let _ = update_user_org_role_with_request(
                                &mut request_json,
                                &user_id,
                                &normalized_role,
                            )?;
                        }
                    }
                    if let Some(is_admin) = value_bool(record.get("grafanaAdmin")) {
                        let _ = update_user_permissions_with_request(
                            &mut request_json,
                            &user_id,
                            is_admin,
                        )?;
                    }
                }
                println!("Created user {}", identity);
                created += 1;
            }
            continue;
        }

        let existing = existing.unwrap();
        if !args.replace_existing {
            skipped += 1;
            if is_dry_run_table_or_json {
                dry_run_rows.push(build_access_import_dry_run_row(
                    index + 1,
                    &identity,
                    "skip",
                    "existing and --replace-existing was not set.",
                ));
            } else {
                println!("Skipped existing user {} ({})", identity, index + 1);
            }
            continue;
        }

        let user_id = {
            let user_id = scalar_text(existing.get("userId"));
            if user_id.is_empty() {
                scalar_text(existing.get("id"))
            } else {
                user_id
            }
        };
        if user_id.is_empty() {
            return Err(message(format!(
                "User import record {} resolved without id: {}",
                index + 1,
                identity
            )));
        }

        let mut profile_payload = Map::new();
        if !login.is_empty() && login != string_field(&existing, "login", "") {
            profile_payload.insert("login".to_string(), Value::String(login.clone()));
        }
        if !email.is_empty() && email != string_field(&existing, "email", "") {
            profile_payload.insert("email".to_string(), Value::String(email.clone()));
        }
        let desired_name = string_field(record, "name", "");
        if !desired_name.is_empty() && desired_name != string_field(&existing, "name", "") {
            profile_payload.insert("name".to_string(), Value::String(desired_name));
        }
        if !profile_payload.is_empty() {
            if args.dry_run {
                if is_dry_run_table_or_json {
                    dry_run_rows.push(build_access_import_dry_run_row(
                        index + 1,
                        &identity,
                        "update-profile",
                        "would update user profile",
                    ));
                } else {
                    println!("Would update user {} profile", identity);
                }
            } else {
                let _ = update_user_with_request(
                    &mut request_json,
                    &user_id,
                    &Value::Object(profile_payload),
                )?;
            }
        }

        let desired_org_role = normalize_org_role(record.get("orgRole"));
        let existing_org_role = match args.scope {
            Scope::Global => {
                let role = normalize_org_role(existing.get("orgRole"));
                if role.is_empty() {
                    normalize_org_role(existing.get("role"))
                } else {
                    role
                }
            }
            Scope::Org => normalize_org_role(existing.get("role")),
        };
        if !desired_org_role.is_empty() && desired_org_role != existing_org_role {
            if args.dry_run {
                if is_dry_run_table_or_json {
                    dry_run_rows.push(build_access_import_dry_run_row(
                        index + 1,
                        &identity,
                        "update-org-role",
                        &format!("would update orgRole -> {desired_org_role}"),
                    ));
                } else {
                    println!(
                        "Would update orgRole for user {} -> {}",
                        identity, desired_org_role
                    );
                }
            } else {
                let _ = update_user_org_role_with_request(
                    &mut request_json,
                    &user_id,
                    &desired_org_role,
                )?;
            }
        }

        let desired_admin = value_bool(record.get("grafanaAdmin"));
        let existing_admin = value_bool(existing.get("isGrafanaAdmin"))
            .or_else(|| value_bool(existing.get("isAdmin")));
        if desired_admin.is_some() && desired_admin != existing_admin {
            if args.dry_run {
                if is_dry_run_table_or_json {
                    dry_run_rows.push(build_access_import_dry_run_row(
                        index + 1,
                        &identity,
                        "update-admin",
                        &format!("would update grafanaAdmin -> {}", bool_label(desired_admin)),
                    ));
                } else {
                    println!(
                        "Would update grafanaAdmin for user {} -> {}",
                        identity,
                        bool_label(desired_admin)
                    );
                }
            } else {
                let _ = update_user_permissions_with_request(
                    &mut request_json,
                    &user_id,
                    desired_admin.unwrap_or(false),
                )?;
            }
        }

        if args.scope != Scope::Global && !target_teams.is_empty() {
            let current_members = list_user_teams_with_request(&mut request_json, &user_id)?
                .into_iter()
                .filter_map(|team| {
                    let name = string_field(&team, "name", "");
                    if name.is_empty() {
                        None
                    } else {
                        let id = scalar_text(team.get("id"));
                        Some((normalize_access_identity(&name), (name, id)))
                    }
                })
                .collect::<std::collections::BTreeMap<String, (String, String)>>();
            let desired_keys = target_teams
                .iter()
                .map(|identity| normalize_access_identity(identity))
                .collect::<BTreeSet<_>>();

            let remove_keys: Vec<String> = current_members
                .keys()
                .filter(|identity| !desired_keys.contains(*identity))
                .cloned()
                .collect();
            if !remove_keys.is_empty() && !args.yes {
                return Err(message(format!(
                    "User import would remove team memberships for {}. Add --yes to confirm.",
                    identity
                )));
            }

            for target in &target_teams {
                let key = normalize_access_identity(target);
                if current_members.contains_key(&key) {
                    continue;
                }
                let team = lookup_team_with_request(&mut request_json, target)?;
                let team_id = scalar_text(team.get("id"));
                if args.dry_run {
                    if is_dry_run_table_or_json {
                        dry_run_rows.push(build_access_import_dry_run_row(
                            index + 1,
                            &identity,
                            "add-team",
                            &format!("would add user to team {target}"),
                        ));
                    } else {
                        println!("Would add user {} to team {}", identity, target);
                    }
                } else {
                    add_team_member_for_user_with_request(&mut request_json, &team_id, &user_id)?;
                }
            }

            for key in remove_keys {
                let team_name = current_members
                    .get(&key)
                    .map(|value| value.0.clone())
                    .unwrap_or_default();
                let team_id = current_members
                    .get(&key)
                    .map(|item| item.1.clone())
                    .unwrap_or_default();
                if args.dry_run {
                    if is_dry_run_table_or_json {
                        dry_run_rows.push(build_access_import_dry_run_row(
                            index + 1,
                            &identity,
                            "remove-team",
                            &format!("would remove user from team {team_name}"),
                        ));
                    } else {
                        println!("Would remove user {} from team {}", identity, team_name);
                    }
                } else {
                    if !team_id.is_empty() {
                        remove_team_member_for_user_with_request(
                            &mut request_json,
                            &team_id,
                            &user_id,
                        )?;
                    }
                }
            }
        }

        updated += 1;
        if is_dry_run_table_or_json {
            dry_run_rows.push(build_access_import_dry_run_row(
                index + 1,
                &identity,
                "updated",
                "would update user",
            ));
        } else {
            println!("Updated user {}", identity);
        }
    }

    if args.dry_run && is_dry_run_table_or_json {
        if args.table {
            for line in format_table(
                &["INDEX", "IDENTITY", "ACTION", "DETAIL"],
                &build_access_import_dry_run_rows(&dry_run_rows),
            ) {
                println!("{line}");
            }
        } else if args.json {
            println!("{}", render_objects_json(&dry_run_rows)?);
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
    Ok(processed)
}

fn normalize_bool_for_diff(value: Option<&Value>) -> Value {
    match value_bool(value) {
        Some(value) => Value::Bool(value),
        None => Value::Null,
    }
}

fn normalize_user_for_diff(record: &Map<String, Value>, include_teams: bool) -> Map<String, Value> {
    let mut payload = Map::from_iter(vec![
        (
            "login".to_string(),
            Value::String(string_field(record, "login", "")),
        ),
        (
            "email".to_string(),
            Value::String(string_field(record, "email", "")),
        ),
        (
            "name".to_string(),
            Value::String(string_field(record, "name", "")),
        ),
        (
            "orgRole".to_string(),
            Value::String(normalize_org_role(
                record.get("orgRole").or_else(|| record.get("role")),
            )),
        ),
        (
            "grafanaAdmin".to_string(),
            normalize_bool_for_diff(
                record
                    .get("grafanaAdmin")
                    .or(record.get("isGrafanaAdmin"))
                    .or(record.get("isAdmin")),
            ),
        ),
    ]);
    if include_teams {
        let mut teams = parse_access_identity_list(record.get("teams").unwrap_or(&Value::Null));
        teams.sort();
        payload.insert(
            "teams".to_string(),
            Value::Array(teams.iter().cloned().map(Value::String).collect()),
        );
    } else {
        payload.insert("teams".to_string(), Value::Array(Vec::new()));
    }
    payload
}

fn build_user_diff_map(
    records: &[Map<String, Value>],
    source: &str,
    include_teams: bool,
) -> Result<BTreeMap<String, (String, Map<String, Value>)>> {
    let mut indexed = BTreeMap::new();
    for record in records {
        let login = string_field(record, "login", "");
        let email = string_field(record, "email", "");
        let identity = if login.is_empty() { email } else { login };
        if identity.trim().is_empty() {
            return Err(message(format!(
                "User diff record in {} does not include login or email.",
                source
            )));
        }
        let key = normalize_access_identity(&identity);
        if indexed.contains_key(&key) {
            return Err(message(format!(
                "Duplicate user identity in {}: {}",
                source, identity
            )));
        }
        let payload = normalize_user_for_diff(record, include_teams);
        indexed.insert(key, (identity, payload));
    }
    Ok(indexed)
}

fn build_user_export_records_for_diff<F>(
    mut request_json: F,
    scope: &Scope,
    include_teams: bool,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut rows = match scope {
        Scope::Org => list_org_users_with_request(&mut request_json)?
            .into_iter()
            .map(|item| normalize_user_row(&item, &Scope::Org))
            .collect::<Vec<Map<String, Value>>>(),
        Scope::Global => iter_global_users_with_request(&mut request_json, DEFAULT_PAGE_SIZE)?
            .into_iter()
            .map(|item| normalize_user_row(&item, &Scope::Global))
            .collect::<Vec<Map<String, Value>>>(),
    };

    if include_teams {
        for row in &mut rows {
            let user_id = map_get_text(row, "id");
            let mut team_names = Vec::new();
            if !user_id.is_empty() {
                team_names = list_user_teams_with_request(&mut request_json, &user_id)?
                    .into_iter()
                    .map(|team| string_field(&team, "name", ""))
                    .filter(|name| !name.is_empty())
                    .collect();
                team_names.sort();
                team_names.dedup();
            }
            row.insert(
                "teams".to_string(),
                Value::Array(team_names.into_iter().map(Value::String).collect()),
            );
        }
    }
    Ok(rows)
}

fn build_record_diff_fields(left: &Map<String, Value>, right: &Map<String, Value>) -> Vec<String> {
    let mut keys = BTreeSet::new();
    for key in left.keys().chain(right.keys()) {
        keys.insert(key.clone());
    }
    let mut changed = Vec::new();
    for key in keys {
        if left.get(&key) != right.get(&key) {
            changed.push(key);
        }
    }
    changed
}

pub(crate) fn diff_users_with_request<F>(mut request_json: F, args: &UserDiffArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let local_records = load_access_import_records(&args.diff_dir, ACCESS_EXPORT_KIND_USERS)?;
    let include_teams = local_records
        .iter()
        .any(|record| match record.get("teams") {
            Some(Value::Array(values)) => !values.is_empty(),
            Some(Value::String(text)) => !text.trim().is_empty(),
            _ => false,
        });
    let local_map = build_user_diff_map(
        &local_records,
        &args.diff_dir.to_string_lossy(),
        include_teams,
    )?;
    let live_records =
        build_user_export_records_for_diff(&mut request_json, &args.scope, include_teams)?;
    let live_map = build_user_diff_map(&live_records, "Grafana live users", include_teams)?;

    let mut differences = 0usize;
    let mut checked = 0usize;
    for key in local_map.keys() {
        checked += 1;
        let (local_identity, local_payload) = &local_map[key];
        match live_map.get(key) {
            None => {
                println!("Diff missing-live user {}", local_identity);
                differences += 1;
            }
            Some((_live_identity, live_payload)) => {
                let changed = build_record_diff_fields(local_payload, live_payload);
                if changed.is_empty() {
                    println!("Diff same user {}", local_identity);
                } else {
                    differences += 1;
                    println!(
                        "Diff different user {} fields={}",
                        local_identity,
                        changed.join(",")
                    );
                }
            }
        }
    }

    for key in live_map.keys() {
        if local_map.contains_key(key) {
            continue;
        }
        differences += 1;
        checked += 1;
        let (live_identity, _) = &live_map[key];
        println!("Diff extra-live user {}", live_identity);
    }

    if differences > 0 {
        println!(
            "Diff checked {} user(s); {} difference(s) found.",
            checked, differences
        );
    } else {
        println!("No user differences across {} user(s).", checked);
    }
    Ok(differences)
}

pub(crate) fn list_org_users_with_request<F>(mut request_json: F) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_array(
        &mut request_json,
        Method::GET,
        "/api/org/users",
        &[],
        None,
        "Unexpected org user list response from Grafana.",
    )
}

fn iter_global_users_with_request<F>(
    mut request_json: F,
    page_size: usize,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut users = Vec::new();
    let mut page = 1usize;
    loop {
        let params = vec![
            ("page".to_string(), page.to_string()),
            ("perpage".to_string(), page_size.to_string()),
        ];
        let batch = request_array(
            &mut request_json,
            Method::GET,
            "/api/users",
            &params,
            None,
            "Unexpected global user list response from Grafana.",
        )?;
        if batch.is_empty() {
            break;
        }
        let batch_len = batch.len();
        users.extend(batch);
        if batch_len < page_size {
            break;
        }
        page += 1;
    }
    Ok(users)
}

fn list_user_teams_with_request<F>(
    mut request_json: F,
    user_id: &str,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_array(
        &mut request_json,
        Method::GET,
        &format!("/api/users/{user_id}/teams"),
        &[],
        None,
        &format!("Unexpected team list response for Grafana user {user_id}."),
    )
}

fn get_user_with_request<F>(mut request_json: F, user_id: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::GET,
        &format!("/api/users/{user_id}"),
        &[],
        None,
        &format!("Unexpected user lookup response for Grafana user {user_id}."),
    )
}

fn create_user_with_request<F>(mut request_json: F, payload: &Value) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        "/api/admin/users",
        &[],
        Some(payload),
        "Unexpected user create response from Grafana.",
    )
}

fn update_user_with_request<F>(
    mut request_json: F,
    user_id: &str,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PUT,
        &format!("/api/users/{user_id}"),
        &[],
        Some(payload),
        &format!("Unexpected user update response for Grafana user {user_id}."),
    )
}

fn update_user_password_with_request<F>(
    mut request_json: F,
    user_id: &str,
    password: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PUT,
        &format!("/api/admin/users/{user_id}/password"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "password".to_string(),
            Value::String(password.to_string()),
        )]))),
        &format!("Unexpected password update response for Grafana user {user_id}."),
    )
}

fn update_user_org_role_with_request<F>(
    mut request_json: F,
    user_id: &str,
    role: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PATCH,
        &format!("/api/org/users/{user_id}"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "role".to_string(),
            Value::String(role.to_string()),
        )]))),
        &format!("Unexpected org-role update response for Grafana user {user_id}."),
    )
}

fn update_user_permissions_with_request<F>(
    mut request_json: F,
    user_id: &str,
    is_admin: bool,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PUT,
        &format!("/api/admin/users/{user_id}/permissions"),
        &[],
        Some(&Value::Object(Map::from_iter(vec![(
            "isGrafanaAdmin".to_string(),
            Value::Bool(is_admin),
        )]))),
        &format!("Unexpected permission update response for Grafana user {user_id}."),
    )
}

fn delete_global_user_with_request<F>(
    mut request_json: F,
    user_id: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/admin/users/{user_id}"),
        &[],
        None,
        &format!("Unexpected global delete response for Grafana user {user_id}."),
    )
}

fn delete_org_user_with_request<F>(mut request_json: F, user_id: &str) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::DELETE,
        &format!("/api/org/users/{user_id}"),
        &[],
        None,
        &format!("Unexpected org delete response for Grafana user {user_id}."),
    )
}

fn lookup_global_user_by_identity<F>(
    mut request_json: F,
    login: Option<&str>,
    email: Option<&str>,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let users = iter_global_users_with_request(&mut request_json, DEFAULT_PAGE_SIZE)?;
    users
        .into_iter()
        .find(|user| {
            login.is_some_and(|value| string_field(user, "login", "") == value)
                || email.is_some_and(|value| string_field(user, "email", "") == value)
        })
        .ok_or_else(|| message("Grafana user lookup did not find a matching global user."))
}

pub(crate) fn lookup_org_user_by_identity<F>(
    mut request_json: F,
    identity: &str,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let users = list_org_users_with_request(&mut request_json)?;
    users
        .into_iter()
        .find(|user| {
            string_field(user, "login", "") == identity
                || string_field(user, "email", "") == identity
                || scalar_text(user.get("userId")) == identity
                || scalar_text(user.get("id")) == identity
        })
        .ok_or_else(|| message(format!("Grafana org user lookup did not find {identity}.")))
}

fn validate_basic_auth_only(auth_mode: &str, operation: &str) -> Result<()> {
    if auth_mode != "basic" {
        Err(message(format!(
            "{operation} requires Basic auth (--basic-user / --basic-password)."
        )))
    } else {
        Ok(())
    }
}

fn validate_user_list_auth(args: &UserListArgs, auth_mode: &str) -> Result<()> {
    validate_user_scope_auth(&args.scope, args.with_teams, auth_mode)
}

fn validate_user_scope_auth(scope: &Scope, with_teams: bool, auth_mode: &str) -> Result<()> {
    if *scope == Scope::Global && auth_mode != "basic" {
        return Err(message(
            "User list with --scope global requires Basic auth (--basic-user / --basic-password).",
        ));
    }
    if with_teams && auth_mode != "basic" {
        return Err(message("--with-teams requires Basic auth."));
    }
    Ok(())
}

fn validate_user_modify_args(args: &UserModifyArgs) -> Result<()> {
    let has_identity = args.user_id.is_some() || args.login.is_some() || args.email.is_some();
    if !has_identity {
        return Err(message(
            "User modify requires one of --user-id, --login, or --email.",
        ));
    }
    if args.set_login.is_none()
        && args.set_email.is_none()
        && args.set_name.is_none()
        && args.set_password.is_none()
        && args.set_org_role.is_none()
        && args.set_grafana_admin.is_none()
    {
        return Err(message(
            "User modify requires at least one of --set-login, --set-email, --set-name, --set-password, --set-org-role, or --set-grafana-admin.",
        ));
    }
    Ok(())
}

fn validate_user_delete_args(args: &UserDeleteArgs) -> Result<()> {
    if !args.yes {
        return Err(message("User delete requires --yes."));
    }
    if args.user_id.is_none() && args.login.is_none() && args.email.is_none() {
        return Err(message(
            "User delete requires one of --user-id, --login, or --email.",
        ));
    }
    Ok(())
}

pub(crate) fn list_users_with_request<F>(mut request_json: F, args: &UserListArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_scope_auth(&args.scope, args.with_teams, &auth_mode)?;
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
            let teams = list_user_teams_with_request(&mut request_json, &user_id)?
                .into_iter()
                .map(|team| string_field(&team, "name", ""))
                .filter(|name| !name.is_empty())
                .map(Value::String)
                .collect::<Vec<Value>>();
            row.insert("teams".to_string(), Value::Array(teams));
        }
    }
    rows.retain(|row| user_matches(row, args));
    let rows = paginate_rows(&rows, args.page, args.per_page);
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.csv {
        for line in render_csv(
            &[
                "id",
                "login",
                "email",
                "name",
                "orgRole",
                "grafanaAdmin",
                "scope",
                "teams",
            ],
            &user_table_rows(&rows),
        ) {
            println!("{line}");
        }
    } else if args.table {
        for line in format_table(
            &[
                "ID",
                "LOGIN",
                "EMAIL",
                "NAME",
                "ORG_ROLE",
                "GRAFANA_ADMIN",
                "SCOPE",
                "TEAMS",
            ],
            &user_table_rows(&rows),
        ) {
            println!("{line}");
        }
        println!();
        println!(
            "Listed {} user(s) from {} scope at {}",
            rows.len(),
            user_scope_text(&args.scope),
            args.common.url
        );
    } else {
        for row in &rows {
            println!("{}", user_summary_line(row));
        }
        println!();
        println!(
            "Listed {} user(s) from {} scope at {}",
            rows.len(),
            user_scope_text(&args.scope),
            args.common.url
        );
    }
    Ok(rows.len())
}

pub(crate) fn add_user_with_request<F>(mut request_json: F, args: &UserAddArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_basic_auth_only(&auth_mode, "User add")?;
    let mut payload = Map::from_iter(vec![
        ("login".to_string(), Value::String(args.login.clone())),
        ("email".to_string(), Value::String(args.email.clone())),
        ("name".to_string(), Value::String(args.name.clone())),
        (
            "password".to_string(),
            Value::String(args.new_user_password.clone()),
        ),
    ]);
    if let Some(org_id) = args.common.org_id {
        payload.insert("OrgId".to_string(), Value::Number(org_id.into()));
    }
    let created = create_user_with_request(&mut request_json, &Value::Object(payload))?;
    let user_id = scalar_text(created.get("id"));
    if user_id.is_empty() {
        return Err(message(
            "Grafana user create response did not include an id.",
        ));
    }
    if let Some(role) = &args.org_role {
        let _ = update_user_org_role_with_request(&mut request_json, &user_id, role)?;
    }
    if let Some(is_admin) = args.grafana_admin {
        let _ = update_user_permissions_with_request(&mut request_json, &user_id, is_admin)?;
    }
    let row = Map::from_iter(vec![
        ("id".to_string(), Value::String(user_id.clone())),
        ("login".to_string(), Value::String(args.login.clone())),
        ("email".to_string(), Value::String(args.email.clone())),
        ("name".to_string(), Value::String(args.name.clone())),
        (
            "orgRole".to_string(),
            Value::String(args.org_role.clone().unwrap_or_default()),
        ),
        (
            "grafanaAdmin".to_string(),
            Value::String(bool_label(args.grafana_admin)),
        ),
        ("scope".to_string(), Value::String("global".to_string())),
        ("teams".to_string(), Value::Array(Vec::new())),
    ]);
    if args.json {
        println!("{}", render_objects_json(&[row])?);
    } else {
        println!(
            "Created user {} -> id={} orgRole={} grafanaAdmin={}",
            args.login,
            user_id,
            args.org_role.clone().unwrap_or_default(),
            bool_label(args.grafana_admin)
        );
    }
    Ok(0)
}

pub(crate) fn modify_user_with_request<F>(
    mut request_json: F,
    args: &UserModifyArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_basic_auth_only(&auth_mode, "User modify")?;
    validate_user_modify_args(args)?;
    let base_user = if let Some(user_id) = &args.user_id {
        get_user_with_request(&mut request_json, user_id)?
    } else {
        lookup_global_user_by_identity(
            &mut request_json,
            args.login.as_deref(),
            args.email.as_deref(),
        )?
    };
    let user_id = string_field(&base_user, "id", "");
    let user_id = if user_id.is_empty() {
        scalar_text(base_user.get("id"))
    } else {
        user_id
    };
    let mut payload = Map::new();
    if let Some(value) = &args.set_login {
        payload.insert("login".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &args.set_email {
        payload.insert("email".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &args.set_name {
        payload.insert("name".to_string(), Value::String(value.clone()));
    }
    if !payload.is_empty() {
        let _ = update_user_with_request(&mut request_json, &user_id, &Value::Object(payload))?;
    }
    if let Some(password) = &args.set_password {
        let _ = update_user_password_with_request(&mut request_json, &user_id, password)?;
    }
    if let Some(role) = &args.set_org_role {
        let _ = update_user_org_role_with_request(&mut request_json, &user_id, role)?;
    }
    if let Some(is_admin) = args.set_grafana_admin {
        let _ = update_user_permissions_with_request(&mut request_json, &user_id, is_admin)?;
    }
    let login = args
        .set_login
        .clone()
        .unwrap_or_else(|| string_field(&base_user, "login", ""));
    let row = Map::from_iter(vec![
        ("id".to_string(), Value::String(user_id.clone())),
        ("login".to_string(), Value::String(login.clone())),
        (
            "email".to_string(),
            Value::String(
                args.set_email
                    .clone()
                    .unwrap_or_else(|| string_field(&base_user, "email", "")),
            ),
        ),
        (
            "name".to_string(),
            Value::String(
                args.set_name
                    .clone()
                    .unwrap_or_else(|| string_field(&base_user, "name", "")),
            ),
        ),
        (
            "orgRole".to_string(),
            Value::String(
                args.set_org_role
                    .clone()
                    .unwrap_or_else(|| normalize_org_role(base_user.get("role"))),
            ),
        ),
        (
            "grafanaAdmin".to_string(),
            Value::String(bool_label(
                args.set_grafana_admin
                    .or_else(|| value_bool(base_user.get("isGrafanaAdmin"))),
            )),
        ),
        ("scope".to_string(), Value::String("global".to_string())),
        ("teams".to_string(), Value::Array(Vec::new())),
    ]);
    if args.json {
        println!("{}", render_objects_json(&[row])?);
    } else {
        println!("Modified user {} -> id={}", login, user_id);
    }
    Ok(0)
}

pub(crate) fn delete_user_with_request<F>(
    mut request_json: F,
    args: &UserDeleteArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_delete_args(args)?;
    if args.scope == Scope::Global {
        validate_basic_auth_only(&auth_mode, "User delete with --scope global")?;
    }
    let base_user = match args.scope {
        Scope::Org => {
            if let Some(user_id) = &args.user_id {
                lookup_org_user_by_identity(&mut request_json, user_id)?
            } else {
                lookup_org_user_by_identity(
                    &mut request_json,
                    args.login
                        .as_deref()
                        .or(args.email.as_deref())
                        .unwrap_or(""),
                )?
            }
        }
        Scope::Global => {
            if let Some(user_id) = &args.user_id {
                get_user_with_request(&mut request_json, user_id)?
            } else {
                lookup_global_user_by_identity(
                    &mut request_json,
                    args.login.as_deref(),
                    args.email.as_deref(),
                )?
            }
        }
    };
    let user_id = {
        let user_id = scalar_text(base_user.get("userId"));
        if user_id.is_empty() {
            scalar_text(base_user.get("id"))
        } else {
            user_id
        }
    };
    match args.scope {
        Scope::Org => {
            let _ = delete_org_user_with_request(&mut request_json, &user_id)?;
        }
        Scope::Global => {
            let _ = delete_global_user_with_request(&mut request_json, &user_id)?;
        }
    }
    let row = Map::from_iter(vec![
        ("id".to_string(), Value::String(user_id.clone())),
        (
            "login".to_string(),
            Value::String(string_field(&base_user, "login", "")),
        ),
        (
            "scope".to_string(),
            Value::String(user_scope_text(&args.scope).to_string()),
        ),
    ]);
    if args.json {
        println!("{}", render_objects_json(&[row])?);
    } else {
        println!(
            "Deleted user {} -> id={} scope={}",
            map_get_text(&row, "login"),
            user_id,
            user_scope_text(&args.scope)
        );
    }
    Ok(0)
}
