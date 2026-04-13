//! Sync CLI orchestration for the staged workspace plan/review/apply workflow.
//!
//! This module owns command wiring, trace propagation, and orchestration-time
//! live overlays. The lower-level builders stay focused on normalized
//! documents so they remain reusable from tests and offline inputs.

pub(crate) use super::output::render_and_emit_sync_command_output;
use super::output::{emit_text_or_json, sync_command_output};
use super::*;
use crate::sync::live::load_apply_intent_operations;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub(crate) fn load_sync_live_array(
    fetch_live: bool,
    common: &CommonCliArgs,
    org_id: Option<i64>,
    page_size: usize,
    live_file: Option<&PathBuf>,
    live_file_error: &'static str,
) -> Result<Vec<Value>> {
    if fetch_live {
        fetch_live_resource_specs(common, org_id, page_size)
    } else {
        let live_file = live_file.ok_or_else(|| message(live_file_error))?;
        load_json_array_file(live_file, "Sync live input")
    }
}

fn load_sync_merged_availability(
    fetch_live: bool,
    common: &CommonCliArgs,
    org_id: Option<i64>,
    availability_file: Option<&PathBuf>,
) -> Result<Option<Value>> {
    let availability =
        load_optional_json_object_file(availability_file, "Sync availability input")?;
    if fetch_live {
        Ok(Some(merge_availability(
            availability,
            &fetch_live_availability(common, org_id)?,
        )?))
    } else {
        Ok(availability)
    }
}

fn build_sync_review_document(
    reviewed_plan_input: &Value,
    review_token: &str,
    trace_id: &str,
    reviewed_by: Option<&str>,
    reviewed_at: Option<&str>,
    review_note: Option<&str>,
) -> Result<Value> {
    let document = mark_plan_reviewed(reviewed_plan_input, review_token)?;
    let document = attach_review_audit(&document, trace_id, reviewed_by, reviewed_at, review_note)?;
    attach_lineage(&document, "review", 2, Some(trace_id))
}

fn build_sync_apply_document(
    args: &SyncApplyArgs,
    plan: &Value,
    preflight_summary: Option<Value>,
    bundle_preflight_summary: Option<Value>,
    trace_id: &str,
) -> Result<Value> {
    let document = build_sync_apply_intent_document(plan, args.approve)?;
    let document = attach_preflight_summary(&document, preflight_summary)?;
    let document = attach_bundle_preflight_summary(&document, bundle_preflight_summary)?;
    let document = attach_apply_audit(
        &document,
        trace_id,
        args.applied_by.as_deref(),
        args.applied_at.as_deref(),
        args.approval_reason.as_deref(),
        args.apply_note.as_deref(),
    )?;
    let document = attach_trace_id(&document, Some(trace_id))?;
    attach_lineage(&document, "apply", 3, Some(trace_id))
}

fn load_sync_apply_preflight_summary(
    trace_id: &str,
    preflight_file: Option<&PathBuf>,
) -> Result<Option<Value>> {
    match preflight_file {
        None => Ok(None),
        Some(path) => {
            let preflight = load_json_value(path, "Workspace input-test input")?;
            require_matching_optional_trace_id(
                &preflight,
                "Workspace input-test document",
                trace_id,
            )?;
            validate_apply_preflight(&preflight).map(Some)
        }
    }
}

fn load_sync_apply_bundle_preflight_summary(
    trace_id: &str,
    bundle_preflight_file: Option<&PathBuf>,
) -> Result<Option<Value>> {
    match bundle_preflight_file {
        None => Ok(None),
        Some(path) => {
            let bundle_preflight = load_json_value(path, "Workspace package-test input")?;
            require_matching_optional_trace_id(
                &bundle_preflight,
                "Workspace package-test document",
                trace_id,
            )?;
            validate_apply_bundle_preflight(&bundle_preflight).map(Some)
        }
    }
}

pub(crate) fn run_sync_audit(args: SyncAuditArgs) -> Result<()> {
    // Audit is a read-only orchestration command: materialize a lock/baseline view,
    // compare drift, then emit a structured document for downstream gate checks.
    if args.managed_file.is_none() && args.lock_file.is_none() {
        return Err(message(
            "Sync audit requires --managed-file, --lock-file, or both.",
        ));
    }
    // Resolve live state here so the lock/audit builders keep accepting plain
    // normalized JSON arrays and stay easy to exercise offline.
    let live = load_sync_live_array(
        args.fetch_live,
        &args.common,
        args.org_id,
        args.page_size,
        args.live_file.as_ref(),
        "Sync audit requires --live-file unless --fetch-live is used.",
    )?;
    let baseline_lock = match args.lock_file.as_ref() {
        Some(path) => Some(load_json_value(path, "Sync lock input")?),
        None => None,
    };
    let current_lock = match args.managed_file.as_ref() {
        Some(path) => {
            let managed = load_json_array_file(path, "Sync managed input")?;
            build_sync_lock_document(&managed, &live)?
        }
        None => {
            let baseline = baseline_lock
                .as_ref()
                .ok_or_else(|| message("Sync audit requires --managed-file or --lock-file."))?;
            build_sync_lock_document_from_lock(baseline, &live)?
        }
    };
    let audit = build_sync_audit_document(&current_lock, baseline_lock.as_ref())?;
    let drift_count = audit
        .get("summary")
        .and_then(Value::as_object)
        .and_then(|summary| summary.get("driftCount"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if let Some(path) = args.write_lock.as_ref() {
        if !(args.fail_on_drift && drift_count > 0) {
            fs::write(
                path,
                format!("{}\n", serde_json::to_string_pretty(&current_lock)?),
            )?;
        }
    }
    if args.fail_on_drift && drift_count > 0 {
        return Err(message(format!(
            "Sync audit detected {drift_count} drifted resource(s)."
        )));
    }
    if args.interactive {
        return run_sync_audit_interactive(&audit);
    }
    emit_text_or_json(&audit, &render_sync_audit_text(&audit)?, args.output_format)
}

pub(crate) fn execute_sync_plan(args: &SyncPlanArgs) -> Result<SyncCommandOutput> {
    // Plan is the normalized contract: consume desired + live, stamp trace metadata,
    // and return a stage-1 artifact ready for review without mutating live state.
    let desired = load_json_array_file(&args.desired_file, "Sync desired input")?;
    // Live fetch stays at orchestration level so the planner only sees
    // normalized arrays and never has to own Grafana transport details.
    let live = load_sync_live_array(
        args.fetch_live,
        &args.common,
        args.org_id,
        args.page_size,
        args.live_file.as_ref(),
        "Sync plan requires --live-file unless --fetch-live is used.",
    )?;
    let document = attach_lineage(
        &attach_trace_id(
            &build_sync_plan_document(&desired, &live, args.allow_prune)?,
            args.trace_id.as_deref(),
        )?,
        "plan",
        1,
        None,
    )?;
    let text_lines = render_sync_plan_text(&document)?;
    Ok(sync_command_output(document, text_lines))
}

pub(crate) fn run_sync_review(args: SyncReviewArgs) -> Result<()> {
    // Review is a pure transform step over a stage-1 plan; it can run interactively,
    // then emits a stage-2 reviewed artifact with mandatory trace and audit details.
    let plan = load_json_value(&args.plan_file, "Sync plan input")?;
    let trace_id = require_trace_id(&plan, "Sync plan document")?;
    require_optional_stage(&plan, "Sync plan document", "plan", 1, None)?;
    let reviewed_plan_input = if args.interactive {
        review_tui::run_sync_review_tui(&plan)?
    } else {
        plan
    };
    let document = build_sync_review_document(
        &reviewed_plan_input,
        &args.review_token,
        &trace_id,
        args.reviewed_by.as_deref(),
        args.reviewed_at.as_deref(),
        args.review_note.as_deref(),
    )?;
    emit_text_or_json(
        &document,
        &render_sync_plan_text(&document)?,
        args.output_format,
    )
}

pub(crate) fn run_sync_apply(args: SyncApplyArgs) -> Result<()> {
    // Apply is the branch point for either dry-run intent output (default) or live execution.
    // The input is always converted into a reviewed/apply-intent document before execution,
    // so both code paths stay audit-friendly.
    let discovered = discover_change_staged_inputs(None)?;
    let preview_file = args
        .plan_file
        .clone()
        .or(discovered.reviewed_plan_file)
        .or_else(|| {
            let default_preview = PathBuf::from("./workspace-preview.json");
            default_preview.is_file().then_some(default_preview)
        })
        .ok_or_else(|| {
            message(
                "Workspace apply could not find a preview artifact. Provide --preview-file or keep ./workspace-preview.json or ./sync-plan-reviewed.json in the current directory.",
            )
        })?;
    let preview = load_json_value(&preview_file, "Workspace preview input")?;
    let trace_id = require_trace_id(&preview, "Workspace preview document")?;
    let plan = match require_optional_stage(
        &preview,
        "Workspace preview document",
        "review",
        2,
        Some(&trace_id),
    ) {
        Ok(()) => preview,
        Err(_) => {
            require_optional_stage(&preview, "Workspace preview document", "plan", 1, None)?;
            build_sync_review_document(&preview, DEFAULT_REVIEW_TOKEN, &trace_id, None, None, None)?
        }
    };
    let preflight_summary =
        load_sync_apply_preflight_summary(&trace_id, args.preflight_file.as_ref())?;
    let bundle_preflight_summary =
        load_sync_apply_bundle_preflight_summary(&trace_id, args.bundle_preflight_file.as_ref())?;
    let document = build_sync_apply_document(
        &args,
        &plan,
        preflight_summary,
        bundle_preflight_summary,
        &trace_id,
    )?;
    if args.execute_live {
        // Keep the reviewed plan document intact even when executing
        // live so the emitted payload remains the audit trail.
        let operations = load_apply_intent_operations(&document)?;
        let live_result = execute_live_apply(
            &args.common,
            args.org_id,
            &operations,
            args.allow_folder_delete,
            args.allow_policy_reset,
        )?;
        emit_text_or_json(
            &live_result,
            &[
                "Sync live apply".to_string(),
                format!(
                    "Applied: {}",
                    live_result
                        .get("appliedCount")
                        .and_then(Value::as_i64)
                        .unwrap_or(0)
                ),
            ],
            args.output_format,
        )
    } else {
        emit_text_or_json(
            &document,
            &render_sync_apply_intent_text(&document)?,
            args.output_format,
        )
    }
}

pub(crate) fn execute_sync_summary(args: &SyncSummaryArgs) -> Result<SyncCommandOutput> {
    let desired = load_json_array_file(&args.desired_file, "Sync desired input")?;
    let document = build_sync_summary_document(&desired)?;
    let text_lines = render_sync_summary_text(&document)?;
    Ok(sync_command_output(document, text_lines))
}

pub(crate) fn execute_sync_preflight(args: &SyncPreflightArgs) -> Result<SyncCommandOutput> {
    // Preflight is the cheapest validation seam: combine staged desired state with
    // optional live-derived availability and produce a stage-ready preflight artifact.
    let desired = load_json_array_file(&args.desired_file, "Sync desired input")?;
    let availability = load_sync_merged_availability(
        args.fetch_live,
        &args.common,
        args.org_id,
        args.availability_file.as_ref(),
    )?;
    // Only the orchestration layer decides whether to supplement staged
    // availability with a live Grafana snapshot.
    let document = build_sync_preflight_document(&desired, availability.as_ref())?;
    let text_lines = render_sync_preflight_text(&document)?;
    Ok(sync_command_output(document, text_lines))
}

pub(crate) fn execute_sync_assess_alerts(args: &SyncAssessAlertsArgs) -> Result<SyncCommandOutput> {
    let alerts = load_json_array_file(&args.alerts_file, "Alert sync input")?;
    let document = assess_alert_sync_specs(&alerts)?;
    let text_lines = render_alert_sync_assessment_text(&document)?;
    Ok(sync_command_output(document, text_lines))
}

pub(crate) fn execute_sync_bundle_preflight(
    args: &SyncBundlePreflightArgs,
) -> Result<SyncCommandOutput> {
    let source_bundle = load_json_value(&args.source_bundle, "Workspace package input")?;
    let target_inventory = load_json_value(&args.target_inventory, "Sync target inventory input")?;
    let availability = load_sync_merged_availability(
        args.fetch_live,
        &args.common,
        args.org_id,
        args.availability_file.as_ref(),
    )?;
    // Keep the bundle-preflight contract identical to preflight: live
    // availability is an optional orchestration-time overlay.
    let document = build_sync_bundle_preflight_document(
        &source_bundle,
        &target_inventory,
        availability.as_ref(),
    )?;
    let text_lines = render_sync_bundle_preflight_text(&document)?;
    Ok(sync_command_output(document, text_lines))
}

pub(crate) fn execute_sync_promotion_preflight(
    args: &SyncPromotionPreflightArgs,
) -> Result<SyncCommandOutput> {
    let source_bundle = load_json_value(&args.source_bundle, "Workspace package input")?;
    let target_inventory = load_json_value(&args.target_inventory, "Sync target inventory input")?;
    let mapping = match args.mapping_file.as_ref() {
        Some(path) => Some(load_json_value(path, "Sync promotion mapping input")?),
        None => None,
    };
    let availability = load_sync_merged_availability(
        args.fetch_live,
        &args.common,
        args.org_id,
        args.availability_file.as_ref(),
    )?;
    let document = build_sync_promotion_preflight_document(
        &source_bundle,
        &target_inventory,
        availability.as_ref(),
        mapping.as_ref(),
    )?;
    let text_lines = render_sync_promotion_preflight_text(&document)?;
    Ok(sync_command_output(document, text_lines))
}

pub(crate) fn execute_sync_bundle(args: &SyncBundleArgs) -> Result<SyncCommandOutput> {
    if args.dashboard_export_dir.is_some() && args.dashboard_provisioning_dir.is_some() {
        return Err(message(
            "Sync bundle accepts only one dashboard input: --dashboard-export-dir or --dashboard-provisioning-dir.",
        ));
    }
    let discovered = match args.workspace.as_ref() {
        Some(workspace) => Some(discover_change_staged_inputs(Some(workspace.as_path()))?),
        None => None,
    };
    let dashboard_export_dir = args.dashboard_export_dir.clone().or_else(|| {
        discovered
            .as_ref()
            .and_then(|found| found.dashboard_export_dir.clone())
    });
    let dashboard_provisioning_dir = args.dashboard_provisioning_dir.clone().or_else(|| {
        if dashboard_export_dir.is_some() {
            None
        } else {
            discovered
                .as_ref()
                .and_then(|found| found.dashboard_provisioning_dir.clone())
        }
    });
    let alert_export_dir = args.alert_export_dir.clone().or_else(|| {
        discovered
            .as_ref()
            .and_then(|found| found.alert_export_dir.clone())
    });
    let datasource_export_file = args.datasource_export_file.clone();
    let datasource_provisioning_file = args.datasource_provisioning_file.clone().or_else(|| {
        if datasource_export_file.is_some() {
            None
        } else {
            discovered
                .as_ref()
                .and_then(|found| found.datasource_provisioning_file.clone())
        }
    });

    let bundle_input_selection = SyncBundleInputSelection {
        workspace_root: args.workspace.as_ref().map(|workspace| {
            discovered
                .as_ref()
                .and_then(|found| found.workspace_root.clone())
                .unwrap_or_else(|| workspace.clone())
        }),
        dashboard_export_dir,
        dashboard_provisioning_dir,
        alert_export_dir,
        datasource_export_file,
        datasource_provisioning_file,
        metadata_file: args.metadata_file.clone(),
    };

    if !bundle_input_selection.has_inputs() {
        return Err(message(
            "Workspace package requires at least one export input such as the positional workspace root, --dashboard-export-dir, --dashboard-provisioning-dir, --alert-export-dir, --datasource-export-file, --datasource-provisioning-file, or --metadata-file.",
        ));
    }
    let artifacts = load_sync_bundle_input_artifacts(&bundle_input_selection)?;
    let mut document = build_sync_source_bundle_document(
        &artifacts.dashboards,
        &artifacts.datasources,
        &artifacts.folders,
        &artifacts.alerts,
        Some(&artifacts.alerting),
        Some(&Value::Object(artifacts.metadata)),
    )?;
    let discovery = build_bundle_discovery_document(
        args.workspace.as_ref(),
        bundle_input_selection.dashboard_export_dir.as_ref(),
        bundle_input_selection.dashboard_provisioning_dir.as_ref(),
        bundle_input_selection.alert_export_dir.as_ref(),
        bundle_input_selection.datasource_export_file.as_ref(),
        bundle_input_selection.datasource_provisioning_file.as_ref(),
        args.metadata_file.as_ref(),
    );
    if let Some(discovery) = discovery {
        if let Some(object) = document.as_object_mut() {
            object.insert("discovery".to_string(), discovery);
        }
    }
    let text_lines = render_sync_source_bundle_text(&document)?;
    Ok(sync_command_output(document, text_lines))
}

fn build_bundle_discovery_document(
    workspace_root: Option<&PathBuf>,
    dashboard_export_dir: Option<&PathBuf>,
    dashboard_provisioning_dir: Option<&PathBuf>,
    alert_export_dir: Option<&PathBuf>,
    datasource_export_file: Option<&PathBuf>,
    datasource_provisioning_file: Option<&PathBuf>,
    metadata_file: Option<&PathBuf>,
) -> Option<Value> {
    let mut document = ChangeDiscoveryDocument::new(workspace_root.cloned());
    if let Some(path) = dashboard_export_dir {
        document.insert(DiscoveryInputKind::DashboardExportDir, path.clone());
    }
    if let Some(path) = dashboard_provisioning_dir {
        document.insert(DiscoveryInputKind::DashboardProvisioningDir, path.clone());
    }
    if let Some(path) = alert_export_dir {
        document.insert(DiscoveryInputKind::AlertExportDir, path.clone());
    }
    if let Some(path) = datasource_export_file {
        document.insert(DiscoveryInputKind::DatasourceExportFile, path.clone());
    }
    if let Some(path) = datasource_provisioning_file {
        document.insert(DiscoveryInputKind::DatasourceProvisioningFile, path.clone());
    }
    if let Some(path) = metadata_file {
        document.insert(DiscoveryInputKind::MetadataFile, path.clone());
    }
    (!document.is_empty()).then(|| document.to_value())
}
