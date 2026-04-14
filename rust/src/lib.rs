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
// Public modules are operator entrypoints; crate-private modules are shared plumbing.
// Add a new public module only when the surface contract (help/docs/contracts) is also updated.
/// Access-management domain: users, orgs, teams, and service accounts.
#[path = "commands/access/mod.rs"]
pub mod access;
/// Alerting export/import/diff/list workflows and shared alert models.
#[path = "commands/alert/mod.rs"]
pub mod alert;
/// Alert-specific sync assessment helpers used by preflight and sync flows.
#[path = "commands/alert/sync.rs"]
pub(crate) mod alert_sync;
/// Cross-resource bundle preflight assembly built above sync resource contracts.
#[cfg(test)]
#[path = "commands/sync/root_preflight/mod.rs"]
pub(crate) mod bundle_preflight;
/// Unified top-level CLI parsing and dispatch for the Rust binary.
#[path = "cli/mod.rs"]
pub mod cli;
/// Shell completion generation for the unified CLI command tree.
#[path = "cli/completion.rs"]
pub(crate) mod cli_completion;
/// Crate-private CLI dispatch spine that routes unified commands to domain runners.
#[path = "cli/dispatch.rs"]
pub(crate) mod cli_dispatch;
/// Unified CLI help rendering and example blocks.
#[path = "cli/help/mod.rs"]
pub(crate) mod cli_help;
/// Structured help/example text used by the unified CLI renderer.
#[path = "cli/help_examples.rs"]
pub(crate) mod cli_help_examples;
/// Shared error, auth, JSON, and filesystem helpers reused across domains.
#[path = "common/mod.rs"]
pub mod common;
/// Dashboard export/import/inspect/screenshot/topology workflows.
#[path = "commands/dashboard/mod.rs"]
pub mod dashboard;
/// Internal contract types for dashboard dependency inspection documents.
#[path = "commands/dashboard/inspection/dependency_contract.rs"]
pub(crate) mod dashboard_inspection_dependency_contract;
/// Internal query-feature analysis helpers for dashboard inspection flows.
#[path = "commands/dashboard/inspection/query_features.rs"]
pub(crate) mod dashboard_inspection_query_features;
/// Shared dashboard reference and dependency summary models.
#[path = "commands/dashboard/reference_models.rs"]
pub mod dashboard_reference_models;
/// Datasource inventory and mutation workflows.
#[path = "commands/datasource/mod.rs"]
pub mod datasource;
/// Built-in datasource type catalog and related metadata helpers.
#[path = "commands/datasource/catalog/mod.rs"]
pub mod datasource_catalog;
/// Datasource-owned live status producer derived from live inventory surfaces.
#[path = "commands/datasource/project_status/live.rs"]
pub(crate) mod datasource_live_project_status;
/// Datasource-owned status producer derived from staged export documents.
#[path = "commands/datasource/project_status/staged.rs"]
pub(crate) mod datasource_project_status;
/// Datasource provider resolution helpers used by sync/bundle validation.
#[path = "commands/datasource/provider/mod.rs"]
pub(crate) mod datasource_provider;
/// Datasource secret placeholder planning helpers used by staged sync review.
#[path = "commands/datasource/secret/mod.rs"]
pub(crate) mod datasource_secret;
/// Shared additive export-metadata contract helpers.
#[path = "common/export_metadata.rs"]
pub(crate) mod export_metadata;
/// Shared internal Grafana connection/client layer used by live runtime paths.
#[path = "grafana/api/mod.rs"]
pub(crate) mod grafana_api;
/// Centralized Clap help styling configuration.
#[path = "common/help/styles.rs"]
pub(crate) mod help_styles;
/// Replaceable JSON HTTP client used by all live Grafana operations.
#[path = "grafana/http.rs"]
pub mod http;
/// Internal browser/session helpers for screenshot and interactive flows.
#[path = "common/browser/session.rs"]
pub(crate) mod interactive_browser;
/// Artifact-driven project overview assembly for staged dashboard and sync inputs.
#[path = "commands/status/overview/mod.rs"]
pub mod overview;
/// Repo-local profile namespace for listing, showing, and initializing config files.
#[path = "commands/config/profile/mod.rs"]
pub mod profile_cli;
/// Repo-local profile/workspace config loading and live connection default resolution.
#[path = "commands/config/profile/config.rs"]
pub mod profile_config;
/// Secret storage backends for repo-local profile credentials.
#[path = "commands/config/profile/secret_store.rs"]
pub(crate) mod profile_secret_store;
/// Shared status contract shapes reused across overview and future status producers.
#[path = "commands/status/contract.rs"]
pub(crate) mod project_status;
/// Top-level status command surface for staged/live project-wide status.
#[path = "commands/status/mod.rs"]
pub mod project_status_command;
/// Shared freshness helpers for live status stamping.
#[path = "commands/status/freshness.rs"]
pub(crate) mod project_status_freshness;
/// Internal runtime for live project-status aggregation and per-domain fanout.
#[path = "commands/status/live.rs"]
pub(crate) mod project_status_live_runtime;
/// Shared staged status builder reused by overview and status staged entrypoints.
#[path = "commands/status/staged.rs"]
pub(crate) mod project_status_staged;
/// Shared support helpers for live project-status client/header construction.
#[path = "commands/status/support.rs"]
pub(crate) mod project_status_support;
/// Shared status interactive workbench for project-home and handoff flows.
#[cfg(any(feature = "tui", test))]
#[path = "commands/status/tui/mod.rs"]
pub(crate) mod project_status_tui;
/// Internal resource reference graph shared by preview/status/impact style consumers.
#[path = "reference_graph.rs"]
pub(crate) mod reference_graph;
/// Generic Grafana resource discovery and read-only query commands.
#[path = "commands/resource/mod.rs"]
pub mod resource;
/// Snapshot export/review wrappers for staged dashboard and datasource bundles.
#[path = "commands/snapshot/mod.rs"]
pub mod snapshot;
/// Shared staged export scope resolution helpers for dashboard and datasource artifacts.
#[path = "common/staged_export_scopes.rs"]
pub(crate) mod staged_export_scopes;
/// Declarative change planning, review, audit, and apply workflows.
#[path = "commands/sync/mod.rs"]
pub mod sync;
#[path = "common/output/tabular.rs"]
pub(crate) mod tabular_output;
/// Shared terminal-shell helpers for the Rust TUI surfaces.
#[cfg(feature = "tui")]
#[path = "common/tui/shell.rs"]
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
#[path = "commands/sync/root_preflight/tests.rs"]
mod bundle_preflight_rust_tests;
#[cfg(test)]
#[path = "commands/datasource/provider/tests.rs"]
mod datasource_provider_rust_tests;
#[cfg(test)]
#[path = "commands/datasource/secret/tests.rs"]
mod datasource_secret_rust_tests;
#[cfg(test)]
#[path = "common/export_metadata_rust_tests.rs"]
mod export_metadata_rust_tests;
#[cfg(test)]
#[path = "commands/status/overview/tests.rs"]
mod overview_rust_tests;
#[cfg(test)]
#[path = "commands/status/tests.rs"]
mod project_status_cli_rust_tests;
#[cfg(test)]
#[path = "commands/snapshot/tests.rs"]
mod snapshot_rust_tests;
