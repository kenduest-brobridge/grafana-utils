//! Access service-account command handlers.
//! Handles service account CRUD and token lifecycle operations behind shared access-request wrappers.
use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::common::{
    load_json_object_file, message, string_field, value_as_object, write_json_file, Result,
};

use super::render::{
    format_table, map_get_text, normalize_service_account_row, render_csv, render_objects_json,
    scalar_text, service_account_role_to_api, service_account_summary_line,
    service_account_table_rows, value_bool,
};
use super::{
    request_object, ServiceAccountAddArgs, ServiceAccountDiffArgs, ServiceAccountExportArgs,
    ServiceAccountImportArgs, ServiceAccountListArgs, ServiceAccountTokenAddArgs,
    ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS, ACCESS_EXPORT_METADATA_FILENAME, ACCESS_EXPORT_VERSION,
    ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME, DEFAULT_PAGE_SIZE,
};

type DiffPayload = (String, Map<String, Value>);
type DiffPayloadMap = BTreeMap<String, DiffPayload>;

/// Fetch one page of service-account search results from Grafana.
///
/// Keep page parameters explicit because Grafana truncates responses by `perpage`.
/// Consumers should treat a returned batch smaller than the requested size as
/// the terminal page and stop pagination immediately.
fn list_service_accounts_with_request<F>(
    mut request_json: F,
    query: Option<&str>,
    page: usize,
    per_page: usize,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let params = vec![
        ("query".to_string(), query.unwrap_or("").to_string()),
        ("page".to_string(), page.to_string()),
        ("perpage".to_string(), per_page.to_string()),
    ];
    let object = request_object(
        &mut request_json,
        Method::GET,
        "/api/serviceaccounts/search",
        &params,
        None,
        "Unexpected service-account list response from Grafana.",
    )?;
    match object.get("serviceAccounts") {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                Ok(value_as_object(
                    value,
                    "Unexpected service-account list response from Grafana.",
                )?
                .clone())
            })
            .collect(),
        _ => Err(message(
            "Unexpected service-account list response from Grafana.",
        )),
    }
}

fn create_service_account_with_request<F>(
    mut request_json: F,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        "/api/serviceaccounts",
        &[],
        Some(payload),
        "Unexpected service-account create response from Grafana.",
    )
}

fn update_service_account_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::PATCH,
        &format!("/api/serviceaccounts/{service_account_id}"),
        &[],
        Some(payload),
        "Unexpected service-account update response from Grafana.",
    )
}

fn create_service_account_token_with_request<F>(
    mut request_json: F,
    service_account_id: &str,
    payload: &Value,
) -> Result<Map<String, Value>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    request_object(
        &mut request_json,
        Method::POST,
        &format!("/api/serviceaccounts/{service_account_id}/tokens"),
        &[],
        Some(payload),
        "Unexpected service-account token create response from Grafana.",
    )
}

fn lookup_service_account_id_by_name<F>(mut request_json: F, name: &str) -> Result<String>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let accounts =
        list_service_accounts_with_request(&mut request_json, Some(name), 1, DEFAULT_PAGE_SIZE)?;
    let account = accounts
        .into_iter()
        .find(|item| string_field(item, "name", "") == name)
        .ok_or_else(|| {
            message(format!(
                "Grafana service-account lookup did not find {name}."
            ))
        })?;
    Ok(scalar_text(account.get("id")))
}

fn list_all_service_accounts_with_request<F>(mut request_json: F) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut page = 1usize;
    let mut rows = Vec::new();
    loop {
        let batch =
            list_service_accounts_with_request(&mut request_json, None, page, DEFAULT_PAGE_SIZE)?;
        if batch.is_empty() {
            break;
        }
        let batch_len = batch.len();
        rows.extend(batch);
        // Stop when Grafana returns a short final page; API responses do not
        // include an explicit "last page" marker in all versions.
        if batch_len < DEFAULT_PAGE_SIZE {
            break;
        }
        page += 1;
    }
    Ok(rows)
}

fn normalize_service_account_for_diff(record: &Map<String, Value>) -> Map<String, Value> {
    Map::from_iter(vec![
        (
            "name".to_string(),
            Value::String(string_field(record, "name", "")),
        ),
        (
            "role".to_string(),
            Value::String(string_field(record, "role", "")),
        ),
        (
            "disabled".to_string(),
            Value::Bool(
                value_bool(record.get("disabled"))
                    .or_else(|| value_bool(record.get("isDisabled")))
                    .unwrap_or(false),
            ),
        ),
    ])
}

fn render_single_object_json(object: &Map<String, Value>) -> Result<String> {
    serde_json::to_string_pretty(&Value::Object(object.clone())).map_err(Into::into)
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

fn build_service_account_export_metadata(
    source_url: &str,
    source_dir: &Path,
    record_count: usize,
) -> Map<String, Value> {
    Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS.to_string()),
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

fn load_service_account_import_records(
    import_dir: &Path,
    expected_kind: &str,
) -> Result<Vec<Map<String, Value>>> {
    let path = import_dir.join(ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME);
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

fn build_service_account_import_dry_run_row(
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

fn build_service_account_import_dry_run_rows(rows: &[Map<String, Value>]) -> Vec<Vec<String>> {
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

fn validate_service_account_import_dry_run_output(args: &ServiceAccountImportArgs) -> Result<()> {
    if (args.table || args.json) && !args.dry_run {
        return Err(message(
            "--table/--json for service-account import are only supported with --dry-run.",
        ));
    }
    if args.table && args.json {
        return Err(message(
            "--table and --json cannot be used together for service-account import.",
        ));
    }
    Ok(())
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

fn build_service_account_diff_map(
    records: &[Map<String, Value>],
    source: &str,
) -> Result<DiffPayloadMap> {
    let mut indexed = BTreeMap::new();
    for record in records {
        let name = string_field(record, "name", "");
        if name.trim().is_empty() {
            return Err(message(format!(
                "Service-account diff record in {} does not include name.",
                source
            )));
        }
        let key = name.trim().to_ascii_lowercase();
        if indexed.contains_key(&key) {
            return Err(message(format!(
                "Duplicate service-account name in {}: {}",
                source, name
            )));
        }
        indexed.insert(
            key,
            (name.clone(), normalize_service_account_for_diff(record)),
        );
    }
    Ok(indexed)
}

/// Purpose: implementation note.
pub(crate) fn list_service_accounts_command_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountListArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut rows = list_service_accounts_with_request(
        &mut request_json,
        args.query.as_deref(),
        args.page,
        args.per_page,
    )?
    .into_iter()
    .map(|item| normalize_service_account_row(&item))
    .collect::<Vec<Map<String, Value>>>();
    if let Some(query) = &args.query {
        let query = query.to_ascii_lowercase();
        rows.retain(|row| {
            map_get_text(row, "name")
                .to_ascii_lowercase()
                .contains(&query)
                || map_get_text(row, "login")
                    .to_ascii_lowercase()
                    .contains(&query)
        });
    }
    if args.json {
        println!("{}", render_objects_json(&rows)?);
    } else if args.csv {
        for line in render_csv(
            &["id", "name", "login", "role", "disabled", "tokens", "orgId"],
            &service_account_table_rows(&rows),
        ) {
            println!("{line}");
        }
    } else if args.table {
        for line in format_table(
            &[
                "ID", "NAME", "LOGIN", "ROLE", "DISABLED", "TOKENS", "ORG_ID",
            ],
            &service_account_table_rows(&rows),
        ) {
            println!("{line}");
        }
        println!();
        println!(
            "Listed {} service account(s) at {}",
            rows.len(),
            args.common.url
        );
    } else {
        for row in &rows {
            println!("{}", service_account_summary_line(row));
        }
        println!();
        println!(
            "Listed {} service account(s) at {}",
            rows.len(),
            args.common.url
        );
    }
    Ok(rows.len())
}

/// Purpose: implementation note.
pub(crate) fn add_service_account_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountAddArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let payload = Value::Object(Map::from_iter(vec![
        ("name".to_string(), Value::String(args.name.clone())),
        (
            "role".to_string(),
            Value::String(service_account_role_to_api(&args.role)),
        ),
        ("isDisabled".to_string(), Value::Bool(args.disabled)),
    ]));
    let created = normalize_service_account_row(&create_service_account_with_request(
        &mut request_json,
        &payload,
    )?);
    if args.json {
        println!("{}", render_single_object_json(&created)?);
    } else {
        println!(
            "Created service-account {} -> id={} role={} disabled={}",
            args.name,
            map_get_text(&created, "id"),
            map_get_text(&created, "role"),
            map_get_text(&created, "disabled")
        );
    }
    Ok(0)
}

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

/// Purpose: implementation note.
pub(crate) fn add_service_account_token_with_request<F>(
    mut request_json: F,
    args: &ServiceAccountTokenAddArgs,
) -> Result<usize>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let service_account_id = match &args.service_account_id {
        Some(value) => value.clone(),
        None => lookup_service_account_id_by_name(
            &mut request_json,
            args.name.as_deref().unwrap_or(""),
        )?,
    };
    let mut payload = Map::from_iter(vec![(
        "name".to_string(),
        Value::String(args.token_name.clone()),
    )]);
    if let Some(seconds) = args.seconds_to_live {
        payload.insert(
            "secondsToLive".to_string(),
            Value::Number((seconds as i64).into()),
        );
    }
    let mut token = create_service_account_token_with_request(
        &mut request_json,
        &service_account_id,
        &Value::Object(payload),
    )?;
    token.insert(
        "serviceAccountId".to_string(),
        Value::String(service_account_id.clone()),
    );
    if args.json {
        println!("{}", render_single_object_json(&token)?);
    } else {
        println!(
            "Created service-account token {} -> serviceAccountId={}",
            args.token_name, service_account_id
        );
    }
    Ok(0)
}

#[cfg(test)]
mod service_account_json_tests {
    use super::render_single_object_json;
    use serde_json::{Map, Value};

    #[test]
    fn render_single_object_json_returns_object_payload() {
        let payload = Map::from_iter(vec![
            ("id".to_string(), Value::Number(4.into())),
            ("name".to_string(), Value::String("svc".to_string())),
        ]);
        let rendered = render_single_object_json(&payload).unwrap();
        assert!(rendered.trim_start().starts_with('{'));
        assert!(!rendered.trim_start().starts_with('['));
        assert!(rendered.contains("\"name\": \"svc\""));
    }
}
