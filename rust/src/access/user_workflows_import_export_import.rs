//! User import workflow helpers.
//!
//! Maintainer notes:
//! - Global import may create missing users, but org-scoped import cannot invent
//!   new org users by login/email; it only reconciles users Grafana can already
//!   resolve in that org.
//! - Team membership removals are destructive and stay behind `--yes`, even when
//!   other profile or role updates in the same record are non-destructive.

use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::access::render::{
    bool_label, format_table, map_get_text, normalize_org_role, scalar_text, value_bool,
};
use crate::common::{
    load_json_object_file, message, render_json_value, string_field, value_as_object, Result,
};

use super::super::{
    build_access_import_dry_run_row, build_access_import_dry_run_rows, build_auth_context,
    build_user_import_dry_run_document, create_user_with_request, list_user_teams_with_request,
    lookup_global_user_by_identity, lookup_org_user_by_identity, normalize_access_identity,
    parse_access_identity_list, request_object, request_object_list_field,
    update_user_org_role_with_request, update_user_permissions_with_request,
    update_user_with_request, user_id_json_value, validate_user_import_dry_run_output, Scope,
    UserImportArgs, ACCESS_EXPORT_KIND_USERS, ACCESS_EXPORT_METADATA_FILENAME,
    ACCESS_EXPORT_VERSION, ACCESS_USER_EXPORT_FILENAME, DEFAULT_PAGE_SIZE,
};

pub(crate) fn load_access_import_records(
    input_dir: &Path,
    expected_kind: &str,
) -> Result<Vec<Map<String, Value>>> {
    let path = input_dir.join(ACCESS_USER_EXPORT_FILENAME);
    if !path.is_file() {
        return Err(message(format!(
            "Access import file not found: {}",
            path.display()
        )));
    }

    // Accept the legacy bare-array export shape, but prefer the bundled object
    // so kind/version checks can stop incompatible imports early.
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

    let metadata_path = input_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
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
    let teams = request_object_list_field(
        &mut request_json,
        Method::GET,
        "/api/teams/search",
        &params,
        None,
        "teams",
        (
            "Unexpected team list response for user import.",
            "Unexpected team lookup response from Grafana.",
        ),
    )?;
    teams
        .iter()
        .find_map(|record| {
            let name = string_field(record, "name", "");
            if name == team_name {
                Some(record.clone())
            } else {
                None
            }
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
            user_id_json_value(user_id),
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

pub(crate) fn import_users_with_request<F>(
    mut request_json: F,
    args: &UserImportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let _auth_mode = build_auth_context(&args.common)?.auth_mode;
    validate_user_import_dry_run_output(args)?;
    let records = load_access_import_records(&args.input_dir, ACCESS_EXPORT_KIND_USERS)?;

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
                args.input_dir.display()
            )));
        }

        // Scope controls both lookup shape and what mutations are legal later.
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
                // Org import is a membership/profile reconciliation flow, not a
                // user-creation path. Creating the global account would require
                // different auth and side effects than this command owns.
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

        let mut current_members = std::collections::BTreeMap::<String, (String, String)>::new();
        let mut remove_keys: Vec<String> = Vec::new();
        if args.scope != Scope::Global && !target_teams.is_empty() {
            // Only org-scoped imports reconcile team memberships because global
            // user records do not have enough org context to remove safely.
            current_members = list_user_teams_with_request(&mut request_json, &user_id)?
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
                .map(|value| normalize_access_identity(value))
                .collect::<BTreeSet<_>>();
            remove_keys = current_members
                .keys()
                .filter(|entry| !desired_keys.contains(*entry))
                .cloned()
                .collect();
            if !remove_keys.is_empty() && !args.yes {
                return Err(message(format!(
                    "User import would remove team memberships for {}. Add --yes to confirm.",
                    identity
                )));
            }
        }

        let existing_login = string_field(&existing, "login", "");
        let existing_email = string_field(&existing, "email", "");
        let existing_name = string_field(&existing, "name", "");
        let desired_name = string_field(record, "name", "");
        let resolved_login = if login.is_empty() {
            existing_login.clone()
        } else {
            login.clone()
        };
        let resolved_email = if email.is_empty() {
            existing_email.clone()
        } else {
            email.clone()
        };
        let resolved_name = if desired_name.is_empty() {
            existing_name.clone()
        } else {
            desired_name
        };
        let profile_changed = resolved_login != existing_login
            || resolved_email != existing_email
            || resolved_name != existing_name;
        if profile_changed {
            // Apply profile changes before role/admin/team reconciliation so
            // later operator output refers to the same resolved identity.
            let profile_payload = Map::from_iter(vec![
                ("login".to_string(), Value::String(resolved_login)),
                ("email".to_string(), Value::String(resolved_email)),
                ("name".to_string(), Value::String(resolved_name)),
            ]);
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
            // Add missing memberships first, then remove stale ones once the
            // operator has explicitly acknowledged destructive sync with `--yes`.
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
                } else if !team_id.is_empty() {
                    remove_team_member_for_user_with_request(
                        &mut request_json,
                        &team_id,
                        &user_id,
                    )?;
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
            print!(
                "{}",
                render_json_value(&build_user_import_dry_run_document(
                    &dry_run_rows,
                    processed,
                    created,
                    updated,
                    skipped,
                    &args.input_dir,
                ))?
            );
            return Ok(0);
        }
    }

    println!(
        "Import summary: processed={} created={} updated={} skipped={} source={}",
        processed,
        created,
        updated,
        skipped,
        args.input_dir.display()
    );
    Ok(processed)
}
