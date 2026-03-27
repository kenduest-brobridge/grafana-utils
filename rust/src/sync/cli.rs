use super::*;
use crate::sync::live::load_apply_intent_operations;

fn emit_text_or_json(document: &Value, lines: Vec<String>, output: SyncOutputFormat) -> Result<()> {
    match output {
        SyncOutputFormat::Json => println!("{}", serde_json::to_string_pretty(document)?),
        SyncOutputFormat::Text => {
            for line in lines {
                println!("{line}");
            }
        }
    }
    Ok(())
}

fn load_sync_live_array(
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
            let preflight = load_json_value(path, "Sync preflight input")?;
            require_matching_optional_trace_id(&preflight, "Sync preflight document", trace_id)?;
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
            let bundle_preflight = load_json_value(path, "Sync bundle preflight input")?;
            require_matching_optional_trace_id(
                &bundle_preflight,
                "Sync bundle preflight document",
                trace_id,
            )?;
            validate_apply_bundle_preflight(&bundle_preflight).map(Some)
        }
    }
}

fn run_sync_audit(args: SyncAuditArgs) -> Result<()> {
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
    emit_text_or_json(&audit, render_sync_audit_text(&audit)?, args.output)
}

fn run_sync_bundle(args: SyncBundleArgs) -> Result<()> {
    if args.dashboard_export_dir.is_none()
        && args.alert_export_dir.is_none()
        && args.datasource_export_file.is_none()
        && args.metadata_file.is_none()
    {
        return Err(message(
            "Sync bundle requires at least one export input such as --dashboard-export-dir, --alert-export-dir, --datasource-export-file, or --metadata-file.",
        ));
    }
    let mut dashboards = Vec::new();
    let mut datasources = Vec::new();
    let mut folders = Vec::new();
    let mut metadata = Map::new();
    if let Some(export_dir) = args.dashboard_export_dir.as_ref() {
        let (dashboard_items, dashboard_datasources, folder_items, dashboard_metadata) =
            load_dashboard_bundle_sections(export_dir)?;
        dashboards = dashboard_items;
        datasources.extend(dashboard_datasources);
        folders = folder_items;
        metadata.extend(dashboard_metadata);
    }
    if let Some(datasource_export_file) = args.datasource_export_file.as_ref() {
        datasources = load_json_array_file(datasource_export_file, "Datasource export inventory")?
            .into_iter()
            .map(|item| normalize_datasource_bundle_item(&item))
            .collect::<Result<Vec<_>>>()?;
        metadata.insert(
            "datasourceExportFile".to_string(),
            Value::String(datasource_export_file.display().to_string()),
        );
    }
    let alerting = match args.alert_export_dir.as_ref() {
        Some(export_dir) => {
            metadata.insert(
                "alertExportDir".to_string(),
                Value::String(export_dir.display().to_string()),
            );
            load_alerting_bundle_section(export_dir)?
        }
        None => Value::Object(Map::new()),
    };
    let alerts = build_alert_sync_specs(&alerting)?;
    if let Some(extra_metadata) =
        load_optional_json_object_file(args.metadata_file.as_ref(), "Sync bundle metadata input")?
    {
        if let Some(object) = extra_metadata.as_object() {
            metadata.extend(object.clone());
        }
    }
    let document = build_sync_source_bundle_document(
        &dashboards,
        &datasources,
        &folders,
        &alerts,
        Some(&alerting),
        Some(&Value::Object(metadata)),
    )?;
    if let Some(output_file) = args.output_file.as_ref() {
        fs::write(
            output_file,
            format!("{}\n", serde_json::to_string_pretty(&document)?),
        )?;
    }
    emit_text_or_json(
        &document,
        render_sync_source_bundle_text(&document)?,
        args.output,
    )
}

fn run_sync_plan(args: SyncPlanArgs) -> Result<()> {
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
    emit_text_or_json(&document, render_sync_plan_text(&document)?, args.output)
}

fn run_sync_review(args: SyncReviewArgs) -> Result<()> {
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
    emit_text_or_json(&document, render_sync_plan_text(&document)?, args.output)
}

fn run_sync_apply(args: SyncApplyArgs) -> Result<()> {
    let plan = load_json_value(&args.plan_file, "Sync plan input")?;
    let trace_id = require_trace_id(&plan, "Sync plan document")?;
    require_optional_stage(&plan, "Sync plan document", "review", 2, Some(&trace_id))?;
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
            vec![
                "Sync live apply".to_string(),
                format!(
                    "Applied: {}",
                    live_result
                        .get("appliedCount")
                        .and_then(Value::as_i64)
                        .unwrap_or(0)
                ),
            ],
            args.output,
        )
    } else {
        emit_text_or_json(
            &document,
            render_sync_apply_intent_text(&document)?,
            args.output,
        )
    }
}

fn run_sync_summary(args: SyncSummaryArgs) -> Result<()> {
    let desired = load_json_array_file(&args.desired_file, "Sync desired input")?;
    let document = build_sync_summary_document(&desired)?;
    emit_text_or_json(&document, render_sync_summary_text(&document)?, args.output)
}

fn run_sync_preflight(args: SyncPreflightArgs) -> Result<()> {
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
    emit_text_or_json(
        &document,
        render_sync_preflight_text(&document)?,
        args.output,
    )
}

fn run_sync_assess_alerts(args: SyncAssessAlertsArgs) -> Result<()> {
    let alerts = load_json_array_file(&args.alerts_file, "Alert sync input")?;
    let document = assess_alert_sync_specs(&alerts)?;
    emit_text_or_json(
        &document,
        render_alert_sync_assessment_text(&document)?,
        args.output,
    )
}

fn run_sync_bundle_preflight(args: SyncBundlePreflightArgs) -> Result<()> {
    let source_bundle = load_json_value(&args.source_bundle, "Sync source bundle input")?;
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
    emit_text_or_json(
        &document,
        render_sync_bundle_preflight_text(&document)?,
        args.output,
    )
}

fn run_sync_promotion_preflight(args: SyncPromotionPreflightArgs) -> Result<()> {
    let source_bundle = load_json_value(&args.source_bundle, "Sync source bundle input")?;
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
    emit_text_or_json(
        &document,
        render_sync_promotion_preflight_text(&document)?,
        args.output,
    )
}

pub fn run_sync_cli(command: SyncGroupCommand) -> Result<()> {
    match command {
        SyncGroupCommand::Plan(args) => run_sync_plan(args),
        SyncGroupCommand::Review(args) => run_sync_review(args),
        SyncGroupCommand::Apply(args) => run_sync_apply(args),
        SyncGroupCommand::Audit(args) => run_sync_audit(args),
        SyncGroupCommand::Summary(args) => run_sync_summary(args),
        SyncGroupCommand::Preflight(args) => run_sync_preflight(args),
        SyncGroupCommand::AssessAlerts(args) => run_sync_assess_alerts(args),
        SyncGroupCommand::BundlePreflight(args) => run_sync_bundle_preflight(args),
        SyncGroupCommand::PromotionPreflight(args) => run_sync_promotion_preflight(args),
        SyncGroupCommand::Bundle(args) => run_sync_bundle(args),
    }
}
