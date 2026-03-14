use clap::{Args, Parser, Subcommand};
use reqwest::Method;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use crate::common::{
    load_json_object_file, message, string_field, write_json_file, Result,
};
use crate::dashboard::{
    build_auth_context, build_http_client, build_http_client_for_org, list_datasources,
    CommonCliArgs, DEFAULT_ORG_ID,
};
use crate::http::JsonHttpClient;

const DEFAULT_EXPORT_DIR: &str = "datasources";
const DATASOURCE_EXPORT_FILENAME: &str = "datasources.json";
const EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
const ROOT_INDEX_KIND: &str = "grafana-utils-datasource-export-index";
const TOOL_SCHEMA_VERSION: i64 = 1;

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
        help = "Import datasources into this Grafana org ID instead of the current org context. Requires Basic auth."
    )]
    pub org_id: Option<i64>,
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
        default_value_t = false,
        help = "For --dry-run --table only, omit the table header row."
    )]
    pub no_header: bool,
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

#[derive(Debug, Clone, Subcommand)]
pub enum DatasourceGroupCommand {
    #[command(about = "List live Grafana datasource inventory.")]
    List(DatasourceListArgs),
    #[command(about = "Export live Grafana datasource inventory as normalized JSON files.")]
    Export(DatasourceExportArgs),
    #[command(about = "Import datasource inventory through the Grafana API.")]
    Import(DatasourceImportArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-utils datasource",
    about = "List, export, and import Grafana datasources."
)]
pub struct DatasourceCliArgs {
    #[command(subcommand)]
    pub command: DatasourceGroupCommand,
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

fn fetch_current_org(client: &JsonHttpClient) -> Result<Map<String, Value>> {
    match client.request_json(Method::GET, "/api/org", &[], None)? {
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or_else(|| message("Unexpected current-org payload from Grafana.")),
        None => Err(message("Grafana did not return current-org metadata.")),
    }
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
        ("indexFile".to_string(), Value::String("index.json".to_string())),
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

fn render_data_source_table(datasources: &[Map<String, Value>], include_header: bool) -> Vec<String> {
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
        ("count".to_string(), Value::Number((records.len() as i64).into())),
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

fn parse_export_metadata(path: &PathBuf) -> Result<DatasourceExportMetadata> {
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

fn load_import_records(import_dir: &PathBuf) -> Result<(DatasourceExportMetadata, Vec<DatasourceImportRecord>)> {
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

fn collect_source_org_ids(import_dir: &PathBuf, metadata: &DatasourceExportMetadata) -> Result<BTreeSet<String>> {
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

fn validate_matching_export_org(
    client: &JsonHttpClient,
    args: &DatasourceImportArgs,
    import_dir: &PathBuf,
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
            source_org_ids.into_iter().collect::<Vec<String>>().join(", ")
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

fn resolve_match(record: &DatasourceImportRecord, live: &[Map<String, Value>], replace_existing: bool, update_existing_only: bool) -> MatchResult {
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
        return MatchResult {
            destination: "exists-name",
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
        ("type".to_string(), Value::String(record.datasource_type.clone())),
        ("url".to_string(), Value::String(record.url.clone())),
        ("access".to_string(), Value::String(record.access.clone())),
        ("uid".to_string(), Value::String(record.uid.clone())),
        ("isDefault".to_string(), Value::Bool(record.is_default)),
    ]))
}

fn render_import_table(rows: &[Vec<String>], include_header: bool) -> Vec<String> {
    let headers = vec![
        "UID".to_string(),
        "NAME".to_string(),
        "TYPE".to_string(),
        "DESTINATION".to_string(),
        "ACTION".to_string(),
        "ORG_ID".to_string(),
        "FILE".to_string(),
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

pub fn run_datasource_cli(command: DatasourceGroupCommand) -> Result<()> {
    match command {
        DatasourceGroupCommand::List(args) => {
            let client = build_http_client(&args.common)?;
            let datasources = list_datasources(&client)?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&render_data_source_json(&datasources))?);
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
        DatasourceGroupCommand::Export(args) => {
            let client = build_http_client(&args.common)?;
            let records = build_export_records(&client)?;
            let output_dir = args.export_dir;
            let datasources_path = output_dir.join(DATASOURCE_EXPORT_FILENAME);
            let index_path = output_dir.join("index.json");
            let metadata_path = output_dir.join(EXPORT_METADATA_FILENAME);
            if !args.dry_run {
                write_json_file(&datasources_path, &Value::Array(records.clone().into_iter().map(Value::Object).collect()), args.overwrite)?;
                write_json_file(&index_path, &build_export_index(&records), args.overwrite)?;
                write_json_file(&metadata_path, &build_datasource_export_metadata(records.len()), args.overwrite)?;
            }
            let summary_verb = if args.dry_run { "Would export" } else { "Exported" };
            println!(
                "{summary_verb} {} datasource(s). Datasources: {} Index: {} Manifest: {}",
                records.len(),
                datasources_path.display(),
                index_path.display(),
                metadata_path.display()
            );
            Ok(())
        }
        DatasourceGroupCommand::Import(args) => {
            if args.table && !args.dry_run {
                return Err(message("--table is only supported with --dry-run for datasource import."));
            }
            if args.json && !args.dry_run {
                return Err(message("--json is only supported with --dry-run for datasource import."));
            }
            if args.table && args.json {
                return Err(message("--table and --json are mutually exclusive for datasource import."));
            }
            if args.no_header && !args.table {
                return Err(message("--no-header is only supported with --dry-run --table for datasource import."));
            }
            let replace_existing = args.replace_existing || args.update_existing_only;
            let client = resolve_target_client(&args.common, args.org_id)?;
            let (metadata, records) = load_import_records(&args.import_dir)?;
            validate_matching_export_org(&client, &args, &args.import_dir, &metadata)?;
            let live = list_datasources(&client)?;
            let target_org = fetch_current_org(&client)?;
            let target_org_id = target_org
                .get("id")
                .map(|value| value.to_string())
                .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
            let mode = if args.update_existing_only {
                "update-or-skip-missing"
            } else if args.replace_existing {
                "create-or-update"
            } else {
                "create-only"
            };
            if !args.json {
                println!("Import mode: {mode}");
            }
            let mut dry_run_rows = Vec::new();
            let mut created = 0usize;
            let mut updated = 0usize;
            let mut skipped = 0usize;
            let mut blocked = 0usize;
            for (index, record) in records.iter().enumerate() {
                let matching = resolve_match(record, &live, replace_existing, args.update_existing_only);
                let file_ref = format!("{}#{}", metadata.datasources_file, index);
                if args.dry_run {
                    if args.table || args.json {
                        dry_run_rows.push(vec![
                            record.uid.clone(),
                            record.name.clone(),
                            record.datasource_type.clone(),
                            matching.destination.to_string(),
                            matching.action.to_string(),
                            target_org_id.clone(),
                            file_ref.clone(),
                        ]);
                    } else {
                        println!(
                            "Dry-run datasource uid={} name={} dest={} action={} file={}",
                            record.uid, record.name, matching.destination, matching.action, file_ref
                        );
                    }
                    match matching.action {
                        "would-create" => created += 1,
                        "would-update" => updated += 1,
                        "would-skip-missing" => skipped += 1,
                        _ => blocked += 1,
                    }
                    continue;
                }
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
                            if record.uid.is_empty() { &record.name } else { &record.uid },
                            matching.destination,
                            matching.action
                        )));
                    }
                }
            }
            if args.dry_run {
                if args.json {
                    let summary = Value::Object(Map::from_iter(vec![
                        ("datasourceCount".to_string(), Value::Number((records.len() as i64).into())),
                        ("wouldCreate".to_string(), Value::Number((created as i64).into())),
                        ("wouldUpdate".to_string(), Value::Number((updated as i64).into())),
                        ("wouldSkip".to_string(), Value::Number((skipped as i64).into())),
                        ("wouldBlock".to_string(), Value::Number((blocked as i64).into())),
                    ]));
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&Value::Object(Map::from_iter(vec![
                            ("mode".to_string(), Value::String(mode.to_string())),
                            (
                                "sourceOrgId".to_string(),
                                Value::String(
                                    records
                                        .iter()
                                        .find(|item| !item.org_id.is_empty())
                                        .map(|item| item.org_id.clone())
                                        .unwrap_or_default(),
                                ),
                            ),
                            ("targetOrgId".to_string(), Value::String(target_org_id)),
                            (
                                "datasources".to_string(),
                                Value::Array(
                                    dry_run_rows
                                        .into_iter()
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
                            ("summary".to_string(), summary),
                        ])))?
                    );
                } else if args.table {
                    for line in render_import_table(&dry_run_rows, !args.no_header) {
                        println!("{line}");
                    }
                    println!(
                        "Dry-run checked {} datasource(s) from {}",
                        records.len(),
                        args.import_dir.display()
                    );
                } else {
                    println!(
                        "Dry-run checked {} datasource(s) from {}",
                        records.len(),
                        args.import_dir.display()
                    );
                }
                return Ok(());
            }
            println!(
                "Imported {} datasource(s) from {}; updated {}, skipped {}, blocked {}",
                created + updated,
                args.import_dir.display(),
                updated,
                skipped,
                blocked
            );
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "datasource_rust_tests.rs"]
mod datasource_rust_tests;
