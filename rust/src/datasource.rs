//! Datasource domain orchestrator.
//!
//! Purpose:
//! - Own datasource command flows (`list`, `add`, `delete`, `export`, `import`, `diff`).
//! - Normalize datasource contract shape across live API payloads and exported metadata.
//! - Keep output serialization (`table`/`csv`/`json`) selection centralized.
//!
//! Flow:
//! - Parse args from `dashboard`-shared auth/common CLI types where possible.
//! - Normalize command variants before branching by subcommand.
//! - Build client and route execution to list/export/import/diff helpers.
//!
//! Caveats:
//! - Keep API-field compatibility logic in `datasource_diff.rs` and import/export helpers.
//! - Avoid side effects in normalization helpers; keep them as pure value transforms.
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{
    load_json_object_file, message, sanitize_path_component, string_field, write_json_file, Result,
};
use crate::dashboard::{
    build_auth_context, build_http_client, build_http_client_for_org, list_datasources,
    CommonCliArgs, DEFAULT_ORG_ID,
};
use crate::datasource::datasource_diff::{
    build_datasource_diff_report, normalize_export_records, normalize_live_records,
    DatasourceDiffEntry, DatasourceDiffReport, DatasourceDiffStatus,
};
use crate::http::JsonHttpClient;

const DEFAULT_EXPORT_DIR: &str = "datasources";
const DATASOURCE_EXPORT_FILENAME: &str = "datasources.json";
const EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
const ROOT_INDEX_KIND: &str = "grafana-utils-datasource-export-index";
const TOOL_SCHEMA_VERSION: i64 = 1;
const DATASOURCE_CONTRACT_FIELDS: &[&str] = &[
    "uid",
    "name",
    "type",
    "access",
    "url",
    "isDefault",
    "org",
    "orgId",
];

#[path = "datasource_diff.rs"]
mod datasource_diff;

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum ListOutputFormat {
    Table,
    Csv,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum DryRunOutputFormat {
    Text,
    Table,
    Json,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceListArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, default_value_t = false, conflicts_with_all = ["csv", "json"], help = "Render datasource summaries as a table.")]
    pub table: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "json"], help = "Render datasource summaries as CSV.")]
    pub csv: bool,
    #[arg(long, default_value_t = false, conflicts_with_all = ["table", "csv"], help = "Render datasource summaries as JSON.")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "csv", "json"],
        help = "Alternative single-flag output selector. Use table, csv, or json."
    )]
    pub output_format: Option<ListOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "Do not print table headers when rendering the default table output."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceExportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        default_value = DEFAULT_EXPORT_DIR,
        help = "Directory to write exported datasource inventory into. Export writes datasources.json plus index/manifest files at that root."
    )]
    pub export_dir: PathBuf,
    #[arg(
        long,
        conflicts_with = "all_orgs",
        help = "Export datasource inventory from this explicit Grafana org ID instead of the current org. Requires Basic auth."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "org_id",
        help = "Enumerate all visible Grafana orgs and export one datasource inventory bundle per org under the export root. Requires Basic auth."
    )]
    pub all_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Replace existing export files in the target directory instead of failing."
    )]
    pub overwrite: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview the datasource export files that would be written without changing disk."
    )]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceImportArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Import datasource inventory from this directory. Point this at the datasource export root that contains datasources.json and export-metadata.json."
    )]
    pub import_dir: PathBuf,
    #[arg(
        long,
        conflicts_with = "use_export_org",
        help = "Import datasources into this Grafana org ID instead of the current org context. Requires Basic auth."
    )]
    pub org_id: Option<i64>,
    #[arg(
        long,
        default_value_t = false,
        conflicts_with = "require_matching_export_org",
        help = "Import a combined multi-org datasource export root by routing each org-scoped datasource bundle back into the matching Grafana org. Requires Basic auth."
    )]
    pub use_export_org: bool,
    #[arg(
        long = "only-org-id",
        requires = "use_export_org",
        conflicts_with = "org_id",
        help = "With --use-export-org, import only these exported source org IDs. Repeat the flag to select multiple orgs."
    )]
    pub only_org_id: Vec<i64>,
    #[arg(
        long,
        default_value_t = false,
        requires = "use_export_org",
        help = "With --use-export-org, create a missing destination org when an exported source org ID does not exist in Grafana. The new org is created from the exported org name and then used as the import target."
    )]
    pub create_missing_orgs: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Require the datasource export orgId to match the target Grafana org before dry-run or live import."
    )]
    pub require_matching_export_org: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Update an existing destination datasource when the imported datasource identity already exists. Without this flag, existing matches are blocked."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Reconcile only datasources that already exist in Grafana. Missing destination identities are skipped instead of created."
    )]
    pub update_existing_only: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what datasource import would do without changing Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render a compact table instead of per-datasource log lines."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render one JSON document with mode, datasource actions, and summary counts."
    )]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        conflicts_with_all = ["table", "json"],
        help = "Alternative single-flag output selector for --dry-run output. Use text, table, or json."
    )]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row."
    )]
    pub no_header: bool,
    #[arg(
        long,
        value_delimiter = ',',
        requires = "dry_run",
        value_parser = parse_datasource_import_output_column,
        help = "For --dry-run --table only, render only these comma-separated columns. Supported values: uid, name, type, destination, action, org_id, file."
    )]
    pub output_columns: Vec<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Show concise per-datasource progress in <current>/<total> form while processing files."
    )]
    pub progress: bool,
    #[arg(
        short = 'v',
        long,
        default_value_t = false,
        help = "Show detailed per-item import output. Overrides --progress output."
    )]
    pub verbose: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceDiffArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Compare datasource inventory from this directory against live Grafana. Point this at the datasource export root that contains datasources.json and export-metadata.json."
    )]
    pub diff_dir: PathBuf,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceAddArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        help = "Datasource UID to create. Optional but recommended for stable identity."
    )]
    pub uid: Option<String>,
    #[arg(long, help = "Datasource name to create.")]
    pub name: String,
    #[arg(long = "type", help = "Grafana datasource plugin type id to create.")]
    pub datasource_type: String,
    #[arg(long, help = "Datasource access mode such as proxy or direct.")]
    pub access: Option<String>,
    #[arg(long, help = "Datasource target URL to store in Grafana.")]
    pub datasource_url: Option<String>,
    #[arg(
        long = "default",
        default_value_t = false,
        help = "Mark the new datasource as the default datasource."
    )]
    pub is_default: bool,
    #[arg(long, help = "Inline JSON object string for datasource jsonData.")]
    pub json_data: Option<String>,
    #[arg(
        long,
        help = "Inline JSON object string for datasource secureJsonData."
    )]
    pub secure_json_data: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what datasource add would do without changing Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render a compact table instead of plain text."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render one JSON document."
    )]
    pub json: bool,
    #[arg(long, value_enum, conflicts_with_all = ["table", "json"], help = "Alternative single-flag output selector for datasource add dry-run output. Use text, table, or json.")]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(
        long,
        required_unless_present = "name",
        conflicts_with = "name",
        help = "Datasource UID to delete."
    )]
    pub uid: Option<String>,
    #[arg(
        long,
        required_unless_present = "uid",
        conflicts_with = "uid",
        help = "Datasource name to delete when UID is not available."
    )]
    pub name: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what datasource delete would do without changing Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render a compact table instead of plain text."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render one JSON document."
    )]
    pub json: bool,
    #[arg(long, value_enum, conflicts_with_all = ["table", "json"], help = "Alternative single-flag output selector for datasource delete dry-run output. Use text, table, or json.")]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Args)]
pub struct DatasourceModifyArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, help = "Datasource UID to modify.")]
    pub uid: String,
    #[arg(long, help = "Replace the datasource URL stored in Grafana.")]
    pub set_url: Option<String>,
    #[arg(
        long,
        help = "Replace the datasource access mode such as proxy or direct."
    )]
    pub set_access: Option<String>,
    #[arg(
        long,
        value_parser = parse_bool_choice,
        help = "Set whether Grafana treats this datasource as default. Use true or false."
    )]
    pub set_default: Option<bool>,
    #[arg(
        long,
        help = "Inline JSON object string to merge into datasource jsonData."
    )]
    pub json_data: Option<String>,
    #[arg(
        long,
        help = "Inline JSON object string to send in datasource secureJsonData."
    )]
    pub secure_json_data: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Preview what datasource modify would do without changing Grafana."
    )]
    pub dry_run: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render a compact table instead of plain text."
    )]
    pub table: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run only, render one JSON document."
    )]
    pub json: bool,
    #[arg(long, value_enum, conflicts_with_all = ["table", "json"], help = "Alternative single-flag output selector for datasource modify dry-run output. Use text, table, or json.")]
    pub output_format: Option<DryRunOutputFormat>,
    #[arg(
        long,
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row."
    )]
    pub no_header: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DatasourceGroupCommand {
    #[command(about = "List live Grafana datasource inventory.")]
    List(DatasourceListArgs),
    #[command(about = "Create one live Grafana datasource through the Grafana API.")]
    Add(DatasourceAddArgs),
    #[command(about = "Modify one live Grafana datasource through the Grafana API.")]
    Modify(DatasourceModifyArgs),
    #[command(about = "Delete one live Grafana datasource through the Grafana API.")]
    Delete(DatasourceDeleteArgs),
    #[command(about = "Export live Grafana datasource inventory as normalized JSON files.")]
    Export(DatasourceExportArgs),
    #[command(about = "Import datasource inventory through the Grafana API.")]
    Import(DatasourceImportArgs),
    #[command(about = "Compare local datasource export files against live Grafana datasources.")]
    Diff(DatasourceDiffArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util datasource",
    about = "List, add, modify, delete, export, import, and diff Grafana datasources."
)]
pub struct DatasourceCliArgs {
    #[command(subcommand)]
    pub command: DatasourceGroupCommand,
}

// Test-only normalization helper keeps parser + output flag coercion behavior in one
// place for datasource CLI contract tests.
#[cfg(test)]
fn normalize_output_formats(args: &mut DatasourceCliArgs) {
    match &mut args.command {
        DatasourceGroupCommand::List(inner) => match inner.output_format {
            Some(ListOutputFormat::Table) => inner.table = true,
            Some(ListOutputFormat::Csv) => inner.csv = true,
            Some(ListOutputFormat::Json) => inner.json = true,
            None => {}
        },
        DatasourceGroupCommand::Import(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Add(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Modify(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Delete(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        _ => {}
    }
}

// Normalize datasource subcommand output-format aliases into boolean render switches so
// execution paths can use a uniform flag contract.
fn normalize_datasource_group_command(
    mut command: DatasourceGroupCommand,
) -> DatasourceGroupCommand {
    match &mut command {
        DatasourceGroupCommand::List(inner) => match inner.output_format {
            Some(ListOutputFormat::Table) => inner.table = true,
            Some(ListOutputFormat::Csv) => inner.csv = true,
            Some(ListOutputFormat::Json) => inner.json = true,
            None => {}
        },
        DatasourceGroupCommand::Import(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Add(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Modify(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        DatasourceGroupCommand::Delete(inner) => match inner.output_format {
            Some(DryRunOutputFormat::Table) => inner.table = true,
            Some(DryRunOutputFormat::Json) => inner.json = true,
            Some(DryRunOutputFormat::Text) | None => {}
        },
        _ => {}
    }
    command
}

// Parse output-column aliases for datasource import dry-run rendering, accepting both
// preferred snake_case and legacy camelCase spellings where applicable.
fn parse_datasource_import_output_column(value: &str) -> std::result::Result<String, String> {
    match value {
        "uid" => Ok("uid".to_string()),
        "name" => Ok("name".to_string()),
        "type" => Ok("type".to_string()),
        "destination" => Ok("destination".to_string()),
        "action" => Ok("action".to_string()),
        "org_id" | "orgId" => Ok("org_id".to_string()),
        "file" => Ok("file".to_string()),
        _ => Err(format!(
            "Unsupported --output-columns value '{value}'. Supported values: uid, name, type, destination, action, org_id, file."
        )),
    }
}

fn parse_bool_choice(value: &str) -> std::result::Result<bool, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err("value must be true or false".to_string()),
    }
}

#[derive(Debug, Clone)]
struct DatasourceExportMetadata {
    schema_version: i64,
    kind: String,
    variant: String,
    resource: String,
    datasources_file: String,
    index_file: String,
}

#[derive(Debug, Clone)]
struct DatasourceImportRecord {
    uid: String,
    name: String,
    datasource_type: String,
    access: String,
    url: String,
    is_default: bool,
    org_id: String,
}

#[derive(Debug, Clone)]
struct MatchResult {
    destination: &'static str,
    action: &'static str,
    #[cfg_attr(not(test), allow(dead_code))]
    target_uid: String,
    target_name: String,
    target_id: Option<i64>,
}

#[derive(Debug, Clone)]
struct DatasourceExportOrgScope {
    source_org_id: i64,
    source_org_name: String,
    import_dir: PathBuf,
}

#[derive(Debug, Clone)]
struct DatasourceExportOrgTargetPlan {
    source_org_id: i64,
    source_org_name: String,
    target_org_id: Option<i64>,
    org_action: &'static str,
    import_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DatasourceImportDryRunReport {
    mode: String,
    import_dir: PathBuf,
    source_org_id: String,
    target_org_id: String,
    rows: Vec<Vec<String>>,
    datasource_count: usize,
    would_create: usize,
    would_update: usize,
    would_skip: usize,
    would_block: usize,
}

fn fetch_current_org(client: &JsonHttpClient) -> Result<Map<String, Value>> {
    match client.request_json(Method::GET, "/api/org", &[], None)? {
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or_else(|| message("Unexpected current-org payload from Grafana.")),
        None => Err(message("Grafana did not return current-org metadata.")),
    }
}

fn list_orgs(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    match client.request_json(Method::GET, "/api/orgs", &[], None)? {
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| {
                item.as_object()
                    .cloned()
                    .ok_or_else(|| message("Unexpected org entry in /api/orgs response."))
            })
            .collect(),
        Some(_) => Err(message("Unexpected /api/orgs payload from Grafana.")),
        None => Ok(Vec::new()),
    }
}

fn create_org(client: &JsonHttpClient, org_name: &str) -> Result<Map<String, Value>> {
    let payload = Value::Object(Map::from_iter(vec![(
        "name".to_string(),
        Value::String(org_name.to_string()),
    )]));
    match client.request_json(Method::POST, "/api/orgs", &[], Some(&payload))? {
        Some(Value::Object(object)) => Ok(object),
        Some(_) => Err(message("Unexpected create-org payload from Grafana.")),
        None => Err(message("Grafana did not return create-org metadata.")),
    }
}

fn org_id_string_from_value(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn build_all_orgs_output_dir(output_dir: &Path, org: &Map<String, Value>) -> PathBuf {
    let org_id = org
        .get("id")
        .map(|value| sanitize_path_component(&value.to_string()))
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    let org_name = sanitize_path_component(&string_field(org, "name", "org"));
    output_dir.join(format!("org_{org_id}_{org_name}"))
}

fn resolve_target_client(common: &CommonCliArgs, org_id: Option<i64>) -> Result<JsonHttpClient> {
    if let Some(org_id) = org_id {
        let context = build_auth_context(common)?;
        if context.auth_mode != "basic" {
            return Err(message(
                "Datasource org switching requires Basic auth (--basic-user / --basic-password).",
            ));
        }
        build_http_client_for_org(common, org_id)
    } else {
        build_http_client(common)
    }
}

fn validate_import_org_auth(common: &CommonCliArgs, args: &DatasourceImportArgs) -> Result<()> {
    let context = build_auth_context(common)?;
    if (args.org_id.is_some() || args.use_export_org) && context.auth_mode != "basic" {
        return Err(message(if args.use_export_org {
            "Datasource import with --use-export-org requires Basic auth (--basic-user / --basic-password)."
        } else {
            "Datasource import with --org-id requires Basic auth (--basic-user / --basic-password)."
        }));
    }
    Ok(())
}

fn describe_datasource_import_mode(
    replace_existing: bool,
    update_existing_only: bool,
) -> &'static str {
    if update_existing_only {
        "update-or-skip-missing"
    } else if replace_existing {
        "create-or-update"
    } else {
        "create-only"
    }
}

fn build_datasource_export_metadata(count: usize) -> Value {
    Value::Object(Map::from_iter(vec![
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
        ),
        (
            "kind".to_string(),
            Value::String(ROOT_INDEX_KIND.to_string()),
        ),
        ("variant".to_string(), Value::String("root".to_string())),
        (
            "resource".to_string(),
            Value::String("datasource".to_string()),
        ),
        (
            "datasourceCount".to_string(),
            Value::Number((count as i64).into()),
        ),
        (
            "datasourcesFile".to_string(),
            Value::String(DATASOURCE_EXPORT_FILENAME.to_string()),
        ),
        (
            "indexFile".to_string(),
            Value::String("index.json".to_string()),
        ),
        (
            "format".to_string(),
            Value::String("grafana-datasource-inventory-v1".to_string()),
        ),
    ]))
}

fn build_data_source_record(datasource: &Map<String, Value>) -> Vec<String> {
    vec![
        string_field(datasource, "uid", ""),
        string_field(datasource, "name", ""),
        string_field(datasource, "type", ""),
        string_field(datasource, "url", ""),
        if datasource
            .get("isDefault")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "true".to_string()
        } else {
            "false".to_string()
        },
    ]
}

fn render_data_source_table(
    datasources: &[Map<String, Value>],
    include_header: bool,
) -> Vec<String> {
    let headers = vec![
        "UID".to_string(),
        "NAME".to_string(),
        "TYPE".to_string(),
        "URL".to_string(),
        "IS_DEFAULT".to_string(),
    ];
    let rows: Vec<Vec<String>> = datasources.iter().map(build_data_source_record).collect();
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in &rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{:<width$}", value, width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let separator: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
    let mut lines = Vec::new();
    if include_header {
        lines.extend([format_row(&headers), format_row(&separator)]);
    }
    lines.extend(rows.iter().map(|row| format_row(row)));
    lines
}

fn render_data_source_csv(datasources: &[Map<String, Value>]) -> Vec<String> {
    let mut lines = vec!["uid,name,type,url,isDefault".to_string()];
    lines.extend(datasources.iter().map(|datasource| {
        build_data_source_record(datasource)
            .into_iter()
            .map(|value| {
                if value.contains(',') || value.contains('"') || value.contains('\n') {
                    format!("\"{}\"", value.replace('"', "\"\""))
                } else {
                    value
                }
            })
            .collect::<Vec<String>>()
            .join(",")
    }));
    lines
}

fn render_data_source_json(datasources: &[Map<String, Value>]) -> Value {
    Value::Array(
        datasources
            .iter()
            .map(|datasource| {
                let row = build_data_source_record(datasource);
                Value::Object(Map::from_iter(vec![
                    ("uid".to_string(), Value::String(row[0].clone())),
                    ("name".to_string(), Value::String(row[1].clone())),
                    ("type".to_string(), Value::String(row[2].clone())),
                    ("url".to_string(), Value::String(row[3].clone())),
                    ("isDefault".to_string(), Value::String(row[4].clone())),
                ]))
            })
            .collect(),
    )
}

fn build_export_index(records: &[Map<String, Value>]) -> Value {
    Value::Object(Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(ROOT_INDEX_KIND.to_string()),
        ),
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
        ),
        (
            "datasourcesFile".to_string(),
            Value::String(DATASOURCE_EXPORT_FILENAME.to_string()),
        ),
        (
            "count".to_string(),
            Value::Number((records.len() as i64).into()),
        ),
        (
            "items".to_string(),
            Value::Array(
                records
                    .iter()
                    .map(|record| {
                        Value::Object(Map::from_iter(vec![
                            (
                                "uid".to_string(),
                                Value::String(string_field(record, "uid", "")),
                            ),
                            (
                                "name".to_string(),
                                Value::String(string_field(record, "name", "")),
                            ),
                            (
                                "type".to_string(),
                                Value::String(string_field(record, "type", "")),
                            ),
                            (
                                "org".to_string(),
                                Value::String(string_field(record, "org", "")),
                            ),
                            (
                                "orgId".to_string(),
                                Value::String(string_field(record, "orgId", "")),
                            ),
                        ]))
                    })
                    .collect(),
            ),
        ),
    ]))
}

fn build_all_orgs_export_index(items: &[Map<String, Value>]) -> Value {
    Value::Object(Map::from_iter(vec![
        (
            "kind".to_string(),
            Value::String(ROOT_INDEX_KIND.to_string()),
        ),
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
        ),
        (
            "variant".to_string(),
            Value::String("all-orgs-root".to_string()),
        ),
        (
            "count".to_string(),
            Value::Number((items.len() as i64).into()),
        ),
        (
            "items".to_string(),
            Value::Array(items.iter().cloned().map(Value::Object).collect()),
        ),
    ]))
}

fn build_all_orgs_export_metadata(org_count: usize, datasource_count: usize) -> Value {
    Value::Object(Map::from_iter(vec![
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
        ),
        (
            "kind".to_string(),
            Value::String(ROOT_INDEX_KIND.to_string()),
        ),
        (
            "variant".to_string(),
            Value::String("all-orgs-root".to_string()),
        ),
        (
            "resource".to_string(),
            Value::String("datasource".to_string()),
        ),
        (
            "orgCount".to_string(),
            Value::Number((org_count as i64).into()),
        ),
        (
            "datasourceCount".to_string(),
            Value::Number((datasource_count as i64).into()),
        ),
        (
            "indexFile".to_string(),
            Value::String("index.json".to_string()),
        ),
        (
            "format".to_string(),
            Value::String("grafana-datasource-inventory-v1".to_string()),
        ),
    ]))
}

fn build_export_records(client: &JsonHttpClient) -> Result<Vec<Map<String, Value>>> {
    let org = fetch_current_org(client)?;
    let org_name = string_field(&org, "name", "");
    let org_id = org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    let datasources = list_datasources(client)?;
    Ok(datasources
        .into_iter()
        .map(|datasource| {
            let mut record = Map::new();
            record.insert(
                "uid".to_string(),
                Value::String(string_field(&datasource, "uid", "")),
            );
            record.insert(
                "name".to_string(),
                Value::String(string_field(&datasource, "name", "")),
            );
            record.insert(
                "type".to_string(),
                Value::String(string_field(&datasource, "type", "")),
            );
            record.insert(
                "access".to_string(),
                Value::String(string_field(&datasource, "access", "")),
            );
            record.insert(
                "url".to_string(),
                Value::String(string_field(&datasource, "url", "")),
            );
            record.insert(
                "isDefault".to_string(),
                Value::String(
                    if datasource
                        .get("isDefault")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        "true"
                    } else {
                        "false"
                    }
                    .to_string(),
                ),
            );
            record.insert("org".to_string(), Value::String(org_name.clone()));
            record.insert("orgId".to_string(), Value::String(org_id.clone()));
            record
        })
        .collect())
}

fn export_datasource_scope(
    client: &JsonHttpClient,
    output_dir: &Path,
    overwrite: bool,
    dry_run: bool,
) -> Result<usize> {
    let records = build_export_records(client)?;
    let datasources_path = output_dir.join(DATASOURCE_EXPORT_FILENAME);
    let index_path = output_dir.join("index.json");
    let metadata_path = output_dir.join(EXPORT_METADATA_FILENAME);
    if !dry_run {
        write_json_file(
            &datasources_path,
            &Value::Array(records.clone().into_iter().map(Value::Object).collect()),
            overwrite,
        )?;
        write_json_file(&index_path, &build_export_index(&records), overwrite)?;
        write_json_file(
            &metadata_path,
            &build_datasource_export_metadata(records.len()),
            overwrite,
        )?;
    }
    let summary_verb = if dry_run { "Would export" } else { "Exported" };
    println!(
        "{summary_verb} {} datasource(s). Datasources: {} Index: {} Manifest: {}",
        records.len(),
        datasources_path.display(),
        index_path.display(),
        metadata_path.display()
    );
    Ok(records.len())
}

// Parse and validate datasource export metadata before importing any inventory data.
fn parse_export_metadata(path: &Path) -> Result<DatasourceExportMetadata> {
    let value = load_json_object_file(path, "Datasource export metadata")?;
    let object = value
        .as_object()
        .ok_or_else(|| message("Datasource export metadata must be a JSON object."))?;
    let schema_version = object
        .get("schemaVersion")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Datasource export metadata is missing schemaVersion."))?;
    object
        .get("datasourceCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| message("Datasource export metadata is missing datasourceCount."))?;
    Ok(DatasourceExportMetadata {
        schema_version,
        kind: string_field(object, "kind", ""),
        variant: string_field(object, "variant", ""),
        resource: string_field(object, "resource", ""),
        datasources_file: string_field(object, "datasourcesFile", DATASOURCE_EXPORT_FILENAME),
        index_file: string_field(object, "indexFile", "index.json"),
    })
}

fn validate_datasource_contract_record(
    record: &Map<String, Value>,
    context_label: &str,
) -> Result<()> {
    let mut extra_fields = record
        .keys()
        .filter(|key| !DATASOURCE_CONTRACT_FIELDS.contains(&key.as_str()))
        .cloned()
        .collect::<Vec<String>>();
    extra_fields.sort();
    if extra_fields.is_empty() {
        return Ok(());
    }
    Err(message(format!(
        "{context_label} contains unsupported datasource field(s): {}. Supported fields: {}.",
        extra_fields.join(", "),
        DATASOURCE_CONTRACT_FIELDS.join(", ")
    )))
}

fn load_import_records(
    import_dir: &Path,
) -> Result<(DatasourceExportMetadata, Vec<DatasourceImportRecord>)> {
    let metadata_path = import_dir.join(EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Err(message(format!(
            "Datasource import directory is missing {}: {}",
            EXPORT_METADATA_FILENAME,
            metadata_path.display()
        )));
    }
    let metadata = parse_export_metadata(&metadata_path)?;
    if metadata.kind != ROOT_INDEX_KIND {
        return Err(message(format!(
            "Unexpected datasource export manifest kind in {}: {:?}",
            metadata_path.display(),
            metadata.kind
        )));
    }
    if metadata.schema_version != TOOL_SCHEMA_VERSION {
        return Err(message(format!(
            "Unsupported datasource export schemaVersion {:?} in {}. Expected {}.",
            metadata.schema_version,
            metadata_path.display(),
            TOOL_SCHEMA_VERSION
        )));
    }
    if metadata.variant != "root" || metadata.resource != "datasource" {
        return Err(message(format!(
            "Datasource export manifest {} is not a datasource export root.",
            metadata_path.display()
        )));
    }
    let datasources_path = import_dir.join(&metadata.datasources_file);
    let raw = fs::read_to_string(&datasources_path)?;
    let value: Value = serde_json::from_str(&raw)?;
    let items = value.as_array().ok_or_else(|| {
        message(format!(
            "Datasource inventory file must contain a JSON array: {}",
            datasources_path.display()
        ))
    })?;
    let mut records = Vec::new();
    for item in items {
        let object = item.as_object().ok_or_else(|| {
            message(format!(
                "Datasource inventory entry must be a JSON object: {}",
                datasources_path.display()
            ))
        })?;
        validate_datasource_contract_record(
            object,
            &format!("Datasource import entry in {}", datasources_path.display()),
        )?;
        records.push(DatasourceImportRecord {
            uid: string_field(object, "uid", ""),
            name: string_field(object, "name", ""),
            datasource_type: string_field(object, "type", ""),
            access: string_field(object, "access", ""),
            url: string_field(object, "url", ""),
            is_default: string_field(object, "isDefault", "false") == "true",
            org_id: string_field(object, "orgId", ""),
        });
    }
    Ok((metadata, records))
}

fn load_diff_record_values(diff_dir: &Path) -> Result<Vec<Value>> {
    let metadata_path = diff_dir.join(EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Err(message(format!(
            "Datasource diff directory is missing {}: {}",
            EXPORT_METADATA_FILENAME,
            metadata_path.display()
        )));
    }
    let metadata = parse_export_metadata(&metadata_path)?;
    if metadata.kind != ROOT_INDEX_KIND {
        return Err(message(format!(
            "Unexpected datasource export manifest kind in {}: {:?}",
            metadata_path.display(),
            metadata.kind
        )));
    }
    if metadata.schema_version != TOOL_SCHEMA_VERSION {
        return Err(message(format!(
            "Unsupported datasource export schemaVersion {:?} in {}. Expected {}.",
            metadata.schema_version,
            metadata_path.display(),
            TOOL_SCHEMA_VERSION
        )));
    }
    if metadata.variant != "root" || metadata.resource != "datasource" {
        return Err(message(format!(
            "Datasource export manifest {} is not a datasource export root.",
            metadata_path.display()
        )));
    }
    let datasources_path = diff_dir.join(&metadata.datasources_file);
    let raw = fs::read_to_string(&datasources_path)?;
    let value: Value = serde_json::from_str(&raw)?;
    let items = value.as_array().ok_or_else(|| {
        message(format!(
            "Datasource inventory file must contain a JSON array: {}",
            datasources_path.display()
        ))
    })?;
    for item in items {
        let object = item.as_object().ok_or_else(|| {
            message(format!(
                "Datasource inventory entry must be a JSON object: {}",
                datasources_path.display()
            ))
        })?;
        validate_datasource_contract_record(
            object,
            &format!("Datasource diff entry in {}", datasources_path.display()),
        )?;
    }
    Ok(items.clone())
}

fn collect_source_org_ids(
    import_dir: &Path,
    metadata: &DatasourceExportMetadata,
) -> Result<BTreeSet<String>> {
    let mut org_ids = BTreeSet::new();
    let datasources_path = import_dir.join(&metadata.datasources_file);
    if datasources_path.is_file() {
        let raw = fs::read_to_string(&datasources_path)?;
        let value: Value = serde_json::from_str(&raw)?;
        if let Some(items) = value.as_array() {
            for item in items {
                if let Some(object) = item.as_object() {
                    let org_id = string_field(object, "orgId", "");
                    if !org_id.is_empty() {
                        org_ids.insert(org_id);
                    }
                }
            }
        }
    }
    let index_path = import_dir.join(&metadata.index_file);
    if index_path.is_file() {
        let raw = fs::read_to_string(&index_path)?;
        let value: Value = serde_json::from_str(&raw)?;
        if let Some(items) = value.get("items").and_then(Value::as_array) {
            for item in items {
                if let Some(object) = item.as_object() {
                    let org_id = string_field(object, "orgId", "");
                    if !org_id.is_empty() {
                        org_ids.insert(org_id);
                    }
                }
            }
        }
    }
    Ok(org_ids)
}

fn collect_source_org_names(
    import_dir: &Path,
    metadata: &DatasourceExportMetadata,
) -> Result<BTreeSet<String>> {
    let mut org_names = BTreeSet::new();
    let datasources_path = import_dir.join(&metadata.datasources_file);
    if datasources_path.is_file() {
        let raw = fs::read_to_string(&datasources_path)?;
        let value: Value = serde_json::from_str(&raw)?;
        if let Some(items) = value.as_array() {
            for item in items {
                if let Some(object) = item.as_object() {
                    let org_name = string_field(object, "org", "");
                    if !org_name.is_empty() {
                        org_names.insert(org_name);
                    }
                }
            }
        }
    }
    let index_path = import_dir.join(&metadata.index_file);
    if index_path.is_file() {
        let raw = fs::read_to_string(&index_path)?;
        let value: Value = serde_json::from_str(&raw)?;
        if let Some(items) = value.get("items").and_then(Value::as_array) {
            for item in items {
                if let Some(object) = item.as_object() {
                    let org_name = string_field(object, "org", "");
                    if !org_name.is_empty() {
                        org_names.insert(org_name);
                    }
                }
            }
        }
    }
    Ok(org_names)
}

fn parse_export_org_scope(
    import_root: &Path,
    scope_dir: &Path,
) -> Result<DatasourceExportOrgScope> {
    let metadata = parse_export_metadata(&scope_dir.join(EXPORT_METADATA_FILENAME))?;
    let export_org_ids = collect_source_org_ids(scope_dir, &metadata)?;
    let (source_org_id, source_org_name_from_dir) = if export_org_ids.is_empty() {
        let scope_name = scope_dir
            .file_name()
            .and_then(|item| item.to_str())
            .unwrap_or_default();
        if let Some(rest) = scope_name.strip_prefix("org_") {
            let mut parts = rest.splitn(2, '_');
            let source_org_id_text = parts.next().unwrap_or_default();
            let source_org_name = parts
                .next()
                .unwrap_or_default()
                .replace('_', " ")
                .trim()
                .to_string();
            let source_org_id = source_org_id_text.parse::<i64>().map_err(|_| {
                message(format!(
                    "Cannot route datasource import by export org for {}: export orgId '{}' from the org directory name is not a valid integer.",
                    scope_dir.display(),
                    source_org_id_text
                ))
            })?;
            (source_org_id, source_org_name)
        } else {
            return Err(message(format!(
                "Cannot route datasource import by export org for {}: export orgId metadata was not found in datasources.json or index.json.",
                scope_dir.display()
            )));
        }
    } else {
        if export_org_ids.len() > 1 {
            return Err(message(format!(
                "Cannot route datasource import by export org for {}: found multiple export orgIds ({}).",
                scope_dir.display(),
                export_org_ids.into_iter().collect::<Vec<String>>().join(", ")
            )));
        }
        let source_org_id_text = export_org_ids.into_iter().next().unwrap_or_default();
        let source_org_id = source_org_id_text.parse::<i64>().map_err(|_| {
            message(format!(
                "Cannot route datasource import by export org for {}: export orgId '{}' is not a valid integer.",
                scope_dir.display(),
                source_org_id_text
            ))
        })?;
        (source_org_id, String::new())
    };
    let org_names = collect_source_org_names(scope_dir, &metadata)?;
    if org_names.len() > 1 {
        return Err(message(format!(
            "Cannot route datasource import by export org for {}: found multiple export org names ({}).",
            scope_dir.display(),
            org_names.into_iter().collect::<Vec<String>>().join(", ")
        )));
    }
    let source_org_name = org_names
        .into_iter()
        .next()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| {
            if !source_org_name_from_dir.is_empty() {
                source_org_name_from_dir
            } else {
                import_root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("org")
                    .to_string()
            }
        });
    Ok(DatasourceExportOrgScope {
        source_org_id,
        source_org_name,
        import_dir: scope_dir.to_path_buf(),
    })
}

fn discover_export_org_import_scopes(
    args: &DatasourceImportArgs,
) -> Result<Vec<DatasourceExportOrgScope>> {
    if !args.use_export_org {
        return Ok(Vec::new());
    }
    let selected_org_ids: BTreeSet<i64> = args.only_org_id.iter().copied().collect();
    let mut scopes = Vec::new();
    let mut matched_source_org_ids = BTreeSet::new();
    for entry in fs::read_dir(&args.import_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|item| item.to_str()) else {
            continue;
        };
        if !name.starts_with("org_") {
            continue;
        }
        if !path.join(EXPORT_METADATA_FILENAME).is_file() {
            continue;
        }
        let scope = parse_export_org_scope(&path, &path)?;
        if !selected_org_ids.is_empty() && !selected_org_ids.contains(&scope.source_org_id) {
            continue;
        }
        matched_source_org_ids.insert(scope.source_org_id);
        scopes.push(scope);
    }
    scopes.sort_by(|left, right| left.source_org_id.cmp(&right.source_org_id));
    if !selected_org_ids.is_empty() {
        let missing: Vec<String> = selected_org_ids
            .difference(&matched_source_org_ids)
            .map(|item| item.to_string())
            .collect();
        if !missing.is_empty() {
            return Err(message(format!(
                "Selected exported org IDs were not found in {}: {}",
                args.import_dir.display(),
                missing.join(", ")
            )));
        }
    }
    if scopes.is_empty() {
        if args.import_dir.join(EXPORT_METADATA_FILENAME).is_file() {
            return Err(message(
                "Datasource import with --use-export-org expects the combined export root, not one org export directory.",
            ));
        }
        if !selected_org_ids.is_empty() {
            return Err(message(format!(
                "Datasource import with --use-export-org did not find the selected exported org IDs ({}) under {}.",
                selected_org_ids
                    .into_iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                args.import_dir.display()
            )));
        }
        return Err(message(format!(
            "Datasource import with --use-export-org did not find any org-scoped datasource exports under {}.",
            args.import_dir.display()
        )));
    }
    let found_org_ids: BTreeSet<i64> = scopes.iter().map(|scope| scope.source_org_id).collect();
    let missing_org_ids: Vec<String> = selected_org_ids
        .difference(&found_org_ids)
        .map(|id| id.to_string())
        .collect();
    if !missing_org_ids.is_empty() {
        return Err(message(format!(
            "Datasource import with --use-export-org did not find the selected exported org IDs ({}).",
            missing_org_ids.join(", ")
        )));
    }
    Ok(scopes)
}

fn resolve_export_org_target_plan(
    admin_client: &JsonHttpClient,
    args: &DatasourceImportArgs,
    scope: &DatasourceExportOrgScope,
) -> Result<DatasourceExportOrgTargetPlan> {
    let orgs = list_orgs(admin_client)?;
    for org in orgs {
        let org_id_text = org_id_string_from_value(org.get("id"));
        if org_id_text == scope.source_org_id.to_string() {
            return Ok(DatasourceExportOrgTargetPlan {
                source_org_id: scope.source_org_id,
                source_org_name: scope.source_org_name.clone(),
                target_org_id: Some(scope.source_org_id),
                org_action: "exists",
                import_dir: scope.import_dir.clone(),
            });
        }
    }
    if args.dry_run && !args.create_missing_orgs {
        return Ok(DatasourceExportOrgTargetPlan {
            source_org_id: scope.source_org_id,
            source_org_name: scope.source_org_name.clone(),
            target_org_id: None,
            org_action: "missing",
            import_dir: scope.import_dir.clone(),
        });
    }
    if args.dry_run && args.create_missing_orgs {
        return Ok(DatasourceExportOrgTargetPlan {
            source_org_id: scope.source_org_id,
            source_org_name: scope.source_org_name.clone(),
            target_org_id: None,
            org_action: "would-create",
            import_dir: scope.import_dir.clone(),
        });
    }
    if !args.create_missing_orgs {
        return Err(message(format!(
            "Datasource import source orgId {} was not found in destination Grafana. Use --create-missing-orgs to create it from export metadata.",
            scope.source_org_id
        )));
    }
    if scope.source_org_name.trim().is_empty() {
        return Err(message(format!(
            "Datasource import with --create-missing-orgs could not determine an exported org name for source orgId {}.",
            scope.source_org_id
        )));
    }
    let created = create_org(admin_client, &scope.source_org_name)?;
    let created_org_id =
        org_id_string_from_value(created.get("orgId").or_else(|| created.get("id")));
    if created_org_id.is_empty() {
        return Err(message(format!(
            "Grafana did not return a usable orgId after creating destination org '{}' for exported org {}.",
            scope.source_org_name, scope.source_org_id
        )));
    }
    let parsed_org_id = created_org_id.parse::<i64>().map_err(|_| {
        message(format!(
            "Grafana returned non-numeric orgId '{}' after creating destination org '{}' for exported org {}.",
            created_org_id, scope.source_org_name, scope.source_org_id
        ))
    })?;
    Ok(DatasourceExportOrgTargetPlan {
        source_org_id: scope.source_org_id,
        source_org_name: scope.source_org_name.clone(),
        target_org_id: Some(parsed_org_id),
        org_action: "created",
        import_dir: scope.import_dir.clone(),
    })
}

fn validate_matching_export_org(
    client: &JsonHttpClient,
    args: &DatasourceImportArgs,
    import_dir: &Path,
    metadata: &DatasourceExportMetadata,
) -> Result<()> {
    if !args.require_matching_export_org {
        return Ok(());
    }
    let source_org_ids = collect_source_org_ids(import_dir, metadata)?;
    if source_org_ids.is_empty() {
        return Err(message(
            "Cannot verify datasource export org: no stable orgId metadata found in datasources.json or index.json.",
        ));
    }
    if source_org_ids.len() > 1 {
        return Err(message(format!(
            "Cannot verify datasource export org: found multiple export orgIds ({}).",
            source_org_ids
                .into_iter()
                .collect::<Vec<String>>()
                .join(", ")
        )));
    }
    let source_org_id = source_org_ids.into_iter().next().unwrap_or_default();
    let target_org = fetch_current_org(client)?;
    let target_org_id = target_org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    if source_org_id != target_org_id {
        return Err(message(format!(
            "Datasource import export org mismatch: raw export orgId {source_org_id} does not match target org {target_org_id}. Use matching credentials/org selection or omit --require-matching-export-org."
        )));
    }
    Ok(())
}

fn collect_datasource_import_dry_run_report(
    client: &JsonHttpClient,
    args: &DatasourceImportArgs,
) -> Result<DatasourceImportDryRunReport> {
    let replace_existing = args.replace_existing || args.update_existing_only;
    let (metadata, records) = load_import_records(&args.import_dir)?;
    validate_matching_export_org(client, args, &args.import_dir, &metadata)?;
    let live = list_datasources(client)?;
    let target_org = fetch_current_org(client)?;
    let target_org_id = target_org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    let mode = describe_datasource_import_mode(args.replace_existing, args.update_existing_only);
    let mut rows = Vec::new();
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut blocked = 0usize;
    for (index, record) in records.iter().enumerate() {
        let matching = resolve_match(record, &live, replace_existing, args.update_existing_only);
        let file_ref = format!("{}#{}", metadata.datasources_file, index);
        rows.push(vec![
            record.uid.clone(),
            record.name.clone(),
            record.datasource_type.clone(),
            matching.destination.to_string(),
            matching.action.to_string(),
            target_org_id.clone(),
            file_ref,
        ]);
        match matching.action {
            "would-create" => created += 1,
            "would-update" => updated += 1,
            "would-skip-missing" => skipped += 1,
            _ => blocked += 1,
        }
    }
    Ok(DatasourceImportDryRunReport {
        mode: mode.to_string(),
        import_dir: args.import_dir.clone(),
        source_org_id: records
            .iter()
            .find(|item| !item.org_id.is_empty())
            .map(|item| item.org_id.clone())
            .unwrap_or_default(),
        target_org_id,
        rows,
        datasource_count: records.len(),
        would_create: created,
        would_update: updated,
        would_skip: skipped,
        would_block: blocked,
    })
}

fn build_datasource_import_dry_run_json_value(report: &DatasourceImportDryRunReport) -> Value {
    Value::Object(Map::from_iter(vec![
        ("mode".to_string(), Value::String(report.mode.clone())),
        (
            "sourceOrgId".to_string(),
            Value::String(report.source_org_id.clone()),
        ),
        (
            "targetOrgId".to_string(),
            Value::String(report.target_org_id.clone()),
        ),
        (
            "datasources".to_string(),
            Value::Array(
                report
                    .rows
                    .iter()
                    .map(|row| {
                        Value::Object(Map::from_iter(vec![
                            ("uid".to_string(), Value::String(row[0].clone())),
                            ("name".to_string(), Value::String(row[1].clone())),
                            ("type".to_string(), Value::String(row[2].clone())),
                            ("destination".to_string(), Value::String(row[3].clone())),
                            ("action".to_string(), Value::String(row[4].clone())),
                            ("orgId".to_string(), Value::String(row[5].clone())),
                            ("file".to_string(), Value::String(row[6].clone())),
                        ]))
                    })
                    .collect(),
            ),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "datasourceCount".to_string(),
                    Value::Number((report.datasource_count as i64).into()),
                ),
                (
                    "wouldCreate".to_string(),
                    Value::Number((report.would_create as i64).into()),
                ),
                (
                    "wouldUpdate".to_string(),
                    Value::Number((report.would_update as i64).into()),
                ),
                (
                    "wouldSkip".to_string(),
                    Value::Number((report.would_skip as i64).into()),
                ),
                (
                    "wouldBlock".to_string(),
                    Value::Number((report.would_block as i64).into()),
                ),
            ])),
        ),
    ]))
}

fn print_datasource_import_dry_run_report(
    report: &DatasourceImportDryRunReport,
    args: &DatasourceImportArgs,
) -> Result<()> {
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&build_datasource_import_dry_run_json_value(report))?
        );
    } else if args.table {
        for line in render_import_table(
            &report.rows,
            !args.no_header,
            if args.output_columns.is_empty() {
                None
            } else {
                Some(args.output_columns.as_slice())
            },
        ) {
            println!("{line}");
        }
        println!(
            "Dry-run checked {} datasource(s) from {}",
            report.datasource_count,
            report.import_dir.display()
        );
    } else {
        println!("Import mode: {}", report.mode);
        for row in &report.rows {
            println!(
                "Dry-run datasource uid={} name={} dest={} action={} file={}",
                row[0], row[1], row[3], row[4], row[6]
            );
        }
        println!(
            "Dry-run checked {} datasource(s) from {}",
            report.datasource_count,
            report.import_dir.display()
        );
    }
    Ok(())
}

fn import_datasources_with_client(
    client: &JsonHttpClient,
    args: &DatasourceImportArgs,
) -> Result<usize> {
    if args.dry_run {
        let report = collect_datasource_import_dry_run_report(client, args)?;
        print_datasource_import_dry_run_report(&report, args)?;
        return Ok(0);
    }
    let replace_existing = args.replace_existing || args.update_existing_only;
    let (metadata, records) = load_import_records(&args.import_dir)?;
    validate_matching_export_org(client, args, &args.import_dir, &metadata)?;
    let live = list_datasources(client)?;
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut skipped = 0usize;
    let blocked = 0usize;
    for record in &records {
        let matching = resolve_match(record, &live, replace_existing, args.update_existing_only);
        match matching.action {
            "would-create" => {
                client.request_json(
                    Method::POST,
                    "/api/datasources",
                    &[],
                    Some(&build_import_payload(record)),
                )?;
                created += 1;
            }
            "would-update" => {
                let target_id = matching.target_id.ok_or_else(|| {
                    message(format!(
                        "Matched datasource {} does not expose a usable numeric id for update.",
                        matching.target_name
                    ))
                })?;
                let payload = build_import_payload(record);
                client.request_json(
                    Method::PUT,
                    &format!("/api/datasources/{target_id}"),
                    &[],
                    Some(&payload),
                )?;
                updated += 1;
            }
            "would-skip-missing" => {
                skipped += 1;
            }
            _ => {
                return Err(message(format!(
                    "Datasource import blocked for {}: destination={} action={}.",
                    if record.uid.is_empty() {
                        &record.name
                    } else {
                        &record.uid
                    },
                    matching.destination,
                    matching.action
                )));
            }
        }
    }
    println!(
        "Imported {} datasource(s) from {}; updated {}, skipped {}, blocked {}",
        created + updated,
        args.import_dir.display(),
        updated,
        skipped,
        blocked
    );
    Ok(created + updated)
}

fn build_routed_datasource_import_org_row(
    plan: &DatasourceExportOrgTargetPlan,
    datasource_count: usize,
) -> Vec<String> {
    vec![
        plan.source_org_id.to_string(),
        if plan.source_org_name.is_empty() {
            "-".to_string()
        } else {
            plan.source_org_name.clone()
        },
        plan.org_action.to_string(),
        plan.target_org_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string()),
        datasource_count.to_string(),
        plan.import_dir.display().to_string(),
    ]
}

fn render_routed_datasource_import_org_table(
    rows: &[Vec<String>],
    include_header: bool,
) -> Vec<String> {
    let headers = vec![
        "SOURCE_ORG_ID".to_string(),
        "SOURCE_ORG_NAME".to_string(),
        "ORG_ACTION".to_string(),
        "TARGET_ORG_ID".to_string(),
        "DATASOURCE_COUNT".to_string(),
        "IMPORT_DIR".to_string(),
    ];
    let mut widths: Vec<usize> = headers.iter().map(|item| item.len()).collect();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{:<width$}", value, width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<String>>();
    let mut lines = Vec::new();
    if include_header {
        lines.push(format_row(&headers));
        lines.push(format_row(&separator));
    }
    lines.extend(rows.iter().map(|row| format_row(row)));
    lines
}

fn build_routed_datasource_import_dry_run_json(args: &DatasourceImportArgs) -> Result<String> {
    let admin_client = build_http_client(&args.common)?;
    let scopes = discover_export_org_import_scopes(args)?;
    let mut orgs = Vec::new();
    let mut imports = Vec::new();
    for scope in scopes {
        let plan = resolve_export_org_target_plan(&admin_client, args, &scope)?;
        let datasource_count = load_import_records(&plan.import_dir)?.1.len();
        orgs.push(serde_json::json!({
            "sourceOrgId": plan.source_org_id,
            "sourceOrgName": plan.source_org_name,
            "orgAction": plan.org_action,
            "targetOrgId": plan.target_org_id,
            "datasourceCount": datasource_count,
            "importDir": plan.import_dir.display().to_string(),
        }));
        let preview = if let Some(target_org_id) = plan.target_org_id {
            let mut scoped_args = args.clone();
            scoped_args.org_id = Some(target_org_id);
            scoped_args.use_export_org = false;
            scoped_args.only_org_id = Vec::new();
            scoped_args.create_missing_orgs = false;
            scoped_args.import_dir = plan.import_dir.clone();
            let scoped_client = build_http_client_for_org(&args.common, target_org_id)?;
            build_datasource_import_dry_run_json_value(&collect_datasource_import_dry_run_report(
                &scoped_client,
                &scoped_args,
            )?)
        } else {
            serde_json::json!({
                "mode": describe_datasource_import_mode(args.replace_existing, args.update_existing_only),
                "sourceOrgId": plan.source_org_id.to_string(),
                "targetOrgId": Value::Null,
                "datasources": [],
                "summary": {
                    "datasourceCount": datasource_count,
                    "wouldCreate": 0,
                    "wouldUpdate": 0,
                    "wouldSkip": 0,
                    "wouldBlock": 0
                }
            })
        };
        let mut import_entry = serde_json::Map::new();
        import_entry.insert("sourceOrgId".to_string(), Value::from(plan.source_org_id));
        import_entry.insert(
            "sourceOrgName".to_string(),
            Value::from(plan.source_org_name.clone()),
        );
        import_entry.insert("orgAction".to_string(), Value::from(plan.org_action));
        import_entry.insert(
            "targetOrgId".to_string(),
            plan.target_org_id.map(Value::from).unwrap_or(Value::Null),
        );
        if let Some(object) = preview.as_object() {
            for (key, value) in object {
                import_entry.insert(key.clone(), value.clone());
            }
        }
        imports.push(Value::Object(import_entry));
    }
    let summary = serde_json::json!({
        "orgCount": orgs.len(),
        "existingOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("exists".to_string()))).count(),
        "missingOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("missing".to_string()))).count(),
        "wouldCreateOrgCount": orgs.iter().filter(|entry| entry.get("orgAction") == Some(&Value::String("would-create".to_string()))).count(),
        "datasourceCount": imports.iter().filter_map(|entry| entry.get("summary").and_then(|summary| summary.get("datasourceCount")).and_then(Value::as_i64)).sum::<i64>(),
    });
    serde_json::to_string_pretty(&serde_json::json!({
        "mode": describe_datasource_import_mode(args.replace_existing, args.update_existing_only),
        "orgs": orgs,
        "imports": imports,
        "summary": summary,
    }))
    .map_err(Into::into)
}

fn import_datasources_by_export_org(args: &DatasourceImportArgs) -> Result<usize> {
    let admin_client = build_http_client(&args.common)?;
    let scopes = discover_export_org_import_scopes(args)?;
    if args.dry_run && args.json {
        println!("{}", build_routed_datasource_import_dry_run_json(args)?);
        return Ok(0);
    }
    let mut org_rows = Vec::new();
    let mut plans = Vec::new();
    for scope in scopes {
        let plan = resolve_export_org_target_plan(&admin_client, args, &scope)?;
        let datasource_count = load_import_records(&plan.import_dir)?.1.len();
        org_rows.push(build_routed_datasource_import_org_row(
            &plan,
            datasource_count,
        ));
        plans.push(plan);
    }
    if args.dry_run && args.table {
        for line in render_routed_datasource_import_org_table(&org_rows, !args.no_header) {
            println!("{line}");
        }
        return Ok(0);
    }
    let mut imported_count = 0usize;
    for plan in plans {
        let target_org_id_label = plan
            .target_org_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string());
        println!(
            "Importing export orgId={} name={} orgAction={} targetOrgId={} from {}",
            plan.source_org_id,
            if plan.source_org_name.is_empty() {
                "-"
            } else {
                &plan.source_org_name
            },
            plan.org_action,
            target_org_id_label,
            plan.import_dir.display()
        );
        let Some(target_org_id) = plan.target_org_id else {
            continue;
        };
        let mut scoped_args = args.clone();
        scoped_args.org_id = Some(target_org_id);
        scoped_args.use_export_org = false;
        scoped_args.only_org_id = Vec::new();
        scoped_args.create_missing_orgs = false;
        scoped_args.import_dir = plan.import_dir.clone();
        let scoped_client = build_http_client_for_org(&args.common, target_org_id)?;
        imported_count += import_datasources_with_client(&scoped_client, &scoped_args)?;
    }
    Ok(imported_count)
}

fn resolve_match(
    record: &DatasourceImportRecord,
    live: &[Map<String, Value>],
    replace_existing: bool,
    update_existing_only: bool,
) -> MatchResult {
    let uid_matches = if !record.uid.is_empty() {
        live.iter()
            .filter(|item| string_field(item, "uid", "") == record.uid)
            .collect::<Vec<&Map<String, Value>>>()
    } else {
        Vec::new()
    };
    let name_matches = if !record.name.is_empty() {
        live.iter()
            .filter(|item| string_field(item, "name", "") == record.name)
            .collect::<Vec<&Map<String, Value>>>()
    } else {
        Vec::new()
    };
    if uid_matches.is_empty() && name_matches.len() > 1 {
        return MatchResult {
            destination: "ambiguous",
            action: "would-fail-ambiguous",
            target_uid: String::new(),
            target_name: record.name.clone(),
            target_id: None,
        };
    }
    if !uid_matches.is_empty() {
        let item = uid_matches[0];
        return MatchResult {
            destination: "exists-uid",
            action: if replace_existing || update_existing_only {
                "would-update"
            } else {
                "would-fail-existing"
            },
            target_uid: string_field(item, "uid", ""),
            target_name: string_field(item, "name", ""),
            target_id: item.get("id").and_then(Value::as_i64),
        };
    }
    if name_matches.len() == 1 {
        let item = name_matches[0];
        let target_uid = string_field(item, "uid", "");
        return MatchResult {
            destination: "exists-name",
            action: if !record.uid.is_empty() && !target_uid.is_empty() && record.uid != target_uid
            {
                "would-fail-uid-mismatch"
            } else if replace_existing || update_existing_only {
                "would-update"
            } else {
                "would-fail-existing"
            },
            target_uid,
            target_name: string_field(item, "name", ""),
            target_id: item.get("id").and_then(Value::as_i64),
        };
    }
    MatchResult {
        destination: "missing",
        action: if update_existing_only {
            "would-skip-missing"
        } else {
            "would-create"
        },
        target_uid: String::new(),
        target_name: String::new(),
        target_id: None,
    }
}

fn build_import_payload(record: &DatasourceImportRecord) -> Value {
    Value::Object(Map::from_iter(vec![
        ("name".to_string(), Value::String(record.name.clone())),
        (
            "type".to_string(),
            Value::String(record.datasource_type.clone()),
        ),
        ("url".to_string(), Value::String(record.url.clone())),
        ("access".to_string(), Value::String(record.access.clone())),
        ("uid".to_string(), Value::String(record.uid.clone())),
        ("isDefault".to_string(), Value::Bool(record.is_default)),
    ]))
}

fn parse_json_object_argument(
    value: Option<&str>,
    label: &str,
) -> Result<Option<Map<String, Value>>> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| message(format!("Invalid JSON for {label}: {error}")))?;
    let object = value
        .as_object()
        .cloned()
        .ok_or_else(|| message(format!("{label} must decode to a JSON object.")))?;
    Ok(Some(object))
}

fn build_add_payload(args: &DatasourceAddArgs) -> Result<Value> {
    let mut payload = Map::from_iter(vec![
        ("name".to_string(), Value::String(args.name.clone())),
        (
            "type".to_string(),
            Value::String(args.datasource_type.clone()),
        ),
    ]);
    if let Some(uid) = &args.uid {
        if !uid.trim().is_empty() {
            payload.insert("uid".to_string(), Value::String(uid.trim().to_string()));
        }
    }
    if let Some(access) = &args.access {
        if !access.trim().is_empty() {
            payload.insert(
                "access".to_string(),
                Value::String(access.trim().to_string()),
            );
        }
    }
    if let Some(url) = &args.datasource_url {
        if !url.trim().is_empty() {
            payload.insert("url".to_string(), Value::String(url.trim().to_string()));
        }
    }
    if args.is_default {
        payload.insert("isDefault".to_string(), Value::Bool(true));
    }
    if let Some(json_data) = parse_json_object_argument(args.json_data.as_deref(), "--json-data")? {
        payload.insert("jsonData".to_string(), Value::Object(json_data));
    }
    if let Some(secure_json_data) =
        parse_json_object_argument(args.secure_json_data.as_deref(), "--secure-json-data")?
    {
        payload.insert(
            "secureJsonData".to_string(),
            Value::Object(secure_json_data),
        );
    }
    Ok(Value::Object(payload))
}

fn build_modify_updates(args: &DatasourceModifyArgs) -> Result<Map<String, Value>> {
    let mut updates = Map::new();
    if let Some(url) = &args.set_url {
        if !url.trim().is_empty() {
            updates.insert("url".to_string(), Value::String(url.trim().to_string()));
        }
    }
    if let Some(access) = &args.set_access {
        if !access.trim().is_empty() {
            updates.insert(
                "access".to_string(),
                Value::String(access.trim().to_string()),
            );
        }
    }
    if let Some(is_default) = args.set_default {
        updates.insert("isDefault".to_string(), Value::Bool(is_default));
    }
    if let Some(json_data) = parse_json_object_argument(args.json_data.as_deref(), "--json-data")? {
        updates.insert("jsonData".to_string(), Value::Object(json_data));
    }
    if let Some(secure_json_data) =
        parse_json_object_argument(args.secure_json_data.as_deref(), "--secure-json-data")?
    {
        updates.insert(
            "secureJsonData".to_string(),
            Value::Object(secure_json_data),
        );
    }
    if updates.is_empty() {
        return Err(message(
            "Datasource modify requires at least one change flag.",
        ));
    }
    Ok(updates)
}

fn fetch_datasource_by_uid_if_exists(
    client: &JsonHttpClient,
    uid: &str,
) -> Result<Option<Map<String, Value>>> {
    match client.request_json(
        Method::GET,
        &format!("/api/datasources/uid/{uid}"),
        &[],
        None,
    ) {
        Ok(Some(value)) => value
            .as_object()
            .cloned()
            .map(Some)
            .ok_or_else(|| message(format!("Unexpected datasource payload for UID {uid}."))),
        Ok(None) => Ok(None),
        Err(error) if error.status_code() == Some(404) => Ok(None),
        Err(error) => Err(error),
    }
}

fn build_modify_payload(existing: &Map<String, Value>, updates: &Map<String, Value>) -> Value {
    let mut payload = Map::from_iter(vec![
        (
            "id".to_string(),
            existing.get("id").cloned().unwrap_or(Value::Null),
        ),
        (
            "uid".to_string(),
            Value::String(string_field(existing, "uid", "")),
        ),
        (
            "name".to_string(),
            Value::String(string_field(existing, "name", "")),
        ),
        (
            "type".to_string(),
            Value::String(string_field(existing, "type", "")),
        ),
        (
            "access".to_string(),
            Value::String(string_field(existing, "access", "")),
        ),
        (
            "url".to_string(),
            Value::String(string_field(existing, "url", "")),
        ),
        (
            "isDefault".to_string(),
            Value::Bool(
                existing
                    .get("isDefault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        ),
    ]);
    if let Some(database) = existing.get("database").cloned() {
        payload.insert("database".to_string(), database);
    }
    let merged_json_data = {
        let mut json_data = existing
            .get("jsonData")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        if let Some(update_json_data) = updates.get("jsonData").and_then(Value::as_object) {
            for (key, value) in update_json_data {
                json_data.insert(key.clone(), value.clone());
            }
        }
        json_data
    };
    if !merged_json_data.is_empty() {
        payload.insert("jsonData".to_string(), Value::Object(merged_json_data));
    }
    if let Some(secure_json_data) = updates.get("secureJsonData").and_then(Value::as_object) {
        if !secure_json_data.is_empty() {
            payload.insert(
                "secureJsonData".to_string(),
                Value::Object(secure_json_data.clone()),
            );
        }
    }
    for key in ["url", "access", "isDefault"] {
        if let Some(value) = updates.get(key).cloned() {
            payload.insert(key.to_string(), value);
        }
    }
    Value::Object(payload)
}

fn resolve_live_mutation_match(
    uid: Option<&str>,
    name: Option<&str>,
    live: &[Map<String, Value>],
) -> MatchResult {
    let normalized_uid = uid.unwrap_or("").trim();
    let normalized_name = name.unwrap_or("").trim();
    let uid_matches = if normalized_uid.is_empty() {
        Vec::new()
    } else {
        live.iter()
            .filter(|item| string_field(item, "uid", "") == normalized_uid)
            .collect::<Vec<&Map<String, Value>>>()
    };
    let name_matches = if normalized_name.is_empty() {
        Vec::new()
    } else {
        live.iter()
            .filter(|item| string_field(item, "name", "") == normalized_name)
            .collect::<Vec<&Map<String, Value>>>()
    };
    if uid_matches.len() > 1 {
        return MatchResult {
            destination: "ambiguous-uid",
            action: "would-fail-ambiguous-uid",
            target_uid: String::new(),
            target_name: normalized_name.to_string(),
            target_id: None,
        };
    }
    if uid_matches.len() == 1 {
        let item = uid_matches[0];
        let target_name = string_field(item, "name", "");
        if !normalized_name.is_empty() && target_name != normalized_name {
            return MatchResult {
                destination: "uid-name-mismatch",
                action: "would-fail-uid-name-mismatch",
                target_uid: string_field(item, "uid", ""),
                target_name,
                target_id: item.get("id").and_then(Value::as_i64),
            };
        }
        return MatchResult {
            destination: "exists-uid",
            action: "would-fail-existing-uid",
            target_uid: string_field(item, "uid", ""),
            target_name,
            target_id: item.get("id").and_then(Value::as_i64),
        };
    }
    if name_matches.len() > 1 {
        return MatchResult {
            destination: "ambiguous-name",
            action: "would-fail-ambiguous-name",
            target_uid: String::new(),
            target_name: normalized_name.to_string(),
            target_id: None,
        };
    }
    if name_matches.len() == 1 {
        let item = name_matches[0];
        let target_uid = string_field(item, "uid", "");
        if !normalized_uid.is_empty() && !target_uid.is_empty() && target_uid != normalized_uid {
            return MatchResult {
                destination: "uid-name-mismatch",
                action: "would-fail-uid-name-mismatch",
                target_uid,
                target_name: string_field(item, "name", ""),
                target_id: item.get("id").and_then(Value::as_i64),
            };
        }
        return MatchResult {
            destination: "exists-name",
            action: "would-fail-existing-name",
            target_uid,
            target_name: string_field(item, "name", ""),
            target_id: item.get("id").and_then(Value::as_i64),
        };
    }
    MatchResult {
        destination: "missing",
        action: "would-create",
        target_uid: String::new(),
        target_name: normalized_name.to_string(),
        target_id: None,
    }
}

fn resolve_delete_match(
    uid: Option<&str>,
    name: Option<&str>,
    live: &[Map<String, Value>],
) -> MatchResult {
    let matching = resolve_live_mutation_match(uid, name, live);
    match matching.destination {
        "exists-uid" | "exists-name" => MatchResult {
            action: "would-delete",
            ..matching
        },
        "missing" => MatchResult {
            action: "would-fail-missing",
            ..matching
        },
        _ => matching,
    }
}

fn render_live_mutation_table(rows: &[Vec<String>], include_header: bool) -> Vec<String> {
    let headers = vec![
        "OPERATION".to_string(),
        "UID".to_string(),
        "NAME".to_string(),
        "TYPE".to_string(),
        "MATCH".to_string(),
        "ACTION".to_string(),
        "TARGET_ID".to_string(),
    ];
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{:<width$}", value, width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<String>>();
    let mut lines = Vec::new();
    if include_header {
        lines.push(format_row(&headers));
        lines.push(format_row(&separator));
    }
    lines.extend(rows.iter().map(|row| format_row(row)));
    lines
}

fn render_live_mutation_json(rows: &[Vec<String>]) -> Value {
    let create_count = rows.iter().filter(|row| row[5] == "would-create").count();
    let update_count = rows.iter().filter(|row| row[5] == "would-update").count();
    let delete_count = rows.iter().filter(|row| row[5] == "would-delete").count();
    let blocked_count = rows
        .iter()
        .filter(|row| row[5].starts_with("would-fail-"))
        .count();
    Value::Object(Map::from_iter(vec![
        (
            "items".to_string(),
            Value::Array(
                rows.iter()
                    .map(|row| {
                        Value::Object(Map::from_iter(vec![
                            ("operation".to_string(), Value::String(row[0].clone())),
                            ("uid".to_string(), Value::String(row[1].clone())),
                            ("name".to_string(), Value::String(row[2].clone())),
                            ("type".to_string(), Value::String(row[3].clone())),
                            ("match".to_string(), Value::String(row[4].clone())),
                            ("action".to_string(), Value::String(row[5].clone())),
                            ("targetId".to_string(), Value::String(row[6].clone())),
                        ]))
                    })
                    .collect(),
            ),
        ),
        (
            "summary".to_string(),
            Value::Object(Map::from_iter(vec![
                (
                    "itemCount".to_string(),
                    Value::Number((rows.len() as i64).into()),
                ),
                (
                    "createCount".to_string(),
                    Value::Number((create_count as i64).into()),
                ),
                (
                    "updateCount".to_string(),
                    Value::Number((update_count as i64).into()),
                ),
                (
                    "deleteCount".to_string(),
                    Value::Number((delete_count as i64).into()),
                ),
                (
                    "blockedCount".to_string(),
                    Value::Number((blocked_count as i64).into()),
                ),
            ])),
        ),
    ]))
}

fn validate_live_mutation_dry_run_args(
    table: bool,
    json: bool,
    dry_run: bool,
    no_header: bool,
    verb: &str,
) -> Result<()> {
    if table && !dry_run {
        return Err(message(format!(
            "--table is only supported with --dry-run for datasource {verb}."
        )));
    }
    if json && !dry_run {
        return Err(message(format!(
            "--json is only supported with --dry-run for datasource {verb}."
        )));
    }
    if table && json {
        return Err(message(format!(
            "--table and --json are mutually exclusive for datasource {verb}."
        )));
    }
    if no_header && !table {
        return Err(message(format!(
            "--no-header is only supported with --dry-run --table for datasource {verb}."
        )));
    }
    Ok(())
}

fn render_import_table(
    rows: &[Vec<String>],
    include_header: bool,
    selected_columns: Option<&[String]>,
) -> Vec<String> {
    let columns = if let Some(selected) = selected_columns {
        selected
            .iter()
            .map(|column| match column.as_str() {
                "uid" => (0usize, "UID"),
                "name" => (1usize, "NAME"),
                "type" => (2usize, "TYPE"),
                "destination" => (3usize, "DESTINATION"),
                "action" => (4usize, "ACTION"),
                "org_id" => (5usize, "ORG_ID"),
                "file" => (6usize, "FILE"),
                _ => unreachable!("validated datasource import output column"),
            })
            .collect::<Vec<(usize, &str)>>()
    } else {
        vec![
            (0usize, "UID"),
            (1usize, "NAME"),
            (2usize, "TYPE"),
            (3usize, "DESTINATION"),
            (4usize, "ACTION"),
            (5usize, "ORG_ID"),
            (6usize, "FILE"),
        ]
    };
    let headers = columns
        .iter()
        .map(|(_, header)| header.to_string())
        .collect::<Vec<String>>();
    let mut widths: Vec<usize> = headers.iter().map(|item| item.len()).collect();
    for row in rows {
        for (index, (source_index, _)) in columns.iter().enumerate() {
            let value = &row[*source_index];
            widths[index] = widths[index].max(value.len());
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{:<width$}", value, width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let separator = widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<String>>();
    let mut lines = Vec::new();
    if include_header {
        lines.push(format_row(&headers));
        lines.push(format_row(&separator));
    }
    lines.extend(rows.iter().map(|row| {
        let values = columns
            .iter()
            .map(|(source_index, _)| row[*source_index].clone())
            .collect::<Vec<String>>();
        format_row(&values)
    }));
    lines
}

fn render_diff_identity(entry: &DatasourceDiffEntry) -> String {
    if let Some(record) = &entry.export_record {
        if !record.uid.is_empty() {
            return format!("uid={} name={}", record.uid, record.name);
        }
        return format!("name={}", record.name);
    }
    if let Some(record) = &entry.live_record {
        if !record.uid.is_empty() {
            return format!("uid={} name={}", record.uid, record.name);
        }
        return format!("name={}", record.name);
    }
    entry.key.clone()
}

fn print_datasource_diff_report(report: &DatasourceDiffReport) {
    for entry in &report.entries {
        let identity = render_diff_identity(entry);
        match entry.status {
            DatasourceDiffStatus::Matches => {
                println!("Diff same datasource {identity}");
            }
            DatasourceDiffStatus::Different => {
                let changed_fields = entry
                    .differences
                    .iter()
                    .map(|item| item.field)
                    .collect::<Vec<&str>>()
                    .join(",");
                println!("Diff different datasource {identity} fields={changed_fields}");
            }
            DatasourceDiffStatus::MissingInLive => {
                println!("Diff missing-live datasource {identity}");
            }
            DatasourceDiffStatus::MissingInExport => {
                println!("Diff extra-live datasource {identity}");
            }
            DatasourceDiffStatus::AmbiguousLiveMatch => {
                println!("Diff ambiguous-live datasource {identity}");
            }
        }
    }
}

pub(crate) fn diff_datasources_with_live(
    diff_dir: &Path,
    live: &[Map<String, Value>],
) -> Result<(usize, usize)> {
    let export_values = load_diff_record_values(diff_dir)?;
    let live_values = live
        .iter()
        .cloned()
        .map(Value::Object)
        .collect::<Vec<Value>>();
    let report = build_datasource_diff_report(
        &normalize_export_records(&export_values),
        &normalize_live_records(&live_values),
    );
    print_datasource_diff_report(&report);
    let difference_count = report.summary.compared_count - report.summary.matches_count;
    println!(
        "Diff checked {} datasource(s); {} difference(s) found.",
        report.summary.compared_count, difference_count
    );
    Ok((report.summary.compared_count, difference_count))
}

/// Datasource runtime entrypoint.
///
/// After command normalization, this function builds required clients, validates constraints
/// for output mode flags, and delegates execution to list/export/import/diff handlers.
pub fn run_datasource_cli(command: DatasourceGroupCommand) -> Result<()> {
    let command = normalize_datasource_group_command(command);
    match command {
        DatasourceGroupCommand::List(args) => {
            let client = build_http_client(&args.common)?;
            let datasources = list_datasources(&client)?;
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&render_data_source_json(&datasources))?
                );
            } else if args.csv {
                for line in render_data_source_csv(&datasources) {
                    println!("{line}");
                }
            } else {
                for line in render_data_source_table(&datasources, !args.no_header) {
                    println!("{line}");
                }
                println!();
                println!("Listed {} data source(s).", datasources.len());
            }
            Ok(())
        }
        DatasourceGroupCommand::Add(args) => {
            validate_live_mutation_dry_run_args(
                args.table,
                args.json,
                args.dry_run,
                args.no_header,
                "add",
            )?;
            let payload = build_add_payload(&args)?;
            let client = build_http_client(&args.common)?;
            let live = list_datasources(&client)?;
            let matching =
                resolve_live_mutation_match(args.uid.as_deref(), Some(&args.name), &live);
            let row = vec![
                "add".to_string(),
                args.uid.clone().unwrap_or_default(),
                args.name.clone(),
                args.datasource_type.clone(),
                matching.destination.to_string(),
                matching.action.to_string(),
                matching
                    .target_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
            ];
            if args.dry_run {
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&render_live_mutation_json(&[row]))?
                    );
                } else if args.table {
                    for line in render_live_mutation_table(&[row], !args.no_header) {
                        println!("{line}");
                    }
                    println!("Dry-run checked 1 datasource add request");
                } else {
                    println!(
                        "Dry-run datasource add uid={} name={} match={} action={}",
                        args.uid.clone().unwrap_or_default(),
                        args.name,
                        matching.destination,
                        matching.action
                    );
                    println!("Dry-run checked 1 datasource add request");
                }
                return Ok(());
            }
            if matching.action != "would-create" {
                return Err(message(format!(
                    "Datasource add blocked for name={} uid={}: destination={} action={}.",
                    args.name,
                    args.uid.clone().unwrap_or_default(),
                    matching.destination,
                    matching.action
                )));
            }
            client.request_json(Method::POST, "/api/datasources", &[], Some(&payload))?;
            println!(
                "Created datasource uid={} name={}",
                args.uid.unwrap_or_default(),
                args.name
            );
            Ok(())
        }
        DatasourceGroupCommand::Modify(args) => {
            validate_live_mutation_dry_run_args(
                args.table,
                args.json,
                args.dry_run,
                args.no_header,
                "modify",
            )?;
            let updates = build_modify_updates(&args)?;
            let client = build_http_client(&args.common)?;
            let existing = fetch_datasource_by_uid_if_exists(&client, &args.uid)?;
            let (action, destination, payload, name, datasource_type, target_id) =
                if let Some(existing) = existing {
                    let payload = build_modify_payload(&existing, &updates);
                    (
                        "would-update",
                        "exists-uid",
                        Some(payload),
                        string_field(&existing, "name", ""),
                        string_field(&existing, "type", ""),
                        existing.get("id").and_then(Value::as_i64),
                    )
                } else {
                    (
                        "would-fail-missing",
                        "missing",
                        None,
                        String::new(),
                        String::new(),
                        None,
                    )
                };
            let row = vec![
                "modify".to_string(),
                args.uid.clone(),
                name.clone(),
                datasource_type.clone(),
                destination.to_string(),
                action.to_string(),
                target_id.map(|id| id.to_string()).unwrap_or_default(),
            ];
            if args.dry_run {
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&render_live_mutation_json(&[row]))?
                    );
                } else if args.table {
                    for line in render_live_mutation_table(&[row], !args.no_header) {
                        println!("{line}");
                    }
                    println!("Dry-run checked 1 datasource modify request");
                } else {
                    println!(
                        "Dry-run datasource modify uid={} name={} match={} action={}",
                        args.uid, name, destination, action
                    );
                    println!("Dry-run checked 1 datasource modify request");
                }
                return Ok(());
            }
            if action != "would-update" {
                return Err(message(format!(
                    "Datasource modify blocked for uid={}: destination={} action={}.",
                    args.uid, destination, action
                )));
            }
            let payload =
                payload.ok_or_else(|| message("Datasource modify did not build a payload."))?;
            let target_id = target_id
                .ok_or_else(|| message("Datasource modify requires a live datasource id."))?;
            client.request_json(
                Method::PUT,
                &format!("/api/datasources/{target_id}"),
                &[],
                Some(&payload),
            )?;
            println!(
                "Modified datasource uid={} name={} id={}",
                args.uid, name, target_id
            );
            Ok(())
        }
        DatasourceGroupCommand::Delete(args) => {
            validate_live_mutation_dry_run_args(
                args.table,
                args.json,
                args.dry_run,
                args.no_header,
                "delete",
            )?;
            let client = build_http_client(&args.common)?;
            let live = list_datasources(&client)?;
            let matching = resolve_delete_match(args.uid.as_deref(), args.name.as_deref(), &live);
            let row = vec![
                "delete".to_string(),
                args.uid
                    .clone()
                    .or_else(|| {
                        if matching.target_uid.is_empty() {
                            None
                        } else {
                            Some(matching.target_uid.clone())
                        }
                    })
                    .unwrap_or_default(),
                args.name
                    .clone()
                    .unwrap_or_else(|| matching.target_name.clone()),
                String::new(),
                matching.destination.to_string(),
                matching.action.to_string(),
                matching
                    .target_id
                    .map(|id| id.to_string())
                    .unwrap_or_default(),
            ];
            if args.dry_run {
                if args.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&render_live_mutation_json(&[row]))?
                    );
                } else if args.table {
                    for line in render_live_mutation_table(&[row], !args.no_header) {
                        println!("{line}");
                    }
                    println!("Dry-run checked 1 datasource delete request");
                } else {
                    println!(
                        "Dry-run datasource delete uid={} name={} match={} action={}",
                        args.uid.clone().unwrap_or_default(),
                        args.name.clone().unwrap_or_default(),
                        matching.destination,
                        matching.action
                    );
                    println!("Dry-run checked 1 datasource delete request");
                }
                return Ok(());
            }
            if matching.action != "would-delete" {
                return Err(message(format!(
                    "Datasource delete blocked for uid={} name={}: destination={} action={}.",
                    args.uid.clone().unwrap_or_default(),
                    args.name.clone().unwrap_or_default(),
                    matching.destination,
                    matching.action
                )));
            }
            let target_id = matching
                .target_id
                .ok_or_else(|| message("Datasource delete requires a live datasource id."))?;
            client.request_json(
                Method::DELETE,
                &format!("/api/datasources/{target_id}"),
                &[],
                None,
            )?;
            println!(
                "Deleted datasource uid={} name={} id={}",
                if matching.target_uid.is_empty() {
                    args.uid.unwrap_or_default()
                } else {
                    matching.target_uid
                },
                if matching.target_name.is_empty() {
                    args.name.unwrap_or_default()
                } else {
                    matching.target_name
                },
                target_id
            );
            Ok(())
        }
        DatasourceGroupCommand::Export(args) => {
            if args.all_orgs {
                let context = build_auth_context(&args.common)?;
                if context.auth_mode != "basic" {
                    return Err(message(
                        "Datasource export with --all-orgs requires Basic auth (--basic-user / --basic-password).",
                    ));
                }
                let admin_client = build_http_client(&args.common)?;
                let mut total = 0usize;
                let mut org_count = 0usize;
                let mut root_items = Vec::new();
                for org in list_orgs(&admin_client)? {
                    let org_id = org
                        .get("id")
                        .and_then(Value::as_i64)
                        .ok_or_else(|| message("Grafana org list entry is missing numeric id."))?;
                    let org_client = build_http_client_for_org(&args.common, org_id)?;
                    let records = build_export_records(&org_client)?;
                    let scoped_output_dir = build_all_orgs_output_dir(&args.export_dir, &org);
                    let datasources_path = scoped_output_dir.join(DATASOURCE_EXPORT_FILENAME);
                    let index_path = scoped_output_dir.join("index.json");
                    let metadata_path = scoped_output_dir.join(EXPORT_METADATA_FILENAME);
                    if !args.dry_run {
                        write_json_file(
                            &datasources_path,
                            &Value::Array(records.clone().into_iter().map(Value::Object).collect()),
                            args.overwrite,
                        )?;
                        write_json_file(
                            &index_path,
                            &build_export_index(&records),
                            args.overwrite,
                        )?;
                        write_json_file(
                            &metadata_path,
                            &build_datasource_export_metadata(records.len()),
                            args.overwrite,
                        )?;
                    }
                    let summary_verb = if args.dry_run {
                        "Would export"
                    } else {
                        "Exported"
                    };
                    println!(
                        "{summary_verb} {} datasource(s). Datasources: {} Index: {} Manifest: {}",
                        records.len(),
                        datasources_path.display(),
                        index_path.display(),
                        metadata_path.display()
                    );
                    for item in build_export_index(&records)
                        .get("items")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        if let Some(object) = item.as_object() {
                            let mut entry = object.clone();
                            entry.insert(
                                "exportDir".to_string(),
                                Value::String(scoped_output_dir.display().to_string()),
                            );
                            root_items.push(entry);
                        }
                    }
                    total += records.len();
                    org_count += 1;
                }
                if !args.dry_run {
                    write_json_file(
                        &args.export_dir.join("index.json"),
                        &build_all_orgs_export_index(&root_items),
                        args.overwrite,
                    )?;
                    write_json_file(
                        &args.export_dir.join(EXPORT_METADATA_FILENAME),
                        &build_all_orgs_export_metadata(org_count, total),
                        args.overwrite,
                    )?;
                }
                println!(
                    "{} datasource(s) across {} exported org(s) under {}",
                    total,
                    org_count,
                    args.export_dir.display()
                );
                return Ok(());
            }
            let client = resolve_target_client(&args.common, args.org_id)?;
            export_datasource_scope(&client, &args.export_dir, args.overwrite, args.dry_run)?;
            Ok(())
        }
        DatasourceGroupCommand::Import(args) => {
            validate_import_org_auth(&args.common, &args)?;
            if args.table && !args.dry_run {
                return Err(message(
                    "--table is only supported with --dry-run for datasource import.",
                ));
            }
            if args.json && !args.dry_run {
                return Err(message(
                    "--json is only supported with --dry-run for datasource import.",
                ));
            }
            if args.table && args.json {
                return Err(message(
                    "--table and --json are mutually exclusive for datasource import.",
                ));
            }
            if args.no_header && !args.table {
                return Err(message(
                    "--no-header is only supported with --dry-run --table for datasource import.",
                ));
            }
            if !args.output_columns.is_empty() && !args.table {
                return Err(message(
                    "--output-columns is only supported with --dry-run --table or table-like --output-format for datasource import.",
                ));
            }
            if args.use_export_org {
                if !args.output_columns.is_empty() {
                    return Err(message(
                        "--output-columns is not supported with --use-export-org for datasource import.",
                    ));
                }
                import_datasources_by_export_org(&args)?;
                return Ok(());
            }
            let client = resolve_target_client(&args.common, args.org_id)?;
            import_datasources_with_client(&client, &args)?;
            Ok(())
        }
        DatasourceGroupCommand::Diff(args) => {
            let client = build_http_client(&args.common)?;
            let live = list_datasources(&client)?;
            let (compared_count, differences) = diff_datasources_with_live(&args.diff_dir, &live)?;
            if differences > 0 {
                return Err(message(format!(
                    "Found {} datasource difference(s) across {} exported datasource(s).",
                    differences, compared_count
                )));
            }
            println!(
                "No datasource differences across {} exported datasource(s).",
                compared_count
            );
            Ok(())
        }
    }
}

#[cfg(test)]
impl DatasourceCliArgs {
    // Parse helper used by tests to validate both clap parsing and normalization
    // results in one shot.
    fn parse_normalized_from<I, T>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let mut args = Self::parse_from(iter);
        normalize_output_formats(&mut args);
        args
    }
}

#[cfg(test)]
#[path = "datasource_rust_tests.rs"]
mod datasource_rust_tests;

#[cfg(test)]
#[path = "datasource_diff_rust_tests.rs"]
mod datasource_diff_rust_tests;
