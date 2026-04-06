//! Output, logging, and summary helpers for raw-to-prompt conversions.

use serde_json::json;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::common::{render_json_value, Result};
use crate::tabular_output::render_yaml;

use super::inspect_render::render_simple_table;
use super::raw_to_prompt_types::{
    RawToPromptItemSummary, RawToPromptPlan, RawToPromptResolutionKind, RawToPromptStatus,
    RawToPromptSummary, RAW_TO_PROMPT_KIND,
};
use super::{
    build_export_metadata, build_variant_index, write_json_document, DashboardIndexItem,
    ExportMetadata, RawToPromptArgs, RawToPromptLogFormat, RawToPromptOutputFormat,
    EXPORT_METADATA_FILENAME, PROMPT_EXPORT_SUBDIR,
};

#[derive(Debug, Clone)]
pub(crate) struct RawToPromptLogEvent<'a> {
    pub status: &'a str,
    pub input_path: &'a Path,
    pub output_path: Option<&'a Path>,
    pub resolution: &'a str,
    pub datasource_slots: usize,
    pub warnings: &'a [String],
    pub error: Option<&'a str>,
}

pub(crate) fn build_summary(
    plan: &RawToPromptPlan,
    items: &[RawToPromptItemSummary],
    log_file: Option<&Path>,
) -> RawToPromptSummary {
    let mut exact = 0usize;
    let mut inferred = 0usize;
    let mut unresolved = 0usize;
    let mut converted = 0usize;
    let mut failed = 0usize;
    for item in items {
        match item.status {
            RawToPromptStatus::Ok => converted += 1,
            RawToPromptStatus::Failed => failed += 1,
        }
        match item.resolution {
            RawToPromptResolutionKind::Exact => exact += 1,
            RawToPromptResolutionKind::Inferred => inferred += 1,
            RawToPromptResolutionKind::Failed => unresolved += 1,
        }
    }
    RawToPromptSummary {
        kind: RAW_TO_PROMPT_KIND.to_string(),
        schema_version: 1,
        mode: plan.mode.clone(),
        scanned: items.len(),
        converted,
        failed,
        exact,
        inferred,
        unresolved,
        output_root: plan
            .output_root
            .as_ref()
            .map(|path| path.display().to_string()),
        log_file: log_file.map(|path| path.display().to_string()),
        items: items.to_vec(),
    }
}

pub(crate) fn print_summary(
    summary: &RawToPromptSummary,
    output_format: RawToPromptOutputFormat,
    no_header: bool,
) -> Result<()> {
    match output_format {
        RawToPromptOutputFormat::Json => {
            print!("{}", render_json_value(summary)?);
        }
        RawToPromptOutputFormat::Yaml => {
            print!("{}", render_yaml(summary)?);
        }
        RawToPromptOutputFormat::Table => {
            let rows = vec![vec![
                if summary.failed == 0 {
                    "ok".to_string()
                } else {
                    "partial".to_string()
                },
                summary.scanned.to_string(),
                summary.converted.to_string(),
                summary.failed.to_string(),
                summary.exact.to_string(),
                summary.inferred.to_string(),
                summary.unresolved.to_string(),
                summary
                    .output_root
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            ]];
            for line in render_simple_table(
                &[
                    "STATUS",
                    "SCANNED",
                    "CONVERTED",
                    "FAILED",
                    "EXACT",
                    "INFERRED",
                    "UNRESOLVED",
                    "OUTPUT",
                ],
                &rows,
                !no_header,
            ) {
                println!("{line}");
            }
        }
        RawToPromptOutputFormat::Text => {
            println!(
                "{}",
                if summary.failed == 0 {
                    "raw-to-prompt completed"
                } else {
                    "raw-to-prompt completed with failures"
                }
            );
            println!("  scanned: {}", summary.scanned);
            println!("  converted: {}", summary.converted);
            println!("  failed: {}", summary.failed);
            println!("  exact: {}", summary.exact);
            println!("  inferred: {}", summary.inferred);
            println!("  unresolved: {}", summary.unresolved);
            if let Some(output_root) = &summary.output_root {
                println!("  output: {output_root}");
            }
            if let Some(log_file) = &summary.log_file {
                println!("  log: {log_file}");
            }
        }
    }
    Ok(())
}

pub(crate) fn build_log_writer(args: &RawToPromptArgs) -> Result<Option<File>> {
    let Some(log_file) = args.log_file.as_ref() else {
        return Ok(None);
    };
    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(Some(File::create(log_file)?))
}

pub(crate) fn write_log_event(
    log_writer: Option<&mut File>,
    log_format: RawToPromptLogFormat,
    event: RawToPromptLogEvent<'_>,
) -> Result<()> {
    let Some(writer) = log_writer else {
        return Ok(());
    };
    match log_format {
        RawToPromptLogFormat::Text => {
            let mut line = format!(
                "{} input={} resolution={} slots={}",
                event.status.to_uppercase(),
                event.input_path.display(),
                event.resolution,
                event.datasource_slots
            );
            if let Some(output_path) = event.output_path {
                line.push_str(&format!(" output={}", output_path.display()));
            }
            if !event.warnings.is_empty() {
                line.push_str(&format!(" warnings={}", event.warnings.join("|")));
            }
            if let Some(error) = event.error {
                line.push_str(&format!(" error={error}"));
            }
            writeln!(writer, "{line}")?;
        }
        RawToPromptLogFormat::Json => {
            writeln!(
                writer,
                "{}",
                serde_json::to_string(&json!({
                    "status": event.status,
                    "inputFile": event.input_path.display().to_string(),
                    "outputFile": event.output_path.map(|path| path.display().to_string()),
                    "resolution": event.resolution,
                    "datasourceSlots": event.datasource_slots,
                    "warnings": event.warnings,
                    "error": event.error,
                }))?
            )?;
        }
    }
    Ok(())
}

pub(crate) fn write_prompt_lane_metadata(
    output_root: Option<&Path>,
    plan: &RawToPromptPlan,
    items: &[RawToPromptItemSummary],
    metadata: Option<&(PathBuf, Option<ExportMetadata>)>,
) -> Result<()> {
    let Some(output_root) = output_root else {
        return Ok(());
    };
    if metadata.is_none() {
        return Ok(());
    }
    let source_metadata = metadata.and_then(|(_, metadata)| metadata.as_ref());
    let source_org = source_metadata.and_then(|item| item.org.as_deref());
    let source_org_id = source_metadata.and_then(|item| item.org_id.as_deref());
    let source_path = metadata.map(|(path, _)| path.as_path());
    let mut index_items = Vec::new();
    for item in items {
        if item.status != RawToPromptStatus::Ok {
            continue;
        }
        let input_path = Path::new(&item.input_file);
        let output_path = Path::new(item.output_file.as_deref().unwrap_or(""));
        let uid = output_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("dashboard")
            .trim_end_matches(".prompt")
            .to_string();
        index_items.push(DashboardIndexItem {
            uid,
            title: output_path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("dashboard")
                .to_string(),
            folder_title: input_path
                .parent()
                .and_then(|value| value.file_name())
                .and_then(|value| value.to_str())
                .unwrap_or("General")
                .to_string(),
            org: source_org.unwrap_or("Main Org.").to_string(),
            org_id: source_org_id.unwrap_or("1").to_string(),
            raw_path: None,
            prompt_path: Some(output_path.display().to_string()),
            provisioning_path: None,
        });
    }
    write_json_document(
        &build_variant_index(
            &index_items,
            |item| item.prompt_path.as_deref(),
            "grafana-web-import-with-datasource-inputs",
        ),
        &output_root.join("index.json"),
    )?;
    write_json_document(
        &build_export_metadata(
            PROMPT_EXPORT_SUBDIR,
            index_items.len(),
            Some("grafana-web-import-with-datasource-inputs"),
            None,
            None,
            None,
            source_org,
            source_org_id,
            None,
            "local",
            None,
            source_path,
            None,
            output_root,
            &output_root.join(EXPORT_METADATA_FILENAME),
        ),
        &output_root.join(EXPORT_METADATA_FILENAME),
    )?;
    let _ = plan;
    Ok(())
}
