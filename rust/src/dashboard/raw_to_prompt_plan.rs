//! Planning helpers for raw-to-prompt conversions.

use std::path::{Path, PathBuf};

use crate::common::{message, Result};

use super::files::resolve_dashboard_export_root;
use super::raw_to_prompt_types::{RawToPromptPlan, RawToPromptPlanItem};
use super::{
    discover_dashboard_files, ExportMetadata, RawToPromptArgs, PROMPT_EXPORT_SUBDIR,
    RAW_EXPORT_SUBDIR,
};

pub(crate) fn build_raw_to_prompt_plan(args: &RawToPromptArgs) -> Result<RawToPromptPlan> {
    if args.output_file.is_some() && args.input_file.len() != 1 {
        return Err(message(
            "--output-file only supports a single --input-file source.",
        ));
    }
    if args.input_dir.is_some() && !args.input_file.is_empty() {
        return Err(message(
            "--input-file and --input-dir cannot be used together.",
        ));
    }
    if let Some(input_dir) = args.input_dir.as_ref() {
        return build_dir_plan(input_dir, args);
    }
    build_file_plan(args)
}

fn build_file_plan(args: &RawToPromptArgs) -> Result<RawToPromptPlan> {
    let mut items = Vec::new();
    let output_dir = args.output_dir.clone();
    for input_path in &args.input_file {
        let file_name = input_path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| message(format!("Invalid input file path: {}", input_path.display())))?;
        let output_path = if let Some(output_file) = args.output_file.as_ref() {
            output_file.clone()
        } else if let Some(output_dir) = output_dir.as_ref() {
            output_dir.join(file_name)
        } else {
            sibling_prompt_path(input_path)
        };
        items.push(RawToPromptPlanItem {
            input_path: input_path.clone(),
            output_path,
        });
    }
    Ok(RawToPromptPlan {
        mode: if items.len() == 1 {
            "single-file".to_string()
        } else {
            "multi-file".to_string()
        },
        output_root: output_dir,
        items,
        metadata_source_dir: None,
    })
}

fn build_dir_plan(input_dir: &Path, args: &RawToPromptArgs) -> Result<RawToPromptPlan> {
    let input_dir = input_dir.to_path_buf();
    if !input_dir.is_dir() {
        return Err(message(format!(
            "Input directory does not exist: {}",
            input_dir.display()
        )));
    }

    let export_root = resolve_dashboard_export_root(&input_dir)?;
    let (dashboard_dir, output_root, metadata_source_dir, mode) = if input_dir
        .join(RAW_EXPORT_SUBDIR)
        .is_dir()
    {
        let output_root = args
            .output_dir
            .clone()
            .unwrap_or_else(|| input_dir.join(PROMPT_EXPORT_SUBDIR));
        (
            input_dir.join(RAW_EXPORT_SUBDIR),
            output_root,
            Some(input_dir.join(RAW_EXPORT_SUBDIR)),
            "export-root".to_string(),
        )
    } else if export_root.is_some()
        || input_dir.file_name().and_then(|value| value.to_str()) == Some(RAW_EXPORT_SUBDIR)
    {
        let output_root = args.output_dir.clone().unwrap_or_else(|| {
            input_dir
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(PROMPT_EXPORT_SUBDIR)
        });
        (
            input_dir.clone(),
            output_root,
            Some(input_dir.clone()),
            "raw-dir".to_string(),
        )
    } else {
        let output_root = args.output_dir.clone().ok_or_else(|| {
            message(
                "Plain directory input requires --output-dir so raw-to-prompt does not mix generated files into the source tree.",
            )
        })?;
        (
            input_dir.clone(),
            output_root,
            None,
            "directory".to_string(),
        )
    };

    let files = discover_dashboard_files(&dashboard_dir)?;
    let mut items = Vec::new();
    for input_path in files {
        let relative_path = input_path
            .strip_prefix(&dashboard_dir)
            .unwrap_or(&input_path)
            .to_path_buf();
        items.push(RawToPromptPlanItem {
            output_path: output_root.join(&relative_path),
            input_path,
        });
    }

    Ok(RawToPromptPlan {
        mode,
        output_root: Some(output_root),
        items,
        metadata_source_dir,
    })
}

fn sibling_prompt_path(input_path: &Path) -> PathBuf {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("dashboard");
    input_path.with_file_name(format!("{stem}.prompt.json"))
}

pub(crate) fn load_raw_to_prompt_metadata(
    metadata_source_dir: Option<&Path>,
) -> Result<Option<(PathBuf, Option<ExportMetadata>)>> {
    let Some(metadata_source_dir) = metadata_source_dir else {
        return Ok(None);
    };
    let metadata = super::load_export_metadata(metadata_source_dir, None)?;
    Ok(Some((metadata_source_dir.to_path_buf(), metadata)))
}
