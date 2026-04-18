//! Local/document-only workspace CLI wrapper.
//!
//! Maintainer note:
//! - Raw JSON inputs are normalized into `SyncResourceSpec` before staged
//!   workspace scan, test, preview, package, review, audit, and apply
//!   documents are built.
//! - Local contract changes usually start in `sync/workbench.rs`; live request
//!   mapping starts in `sync/live.rs` and `grafana/api/sync_live*.rs`; routing
//!   stays in `sync/cli.rs`.
//! - Keep dry-run/reviewable workspace artifacts available even when optional
//!   live fetch/apply wiring is enabled.

mod apply_builder;
mod apply_contract;
pub mod audit;
pub mod audit_tui;
pub(crate) mod blocked_reasons;
pub mod bundle_alert_contracts;
mod bundle_builder;
mod bundle_inputs;
mod bundle_inputs_alert_export;
mod bundle_inputs_alert_registry;
mod bundle_inputs_alert_specs;
mod bundle_inputs_dashboard;
mod bundle_inputs_datasource;
mod bundle_inputs_pipeline;
pub mod bundle_preflight;
mod bundle_preflight_assessments;
pub mod cli;
mod cli_args;
mod discovery_model;
mod dispatch;
mod help_texts;
mod input_normalization;
mod json;
pub mod live;
mod live_project_status;
mod live_project_status_promotion;
mod live_project_status_sync;
mod output;
mod plan_builder;
pub mod preflight;
mod project_status;
mod project_status_promotion;
pub mod promotion_preflight;
pub mod review_tui;
mod staged_documents;
mod summary_builder;
mod task_first;
pub mod workbench;
mod workspace_discovery;
pub(crate) mod workspace_preview_contract;
pub(crate) mod workspace_preview_review_view;

pub(crate) use self::audit::{
    build_sync_audit_document, build_sync_lock_document, build_sync_lock_document_from_lock,
    render_sync_audit_text,
};
pub(crate) use self::audit_tui::run_sync_audit_interactive;
pub(crate) use self::bundle_preflight::{
    build_sync_bundle_preflight_document, render_sync_bundle_preflight_text,
};
pub(crate) use self::discovery_model::{
    render_discovery_summary_from_value, ChangeDiscoveryDocument, DiscoveryInputKind,
};
pub(crate) use self::preflight::{build_sync_preflight_document, render_sync_preflight_text};
pub(crate) use self::project_status::{build_sync_domain_status, SyncDomainStatusInputs};
pub(crate) use self::project_status_promotion::build_promotion_domain_status;
pub(crate) use self::promotion_preflight::{
    build_sync_promotion_preflight_document, render_sync_promotion_preflight_text,
};
pub(crate) use self::task_first::{run_sync_check, run_sync_inspect, run_sync_preview};
pub(crate) use self::workbench::{
    build_sync_apply_intent_document, build_sync_plan_document, build_sync_source_bundle_document,
    build_sync_summary_document, render_sync_source_bundle_text,
};
pub(crate) use self::workspace_discovery::discover_change_staged_inputs;
pub(crate) use crate::alert_sync::assess_alert_sync_specs;
pub(crate) use crate::common::{message, Result};
pub const DEFAULT_REVIEW_TOKEN: &str = "reviewed-workspace-plan";
pub(crate) use bundle_inputs::{
    build_alert_sync_specs, load_alerting_bundle_section, load_dashboard_bundle_sections,
    load_dashboard_provisioning_bundle_sections, load_datasource_provisioning_records,
    load_sync_bundle_input_artifacts, normalize_alert_managed_fields,
    normalize_alert_resource_identity_and_title, normalize_datasource_bundle_item,
    SyncBundleInputSelection,
};
pub(crate) use json::{
    append_unique_strings, load_json_array_file, load_json_value, load_optional_json_object_file,
    require_json_object,
};
pub(crate) use live::{
    execute_live_apply, fetch_live_availability, fetch_live_resource_specs, merge_availability,
};
#[allow(unused_imports)]
pub(crate) use live_project_status::{
    build_live_promotion_domain_status as build_live_promotion_domain_status_transport,
    build_live_sync_domain_status as build_live_sync_domain_status_transport,
};
pub(crate) use live_project_status_promotion::{
    build_live_promotion_project_status, LivePromotionProjectStatusInputs,
};
pub(crate) use live_project_status_sync::{
    build_live_sync_domain_status, SyncLiveProjectStatusInputs,
};
// Lineage helpers stay separate from staged document mutation helpers so the
// review/apply flow is easy to trace at the call site.
pub(crate) use staged_documents::{
    attach_apply_audit, attach_bundle_preflight_summary, attach_lineage, attach_preflight_summary,
    attach_review_audit, attach_trace_id, mark_plan_reviewed, require_matching_optional_trace_id,
    require_optional_stage, require_trace_id, validate_apply_bundle_preflight,
    validate_apply_preflight,
};
// Renderers remain public because the CLI and tests consume their stable text
// output directly.
pub use self::cli_args::*;
pub use self::dispatch::{execute_sync_command, run_sync_cli};
pub use self::output::SyncCommandOutput;
pub(crate) use crate::dashboard::CommonCliArgs;
pub use staged_documents::{
    render_alert_sync_assessment_text, render_sync_apply_intent_text, render_sync_plan_text,
    render_sync_summary_text,
};
#[cfg(feature = "tui")]
pub(crate) use staged_documents::{
    sync_audit_drift_cmp, sync_audit_drift_details, sync_audit_drift_meta, sync_audit_drift_title,
};

#[cfg(test)]
pub(crate) use audit_tui::{build_sync_audit_tui_groups, build_sync_audit_tui_rows};

#[cfg(test)]
#[path = "cli_render_rust_tests.rs"]
mod cli_render_rust_tests;

#[cfg(test)]
#[path = "cli_rust_tests.rs"]
mod sync_cli_rust_tests;

#[cfg(test)]
#[path = "cli_apply_review_rust_tests.rs"]
mod cli_apply_review_rust_tests;

#[cfg(test)]
#[path = "cli_apply_review_exec_rust_tests.rs"]
mod cli_apply_review_exec_rust_tests;

#[cfg(test)]
#[path = "cli_review_tui_rust_tests.rs"]
mod cli_review_tui_rust_tests;

#[cfg(test)]
#[path = "cli_audit_preflight_rust_tests.rs"]
mod cli_audit_preflight_rust_tests;

#[cfg(test)]
#[path = "cli_help_rust_tests.rs"]
mod cli_help_rust_tests;

#[cfg(test)]
#[path = "cli_task_first_smoke_rust_tests.rs"]
mod cli_task_first_smoke_rust_tests;

#[cfg(test)]
#[path = "live_rust_tests.rs"]
mod sync_live_rust_tests;

#[cfg(test)]
#[path = "rust_tests.rs"]
mod sync_rust_tests;

#[cfg(test)]
#[path = "workspace_preview_review_view_rust_tests.rs"]
mod workspace_preview_review_view_rust_tests;
