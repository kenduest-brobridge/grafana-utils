//! Shared org import/export/diff helpers.

use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::super::{ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_VERSION, ACCESS_ORG_EXPORT_FILENAME};
use super::{list_org_users_with_request, list_organizations_with_request};
use crate::access::render::{normalize_org_role, scalar_text};
use crate::common::{message, string_field, value_as_object, Result};

type OrgDiffRecord = (String, Map<String, Value>);
type OrgDiffMap = BTreeMap<String, OrgDiffRecord>;

fn org_user_identity(user: &Map<String, Value>) -> String {
    let login = string_field(user, "login", "");
    if !login.is_empty() {
        return login;
    }
    let email = string_field(user, "email", "");
    if !email.is_empty() {
        return email;
    }
    let user_id = scalar_text(user.get("userId"));
    if !user_id.is_empty() {
        return user_id;
    }
    scalar_text(user.get("id"))
}

fn normalize_org_user_for_diff(user: &Map<String, Value>) -> Map<String, Value> {
    Map::from_iter(vec![
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
            Value::String(normalize_org_role(
                user.get("role").or_else(|| user.get("orgRole")),
            )),
        ),
    ])
}

fn build_org_user_diff_array(users: &[Map<String, Value>], source: &str) -> Result<Vec<Value>> {
    let mut indexed = BTreeMap::new();
    for user in users {
        let identity = org_user_identity(user);
        if identity.trim().is_empty() {
            return Err(message(format!(
                "Organization user diff record in {} does not include login, email, or id.",
                source
            )));
        }
        let key = identity.trim().to_ascii_lowercase();
        if indexed.contains_key(&key) {
            return Err(message(format!(
                "Duplicate organization user identity in {}: {}",
                source, identity
            )));
        }
        indexed.insert(key, (identity, normalize_org_user_for_diff(user)));
    }
    Ok(indexed
        .into_values()
        .map(|(_, payload)| Value::Object(payload))
        .collect())
}

pub(super) fn build_org_live_records_for_diff<F>(
    mut request_json: F,
    include_users: bool,
) -> Result<Vec<Map<String, Value>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let orgs = list_organizations_with_request(&mut request_json)?;
    let mut records = Vec::new();
    for org in orgs {
        let mut row = Map::from_iter(vec![(
            "name".to_string(),
            Value::String(string_field(&org, "name", "")),
        )]);
        if include_users {
            let org_id = scalar_text(org.get("id"));
            let users = list_org_users_with_request(&mut request_json, &org_id)?
                .into_iter()
                .map(|user| normalize_org_user_for_diff(&user))
                .collect::<Vec<Map<String, Value>>>();
            let users = build_org_user_diff_array(&users, "Grafana live org users")?;
            row.insert("users".to_string(), Value::Array(users));
        } else {
            row.insert("users".to_string(), Value::Array(Vec::new()));
        }
        records.push(row);
    }
    Ok(records)
}

pub(super) fn build_org_diff_map(
    records: &[Map<String, Value>],
    source: &str,
    include_users: bool,
) -> Result<OrgDiffMap> {
    let mut indexed = BTreeMap::new();
    for record in records {
        let org_name = string_field(record, "name", "");
        if org_name.trim().is_empty() {
            return Err(message(format!(
                "Organization diff record in {} does not include name.",
                source
            )));
        }
        let key = org_name.trim().to_ascii_lowercase();
        if indexed.contains_key(&key) {
            return Err(message(format!(
                "Duplicate organization name in {}: {}",
                source, org_name
            )));
        }
        let mut payload =
            Map::from_iter(vec![("name".to_string(), Value::String(org_name.clone()))]);
        let users = if include_users {
            match record.get("users") {
                Some(Value::Array(values)) => {
                    let users = values
                        .iter()
                        .map(|value| value_as_object(value, "Unexpected org user record.").cloned())
                        .collect::<Result<Vec<Map<String, Value>>>>()?;
                    build_org_user_diff_array(&users, source)?
                }
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };
        payload.insert("users".to_string(), Value::Array(users));
        indexed.insert(key, (org_name, payload));
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

pub(super) fn build_org_export_metadata(
    source_url: &str,
    source_dir: &Path,
    record_count: usize,
) -> Map<String, Value> {
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

pub(super) fn load_org_import_records(import_dir: &Path) -> Result<Vec<Map<String, Value>>> {
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
        .map(|value| value_as_object(&value, "Access import entry must be an object.").cloned())
        .collect()
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
