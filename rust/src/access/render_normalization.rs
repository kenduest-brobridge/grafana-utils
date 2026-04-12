use serde_json::{Map, Value};

use crate::access::Scope;

/// bool label.
pub(crate) fn bool_label(value: Option<bool>) -> String {
    match value {
        Some(true) => "true".to_string(),
        Some(false) => "false".to_string(),
        None => String::new(),
    }
}

/// scalar text.
pub(crate) fn scalar_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

/// value bool.
pub(crate) fn value_bool(value: Option<&Value>) -> Option<bool> {
    match value {
        Some(Value::Bool(v)) => Some(*v),
        Some(Value::String(text)) => match text.to_ascii_lowercase().as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        Some(Value::Number(number)) => match number.as_i64() {
            Some(1) => Some(true),
            Some(0) => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// map get text.
pub(crate) fn map_get_text(map: &Map<String, Value>, key: &str) -> String {
    match map.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<&str>>()
            .join(","),
        _ => String::new(),
    }
}

// Normalize user/team role payloads into a canonical display/case convention used by
// list output and diffing.
/// Purpose: implementation note.
pub(crate) fn normalize_org_role(value: Option<&Value>) -> String {
    let text = match value {
        Some(Value::String(text)) => text.trim(),
        _ => "",
    };
    match text.to_ascii_lowercase().as_str() {
        "" => String::new(),
        "nobasicrole" | "none" => "None".to_string(),
        lowered => {
            let mut chars = lowered.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        }
    }
}

/// service account role to api.
pub(crate) fn service_account_role_to_api(role: &str) -> String {
    match role.trim().to_ascii_lowercase().as_str() {
        "none" => "NoBasicRole".to_string(),
        "viewer" => "Viewer".to_string(),
        "editor" => "Editor".to_string(),
        "admin" => "Admin".to_string(),
        other => other.to_string(),
    }
}

/// user scope text.
pub(crate) fn user_scope_text(scope: &Scope) -> &'static str {
    match scope {
        Scope::Org => "org",
        Scope::Global => "global",
    }
}

/// User account identity scope text.
pub(crate) fn user_account_scope_text() -> &'static str {
    "global-shared"
}

#[cfg(test)]
mod tests {
    use super::{
        bool_label, map_get_text, normalize_org_role, scalar_text, service_account_role_to_api,
        value_bool,
    };
    use serde_json::{json, Map, Value};

    #[test]
    fn value_bool_handles_strings_numbers_and_bools() {
        assert_eq!(value_bool(Some(&json!(true))), Some(true));
        assert_eq!(value_bool(Some(&json!(false))), Some(false));
        assert_eq!(value_bool(Some(&json!("true"))), Some(true));
        assert_eq!(value_bool(Some(&json!("false"))), Some(false));
        assert_eq!(value_bool(Some(&json!(1))), Some(true));
        assert_eq!(value_bool(Some(&json!(0))), Some(false));
        assert_eq!(value_bool(Some(&json!("maybe"))), None);
    }

    #[test]
    fn normalize_org_role_canonicalizes_known_labels() {
        assert_eq!(normalize_org_role(Some(&json!("none"))), "None");
        assert_eq!(normalize_org_role(Some(&json!("NoBasicRole"))), "None");
        assert_eq!(normalize_org_role(Some(&json!("viewer"))), "Viewer");
        assert_eq!(normalize_org_role(Some(&json!(""))), "");
    }

    #[test]
    fn map_get_text_joins_string_arrays() {
        let map = Map::from_iter(vec![
            ("name".to_string(), Value::String("alice".to_string())),
            (
                "teams".to_string(),
                Value::Array(vec![json!("A"), json!(""), json!("B")]),
            ),
        ]);

        assert_eq!(map_get_text(&map, "name"), "alice");
        assert_eq!(map_get_text(&map, "teams"), "A,B");
        assert_eq!(scalar_text(Some(&json!(42))), "42");
        assert_eq!(bool_label(Some(true)), "true");
        assert_eq!(service_account_role_to_api("none"), "NoBasicRole");
    }
}
