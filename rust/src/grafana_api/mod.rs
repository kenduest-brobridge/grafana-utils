//! Shared internal Grafana connection and client layer.
//!
//! This module centralizes profile/auth/header resolution and exposes one
//! root client that domain runtimes can share without each rebuilding the
//! same `JsonHttpClient` wiring.

mod access;
pub(crate) mod alert_live;
mod alerting;
mod client;
mod connection;
mod dashboard;
mod datasource;
pub(crate) mod datasource_live_project_status;
pub(crate) mod project_status_live;
mod sync_live;

pub(crate) use access::AccessResourceClient;
pub(crate) use alerting::{
    expect_object, expect_object_list, parse_template_list_response, AlertingResourceClient,
};
pub(crate) use client::GrafanaApiClient;
pub(crate) use connection::{AuthInputs, GrafanaConnection};
pub(crate) use dashboard::DashboardResourceClient;
pub(crate) use datasource::DatasourceResourceClient;
pub(crate) use sync_live::{
    execute_live_apply_with_client as execute_sync_live_apply_with_client,
    fetch_live_availability_with_client as fetch_sync_live_availability_with_client,
    fetch_live_resource_specs_with_client as fetch_sync_live_resource_specs_with_client,
    merge_availability as merge_sync_live_availability, SyncLiveClient,
};
#[cfg(test)]
pub(crate) use sync_live::{
    execute_live_apply_with_request as execute_sync_live_apply_with_request,
    fetch_live_availability_with_request as fetch_sync_live_availability_with_request,
    fetch_live_resource_specs_with_request as fetch_sync_live_resource_specs_with_request,
};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
