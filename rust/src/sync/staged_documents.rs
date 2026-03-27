//! Sync staged document lineage, validation, rendering, and audit helpers.
//! This module now acts as a facade over focused sibling helpers.

#[path = "staged_documents_lineage.rs"]
mod staged_documents_lineage;
#[path = "staged_documents_render.rs"]
mod staged_documents_render;

#[allow(unused_imports)]
pub(crate) use staged_documents_lineage::{
    attach_lineage, attach_trace_id, derive_trace_id, deterministic_stage_marker, fnv1a64_hex,
    get_trace_id, has_lineage_metadata, normalize_optional_text, normalize_trace_id,
    require_matching_optional_trace_id, require_optional_stage, require_trace_id,
};
pub(crate) use staged_documents_render::{
    attach_apply_audit, attach_bundle_preflight_summary, attach_preflight_summary,
    attach_review_audit, mark_plan_reviewed,
};
pub use staged_documents_render::{
    render_alert_sync_assessment_text, render_sync_apply_intent_text, render_sync_plan_text,
    render_sync_summary_text,
};
pub(crate) use staged_documents_render::{
    sync_audit_drift_cmp, sync_audit_drift_details, sync_audit_drift_meta, sync_audit_drift_title,
    validate_apply_bundle_preflight, validate_apply_preflight,
};
