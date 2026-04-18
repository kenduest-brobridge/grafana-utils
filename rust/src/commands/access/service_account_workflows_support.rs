//! Shared service-account import/export/diff helpers.

use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::super::list_service_accounts_with_request;
use crate::access::render::{access_diff_review_line, map_get_text, value_bool};
use crate::access::{
    ServiceAccountImportArgs, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS, ACCESS_EXPORT_METADATA_FILENAME,
    ACCESS_EXPORT_VERSION, ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME, DEFAULT_PAGE_SIZE,
};
use crate::common::{
    load_json_object_file, message, render_json_value, string_field, value_as_object, Result,
};
use crate::export_metadata::{
    build_export_metadata_common, export_metadata_common_map, EXPORT_BUNDLE_KIND_ROOT,
};

type DiffPayload = (String, Map<String, Value>);
type DiffPayloadMap = BTreeMap<String, DiffPayload>;

pub(super) fn render_single_object_json(object: &Map<String, Value>) -> Result<String> {
    render_json_value(&Value::Object(object.clone()))
}

pub(super) fn assert_not_overwrite(path: &Path, dry_run: bool, overwrite: bool) -> Result<()> {
    if dry_run || !path.exists() || overwrite {
        return Ok(());
    }
    Err(message(format!(
        "Refusing to overwrite existing file: {}. Use --overwrite.",
        path.display()
    )))
}

pub(super) fn build_service_account_export_metadata(
    source_url: &str,
    source_profile: Option<&str>,
    source_dir: &Path,
    record_count: usize,
) -> Map<String, Value> {
    let metadata = Map::from_iter(vec![
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
        (
            "serviceAccountCount".to_string(),
            Value::Number((record_count as i64).into()),
        ),
        ("tokenFilePresent".to_string(), Value::Bool(false)),
        (
            "tokenMaterial".to_string(),
            Value::String("omitted".to_string()),
        ),
    ]);
    let common = build_export_metadata_common(
        "access",
        "service-accounts",
        EXPORT_BUNDLE_KIND_ROOT,
        "live",
        Some(source_url),
        None,
        source_profile,
        Some("org"),
        None,
        None,
        source_dir,
        &source_dir.join(ACCESS_EXPORT_METADATA_FILENAME),
        record_count,
    );
    let mut metadata = metadata;
    metadata.extend(export_metadata_common_map(&common));
    metadata
}

pub(crate) fn load_service_account_import_records(
    input_dir: &Path,
    expected_kind: &str,
) -> Result<Vec<Map<String, Value>>> {
    let path = input_dir.join(ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME);
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

pub(super) fn build_service_account_import_dry_run_row(
    index: usize,
    identity: &str,
    action: &str,
    detail: &str,
) -> Map<String, Value> {
    Map::from_iter(vec![
        ("index".to_string(), Value::String(index.to_string())),
        ("identity".to_string(), Value::String(identity.to_string())),
        ("action".to_string(), Value::String(action.to_string())),
        ("status".to_string(), Value::String("planned".to_string())),
        ("blocked".to_string(), Value::Bool(false)),
        ("detail".to_string(), Value::String(detail.to_string())),
    ])
}

pub(super) fn mark_service_account_import_row_blocked(row: &mut Map<String, Value>, blocker: &str) {
    row.insert("status".to_string(), Value::String("blocked".to_string()));
    row.insert("blocked".to_string(), Value::Bool(true));
    row.insert(
        "blockers".to_string(),
        Value::Array(vec![Value::String(blocker.to_string())]),
    );
}

pub(super) fn attach_service_account_target(
    row: &mut Map<String, Value>,
    service_account: &Map<String, Value>,
) {
    let mut target = Map::new();
    for (source, dest) in [
        ("id", "targetId"),
        ("serviceAccountId", "targetId"),
        ("uid", "targetUid"),
        ("name", "name"),
        ("login", "login"),
        ("role", "role"),
        ("tokens", "tokens"),
        ("orgId", "orgId"),
    ] {
        let value = map_get_text(service_account, source);
        if !value.is_empty() && !target.contains_key(dest) {
            target.insert(dest.to_string(), Value::String(value));
        }
    }
    if let Some(is_disabled) = value_bool(service_account.get("isDisabled"))
        .or_else(|| value_bool(service_account.get("disabled")))
    {
        target.insert("isDisabled".to_string(), Value::Bool(is_disabled));
    }
    if !target.is_empty() {
        row.insert("target".to_string(), Value::Object(target));
    }
}

pub(super) fn normalize_service_account_import_role(role: &str) -> Result<Option<String>> {
    let trimmed = role.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let normalized = match trimmed.to_ascii_lowercase().as_str() {
        "none" => "None",
        "viewer" => "Viewer",
        "editor" => "Editor",
        "admin" => "Admin",
        _ => {
            return Err(message(format!(
                "Service account import role must be one of None, Viewer, Editor, Admin; got {trimmed}."
            )));
        }
    };
    Ok(Some(normalized.to_string()))
}

pub(super) fn build_service_account_import_dry_run_rows(
    rows: &[Map<String, Value>],
) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            vec![
                map_get_text(row, "index"),
                map_get_text(row, "identity"),
                map_get_text(row, "action"),
                map_get_text(row, "status"),
                map_get_text(row, "detail"),
            ]
        })
        .collect()
}

pub(super) fn build_service_account_import_dry_run_document(
    rows: &[Map<String, Value>],
    processed: usize,
    created: usize,
    updated: usize,
    skipped: usize,
    source: &Path,
) -> Value {
    crate::access::build_access_import_dry_run_document(
        "service-account",
        rows,
        processed,
        created,
        updated,
        skipped,
        source,
    )
}

pub(super) fn build_service_account_diff_review_line(
    checked: usize,
    differences: usize,
    local_source: &str,
    live_source: &str,
) -> String {
    access_diff_review_line(
        "service-account",
        checked,
        differences,
        local_source,
        live_source,
    )
}

pub(super) fn validate_service_account_import_dry_run_output(
    args: &ServiceAccountImportArgs,
) -> Result<()> {
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

pub(crate) fn build_record_diff_fields(
    left: &Map<String, Value>,
    right: &Map<String, Value>,
) -> Vec<String> {
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

pub(crate) fn build_service_account_diff_map(
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

pub(crate) fn list_all_service_accounts_with_request<F>(
    mut request_json: F,
) -> Result<Vec<Map<String, Value>>>
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
        if batch_len < DEFAULT_PAGE_SIZE {
            break;
        }
        page += 1;
    }
    Ok(rows)
}

#[cfg(test)]
mod service_account_json_tests {
    use super::{
        build_service_account_diff_review_line, build_service_account_import_dry_run_document,
        render_single_object_json,
    };
    use serde_json::{Map, Value};
    use std::path::Path;

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

    #[test]
    fn service_account_import_dry_run_document_reports_review_envelope() {
        let rows = vec![Map::from_iter(vec![
            ("index".to_string(), Value::String("1".to_string())),
            ("identity".to_string(), Value::String("svc".to_string())),
            ("action".to_string(), Value::String("created".to_string())),
            (
                "detail".to_string(),
                Value::String("would create service-account".to_string()),
            ),
        ])];

        let document = build_service_account_import_dry_run_document(
            &rows,
            1,
            1,
            0,
            0,
            Path::new("/tmp/access-service-accounts"),
        );

        assert_eq!(
            document.get("kind"),
            Some(&Value::String(
                "grafana-utils-access-import-dry-run".to_string()
            ))
        );
        assert_eq!(
            document.get("resourceKind"),
            Some(&Value::String("service-account".to_string()))
        );
        assert_eq!(
            document.get("schemaVersion"),
            Some(&Value::Number(1.into()))
        );
        assert_eq!(document.get("reviewRequired"), Some(&Value::Bool(true)));
        assert_eq!(document.get("reviewed"), Some(&Value::Bool(false)));
        assert!(document.get("toolVersion").is_some());
        assert_eq!(
            document
                .get("summary")
                .and_then(|summary| summary.get("source")),
            Some(&Value::String("/tmp/access-service-accounts".to_string()))
        );
        assert_eq!(
            document.get("rows").and_then(Value::as_array).map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn service_account_diff_review_line_surfaces_shared_review_contract() {
        let line = build_service_account_diff_review_line(
            3,
            1,
            "./access-service-accounts",
            "Grafana live service accounts",
        );

        assert_eq!(
            line,
            "Review: required=true reviewed=false kind=service-account checked=3 same=2 different=1 source=./access-service-accounts live=Grafana live service accounts"
        );
    }
}
