use crate::common::{emit_plain_output, message, Result};

use super::cli::{
    execute_sync_assess_alerts, execute_sync_bundle, execute_sync_bundle_preflight,
    execute_sync_plan, execute_sync_preflight, execute_sync_promotion_preflight,
    execute_sync_summary, run_sync_apply, run_sync_audit, run_sync_review,
};
use super::cli_args::{SyncAdvancedCliArgs, SyncAdvancedCommand, SyncGroupCommand};
use super::output::emit_text_or_json;
use super::{run_sync_check, run_sync_inspect, run_sync_preview};

/// Execute reusable sync commands into structured output.
/// This path is used by callers that need deterministic documents for tests and
/// downstream programmatic checks, so output formatting is intentionally deferred.
pub fn execute_sync_command(
    command: &SyncGroupCommand,
) -> Result<super::output::SyncCommandOutput> {
    match command {
        SyncGroupCommand::Preview(_) => Err(message(
            "Task-first preview is not exposed through reusable execution output.",
        )),
        SyncGroupCommand::Inspect(_) | SyncGroupCommand::Check(_) => Err(message(
            "Task-first inspect/check are not exposed through reusable execution output.",
        )),
        SyncGroupCommand::Advanced(SyncAdvancedCliArgs { command }) => match command {
            SyncAdvancedCommand::Plan(args) => execute_sync_plan(args),
            SyncAdvancedCommand::Summary(args) => execute_sync_summary(args),
            SyncAdvancedCommand::Preflight(args) => execute_sync_preflight(args),
            SyncAdvancedCommand::AssessAlerts(args) => execute_sync_assess_alerts(args),
            SyncAdvancedCommand::BundlePreflight(args) => execute_sync_bundle_preflight(args),
            SyncAdvancedCommand::PromotionPreflight(args) => execute_sync_promotion_preflight(args),
            SyncAdvancedCommand::Review(_) => Err(message(
                "Sync review is not exposed through reusable execution output.",
            )),
            SyncAdvancedCommand::Audit(_) => Err(message(
                "Sync audit is not exposed through reusable execution output.",
            )),
        },
        SyncGroupCommand::Bundle(args) => execute_sync_bundle(args),
        SyncGroupCommand::Apply(args) if args.execute_live => Err(message(
            "Sync live apply is not exposed through reusable execution output.",
        )),
        SyncGroupCommand::Apply(_) => Err(message(
            "Sync apply is not exposed through reusable execution output.",
        )),
    }
}

pub fn run_sync_cli(command: SyncGroupCommand) -> Result<()> {
    // Interactive/run-time path: each variant chooses either staged plan/render
    // outputs or live mutation behavior, but all exits as a single CLI result.
    match command {
        SyncGroupCommand::Inspect(args) => run_sync_inspect(args),
        SyncGroupCommand::Check(args) => run_sync_check(args),
        SyncGroupCommand::Preview(args) => run_sync_preview(args),
        SyncGroupCommand::Apply(args) => run_sync_apply(args),
        SyncGroupCommand::Advanced(SyncAdvancedCliArgs { command }) => match command {
            SyncAdvancedCommand::Summary(args) => {
                let output = execute_sync_summary(&args)?;
                emit_text_or_json(&output.document, &output.text_lines, args.output_format)
            }
            SyncAdvancedCommand::Plan(args) => {
                let output = execute_sync_plan(&args)?;
                emit_text_or_json(&output.document, &output.text_lines, args.output_format)?;
                Ok(())
            }
            SyncAdvancedCommand::Review(args) => run_sync_review(args),
            SyncAdvancedCommand::Preflight(args) => {
                let output = execute_sync_preflight(&args)?;
                emit_text_or_json(&output.document, &output.text_lines, args.output_format)
            }
            SyncAdvancedCommand::Audit(args) => run_sync_audit(args),
            SyncAdvancedCommand::AssessAlerts(args) => {
                let output = execute_sync_assess_alerts(&args)?;
                emit_text_or_json(&output.document, &output.text_lines, args.output_format)
            }
            SyncAdvancedCommand::BundlePreflight(args) => {
                let output = execute_sync_bundle_preflight(&args)?;
                emit_text_or_json(&output.document, &output.text_lines, args.output_format)
            }
            SyncAdvancedCommand::PromotionPreflight(args) => {
                let output = execute_sync_promotion_preflight(&args)?;
                emit_text_or_json(&output.document, &output.text_lines, args.output_format)
            }
        },
        SyncGroupCommand::Bundle(args) => {
            let output = execute_sync_bundle(&args)?;
            if let Some(output_file) = args.output_file.as_ref() {
                emit_plain_output(
                    &serde_json::to_string_pretty(&output.document)?,
                    Some(output_file.as_path()),
                    false,
                )?;
            }
            if args.output_file.is_none() || args.also_stdout {
                emit_text_or_json(&output.document, &output.text_lines, args.output_format)?;
            }
            Ok(())
        }
    }
}
