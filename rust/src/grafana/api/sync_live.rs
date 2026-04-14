use reqwest::Method;
use serde_json::{Map, Value};

use super::GrafanaApiClient;
use crate::common::{message, Result};
use crate::sync::{append_unique_strings, require_json_object};

#[path = "sync_live_apply.rs"]
mod sync_live_apply;
#[path = "sync_live_apply_result.rs"]
mod sync_live_apply_result;
#[path = "sync_live_read.rs"]
mod sync_live_read;

pub(crate) use sync_live_apply::execute_live_apply_with_client;
#[cfg(test)]
pub(crate) use sync_live_apply::execute_live_apply_with_request;
pub(crate) use sync_live_read::{
    fetch_live_availability_with_client, fetch_live_resource_specs_with_client,
};
#[cfg(test)]
pub(crate) use sync_live_read::{
    fetch_live_availability_with_request, fetch_live_resource_specs_with_request,
};

pub(crate) fn merge_availability(base: Option<Value>, extra: &Value) -> Result<Value> {
    let mut merged = match base {
        Some(Value::Object(object)) => object,
        Some(_) => {
            return Err(message(
                "Sync availability input file must contain a JSON object.",
            ))
        }
        None => Map::new(),
    };
    let extra_object = require_json_object(extra, "Live availability document")?;
    for (key, value) in extra_object {
        if matches!(
            key.as_str(),
            "datasourceUids" | "datasourceNames" | "pluginIds" | "contactPoints"
        ) {
            let existing = merged
                .remove(key)
                .and_then(|item| item.as_array().cloned())
                .unwrap_or_default();
            let mut combined = existing;
            let extra_items = value
                .as_array()
                .ok_or_else(|| message(format!("Live availability field {key} must be a list.")))?;
            let strings = extra_items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            append_unique_strings(&mut combined, &strings);
            merged.insert(key.clone(), Value::Array(combined));
        } else {
            merged.insert(key.clone(), value.clone());
        }
    }
    Ok(Value::Object(merged))
}

pub(crate) struct SyncLiveClient<'a> {
    api: &'a GrafanaApiClient,
}

impl<'a> SyncLiveClient<'a> {
    pub(crate) fn new(api: &'a GrafanaApiClient) -> Self {
        Self { api }
    }

    fn request_json(
        &self,
        method: Method,
        path: &str,
        params: &[(String, String)],
        payload: Option<&Value>,
    ) -> Result<Option<Value>> {
        self.api
            .http_client()
            .request_json(method, path, params, payload)
    }
}
