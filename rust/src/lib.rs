//! Grafana Utils Rust crate.
//!
//! Maintainers should read the full architecture overview here:
//! <docs/overview-rust.md>
/// Access-management domain: users, orgs, teams, and service accounts.
pub mod access;
/// Alerting export/import/diff/list workflows and shared alert models.
pub mod alert;
/// Alert-specific sync assessment helpers used by preflight and sync flows.
pub(crate) mod alert_sync;
/// Cross-resource bundle preflight assembly built above sync resource contracts.
#[cfg(test)]
pub(crate) mod bundle_preflight;
/// Unified top-level CLI parsing and dispatch for the Rust binary.
pub mod cli;
/// Structured help/example text used by the unified CLI renderer.
pub(crate) mod cli_help_examples;
/// Shared error, auth, JSON, and filesystem helpers reused across domains.
pub mod common;
/// Dashboard export/import/inspect/screenshot/topology workflows.
pub mod dashboard;
/// Internal contract types for dashboard dependency inspection documents.
pub(crate) mod dashboard_inspection_dependency_contract;
/// Internal query-feature analysis helpers for dashboard inspection flows.
pub(crate) mod dashboard_inspection_query_features;
/// Shared dashboard reference and dependency summary models.
pub mod dashboard_reference_models;
/// Datasource inventory and mutation workflows.
pub mod datasource;
/// Built-in datasource type catalog and related metadata helpers.
pub mod datasource_catalog;
/// Datasource provider resolution helpers used by sync/bundle validation.
pub(crate) mod datasource_provider;
/// Datasource secret placeholder planning helpers used by staged sync review.
pub(crate) mod datasource_secret;
/// Centralized Clap help styling configuration.
pub(crate) mod help_styles;
/// Replaceable JSON HTTP client used by all live Grafana operations.
pub mod http;
/// Internal browser/session helpers for screenshot and interactive flows.
pub(crate) mod interactive_browser;
/// Declarative sync planning, review, audit, and apply workflows.
pub mod sync;
/// Re-exported alert bundle contract helpers for compatibility with older paths.
pub use sync::bundle_alert_contracts as sync_bundle_alert_contracts;
/// Re-exported sync bundle preflight helpers for compatibility with older paths.
pub use sync::bundle_preflight as sync_bundle_preflight;
/// Re-exported sync preflight helpers for compatibility with older paths.
pub use sync::preflight as sync_preflight;
/// Re-exported sync workbench helpers for compatibility with older paths.
pub use sync::workbench as sync_workbench;

#[cfg(test)]
mod bundle_preflight_rust_tests;
#[cfg(test)]
mod datasource_provider_rust_tests;
#[cfg(test)]
mod datasource_secret_rust_tests;
