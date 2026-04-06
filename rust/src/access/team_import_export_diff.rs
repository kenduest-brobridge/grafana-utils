//! Shared team import/export/diff helpers.

use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::common::{load_json_object_file, message, string_field, value_as_object, Result};
use crate::export_metadata::{
    build_export_metadata_common, export_metadata_common_map, EXPORT_BUNDLE_KIND_ROOT,
};

use super::render::map_get_text;
use super::team_runtime::normalize_access_identity;
use super::{
    TeamImportArgs, ACCESS_EXPORT_KIND_TEAMS, ACCESS_EXPORT_METADATA_FILENAME,
    ACCESS_EXPORT_VERSION, ACCESS_TEAM_EXPORT_FILENAME,
};

type DiffPayload = (String, Map<String, Value>);
type DiffPayloadMap = BTreeMap<String, DiffPayload>;

fn normalize_team_for_diff(
    record: &Map<String, Value>,
    include_members: bool,
) -> Map<String, Value> {
    let mut members = parse_access_identity_list(record.get("members").unwrap_or(&Value::Null));
    let mut admins = parse_access_identity_list(record.get("admins").unwrap_or(&Value::Null));
    members.sort();
    admins.sort();
    let mut payload = Map::from_iter(vec![
        (
            "name".to_string(),
            Value::String(string_field(record, "name", "")),
        ),
        (
            "email".to_string(),
            Value::String(string_field(record, "email", "")),
        ),
    ]);
    if include_members {
        payload.insert(
            "members".to_string(),
            Value::Array(members.into_iter().map(Value::String).collect()),
        );
        payload.insert(
            "admins".to_string(),
            Value::Array(admins.into_iter().map(Value::String).collect()),
        );
    } else {
        payload.insert("members".to_string(), Value::Array(Vec::new()));
        payload.insert("admins".to_string(), Value::Array(Vec::new()));
    }
    payload
}

pub(super) fn build_team_diff_map(
    records: &[Map<String, Value>],
    source: &str,
    include_members: bool,
) -> Result<DiffPayloadMap> {
    let mut indexed = BTreeMap::new();
    for record in records {
        let team_name = string_field(record, "name", "");
        if team_name.trim().is_empty() {
            return Err(message(format!(
                "Team diff record in {} does not include name.",
                source
            )));
        }
        let key = normalize_access_identity(&team_name);
        if indexed.contains_key(&key) {
            return Err(message(format!(
                "Duplicate team name in {}: {}",
                source, team_name
            )));
        }
        let payload = normalize_team_for_diff(record, include_members);
        indexed.insert(key, (team_name, payload));
    }
    Ok(indexed)
}

pub(super) fn build_record_diff_fields(
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

pub(super) fn sorted_membership_union(members: &[String], admins: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut merged = Vec::new();
    for identity in members.iter().chain(admins.iter()) {
        let key = normalize_access_identity(identity);
        if seen.insert(key) {
            merged.push(identity.clone());
        }
    }
    merged
}

pub(super) fn build_membership_payloads(
    members: &[String],
    admins: &[String],
) -> (Vec<String>, Vec<String>) {
    let admin_keys = admins
        .iter()
        .map(|identity| normalize_access_identity(identity))
        .collect::<BTreeSet<_>>();
    let mut regular_members = Vec::new();
    for identity in members {
        if !admin_keys.contains(&normalize_access_identity(identity))
            && !regular_members.contains(identity)
        {
            regular_members.push(identity.clone());
        }
    }
    (regular_members, admins.to_vec())
}

pub(super) fn parse_access_identity_list(value: &Value) -> Vec<String> {
    // Call graph (hierarchy): this function is used in related modules.
    // Upstream callers: team.rs:normalize_team_for_diff
    // Downstream callees: team.rs:normalize_access_identity

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

pub(super) fn build_team_import_dry_run_row(
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

pub(super) fn build_team_import_dry_run_rows(rows: &[Map<String, Value>]) -> Vec<Vec<String>> {
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

pub(crate) fn build_team_import_dry_run_document(
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

pub(super) fn validate_team_import_dry_run_output(args: &TeamImportArgs) -> Result<()> {
    if (args.table || args.json) && !args.dry_run {
        return Err(message(
            "--table/--json for team import are only supported with --dry-run.",
        ));
    }
    if args.table && args.json {
        return Err(message(
            "--table and --json cannot be used together for team import.",
        ));
    }
    Ok(())
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

pub(super) fn build_team_access_export_metadata(
    source_url: &str,
    source_profile: Option<&str>,
    source_dir: &Path,
    record_count: usize,
    with_members: bool,
) -> Map<String, Value> {
    let metadata = Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(ACCESS_EXPORT_KIND_TEAMS.to_string()),
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
        ("withMembers".to_string(), Value::Bool(with_members)),
        (
            "teamCount".to_string(),
            Value::Number((record_count as i64).into()),
        ),
    ]);
    let common = build_export_metadata_common(
        "access",
        "teams",
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

pub(super) fn load_team_import_records(
    import_dir: &Path,
    expected_kind: &str,
) -> Result<Vec<Map<String, Value>>> {
    let path = import_dir.join(ACCESS_TEAM_EXPORT_FILENAME);
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
