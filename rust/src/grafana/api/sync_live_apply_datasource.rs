use serde_json::{Map, Value};

use crate::common::Result;

use super::sync_live_apply_error::datasource_sync_requires_live_id;

pub(crate) fn resolve_live_datasource_target(
    datasources: &[Map<String, Value>],
    identity: &str,
) -> Result<Option<Map<String, Value>>> {
    for datasource in datasources {
        if datasource.get("uid").and_then(Value::as_str).map(str::trim) == Some(identity) {
            return Ok(Some(datasource.clone()));
        }
    }
    for datasource in datasources {
        if datasource
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            == Some(identity)
        {
            return Ok(Some(datasource.clone()));
        }
    }
    Ok(None)
}

pub(crate) fn resolve_live_datasource_id(
    target: &Map<String, Value>,
    action: &str,
) -> Result<String> {
    target
        .get("id")
        .map(|value| match value {
            Value::String(text) => text.clone(),
            _ => value.to_string(),
        })
        .filter(|value: &String| !value.is_empty())
        .ok_or_else(|| datasource_sync_requires_live_id(action))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn datasource(uid: &str, name: &str, id: Value) -> Map<String, Value> {
        serde_json::from_value(json!({
            "uid": uid,
            "name": name,
            "id": id,
        }))
        .expect("datasource object")
    }

    #[test]
    fn resolves_datasource_target_by_uid_before_name() {
        let datasources = vec![
            datasource("uid-a", "shared-name", Value::from(1)),
            datasource("uid-b", "uid-a", Value::from(2)),
        ];

        let target = resolve_live_datasource_target(&datasources, "uid-a")
            .unwrap()
            .unwrap();

        assert_eq!(target["uid"], json!("uid-a"));
        assert_eq!(target["id"], json!(1));
    }

    #[test]
    fn resolves_datasource_target_by_name_when_uid_does_not_match() {
        let datasources = vec![datasource("uid-a", "shared-name", Value::from(1))];

        let target = resolve_live_datasource_target(&datasources, "shared-name")
            .unwrap()
            .unwrap();

        assert_eq!(target["uid"], json!("uid-a"));
        assert_eq!(target["name"], json!("shared-name"));
    }

    #[test]
    fn resolves_datasource_id_from_string_or_number() {
        let string_id = datasource("uid-a", "name-a", Value::String("17".to_string()));
        let number_id = datasource("uid-b", "name-b", Value::from(21));

        assert_eq!(
            resolve_live_datasource_id(&string_id, "update").unwrap(),
            "17"
        );
        assert_eq!(
            resolve_live_datasource_id(&number_id, "delete").unwrap(),
            "21"
        );
    }

    #[test]
    fn rejects_missing_datasource_id() {
        let mut target = datasource("uid-a", "name-a", Value::from(1));
        target.remove("id");

        assert_eq!(
            resolve_live_datasource_id(&target, "update")
                .unwrap_err()
                .to_string(),
            "Datasource sync update requires a live datasource id."
        );
    }
}
