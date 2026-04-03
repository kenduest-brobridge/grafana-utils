//! Request-backed sync transport wiring.
//!
//! This layer chooses the Grafana client once and then forwards the request
//! function into the testable live fetch/apply helpers. Apply-intent parsing
//! lives in `live_intent.rs` so this module stays transport-focused.

#[path = "live_apply.rs"]
mod live_apply;
#[path = "live_fetch.rs"]
mod live_fetch;

use crate::common::Result;
use crate::dashboard::{build_http_client, build_http_client_for_org, CommonCliArgs};
use serde_json::Value;

pub(crate) use super::apply_contract::{load_apply_intent_operations, SyncApplyOperation};
pub(crate) use live_apply::execute_live_apply_with_request;
pub(crate) use live_fetch::{
    fetch_live_availability_with_request, fetch_live_resource_specs_with_request,
    merge_availability,
};

fn build_sync_http_client(
    common: &CommonCliArgs,
    org_id: Option<i64>,
) -> Result<crate::http::JsonHttpClient> {
    match org_id {
        Some(org_id) => build_http_client_for_org(common, org_id),
        None => build_http_client(common),
    }
}

pub(crate) fn fetch_live_resource_specs(
    common: &CommonCliArgs,
    org_id: Option<i64>,
    page_size: usize,
) -> Result<Vec<Value>> {
    let client = build_sync_http_client(common, org_id)?;
    fetch_live_resource_specs_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        page_size,
    )
}

pub(crate) fn fetch_live_availability(
    common: &CommonCliArgs,
    org_id: Option<i64>,
) -> Result<Value> {
    let client = build_sync_http_client(common, org_id)?;
    fetch_live_availability_with_request(|method, path, params, payload| {
        client.request_json(method, path, params, payload)
    })
}

pub(crate) fn execute_live_apply(
    common: &CommonCliArgs,
    org_id: Option<i64>,
    operations: &[SyncApplyOperation],
    allow_folder_delete: bool,
    allow_policy_reset: bool,
) -> Result<Value> {
    let client = build_sync_http_client(common, org_id)?;
    execute_live_apply_with_request(
        |method, path, params, payload| client.request_json(method, path, params, payload),
        operations,
        allow_folder_delete,
        allow_policy_reset,
    )
}
