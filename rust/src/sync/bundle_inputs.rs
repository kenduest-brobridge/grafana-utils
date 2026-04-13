//! Facade for source-bundle input loading and normalization.

#![allow(unused_imports)]

pub(crate) use super::bundle_inputs_alert_export::load_alerting_bundle_section;
pub(crate) use super::bundle_inputs_alert_specs::{
    build_alert_sync_specs, normalize_alert_managed_fields,
    normalize_alert_resource_identity_and_title,
};
pub(crate) use super::bundle_inputs_dashboard::{
    load_dashboard_bundle_sections, load_dashboard_provisioning_bundle_sections,
    normalize_dashboard_bundle_item, normalize_folder_bundle_item, DashboardBundleSections,
};
pub(crate) use super::bundle_inputs_datasource::{
    load_datasource_provisioning_records, normalize_datasource_bundle_item,
};
pub(crate) use super::bundle_inputs_pipeline::{
    load_sync_bundle_input_artifacts, SyncBundleInputArtifacts, SyncBundleInputSelection,
};
