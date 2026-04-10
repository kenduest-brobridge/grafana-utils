//! Organization import/export/diff workflow helpers.
//!
//! Maintainer notes:
//! - Org workflows require Basic auth because they touch Grafana admin APIs and
//!   can create, rename, delete, or enumerate orgs outside the current context.
//! - Org import reconciles org existence and listed user roles, but it does not
//!   remove extra live org users that are absent from the export bundle.

use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, value_as_object, write_json_file, Result};

use super::super::pending_delete::{
    format_prompt_row, print_delete_confirmation_summary, prompt_confirm_delete,
    prompt_select_indexes, validate_delete_prompt,
};
use super::super::render::{
    access_diff_summary_line, access_import_summary_line, format_table, render_csv,
    render_objects_json, render_yaml, scalar_text,
};
use super::super::{
    OrgAddArgs, OrgDeleteArgs, OrgDiffArgs, OrgExportArgs, OrgImportArgs, OrgListArgs,
    OrgModifyArgs, ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_METADATA_FILENAME, ACCESS_EXPORT_VERSION,
    ACCESS_ORG_EXPORT_FILENAME,
};
use super::org_import_export_diff::{
    assert_not_overwrite, build_org_diff_map, build_org_export_metadata,
    build_org_live_records_for_diff, build_record_diff_fields, load_org_import_records,
};
use super::{
    add_user_to_org_with_request, create_organization_with_request,
    delete_organization_with_request, list_org_users_with_request, list_organizations_with_request,
    lookup_org_by_identity, normalize_org_row, normalize_org_user_row, org_matches,
    org_summary_line, org_table_rows, update_org_user_role_with_request,
    update_organization_with_request, validate_basic_auth_only,
};

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
    } else if args.yaml {
        println!("{}", render_yaml(&rows)?);
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

pub(crate) fn list_orgs_from_input_dir(args: &OrgListArgs) -> Result<usize> {
    let input_dir = args
        .input_dir
        .as_ref()
        .ok_or_else(|| message("Organization list local mode requires --input-dir."))?;
    let mut rows = load_org_import_records(input_dir)?
        .into_iter()
        .map(|item| normalize_org_row(&item))
        .collect::<Vec<Map<String, Value>>>();
    rows.retain(|row| org_matches(row, args));
    if !args.with_users {
        for row in &mut rows {
            row.insert("users".to_string(), Value::Array(Vec::new()));
        }
    }
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.yaml {
        println!("{}", render_yaml(&rows)?);
    } else if args.table {
        for line in format_table(&["ID", "NAME", "USER_COUNT"], &org_table_rows(&rows)) {
            println!("{line}");
        }
        println!();
        println!(
            "Listed {} org(s) from local bundle at {}",
            rows.len(),
            input_dir.display()
        );
    } else if args.csv {
        for line in render_csv(&["id", "name", "userCount"], &org_table_rows(&rows)) {
            println!("{line}");
        }
    } else {
        for row in &rows {
            println!("{}", org_summary_line(row));
        }
        println!();
        println!(
            "Listed {} org(s) from local bundle at {}",
            rows.len(),
            input_dir.display()
        );
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
    let org = lookup_org_by_identity(&mut request_json, args.org_id, args.name.as_deref())?;
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
    validate_delete_prompt(args.prompt, args.json, "Org")?;
    if !args.prompt && !args.yes {
        return Err(message("Org delete requires --yes."));
    }
    let orgs = if args.prompt && args.org_id.is_none() && args.name.is_none() {
        let orgs = list_organizations_with_request(&mut request_json)?
            .into_iter()
            .map(|org| normalize_org_row(&org))
            .collect::<Vec<_>>();
        if orgs.is_empty() {
            return Err(message(
                "Org delete --prompt did not find any matching organizations.",
            ));
        }
        let labels = orgs.iter().map(org_delete_prompt_label).collect::<Vec<_>>();
        let Some(indexes) = prompt_select_indexes("Organizations To Delete", &labels)? else {
            println!("Cancelled org delete.");
            return Ok(0);
        };
        indexes
            .into_iter()
            .filter_map(|index| orgs.get(index).cloned())
            .collect::<Vec<Map<String, Value>>>()
    } else {
        vec![lookup_org_by_identity(
            &mut request_json,
            args.org_id,
            args.name.as_deref(),
        )?]
    };
    if args.prompt {
        let labels = orgs.iter().map(org_delete_prompt_label).collect::<Vec<_>>();
        print_delete_confirmation_summary("The following organizations will be deleted:", &labels);
    }
    if args.prompt && !prompt_confirm_delete(&format!("Delete {} organization(s)?", orgs.len()))? {
        println!("Cancelled org delete.");
        return Ok(0);
    }
    let mut rows = Vec::new();
    for org in &orgs {
        let org_id = scalar_text(org.get("id"));
        let delete_payload = delete_organization_with_request(&mut request_json, &org_id)?;
        rows.push(Map::from_iter(vec![
            ("id".to_string(), Value::String(org_id.clone())),
            (
                "name".to_string(),
                Value::String(string_field(org, "name", "")),
            ),
            (
                "userCount".to_string(),
                Value::String(string_field(org, "userCount", "")),
            ),
            (
                "message".to_string(),
                Value::String(string_field(&delete_payload, "message", "")),
            ),
        ]));
    }
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else {
        for row in &rows {
            let mut parts = vec![
                format!("Deleted org {}", string_field(row, "name", "")),
                format!("id={}", scalar_text(row.get("id"))),
            ];
            let user_count = string_field(row, "userCount", "");
            if !user_count.is_empty() {
                parts.push(format!("userCount={user_count}"));
            }
            let message = string_field(row, "message", "");
            if !message.is_empty() {
                parts.push(format!("message={message}"));
            }
            println!("{}", parts.join(" "));
        }
        if rows.len() > 1 {
            println!("Deleted {} organization(s).", rows.len());
        }
    }
    Ok(rows.len())
}

fn org_delete_prompt_label(org: &Map<String, Value>) -> String {
    let name = string_field(org, "name", "-");
    let id = scalar_text(org.get("id"));
    let user_count = string_field(org, "userCount", "");
    let trailer = if user_count.is_empty() {
        format!("id={id}")
    } else {
        format!("id={id} users={user_count}")
    };
    format_prompt_row(&[(&name, 32)], &trailer)
}

#[cfg(test)]
mod org_delete_prompt_tests {
    use super::*;

    #[test]
    fn org_delete_prompt_label_includes_user_count_when_present() {
        let org = Map::from_iter(vec![
            ("id".to_string(), Value::String("4".to_string())),
            ("name".to_string(), Value::String("Main Org".to_string())),
            ("userCount".to_string(), Value::String("12".to_string())),
        ]);

        let label = org_delete_prompt_label(&org);

        assert!(label.contains("Main Org"));
        assert!(label.contains("id=4 users=12"));
    }
}

pub(crate) fn export_orgs_with_request<F>(
    mut request_json: F,
    args: &OrgExportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_basic_auth_only(&args.common)?;
    let payload_path = args.output_dir.join(ACCESS_ORG_EXPORT_FILENAME);
    let metadata_path = args.output_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
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
                args.common.profile.as_deref(),
                &args.output_dir,
                records.len(),
                args.with_users,
                args.org_id,
                args.name.as_deref(),
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
        "{action} {} org(s) from {} -> {} and {}",
        records.len(),
        args.common.url,
        payload_path.display(),
        metadata_path.display()
    );
    Ok(records.len())
}

pub(crate) fn import_orgs_with_request<F>(
    mut request_json: F,
    args: &OrgImportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_basic_auth_only(&args.common)?;
    let records = load_org_import_records(&args.input_dir)?
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
        let existing = live_orgs
            .iter()
            .find(|org| {
                // Match by stable exported id when present, but still allow
                // name-based reconciliation for bundles that only carry names.
                string_field(org, "name", "") == desired_name
                    || (!exported_id.is_empty() && scalar_text(org.get("id")) == exported_id)
            })
            .cloned();
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
                && string_field(existing, "name", "") != desired_name
            {
                // Same exported org id but different current name means rename,
                // not create. This preserves downstream user-role sync in-place.
                if args.dry_run {
                    println!(
                        "Would rename org {} -> {}",
                        string_field(existing, "name", ""),
                        desired_name
                    );
                } else {
                    let payload = Value::Object(Map::from_iter(vec![(
                        "name".to_string(),
                        Value::String(desired_name.clone()),
                    )]));
                    let _ = update_organization_with_request(
                        &mut request_json,
                        &existing_id,
                        &payload,
                    )?;
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
                let created_payload =
                    create_organization_with_request(&mut request_json, &payload)?;
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
                    .map(normalize_org_user_row)
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
                        // Import adds missing listed users, but intentionally does
                        // not remove extra live users absent from the bundle.
                        if args.dry_run {
                            println!(
                                "Would add org user {} -> {} in org {}",
                                identity, desired_role, desired_name
                            );
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
        "{}",
        access_import_summary_line(
            "org",
            processed,
            created,
            updated,
            skipped,
            &args.input_dir.to_string_lossy(),
        )
    );
    Ok(0)
}

pub(crate) fn diff_orgs_with_request<F>(mut request_json: F, args: &OrgDiffArgs) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_basic_auth_only(&args.common)?;
    let local_records = load_org_import_records(&args.diff_dir)?;
    let include_users = local_records
        .iter()
        .any(|record| record.contains_key("users"));
    let local_map = build_org_diff_map(
        &local_records,
        &args.diff_dir.to_string_lossy(),
        include_users,
    )?;
    let live_records = build_org_live_records_for_diff(&mut request_json, include_users)?;
    let live_map = build_org_diff_map(&live_records, "Grafana live orgs", include_users)?;

    let mut differences = 0usize;
    let mut checked = 0usize;
    for key in local_map.keys() {
        checked += 1;
        let (local_identity, local_payload) = &local_map[key];
        match live_map.get(key) {
            None => {
                println!("Diff missing-live org {}", local_identity);
                differences += 1;
            }
            Some((_live_identity, live_payload)) => {
                let changed = build_record_diff_fields(local_payload, live_payload);
                if changed.is_empty() {
                    println!("Diff same org {}", local_identity);
                } else {
                    differences += 1;
                    println!(
                        "Diff different org {} fields={}",
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
        checked += 1;
        differences += 1;
        let (identity, _) = &live_map[key];
        println!("Diff extra-live org {}", identity);
    }
    println!(
        "{}",
        access_diff_summary_line(
            "org",
            checked,
            differences,
            &args.diff_dir.to_string_lossy(),
            "Grafana live orgs",
        )
    );
    Ok(differences)
}
