use reqwest::Method;
use serde_json::{Map, Value};

use crate::common::{message, string_field, Result};
use crate::datasource_catalog::{
    build_add_defaults_for_supported_type, normalize_supported_datasource_type,
    DatasourcePresetProfile,
};
use crate::http::JsonHttpClient;

use super::super::{DatasourceAddArgs, DatasourceModifyArgs};

pub(crate) fn parse_json_object_argument(
    value: Option<&str>,
    label: &str,
) -> Result<Option<Map<String, Value>>> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| message(format!("Invalid JSON for {label}: {error}")))?;
    let object = value
        .as_object()
        .cloned()
        .ok_or_else(|| message(format!("{label} must decode to a JSON object.")))?;
    Ok(Some(object))
}

fn merge_json_object_defaults(existing: &mut Map<String, Value>, incoming: Map<String, Value>) {
    for (key, value) in incoming {
        match (existing.get_mut(&key), value) {
            (Some(Value::Object(existing_value)), Value::Object(incoming_value)) => {
                merge_json_object_defaults(existing_value, incoming_value);
            }
            (_, value) => {
                existing.insert(key, value);
            }
        }
    }
}

fn merge_json_object_fields(
    base: Option<Map<String, Value>>,
    extra: Map<String, Value>,
    label: &str,
) -> Result<Map<String, Value>> {
    let mut merged = base.unwrap_or_default();
    for (key, value) in extra {
        if merged.contains_key(&key) {
            return Err(message(format!(
                "{label} would overwrite existing key {key:?}. Move that field to one place."
            )));
        }
        merged.insert(key, value);
    }
    Ok(merged)
}

fn parse_http_header_arguments(
    values: &[String],
) -> Result<(Map<String, Value>, Map<String, Value>)> {
    let mut json_data = Map::new();
    let mut secure_json_data = Map::new();
    for (index, item) in values.iter().enumerate() {
        let raw = item.trim();
        let Some((name, value)) = raw.split_once('=') else {
            return Err(message(format!(
                "--http-header requires NAME=VALUE form. Invalid value: {raw:?}."
            )));
        };
        let header_name = name.trim();
        if header_name.is_empty() {
            return Err(message(format!(
                "--http-header requires a non-empty header name. Invalid value: {raw:?}."
            )));
        }
        let suffix = index + 1;
        json_data.insert(
            format!("httpHeaderName{suffix}"),
            Value::String(header_name.to_string()),
        );
        secure_json_data.insert(
            format!("httpHeaderValue{suffix}"),
            Value::String(value.to_string()),
        );
    }
    Ok((json_data, secure_json_data))
}

pub(crate) fn build_add_payload(args: &DatasourceAddArgs) -> Result<Value> {
    let normalized_type = normalize_supported_datasource_type(&args.datasource_type);
    let preset_profile = args
        .preset_profile
        .unwrap_or(DatasourcePresetProfile::Starter);
    let use_preset_defaults = args.apply_supported_defaults || args.preset_profile.is_some();
    let mut payload = Map::from_iter(vec![
        ("name".to_string(), Value::String(args.name.clone())),
        ("type".to_string(), Value::String(normalized_type.clone())),
    ]);
    if use_preset_defaults {
        for (key, value) in build_add_defaults_for_supported_type(&normalized_type, preset_profile)
        {
            payload.insert(key, value);
        }
    }
    if let Some(uid) = &args.uid {
        if !uid.trim().is_empty() {
            payload.insert("uid".to_string(), Value::String(uid.trim().to_string()));
        }
    }
    if let Some(access) = &args.access {
        if !access.trim().is_empty() {
            payload.insert(
                "access".to_string(),
                Value::String(access.trim().to_string()),
            );
        }
    }
    if let Some(url) = &args.datasource_url {
        if !url.trim().is_empty() {
            payload.insert("url".to_string(), Value::String(url.trim().to_string()));
        }
    }
    if args.is_default {
        payload.insert("isDefault".to_string(), Value::Bool(true));
    }
    if args.basic_auth || args.basic_auth_user.is_some() || args.basic_auth_password.is_some() {
        payload.insert("basicAuth".to_string(), Value::Bool(true));
    }
    if let Some(basic_auth_user) = &args.basic_auth_user {
        if !basic_auth_user.trim().is_empty() {
            payload.insert(
                "basicAuthUser".to_string(),
                Value::String(basic_auth_user.trim().to_string()),
            );
        }
    }
    if let Some(user) = &args.user {
        if !user.trim().is_empty() {
            payload.insert("user".to_string(), Value::String(user.trim().to_string()));
        }
    }
    if args.with_credentials {
        payload.insert("withCredentials".to_string(), Value::Bool(true));
    }

    let mut json_data = parse_json_object_argument(args.json_data.as_deref(), "--json-data")?;
    let mut secure_json_data =
        parse_json_object_argument(args.secure_json_data.as_deref(), "--secure-json-data")?;
    let mut derived_json_data = Map::new();
    if args.tls_skip_verify {
        derived_json_data.insert("tlsSkipVerify".to_string(), Value::Bool(true));
    }
    if let Some(server_name) = &args.server_name {
        if !server_name.trim().is_empty() {
            derived_json_data.insert(
                "serverName".to_string(),
                Value::String(server_name.trim().to_string()),
            );
        }
    }
    let (header_json_data, header_secure_json_data) =
        parse_http_header_arguments(&args.http_header)?;
    derived_json_data.extend(header_json_data);
    if !derived_json_data.is_empty() || json_data.is_some() {
        json_data = Some(merge_json_object_fields(
            json_data,
            derived_json_data,
            "--json-data",
        )?);
    }
    let mut derived_secure_json_data = Map::new();
    if let Some(basic_auth_password) = &args.basic_auth_password {
        derived_secure_json_data.insert(
            "basicAuthPassword".to_string(),
            Value::String(basic_auth_password.to_string()),
        );
    }
    if let Some(password) = &args.datasource_password {
        derived_secure_json_data
            .insert("password".to_string(), Value::String(password.to_string()));
    }
    derived_secure_json_data.extend(header_secure_json_data);
    if !derived_secure_json_data.is_empty() || secure_json_data.is_some() {
        secure_json_data = Some(merge_json_object_fields(
            secure_json_data,
            derived_secure_json_data,
            "--secure-json-data",
        )?);
    }
    if let Some(json_data) = json_data {
        let merged_json_data = match payload.remove("jsonData") {
            Some(Value::Object(mut existing)) => {
                merge_json_object_defaults(&mut existing, json_data);
                Value::Object(existing)
            }
            _ => Value::Object(json_data),
        };
        payload.insert("jsonData".to_string(), merged_json_data);
    }
    if let Some(secure_json_data) = secure_json_data {
        payload.insert(
            "secureJsonData".to_string(),
            Value::Object(secure_json_data),
        );
    }
    if args.basic_auth_password.is_some() && args.basic_auth_user.is_none() {
        return Err(message("--basic-auth-password requires --basic-auth-user."));
    }
    Ok(Value::Object(payload))
}

pub(crate) fn build_modify_updates(args: &DatasourceModifyArgs) -> Result<Map<String, Value>> {
    let mut updates = Map::new();
    if let Some(url) = &args.set_url {
        if !url.trim().is_empty() {
            updates.insert("url".to_string(), Value::String(url.trim().to_string()));
        }
    }
    if let Some(access) = &args.set_access {
        if !access.trim().is_empty() {
            updates.insert(
                "access".to_string(),
                Value::String(access.trim().to_string()),
            );
        }
    }
    if let Some(is_default) = args.set_default {
        updates.insert("isDefault".to_string(), Value::Bool(is_default));
    }
    if args.basic_auth || args.basic_auth_user.is_some() || args.basic_auth_password.is_some() {
        updates.insert("basicAuth".to_string(), Value::Bool(true));
    }
    if let Some(basic_auth_user) = &args.basic_auth_user {
        updates.insert(
            "basicAuthUser".to_string(),
            Value::String(basic_auth_user.to_string()),
        );
    }
    if let Some(user) = &args.user {
        updates.insert("user".to_string(), Value::String(user.to_string()));
    }
    if args.with_credentials {
        updates.insert("withCredentials".to_string(), Value::Bool(true));
    }

    let mut json_data = parse_json_object_argument(args.json_data.as_deref(), "--json-data")?;
    let mut secure_json_data =
        parse_json_object_argument(args.secure_json_data.as_deref(), "--secure-json-data")?;
    let mut derived_json_data = Map::new();
    if args.tls_skip_verify {
        derived_json_data.insert("tlsSkipVerify".to_string(), Value::Bool(true));
    }
    if let Some(server_name) = &args.server_name {
        if !server_name.trim().is_empty() {
            derived_json_data.insert(
                "serverName".to_string(),
                Value::String(server_name.trim().to_string()),
            );
        }
    }
    let (header_json_data, header_secure_json_data) =
        parse_http_header_arguments(&args.http_header)?;
    derived_json_data.extend(header_json_data);
    if !derived_json_data.is_empty() || json_data.is_some() {
        json_data = Some(merge_json_object_fields(
            json_data,
            derived_json_data,
            "--json-data",
        )?);
    }
    if let Some(json_data) = json_data {
        updates.insert("jsonData".to_string(), Value::Object(json_data));
    }

    let mut derived_secure_json_data = Map::new();
    if let Some(basic_auth_password) = &args.basic_auth_password {
        derived_secure_json_data.insert(
            "basicAuthPassword".to_string(),
            Value::String(basic_auth_password.to_string()),
        );
    }
    if let Some(password) = &args.datasource_password {
        derived_secure_json_data
            .insert("password".to_string(), Value::String(password.to_string()));
    }
    derived_secure_json_data.extend(header_secure_json_data);
    if !derived_secure_json_data.is_empty() || secure_json_data.is_some() {
        secure_json_data = Some(merge_json_object_fields(
            secure_json_data,
            derived_secure_json_data,
            "--secure-json-data",
        )?);
    }
    if let Some(secure_json_data) = secure_json_data {
        updates.insert(
            "secureJsonData".to_string(),
            Value::Object(secure_json_data),
        );
    }
    if updates.is_empty() {
        return Err(message(
            "Datasource modify requires at least one change flag.",
        ));
    }
    Ok(updates)
}

pub(crate) fn fetch_datasource_by_uid_if_exists(
    client: &JsonHttpClient,
    uid: &str,
) -> Result<Option<Map<String, Value>>> {
    match client.request_json(
        Method::GET,
        &format!("/api/datasources/uid/{uid}"),
        &[],
        None,
    ) {
        Ok(Some(value)) => value
            .as_object()
            .cloned()
            .map(Some)
            .ok_or_else(|| message(format!("Unexpected datasource payload for UID {uid}."))),
        Ok(None) => Ok(None),
        Err(error) if error.status_code() == Some(404) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn build_modify_payload(
    existing: &Map<String, Value>,
    updates: &Map<String, Value>,
) -> Result<Value> {
    let mut payload = Map::from_iter(vec![
        (
            "id".to_string(),
            existing.get("id").cloned().unwrap_or(Value::Null),
        ),
        (
            "uid".to_string(),
            Value::String(string_field(existing, "uid", "")),
        ),
        (
            "name".to_string(),
            Value::String(string_field(existing, "name", "")),
        ),
        (
            "type".to_string(),
            Value::String(string_field(existing, "type", "")),
        ),
        (
            "access".to_string(),
            Value::String(string_field(existing, "access", "")),
        ),
        (
            "url".to_string(),
            Value::String(string_field(existing, "url", "")),
        ),
        (
            "isDefault".to_string(),
            Value::Bool(
                existing
                    .get("isDefault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        ),
    ]);
    if let Some(database) = existing.get("database").cloned() {
        payload.insert("database".to_string(), database);
    }
    if let Some(value) = updates.get("basicAuth").cloned() {
        payload.insert("basicAuth".to_string(), value);
    } else if let Some(value) = existing.get("basicAuth").cloned() {
        payload.insert("basicAuth".to_string(), value);
    }
    if let Some(value) = updates.get("basicAuthUser").cloned() {
        payload.insert("basicAuthUser".to_string(), value);
    } else if let Some(value) = existing.get("basicAuthUser").cloned() {
        payload.insert("basicAuthUser".to_string(), value);
    }
    if let Some(value) = updates.get("user").cloned() {
        payload.insert("user".to_string(), value);
    } else if let Some(value) = existing.get("user").cloned() {
        payload.insert("user".to_string(), value);
    }
    if let Some(value) = updates.get("withCredentials").cloned() {
        payload.insert("withCredentials".to_string(), value);
    } else if let Some(value) = existing.get("withCredentials").cloned() {
        payload.insert("withCredentials".to_string(), value);
    }
    let merged_json_data = {
        let mut json_data = existing
            .get("jsonData")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if let Some(update_json_data) = updates.get("jsonData").and_then(Value::as_object) {
            merge_json_object_defaults(&mut json_data, update_json_data.clone());
        }
        json_data
    };
    if !merged_json_data.is_empty() {
        payload.insert("jsonData".to_string(), Value::Object(merged_json_data));
    }
    if let Some(secure_json_data) = updates.get("secureJsonData").and_then(Value::as_object) {
        if !secure_json_data.is_empty() {
            payload.insert(
                "secureJsonData".to_string(),
                Value::Object(secure_json_data.clone()),
            );
        }
    }
    if updates
        .get("secureJsonData")
        .and_then(Value::as_object)
        .is_some_and(|secure_json_data| secure_json_data.contains_key("basicAuthPassword"))
        && !payload.contains_key("basicAuthUser")
    {
        return Err(message(
            "--basic-auth-password requires --basic-auth-user or an existing basicAuthUser.",
        ));
    }
    for key in ["url", "access", "isDefault"] {
        if let Some(value) = updates.get(key).cloned() {
            payload.insert(key.to_string(), value);
        }
    }
    Ok(Value::Object(payload))
}
