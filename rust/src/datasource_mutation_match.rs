use serde_json::{Map, Value};

use crate::common::string_field;

use super::super::DatasourceImportRecord;

pub(crate) struct MatchResult {
    pub(crate) destination: &'static str,
    pub(crate) action: &'static str,
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) target_uid: String,
    pub(crate) target_name: String,
    pub(crate) target_id: Option<i64>,
}

pub(crate) fn resolve_match(
    record: &DatasourceImportRecord,
    live: &[Map<String, Value>],
    replace_existing: bool,
    update_existing_only: bool,
) -> MatchResult {
    let uid_matches = if !record.uid.is_empty() {
        live.iter()
            .filter(|item| string_field(item, "uid", "") == record.uid)
            .collect::<Vec<&Map<String, Value>>>()
    } else {
        Vec::new()
    };
    let name_matches = if !record.name.is_empty() {
        live.iter()
            .filter(|item| string_field(item, "name", "") == record.name)
            .collect::<Vec<&Map<String, Value>>>()
    } else {
        Vec::new()
    };
    if uid_matches.is_empty() && name_matches.len() > 1 {
        return MatchResult {
            destination: "ambiguous",
            action: "would-fail-ambiguous",
            target_uid: String::new(),
            target_name: record.name.clone(),
            target_id: None,
        };
    }
    if !uid_matches.is_empty() {
        let item = uid_matches[0];
        return MatchResult {
            destination: "exists-uid",
            action: if replace_existing || update_existing_only {
                "would-update"
            } else {
                "would-fail-existing"
            },
            target_uid: string_field(item, "uid", ""),
            target_name: string_field(item, "name", ""),
            target_id: item.get("id").and_then(Value::as_i64),
        };
    }
    if name_matches.len() == 1 {
        let item = name_matches[0];
        let target_uid = string_field(item, "uid", "");
        return MatchResult {
            destination: "exists-name",
            action: if !record.uid.is_empty() && !target_uid.is_empty() && record.uid != target_uid
            {
                "would-fail-uid-mismatch"
            } else if replace_existing || update_existing_only {
                "would-update"
            } else {
                "would-fail-existing"
            },
            target_uid,
            target_name: string_field(item, "name", ""),
            target_id: item.get("id").and_then(Value::as_i64),
        };
    }
    MatchResult {
        destination: "missing",
        action: if update_existing_only {
            "would-skip-missing"
        } else {
            "would-create"
        },
        target_uid: String::new(),
        target_name: String::new(),
        target_id: None,
    }
}

pub(crate) fn resolve_live_mutation_match(
    uid: Option<&str>,
    name: Option<&str>,
    live: &[Map<String, Value>],
) -> MatchResult {
    let normalized_uid = uid.unwrap_or("").trim();
    let normalized_name = name.unwrap_or("").trim();
    let uid_matches = if normalized_uid.is_empty() {
        Vec::new()
    } else {
        live.iter()
            .filter(|item| string_field(item, "uid", "") == normalized_uid)
            .collect::<Vec<&Map<String, Value>>>()
    };
    let name_matches = if normalized_name.is_empty() {
        Vec::new()
    } else {
        live.iter()
            .filter(|item| string_field(item, "name", "") == normalized_name)
            .collect::<Vec<&Map<String, Value>>>()
    };
    if uid_matches.len() > 1 {
        return MatchResult {
            destination: "ambiguous-uid",
            action: "would-fail-ambiguous-uid",
            target_uid: String::new(),
            target_name: normalized_name.to_string(),
            target_id: None,
        };
    }
    if uid_matches.len() == 1 {
        let item = uid_matches[0];
        let target_name = string_field(item, "name", "");
        if !normalized_name.is_empty() && target_name != normalized_name {
            return MatchResult {
                destination: "uid-name-mismatch",
                action: "would-fail-uid-name-mismatch",
                target_uid: string_field(item, "uid", ""),
                target_name,
                target_id: item.get("id").and_then(Value::as_i64),
            };
        }
        return MatchResult {
            destination: "exists-uid",
            action: "would-fail-existing-uid",
            target_uid: string_field(item, "uid", ""),
            target_name,
            target_id: item.get("id").and_then(Value::as_i64),
        };
    }
    if name_matches.len() > 1 {
        return MatchResult {
            destination: "ambiguous-name",
            action: "would-fail-ambiguous-name",
            target_uid: String::new(),
            target_name: normalized_name.to_string(),
            target_id: None,
        };
    }
    if name_matches.len() == 1 {
        let item = name_matches[0];
        let target_uid = string_field(item, "uid", "");
        if !normalized_uid.is_empty() && !target_uid.is_empty() && target_uid != normalized_uid {
            return MatchResult {
                destination: "uid-name-mismatch",
                action: "would-fail-uid-name-mismatch",
                target_uid,
                target_name: string_field(item, "name", ""),
                target_id: item.get("id").and_then(Value::as_i64),
            };
        }
        return MatchResult {
            destination: "exists-name",
            action: "would-fail-existing-name",
            target_uid,
            target_name: string_field(item, "name", ""),
            target_id: item.get("id").and_then(Value::as_i64),
        };
    }
    MatchResult {
        destination: "missing",
        action: "would-create",
        target_uid: String::new(),
        target_name: normalized_name.to_string(),
        target_id: None,
    }
}

pub(crate) fn resolve_delete_match(
    uid: Option<&str>,
    name: Option<&str>,
    live: &[Map<String, Value>],
) -> MatchResult {
    let matching = resolve_live_mutation_match(uid, name, live);
    match matching.destination {
        "exists-uid" | "exists-name" => MatchResult {
            action: "would-delete",
            ..matching
        },
        "missing" => MatchResult {
            action: "would-fail-missing",
            ..matching
        },
        _ => matching,
    }
}
