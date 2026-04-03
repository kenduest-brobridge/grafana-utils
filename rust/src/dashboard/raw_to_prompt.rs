//! Offline migration path for converting raw dashboard JSON into prompt-lane artifacts.

use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::common::{message, render_json_value, sanitize_path_component, Result};
use crate::dashboard::inspect_query::{
    resolve_query_analyzer_family_from_datasource_type,
    resolve_query_analyzer_family_from_query_signature,
};
use crate::tabular_output::render_yaml;

use super::files::resolve_dashboard_export_root;
use super::inspect_render::render_simple_table;
use super::list::fetch_current_org_with_request;
use super::{
    build_datasource_catalog, build_datasource_inventory_record, build_export_metadata,
    build_external_export_document, build_http_client, build_http_client_for_org,
    build_variant_index, discover_dashboard_files, list_datasources_with_request,
    load_datasource_inventory, load_json_file, write_dashboard, write_json_document, CommonCliArgs,
    DashboardIndexItem, DatasourceInventoryItem, ExportMetadata, RawToPromptArgs,
    RawToPromptLogFormat, RawToPromptOutputFormat, RawToPromptResolution, DEFAULT_TIMEOUT,
    DEFAULT_URL, EXPORT_METADATA_FILENAME, PROMPT_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR,
};

const RAW_TO_PROMPT_KIND: &str = "grafana-utils-dashboard-raw-to-prompt-summary";
const MAPPING_KIND: &str = "grafana-utils-dashboard-datasource-map";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum RawToPromptStatus {
    Ok,
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum RawToPromptResolutionKind {
    Exact,
    Inferred,
    Failed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct RawToPromptItemSummary {
    input_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_file: Option<String>,
    status: RawToPromptStatus,
    resolution: RawToPromptResolutionKind,
    datasource_slots: usize,
    warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct RawToPromptSummary {
    kind: String,
    #[serde(rename = "schemaVersion")]
    schema_version: i64,
    mode: String,
    scanned: usize,
    converted: usize,
    failed: usize,
    exact: usize,
    inferred: usize,
    unresolved: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_file: Option<String>,
    items: Vec<RawToPromptItemSummary>,
}

#[derive(Debug, Clone)]
struct RawToPromptPlanItem {
    input_path: PathBuf,
    output_path: PathBuf,
}

#[derive(Debug, Clone)]
struct RawToPromptPlan {
    mode: String,
    output_root: Option<PathBuf>,
    items: Vec<RawToPromptPlanItem>,
    metadata_source_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RawToPromptStats {
    exact: usize,
    inferred: usize,
    unresolved: usize,
}

#[derive(Debug, Clone)]
struct RawToPromptOutcome {
    prompt_document: Value,
    datasource_slots: usize,
    resolution: RawToPromptResolutionKind,
    warnings: Vec<String>,
}

struct RawToPromptLogEvent<'a> {
    status: &'a str,
    input_path: &'a Path,
    output_path: Option<&'a Path>,
    resolution: &'a str,
    datasource_slots: usize,
    warnings: &'a [String],
    error: Option<&'a str>,
}

#[derive(Debug, Clone, Default)]
struct DashboardScanContext {
    ref_families: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedDatasourceReplacement {
    key: String,
    uid: String,
    name: String,
    datasource_type: String,
    exact: bool,
    warning: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq, serde::Deserialize)]
struct DatasourceMapDocument {
    #[serde(default)]
    kind: String,
    #[serde(rename = "schemaVersion", default)]
    schema_version: Option<i64>,
    #[serde(default)]
    datasources: Vec<DatasourceMapEntry>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq, serde::Deserialize)]
struct DatasourceMapEntry {
    #[serde(default)]
    r#match: DatasourceMatchRule,
    replace: DatasourceReplaceRule,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq, serde::Deserialize)]
struct DatasourceMatchRule {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    uid: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    reference: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq, serde::Deserialize)]
struct DatasourceReplaceRule {
    #[serde(default)]
    uid: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(rename = "type")]
    datasource_type: String,
}

pub(crate) fn run_raw_to_prompt(args: &RawToPromptArgs) -> Result<()> {
    let mapping = load_datasource_mapping(args.datasource_map.as_deref())?;
    let plan = build_raw_to_prompt_plan(args)?;
    let metadata = load_raw_to_prompt_metadata(plan.metadata_source_dir.as_deref())?;
    let staged_inventory = if let Some((metadata_dir, metadata)) = metadata.as_ref() {
        load_datasource_inventory(metadata_dir, metadata.as_ref())?
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
        );

        match result {
            Ok(outcome) => {
                if !args.dry_run {
                    write_dashboard(&outcome.prompt_document, &item.output_path, args.overwrite)?;
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
                    resolution: outcome.resolution.clone(),
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

fn load_live_datasource_inventory(args: &RawToPromptArgs) -> Result<Vec<DatasourceInventoryItem>> {
    if !raw_to_prompt_live_lookup_requested(args) {
        return Ok(Vec::new());
    }
    let common = CommonCliArgs {
        color: args.color,
        profile: args.profile.clone(),
        url: args.url.clone().unwrap_or_else(|| DEFAULT_URL.to_string()),
        api_token: args.api_token.clone(),
        username: args.username.clone(),
        password: args.password.clone(),
        prompt_password: args.prompt_password,
        prompt_token: args.prompt_token,
        timeout: args.timeout.unwrap_or(DEFAULT_TIMEOUT),
        verify_ssl: args.verify_ssl,
    };
    let client = match args.org_id {
        Some(org_id) => build_http_client_for_org(&common, org_id)?,
        None => build_http_client(&common)?,
    };
    let current_org = fetch_current_org_with_request(|method, path, params, payload| {
        client.request_json(method, path, params, payload)
    })?;
    let datasources = list_datasources_with_request(|method, path, params, payload| {
        client.request_json(method, path, params, payload)
    })?;
    Ok(datasources
        .iter()
        .map(|datasource| build_datasource_inventory_record(datasource, &current_org))
        .collect())
}

fn raw_to_prompt_live_lookup_requested(args: &RawToPromptArgs) -> bool {
    args.profile.is_some()
        || args.url.is_some()
        || args.api_token.is_some()
        || args.username.is_some()
        || args.password.is_some()
        || args.prompt_password
        || args.prompt_token
        || args.org_id.is_some()
        || args.timeout.is_some()
        || args.verify_ssl
}

trait ResolutionString {
    fn resolution_string(&self) -> &'static str;
}

impl ResolutionString for RawToPromptOutcome {
    fn resolution_string(&self) -> &'static str {
        match self.resolution {
            RawToPromptResolutionKind::Exact => "exact",
            RawToPromptResolutionKind::Inferred => "inferred",
            RawToPromptResolutionKind::Failed => "failed",
        }
    }
}

fn build_summary(
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

fn print_summary(
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

fn build_log_writer(args: &RawToPromptArgs) -> Result<Option<File>> {
    let Some(log_file) = args.log_file.as_ref() else {
        return Ok(None);
    };
    if let Some(parent) = log_file.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(Some(File::create(log_file)?))
}

fn write_log_event(
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

fn build_raw_to_prompt_plan(args: &RawToPromptArgs) -> Result<RawToPromptPlan> {
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

fn convert_raw_dashboard_file(
    input_path: &Path,
    datasource_inventory: &[DatasourceInventoryItem],
    mapping: Option<&DatasourceMapDocument>,
    resolution: RawToPromptResolution,
) -> Result<RawToPromptOutcome> {
    let payload = load_json_file(input_path)?;
    let mut dashboard = super::build_preserved_web_import_document(&payload)?;
    let placeholder_paths = collect_panel_placeholder_datasource_paths(&dashboard);
    let mut scan = DashboardScanContext::default();
    collect_reference_families(&mut dashboard, &mut scan);
    let mut warnings = Vec::new();
    let mut stats = RawToPromptStats::default();
    rewrite_datasource_refs(
        &mut dashboard,
        datasource_inventory,
        mapping,
        &scan,
        resolution,
        &mut warnings,
        &mut stats,
    )?;
    let datasource_catalog = build_datasource_catalog(&build_synthetic_catalog(&dashboard));
    let mut prompt_document = build_external_export_document(&dashboard, &datasource_catalog)?;
    rewrite_prompt_panel_placeholder_paths(&mut prompt_document, &placeholder_paths);
    let datasource_slots = prompt_document
        .get("__inputs")
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);
    let resolution_kind = if stats.inferred > 0 {
        RawToPromptResolutionKind::Inferred
    } else {
        RawToPromptResolutionKind::Exact
    };
    Ok(RawToPromptOutcome {
        prompt_document,
        datasource_slots,
        resolution: resolution_kind,
        warnings,
    })
}

fn is_placeholder_datasource_reference(reference: &Value) -> bool {
    match reference {
        Value::String(text) => text.starts_with('$'),
        Value::Object(object) => {
            object
                .get("uid")
                .and_then(Value::as_str)
                .is_some_and(|value| value.starts_with('$'))
                || object
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value.starts_with('$'))
        }
        _ => false,
    }
}

fn collect_panel_placeholder_datasource_paths(document: &Value) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    collect_panel_placeholder_datasource_paths_recursive(document, "root", &mut paths);
    paths
}

fn collect_panel_placeholder_datasource_paths_recursive(
    node: &Value,
    current_path: &str,
    paths: &mut BTreeSet<String>,
) {
    match node {
        Value::Object(object) => {
            for (key, value) in object {
                let next_path = format!("{current_path}.{key}");
                if key == "datasource"
                    && current_path.contains(".panels[")
                    && is_placeholder_datasource_reference(value)
                {
                    paths.insert(next_path.clone());
                }
                collect_panel_placeholder_datasource_paths_recursive(value, &next_path, paths);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_panel_placeholder_datasource_paths_recursive(
                    item,
                    &format!("{current_path}[{index}]"),
                    paths,
                );
            }
        }
        _ => {}
    }
}

fn rewrite_prompt_panel_placeholder_paths(document: &mut Value, paths: &BTreeSet<String>) {
    rewrite_prompt_panel_placeholder_paths_recursive(document, "root", paths);
}

fn rewrite_prompt_panel_placeholder_paths_recursive(
    node: &mut Value,
    current_path: &str,
    paths: &BTreeSet<String>,
) {
    match node {
        Value::Object(object) => {
            if let Some(datasource) = object.get_mut("datasource") {
                let datasource_path = format!("{current_path}.datasource");
                if paths.contains(&datasource_path) {
                    *datasource = json!({"uid": "$datasource"});
                }
            }
            for (key, value) in object {
                rewrite_prompt_panel_placeholder_paths_recursive(
                    value,
                    &format!("{current_path}.{key}"),
                    paths,
                );
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                rewrite_prompt_panel_placeholder_paths_recursive(
                    item,
                    &format!("{current_path}[{index}]"),
                    paths,
                );
            }
        }
        _ => {}
    }
}

fn collect_reference_families(document: &mut Value, context: &mut DashboardScanContext) {
    let Some(dashboard) = document.as_object_mut() else {
        return;
    };
    let panel_default = dashboard.get("datasource").cloned();
    if let Some(panels) = dashboard.get_mut("panels").and_then(Value::as_array_mut) {
        for panel in panels {
            collect_panel_reference_families(panel, panel_default.as_ref(), context);
        }
    }
}

fn collect_panel_reference_families(
    panel: &mut Value,
    inherited_panel_datasource: Option<&Value>,
    context: &mut DashboardScanContext,
) {
    let Some(panel_object) = panel.as_object_mut() else {
        return;
    };
    let panel_datasource = panel_object
        .get("datasource")
        .cloned()
        .or_else(|| inherited_panel_datasource.cloned());
    if let Some(targets) = panel_object
        .get_mut("targets")
        .and_then(Value::as_array_mut)
    {
        for target in targets {
            let Some(target_object) = target.as_object_mut() else {
                continue;
            };
            let reference = target_object
                .get("datasource")
                .or(panel_datasource.as_ref());
            let Some(reference) = reference else {
                continue;
            };
            let ref_key = reference_identity_key(reference);
            let Some(ref_key) = ref_key else {
                continue;
            };
            if let Some(ds_type) = datasource_type_from_reference(reference) {
                if let Some(family) = resolve_query_analyzer_family_from_datasource_type(&ds_type) {
                    context
                        .ref_families
                        .entry(ref_key.clone())
                        .or_default()
                        .insert(family.to_string());
                }
            }
            for (field_name, field_value) in target_object.iter() {
                let Some(query_text) = field_value.as_str() else {
                    continue;
                };
                if let Some(family) =
                    resolve_query_analyzer_family_from_query_signature(field_name, query_text)
                {
                    context
                        .ref_families
                        .entry(ref_key.clone())
                        .or_default()
                        .insert(family.to_string());
                }
            }
        }
    }
    if let Some(rows) = panel_object.get_mut("rows").and_then(Value::as_array_mut) {
        for nested in rows {
            collect_panel_reference_families(nested, panel_datasource.as_ref(), context);
        }
    }
    if let Some(nested_panels) = panel_object.get_mut("panels").and_then(Value::as_array_mut) {
        for nested in nested_panels {
            collect_panel_reference_families(nested, panel_datasource.as_ref(), context);
        }
    }
}

fn rewrite_datasource_refs(
    document: &mut Value,
    datasource_inventory: &[DatasourceInventoryItem],
    mapping: Option<&DatasourceMapDocument>,
    scan: &DashboardScanContext,
    resolution: RawToPromptResolution,
    warnings: &mut Vec<String>,
    stats: &mut RawToPromptStats,
) -> Result<()> {
    let Some(dashboard) = document.as_object_mut() else {
        return Ok(());
    };
    rewrite_value_datasource_fields(
        dashboard,
        datasource_inventory,
        mapping,
        scan,
        resolution,
        warnings,
        stats,
    )
}

fn rewrite_value_datasource_fields(
    node: &mut Map<String, Value>,
    datasource_inventory: &[DatasourceInventoryItem],
    mapping: Option<&DatasourceMapDocument>,
    scan: &DashboardScanContext,
    resolution: RawToPromptResolution,
    warnings: &mut Vec<String>,
    stats: &mut RawToPromptStats,
) -> Result<()> {
    if let Some(reference) = node.get_mut("datasource") {
        rewrite_datasource_ref_value(
            reference,
            datasource_inventory,
            mapping,
            scan,
            resolution,
            warnings,
            stats,
        )?;
    }
    for value in node.values_mut() {
        match value {
            Value::Object(object) => rewrite_value_datasource_fields(
                object,
                datasource_inventory,
                mapping,
                scan,
                resolution,
                warnings,
                stats,
            )?,
            Value::Array(items) => {
                for item in items {
                    if let Value::Object(object) = item {
                        rewrite_value_datasource_fields(
                            object,
                            datasource_inventory,
                            mapping,
                            scan,
                            resolution,
                            warnings,
                            stats,
                        )?;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn rewrite_datasource_ref_value(
    reference: &mut Value,
    datasource_inventory: &[DatasourceInventoryItem],
    mapping: Option<&DatasourceMapDocument>,
    scan: &DashboardScanContext,
    resolution: RawToPromptResolution,
    warnings: &mut Vec<String>,
    stats: &mut RawToPromptStats,
) -> Result<()> {
    if super::is_builtin_datasource_ref(reference) || reference.is_null() {
        return Ok(());
    }
    let Some(resolved) =
        resolve_replacement(reference, datasource_inventory, mapping, scan, resolution)?
    else {
        return Ok(());
    };
    if resolved.exact {
        stats.exact += 1;
    } else {
        stats.inferred += 1;
    }
    if let Some(warning) = resolved.warning.clone() {
        warnings.push(warning);
    }
    *reference = json!({
        "uid": resolved.uid,
        "name": resolved.name,
        "type": resolved.datasource_type,
    });
    Ok(())
}

fn resolve_replacement(
    reference: &Value,
    datasource_inventory: &[DatasourceInventoryItem],
    mapping: Option<&DatasourceMapDocument>,
    scan: &DashboardScanContext,
    resolution: RawToPromptResolution,
) -> Result<Option<ResolvedDatasourceReplacement>> {
    if let Some(resolved) = resolve_from_mapping(reference, mapping)? {
        return Ok(Some(resolved));
    }
    if let Some(resolved) = resolve_from_inventory(reference, datasource_inventory) {
        return Ok(Some(resolved));
    }
    if let Some(resolved) = resolve_from_embedded_reference(reference) {
        return Ok(Some(resolved));
    }
    match resolution {
        RawToPromptResolution::Exact => Err(message(format!(
            "Cannot resolve datasource reference exactly: {}",
            reference_identity_key(reference).unwrap_or_else(|| reference.to_string())
        ))),
        RawToPromptResolution::Strict => Err(message(format!(
            "Strict datasource resolution failed for reference: {}",
            reference_identity_key(reference).unwrap_or_else(|| reference.to_string())
        ))),
        RawToPromptResolution::InferFamily => {
            if let Some(resolved) = resolve_from_family_inference(reference, scan) {
                return Ok(Some(resolved));
            }
            Err(message(format!(
                "Cannot infer datasource family for reference: {}",
                reference_identity_key(reference).unwrap_or_else(|| reference.to_string())
            )))
        }
    }
}

fn resolve_from_mapping(
    reference: &Value,
    mapping: Option<&DatasourceMapDocument>,
) -> Result<Option<ResolvedDatasourceReplacement>> {
    let Some(mapping) = mapping else {
        return Ok(None);
    };
    let reference_key = reference_identity_key(reference);
    let object = reference.as_object();
    let id_value = object
        .and_then(|item| item.get("id"))
        .map(|value| match value {
            Value::String(text) => text.clone(),
            _ => value.to_string(),
        });
    let uid_value = object
        .and_then(|item| item.get("uid"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let name_value = object
        .and_then(|item| item.get("name"))
        .and_then(Value::as_str)
        .map(str::to_string);

    for entry in &mapping.datasources {
        let is_match = entry
            .r#match
            .reference
            .as_ref()
            .is_some_and(|value| reference_key.as_ref().is_some_and(|key| key == value))
            || entry.r#match.id.as_ref().is_some_and(|value| {
                id_value
                    .as_ref()
                    .is_some_and(|candidate| candidate == value)
            })
            || entry.r#match.uid.as_ref().is_some_and(|value| {
                uid_value
                    .as_ref()
                    .is_some_and(|candidate| candidate == value)
            })
            || entry.r#match.name.as_ref().is_some_and(|value| {
                name_value
                    .as_ref()
                    .is_some_and(|candidate| candidate == value)
                    || matches!(reference, Value::String(text) if text == value)
            });
        if !is_match {
            continue;
        }
        if entry.replace.datasource_type.trim().is_empty() {
            return Err(message("Datasource mapping replace.type cannot be empty."));
        }
        let uid = entry
            .replace
            .uid
            .clone()
            .unwrap_or_else(|| synthetic_uid_from_reference(reference));
        let name = entry
            .replace
            .name
            .clone()
            .unwrap_or_else(|| synthetic_name(&entry.replace.datasource_type, &uid));
        return Ok(Some(ResolvedDatasourceReplacement {
            key: reference_key.unwrap_or_else(|| uid.clone()),
            uid,
            name,
            datasource_type: entry.replace.datasource_type.clone(),
            exact: true,
            warning: None,
        }));
    }
    Ok(None)
}

fn resolve_from_inventory(
    reference: &Value,
    datasource_inventory: &[DatasourceInventoryItem],
) -> Option<ResolvedDatasourceReplacement> {
    let object = reference.as_object();
    let reference_uid = object
        .and_then(|item| item.get("uid"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let reference_name = object
        .and_then(|item| item.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| match reference {
            Value::String(text) => Some(text.trim().to_string()),
            _ => None,
        });
    let matched = datasource_inventory.iter().find(|item| {
        reference_uid
            .as_ref()
            .is_some_and(|value| item.uid == *value)
            || reference_name
                .as_ref()
                .is_some_and(|value| item.name == *value || item.uid == *value)
    })?;
    Some(ResolvedDatasourceReplacement {
        key: reference_identity_key(reference).unwrap_or_else(|| matched.uid.clone()),
        uid: matched.uid.clone(),
        name: matched.name.clone(),
        datasource_type: matched.datasource_type.clone(),
        exact: true,
        warning: None,
    })
}

fn resolve_from_embedded_reference(reference: &Value) -> Option<ResolvedDatasourceReplacement> {
    let object = reference.as_object()?;
    let ds_type = object
        .get("type")
        .or_else(|| object.get("pluginId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let uid = object
        .get("uid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| synthetic_uid_from_reference(reference));
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| synthetic_name(ds_type, &uid));
    Some(ResolvedDatasourceReplacement {
        key: reference_identity_key(reference).unwrap_or_else(|| uid.clone()),
        uid,
        name,
        datasource_type: ds_type.to_string(),
        exact: true,
        warning: None,
    })
}

fn resolve_from_family_inference(
    reference: &Value,
    scan: &DashboardScanContext,
) -> Option<ResolvedDatasourceReplacement> {
    let key = reference_identity_key(reference)?;
    let families = scan.ref_families.get(&key)?;
    if families.len() != 1 {
        return None;
    }
    let family = families.iter().next()?.as_str();
    let datasource_type = match family {
        "prometheus" => "prometheus",
        "loki" => "loki",
        "flux" => "influxdb",
        _ => return None,
    };
    let uid = format!(
        "prompt-{}-{}",
        datasource_type,
        sanitize_path_component(&key).replace('_', "-")
    );
    Some(ResolvedDatasourceReplacement {
        key,
        uid: uid.clone(),
        name: synthetic_name(datasource_type, &uid),
        datasource_type: datasource_type.to_string(),
        exact: false,
        warning: Some(format!(
            "inferred datasource family {family} for unresolved dashboard reference"
        )),
    })
}

fn build_synthetic_catalog(document: &Value) -> Vec<Map<String, Value>> {
    let mut refs = Vec::new();
    super::collect_datasource_refs(document, &mut refs);
    let mut catalog = BTreeMap::<String, Map<String, Value>>::new();
    for reference in refs {
        let Some(object) = reference.as_object() else {
            continue;
        };
        let Some(ds_type) = object.get("type").and_then(Value::as_str) else {
            continue;
        };
        let uid = object
            .get("uid")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or(ds_type);
        let name = object
            .get("name")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .unwrap_or(uid);
        catalog.entry(uid.to_string()).or_insert_with(|| {
            Map::from_iter([
                ("uid".to_string(), Value::String(uid.to_string())),
                ("name".to_string(), Value::String(name.to_string())),
                ("type".to_string(), Value::String(ds_type.to_string())),
            ])
        });
    }
    catalog.into_values().collect()
}

fn load_raw_to_prompt_metadata(
    metadata_source_dir: Option<&Path>,
) -> Result<Option<(PathBuf, Option<ExportMetadata>)>> {
    let Some(metadata_source_dir) = metadata_source_dir else {
        return Ok(None);
    };
    let metadata = super::load_export_metadata(metadata_source_dir, None)?;
    Ok(Some((metadata_source_dir.to_path_buf(), metadata)))
}

fn write_prompt_lane_metadata(
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
        ),
        &output_root.join(EXPORT_METADATA_FILENAME),
    )?;
    let _ = plan;
    Ok(())
}

fn load_datasource_mapping(mapping_path: Option<&Path>) -> Result<Option<DatasourceMapDocument>> {
    let Some(mapping_path) = mapping_path else {
        return Ok(None);
    };
    let raw = fs::read_to_string(mapping_path)?;
    let parsed = match mapping_path.extension().and_then(|value| value.to_str()) {
        Some("yaml") | Some("yml") => {
            serde_yaml::from_str::<DatasourceMapDocument>(&raw).map_err(|error| {
                message(format!(
                    "Invalid YAML datasource map in {}: {error}",
                    mapping_path.display()
                ))
            })?
        }
        _ => serde_json::from_str::<DatasourceMapDocument>(&raw).map_err(|error| {
            message(format!(
                "Invalid JSON datasource map in {}: {error}",
                mapping_path.display()
            ))
        })?,
    };
    if !parsed.kind.is_empty() && parsed.kind != MAPPING_KIND {
        return Err(message(format!(
            "Unsupported datasource map kind {:?} in {}",
            parsed.kind,
            mapping_path.display()
        )));
    }
    Ok(Some(parsed))
}

fn reference_identity_key(reference: &Value) -> Option<String> {
    match reference {
        Value::String(text) => {
            let normalized = text.trim();
            if normalized.is_empty() {
                None
            } else {
                Some(format!("ref:{normalized}"))
            }
        }
        Value::Object(object) => {
            if let Some(uid) = object
                .get("uid")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                return Some(format!("uid:{uid}"));
            }
            if let Some(name) = object
                .get("name")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                return Some(format!("name:{name}"));
            }
            if let Some(id) = object.get("id") {
                return Some(format!("id:{id}"));
            }
            if let Some(ds_type) = object
                .get("type")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                return Some(format!(
                    "type:{ds_type}:{}",
                    sanitize_path_component(&reference.to_string())
                ));
            }
            None
        }
        _ => None,
    }
}

fn datasource_type_from_reference(reference: &Value) -> Option<String> {
    match reference {
        Value::Object(object) => object
            .get("type")
            .or_else(|| object.get("pluginId"))
            .and_then(Value::as_str)
            .map(str::to_string),
        Value::String(_) => None,
        _ => None,
    }
}

fn synthetic_uid_from_reference(reference: &Value) -> String {
    let key = reference_identity_key(reference).unwrap_or_else(|| "dashboard-ref".to_string());
    format!("prompt-{}", sanitize_path_component(&key).replace('_', "-"))
}

fn synthetic_name(datasource_type: &str, uid: &str) -> String {
    format!(
        "{} ({})",
        datasource_type,
        uid.trim_start_matches("prompt-")
    )
}
