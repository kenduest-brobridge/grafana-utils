//! Offline migration path for converting raw dashboard JSON into prompt-lane artifacts.

use crate::common::{message, Result};

use super::raw_to_prompt_output::{
    build_log_writer, build_summary, print_summary, write_log_event, write_prompt_lane_metadata,
    RawToPromptLogEvent,
};
use super::raw_to_prompt_plan::{build_raw_to_prompt_plan, load_raw_to_prompt_metadata};
use super::raw_to_prompt_resolution::{
    convert_raw_dashboard_file, load_datasource_mapping, load_live_datasource_inventory,
};
use super::raw_to_prompt_types::{
    RawToPromptItemSummary, RawToPromptResolutionKind, RawToPromptStatus,
};
use super::RawToPromptArgs;

pub(crate) fn run_raw_to_prompt(args: &RawToPromptArgs) -> Result<()> {
    let mapping = load_datasource_mapping(args.datasource_map.as_deref())?;
    let plan = build_raw_to_prompt_plan(args)?;
    let metadata = load_raw_to_prompt_metadata(plan.metadata_source_dir.as_deref())?;
    let staged_inventory = if let Some((metadata_dir, metadata)) = metadata.as_ref() {
        super::load_datasource_inventory(metadata_dir, metadata.as_ref())?
    } else {
        Vec::new()
    };
    let mut inventory = load_live_datasource_inventory(args)?;
    inventory.extend(staged_inventory);

    let mut log_writer = build_log_writer(args)?;
    let mut items = Vec::new();

    for (index, item) in plan.items.iter().enumerate() {
        if args.verbose {
            println!(
                "Converting prompt {:>3}/{:<3} input={} output={}",
                index + 1,
                plan.items.len(),
                item.input_path.display(),
                item.output_path.display()
            );
        } else if args.progress {
            println!(
                "Converting dashboard {}/{}: {}",
                index + 1,
                plan.items.len(),
                item.input_path.display()
            );
        }

        let result = convert_raw_dashboard_file(
            &item.input_path,
            &inventory,
            mapping.as_ref(),
            args.resolution,
            Some(args),
        );

        match result {
            Ok(outcome) => {
                if !args.dry_run {
                    super::write_dashboard(
                        &outcome.prompt_document,
                        &item.output_path,
                        args.overwrite,
                    )?;
                }
                write_log_event(
                    log_writer.as_mut(),
                    args.log_format,
                    RawToPromptLogEvent {
                        status: "ok",
                        input_path: &item.input_path,
                        output_path: Some(&item.output_path),
                        resolution: outcome.resolution_string(),
                        datasource_slots: outcome.datasource_slots,
                        warnings: &outcome.warnings,
                        error: None,
                    },
                )?;
                if args.verbose {
                    println!(
                        "Converted prompt  mode={} slots={} output={}",
                        outcome.resolution_string(),
                        outcome.datasource_slots,
                        item.output_path.display()
                    );
                }
                items.push(RawToPromptItemSummary {
                    input_file: item.input_path.display().to_string(),
                    output_file: Some(item.output_path.display().to_string()),
                    status: RawToPromptStatus::Ok,
                    resolution: outcome.resolution,
                    datasource_slots: outcome.datasource_slots,
                    warnings: outcome.warnings,
                    error: None,
                });
            }
            Err(error) => {
                let error_text = error.to_string();
                write_log_event(
                    log_writer.as_mut(),
                    args.log_format,
                    RawToPromptLogEvent {
                        status: "fail",
                        input_path: &item.input_path,
                        output_path: Some(&item.output_path),
                        resolution: "failed",
                        datasource_slots: 0,
                        warnings: &[],
                        error: Some(&error_text),
                    },
                )?;
                if args.verbose {
                    println!(
                        "Failed prompt     reason={} input={}",
                        error_text,
                        item.input_path.display()
                    );
                }
                items.push(RawToPromptItemSummary {
                    input_file: item.input_path.display().to_string(),
                    output_file: Some(item.output_path.display().to_string()),
                    status: RawToPromptStatus::Failed,
                    resolution: RawToPromptResolutionKind::Failed,
                    datasource_slots: 0,
                    warnings: Vec::new(),
                    error: Some(error_text),
                });
            }
        }
    }

    if !args.dry_run {
        write_prompt_lane_metadata(
            plan.output_root.as_deref(),
            &plan,
            &items,
            metadata.as_ref(),
        )?;
    }

    let summary = build_summary(&plan, &items, args.log_file.as_deref());
    print_summary(&summary, args.output_format, args.no_header)?;

    if summary.failed > 0 {
        return Err(message(format!(
            "dashboard raw-to-prompt completed with {} failure(s).",
            summary.failed
        )));
    }
    Ok(())
}
