//! Grafana Utils Rust crate.
//!
//! Maintainers should read the full architecture overview here:
//! <docs/overview-rust.md>
pub mod access;
pub mod alert;
pub mod alert_sync;
pub mod bundle_preflight;
pub mod cli;
pub mod common;
pub mod dashboard;
pub mod datasource;
pub mod datasource_provider;
pub mod http;
pub mod sync;
pub mod sync_bundle_preflight;
pub mod sync_preflight;
pub mod sync_workbench;
pub mod sync_contracts {
    //! Canonical staged sync contract re-exports.
    //!
    //! New stable code should prefer `crate::sync_contracts`. The older
    //! `crate::sync_workbench` path remains as a compatibility module.

    pub use super::sync_workbench::*;
}

#[cfg(test)]
mod bundle_preflight_rust_tests;
#[cfg(test)]
mod datasource_provider_rust_tests;
#[cfg(test)]
mod sync_rust_tests;
#[cfg(test)]
mod sync_schema_rust_tests;
