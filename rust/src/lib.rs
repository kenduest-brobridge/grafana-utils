//! Grafana Utils Rust crate.
//!
//! Maintainers should read the full architecture overview here:
//! <docs/overview-rust.md>
/// Module definition for access.
pub mod access;
/// Module definition for alert.
pub mod alert;
/// Module definition for alert_sync.
pub mod alert_sync;
/// Module definition for bundle_preflight.
pub mod bundle_preflight;
/// Module definition for cli.
pub mod cli;
/// Module definition for common.
pub mod common;
/// Module definition for dashboard.
pub mod dashboard;
/// Module definition for dashboard_inspection_dependency_contract.
pub mod dashboard_inspection_dependency_contract;
/// Module definition for dashboard_inspection_query_features.
pub(crate) mod dashboard_inspection_query_features;
/// Module definition for dashboard_reference_models.
pub mod dashboard_reference_models;
/// Module definition for datasource.
pub mod datasource;
/// Module definition for datasource_catalog.
pub mod datasource_catalog;
/// Module definition for datasource_provider.
pub mod datasource_provider;
/// Module definition for help_styles.
pub mod help_styles;
/// Module definition for http.
pub mod http;
/// Module definition for sync.
pub mod sync;
/// Module definition for sync_bundle_alert_contracts.
pub use sync::bundle_alert_contracts as sync_bundle_alert_contracts;
/// Module definition for sync_bundle_preflight.
pub use sync::bundle_preflight as sync_bundle_preflight;
/// Module definition for sync_preflight.
pub use sync::preflight as sync_preflight;
/// Module definition for sync_workbench.
pub use sync::workbench as sync_workbench;

#[cfg(test)]
mod bundle_preflight_rust_tests;
#[cfg(test)]
mod datasource_provider_rust_tests;
