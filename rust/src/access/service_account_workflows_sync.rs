use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, write_json_file, Result};

use super::super::super::render::{
    format_table, normalize_service_account_row, scalar_text, service_account_role_to_api,
    value_bool,
};
use super::super::super::{
    ServiceAccountDiffArgs, ServiceAccountExportArgs, ServiceAccountImportArgs,
    ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS, ACCESS_EXPORT_METADATA_FILENAME, ACCESS_EXPORT_VERSION,
    ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME, DEFAULT_PAGE_SIZE,
};
use super::super::{
    create_service_account_with_request, list_service_accounts_with_request,
    update_service_account_with_request,
};
use super::service_account_workflows_support::{
    assert_not_overwrite, build_record_diff_fields, build_service_account_diff_map,
    build_service_account_export_metadata, build_service_account_import_dry_run_row,
    build_service_account_import_dry_run_rows, list_all_service_accounts_with_request,
    load_service_account_import_records, validate_service_account_import_dry_run_output,
};

/// Purpose: implementation note.
pub(crate) fn export_service_accounts_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountExportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let records = list_all_service_accounts_with_request(&mut request_json)?
        .into_iter()
        .map(|item| normalize_service_account_row(&item))
        .collect::<Vec<Map<String, Value>>>();
    let bundle_path = args.export_dir.join(ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME);
    let metadata_path = args.export_dir.join(ACCESS_EXPORT_METADATA_FILENAME);
    assert_not_overwrite(&bundle_path, args.dry_run, args.overwrite)?;
    assert_not_overwrite(&metadata_path, args.dry_run, args.overwrite)?;
    if !args.dry_run {
        let payload = Value::Object(Map::from_iter(vec![
            (
                "kind".to_string(),
                Value::String(ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS.to_string()),
            ),
            (
                "version".to_string(),
                Value::Number(ACCESS_EXPORT_VERSION.into()),
            ),
            (
                "records".to_string(),
                Value::Array(records.iter().cloned().map(Value::Object).collect()),
            ),
        ]));
        write_json_file(&bundle_path, &payload, args.overwrite)?;
        write_json_file(
            &metadata_path,
            &Value::Object(build_service_account_export_metadata(
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
        "{} {} service-account(s) from {} -> {} and {}",
        action,
        records.len(),
        args.common.url,
        bundle_path.display(),
        metadata_path.display()
    );
    Ok(records.len())
}

/// Purpose: implementation note.
pub(crate) fn import_service_accounts_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountImportArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    validate_service_account_import_dry_run_output(args)?;
    let records =
        load_service_account_import_records(&args.import_dir, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS)?;
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut processed = 0usize;
    let mut dry_run_rows = Vec::new();
    let structured_output = args.dry_run && (args.table || args.json);

    for (index, record) in records.iter().enumerate() {
        processed += 1;
        let identity = string_field(record, "name", "");
        if identity.is_empty() {
            return Err(message(format!(
                "Access service-account import record {} in {} lacks name.",
                index + 1,
                args.import_dir.display()
            )));
        }
        let existing = list_service_accounts_with_request(
            &mut request_json,
            Some(&identity),
            1,
            DEFAULT_PAGE_SIZE,
        )?
        .into_iter()
        .find(|item| string_field(item, "name", "") == identity);

        if existing.is_none() {
            if !args.replace_existing {
                skipped += 1;
                let detail = "missing and --replace-existing was not set.";
                if structured_output {
                    dry_run_rows.push(build_service_account_import_dry_run_row(
                        index + 1,
                        &identity,
                        "skip",
                        detail,
                    ));
                } else {
                    println!(
                        "Skipped service-account {} ({}): {}",
                        identity,
                        index + 1,
                        detail
                    );
                }
                continue;
            }
            if args.dry_run {
                created += 1;
                if structured_output {
                    dry_run_rows.push(build_service_account_import_dry_run_row(
                        index + 1,
                        &identity,
                        "create",
                        "would create service account",
                    ));
                } else {
                    println!("Would create service-account {}", identity);
                }
                continue;
            }
            let payload = Value::Object(Map::from_iter(vec![
                ("name".to_string(), Value::String(identity.clone())),
                (
                    "role".to_string(),
                    Value::String(service_account_role_to_api(&{
                        let role = string_field(record, "role", "");
                        if role.is_empty() {
                            "Viewer".to_string()
                        } else {
                            role
                        }
                    })),
                ),
                (
                    "isDisabled".to_string(),
                    Value::Bool(
                        value_bool(record.get("disabled"))
                            .or_else(|| value_bool(record.get("isDisabled")))
                            .unwrap_or(false),
                    ),
                ),
            ]));
            let _ = create_service_account_with_request(&mut request_json, &payload)?;
            created += 1;
            println!("Created service-account {}", identity);
            continue;
        }

        let existing = existing.unwrap();
        if !args.replace_existing {
            skipped += 1;
            let detail = "existing and --replace-existing was not set.";
            if structured_output {
                dry_run_rows.push(build_service_account_import_dry_run_row(
                    index + 1,
                    &identity,
                    "skip",
                    detail,
                ));
            } else {
                println!(
                    "Skipped existing service-account {} ({})",
                    identity,
                    index + 1
                );
            }
            continue;
        }

        let desired_role = string_field(record, "role", "");
        let existing_role = string_field(&existing, "role", "");
        let desired_disabled =
            value_bool(record.get("disabled")).or_else(|| value_bool(record.get("isDisabled")));
        let existing_disabled =
            value_bool(existing.get("disabled")).or_else(|| value_bool(existing.get("isDisabled")));
        let mut update_payload =
            Map::from_iter(vec![("name".to_string(), Value::String(identity.clone()))]);
        let mut changed = Vec::new();
        if !desired_role.is_empty() && desired_role != existing_role {
            update_payload.insert(
                "role".to_string(),
                Value::String(service_account_role_to_api(&desired_role)),
            );
            changed.push("role".to_string());
        }
        if desired_disabled.is_some() && desired_disabled != existing_disabled {
            update_payload.insert(
                "isDisabled".to_string(),
                Value::Bool(desired_disabled.unwrap_or(false)),
            );
            changed.push("disabled".to_string());
        }
        if changed.is_empty() {
            skipped += 1;
            let detail = "already matched live state.";
            if structured_output {
                dry_run_rows.push(build_service_account_import_dry_run_row(
                    index + 1,
                    &identity,
                    "skip",
                    detail,
                ));
            } else {
                println!(
                    "Skipped service-account {} ({}): {}",
                    identity,
                    index + 1,
                    detail
                );
            }
            continue;
        }
        if args.dry_run {
            updated += 1;
            let detail = format!("would update fields={}", changed.join(","));
            if structured_output {
                dry_run_rows.push(build_service_account_import_dry_run_row(
                    index + 1,
                    &identity,
                    "update",
                    &detail,
                ));
            } else {
                println!("Would update service-account {} {}", identity, detail);
            }
            continue;
        }
        let service_account_id = scalar_text(existing.get("id"));
        if service_account_id.is_empty() {
            return Err(message(format!(
                "Resolved service-account did not include an id: {}",
                identity
            )));
        }
        let _ = update_service_account_with_request(
            &mut request_json,
            &service_account_id,
            &Value::Object(update_payload),
        )?;
        updated += 1;
        println!("Updated service-account {}", identity);
    }

    if structured_output {
        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&Value::Object(Map::from_iter(vec![
                    (
                        "rows".to_string(),
                        Value::Array(dry_run_rows.iter().cloned().map(Value::Object).collect()),
                    ),
                    (
                        "summary".to_string(),
                        Value::Object(Map::from_iter(vec![
                            (
                                "processed".to_string(),
                                Value::Number((processed as i64).into())
                            ),
                            (
                                "created".to_string(),
                                Value::Number((created as i64).into())
                            ),
                            (
                                "updated".to_string(),
                                Value::Number((updated as i64).into())
                            ),
                            (
                                "skipped".to_string(),
                                Value::Number((skipped as i64).into())
                            ),
                            (
                                "source".to_string(),
                                Value::String(args.import_dir.to_string_lossy().to_string()),
                            ),
                        ])),
                    ),
                ])))?
            );
            return Ok(0);
        }
        if args.table {
            for line in format_table(
                &["INDEX", "IDENTITY", "ACTION", "DETAIL"],
                &build_service_account_import_dry_run_rows(&dry_run_rows),
            ) {
                println!("{line}");
            }
            println!();
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

/// Purpose: implementation note.
pub(crate) fn diff_service_accounts_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountDiffArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let local_records =
        load_service_account_import_records(&args.diff_dir, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS)?;
    let local_map =
        build_service_account_diff_map(&local_records, &args.diff_dir.to_string_lossy())?;
    let live_records = list_all_service_accounts_with_request(&mut request_json)?
        .into_iter()
        .map(|item| normalize_service_account_row(&item))
        .collect::<Vec<Map<String, Value>>>();
    let live_map = build_service_account_diff_map(&live_records, "Grafana live service accounts")?;

    let mut differences = 0usize;
    let mut checked = 0usize;
    for key in local_map.keys() {
        checked += 1;
        let (identity, local_payload) = &local_map[key];
        match live_map.get(key) {
            None => {
                println!("Diff missing-live service-account {}", identity);
                differences += 1;
            }
            Some((_live_identity, live_payload)) => {
                let changed = build_record_diff_fields(local_payload, live_payload);
                if changed.is_empty() {
                    println!("Diff same service-account {}", identity);
                } else {
                    differences += 1;
                    println!(
                        "Diff different service-account {} fields={}",
                        identity,
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
        println!("Diff extra-live service-account {}", identity);
    }
    if differences > 0 {
        println!(
            "Diff checked {} service-account(s); {} difference(s) found.",
            checked, differences
        );
    } else {
        println!(
            "No service-account differences across {} service-account(s).",
            checked
        );
    }
    Ok(differences)
}
