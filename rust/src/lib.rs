//! Grafana Utils Rust crate.
//!
//! Maintainers should read the full architecture overview here:
//! <docs/overview-rust.md>
//!
//! Crate shape:
//! - `cli` owns only unified command topology, parsing, and dispatch.
//! - Domain facades (`dashboard`, `alert`, `access`, `datasource`, `sync`, `snapshot`) own
//!   command normalization, client/request wiring, and top-level routing.
//! - Shared infrastructure (`common`, `http`) owns errors, JSON/filesystem
//!   helpers, auth/client setup primitives, and live transport behavior.
//! - Crate-private helper modules below are mostly internal contracts or
//!   subsystem-specific support code; they should not grow into new public
//!   maintainer entrypoints without an explicit docs update.
//!
//! Non-obvious relationships:
//! - `datasource` reuses dashboard auth/client helpers instead of owning a
//!   separate transport/auth stack.
//! - `sync` composes staged document builders with `alert_sync`,
//!   `datasource_provider`, and `datasource_secret` assessments.
//! - Interactive/TUI flows stay inside their owning domains, with shared shell
//!   chrome isolated in `tui_shell` when that feature is enabled.
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
/// Datasource-owned live status producer derived from live inventory surfaces.
pub(crate) mod datasource_live_project_status;
/// Datasource-owned status producer derived from staged export documents.
pub(crate) mod datasource_project_status;
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
/// Artifact-driven project overview assembly for staged dashboard and sync inputs.
pub mod overview;
/// Repo-local profile namespace for listing, showing, and initializing config files.
pub mod profile_cli;
/// Repo-local profile/workspace config loading and live connection default resolution.
pub mod profile_config;
/// Shared status contract shapes reused across overview and future status producers.
pub(crate) mod project_status;
/// Top-level status command surface for staged/live project-wide status.
pub mod project_status_command;
/// Shared freshness helpers for live status stamping.
pub(crate) mod project_status_freshness;
/// Internal runtime for live project-status aggregation and per-domain fanout.
pub(crate) mod project_status_live_runtime;
/// Shared staged status builder reused by overview and status staged entrypoints.
pub(crate) mod project_status_staged;
/// Shared support helpers for live project-status client/header construction.
pub(crate) mod project_status_support;
/// Shared status interactive workbench for project-home and handoff flows.
#[cfg(any(feature = "tui", test))]
pub(crate) mod project_status_tui;
/// Snapshot export/review wrappers for staged dashboard and datasource bundles.
pub mod snapshot;
/// Shared staged export scope resolution helpers for dashboard and datasource artifacts.
pub(crate) mod staged_export_scopes;
/// Declarative change planning, review, audit, and apply workflows.
pub mod sync;
pub(crate) mod tabular_output;
/// Shared terminal-shell helpers for the Rust TUI surfaces.
#[cfg(feature = "tui")]
pub(crate) mod tui_shell;
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
#[cfg(test)]
mod overview_rust_tests;
#[cfg(test)]
mod project_status_cli_rust_tests;
#[cfg(test)]
mod snapshot_rust_tests;
