use serde_json::Value;

use crate::common::{render_json_value, Result};

use super::cli_args::SyncOutputFormat;

/// Reusable sync execution output for JSON/text consumers such as the web workbench.
#[derive(Debug, Clone, PartialEq)]
pub struct SyncCommandOutput {
    pub document: Value,
    pub text_lines: Vec<String>,
}

pub(crate) fn emit_text_or_json(
    document: &Value,
    lines: &[String],
    output: SyncOutputFormat,
) -> Result<()> {
    match output {
        SyncOutputFormat::Json => print!("{}", render_json_value(document)?),
        SyncOutputFormat::Text => {
            for line in lines {
                println!("{line}");
            }
        }
    }
    Ok(())
}

pub(crate) fn sync_command_output(document: Value, text_lines: Vec<String>) -> SyncCommandOutput {
    SyncCommandOutput {
        document,
        text_lines,
    }
}

pub(crate) fn render_and_emit_sync_command_output(
    output: SyncCommandOutput,
    format: SyncOutputFormat,
) -> Result<()> {
    emit_text_or_json(&output.document, &output.text_lines, format)
}
