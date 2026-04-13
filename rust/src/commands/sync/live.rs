//! Request-backed sync transport wiring.
//!
//! This layer chooses the Grafana client once and then forwards work into the
//! shared `grafana_api::sync_live_*` workflow helpers. Apply-intent parsing
//! lives in `live_intent.rs` so this module stays transport-focused.

use crate::common::Result;
use crate::dashboard::{CommonCliArgs, DEFAULT_TIMEOUT, DEFAULT_URL};
use crate::grafana_api::{
    execute_sync_live_apply_with_client, fetch_sync_live_availability_with_client,
    fetch_sync_live_resource_specs_with_client, merge_sync_live_availability, AuthInputs,
    GrafanaApiClient, GrafanaConnection, SyncLiveClient,
};
use crate::profile_config::ConnectionMergeInput;
use serde_json::Value;

pub(crate) use super::apply_contract::{load_apply_intent_operations, SyncApplyOperation};
#[cfg(test)]
pub(crate) use crate::grafana_api::{
    execute_sync_live_apply_with_request as execute_live_apply_with_request,
    fetch_sync_live_availability_with_request as fetch_live_availability_with_request,
    fetch_sync_live_resource_specs_with_request as fetch_live_resource_specs_with_request,
};

fn build_sync_api_client(common: &CommonCliArgs) -> Result<GrafanaApiClient> {
    let connection = GrafanaConnection::resolve(
        common.profile.as_deref(),
        ConnectionMergeInput {
            url: &common.url,
            url_default: DEFAULT_URL,
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            org_id: None,
            timeout: common.timeout,
            timeout_default: DEFAULT_TIMEOUT,
            verify_ssl: common.verify_ssl,
            insecure: false,
            ca_cert: None,
        },
        AuthInputs {
            api_token: common.api_token.as_deref(),
            username: common.username.as_deref(),
            password: common.password.as_deref(),
            prompt_password: common.prompt_password,
            prompt_token: common.prompt_token,
        },
        false,
    )?;
    GrafanaApiClient::from_connection(connection)
}

fn build_sync_scoped_api_client(
    common: &CommonCliArgs,
    org_id: Option<i64>,
) -> Result<GrafanaApiClient> {
    let api = build_sync_api_client(common)?;
    match org_id {
        Some(org_id) => api.scoped_to_org(org_id),
        None => Ok(api),
    }
}

pub(crate) fn fetch_live_resource_specs(
    common: &CommonCliArgs,
    org_id: Option<i64>,
    page_size: usize,
) -> Result<Vec<Value>> {
    let api = build_sync_scoped_api_client(common, org_id)?;
    let client = SyncLiveClient::new(&api);
    fetch_sync_live_resource_specs_with_client(&client, page_size)
}

pub(crate) fn fetch_live_availability(
    common: &CommonCliArgs,
    org_id: Option<i64>,
) -> Result<Value> {
    let api = build_sync_scoped_api_client(common, org_id)?;
    let client = SyncLiveClient::new(&api);
    fetch_sync_live_availability_with_client(&client)
}

pub(crate) fn execute_live_apply(
    common: &CommonCliArgs,
    org_id: Option<i64>,
    operations: &[SyncApplyOperation],
    allow_folder_delete: bool,
    allow_policy_reset: bool,
) -> Result<Value> {
    let api = build_sync_scoped_api_client(common, org_id)?;
    let client = SyncLiveClient::new(&api);
    execute_sync_live_apply_with_client(
        &client,
        operations,
        allow_folder_delete,
        allow_policy_reset,
    )
}

pub(crate) fn merge_availability(base: Option<Value>, extra: &Value) -> Result<Value> {
    merge_sync_live_availability(base, extra)
}
