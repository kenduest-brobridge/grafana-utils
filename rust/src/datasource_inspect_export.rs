//! Datasource local inventory support helpers.

use serde_json::{Map, Value};
#[cfg(not(feature = "tui"))]
use std::io::Write;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

#[cfg(any(feature = "tui", test))]
use crate::common::string_field;
use crate::common::{load_json_object_file, message, render_json_value, Result};
#[cfg(any(feature = "tui", test))]
use crate::interactive_browser::BrowserItem;
use crate::tabular_output::render_yaml;
#[cfg(feature = "tui")]
use crate::tui_shell;

#[cfg(feature = "tui")]
use super::datasource_browse_terminal::TerminalSession;
use super::{
    load_datasource_inventory_records_from_export_root, load_import_records,
    render_data_source_csv, render_data_source_table, resolve_datasource_export_root_dir,
    DatasourceImportInputFormat, DatasourceImportRecord, DATASOURCE_PROVISIONING_FILENAME,
    DATASOURCE_PROVISIONING_SUBDIR, EXPORT_METADATA_FILENAME,
};
#[cfg(feature = "tui")]
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
#[cfg(feature = "tui")]
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
#[cfg(feature = "tui")]
use ratatui::style::{Color, Modifier, Style};
#[cfg(feature = "tui")]
use ratatui::text::{Line, Span};
#[cfg(feature = "tui")]
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DatasourceInspectExportRenderFormat {
    #[cfg_attr(not(test), allow(dead_code))]
    Text,
    Table,
    Csv,
    Json,
    Yaml,
}

pub(crate) struct DatasourceInspectExportSource {
    pub(crate) input_mode: &'static str,
    pub(crate) input_path: String,
    pub(crate) metadata: Option<Value>,
    pub(crate) records: Vec<Map<String, Value>>,
}

fn build_datasource_inspect_export_metadata(mut metadata: Map<String, Value>) -> Value {
    metadata.insert(
        "bundleKind".to_string(),
        Value::String("masked-recovery".to_string()),
    );
    metadata.insert("masked".to_string(), Value::Bool(true));
    metadata.insert("recoveryCapable".to_string(), Value::Bool(true));
    Value::Object(metadata)
}

fn datasource_inspect_provisioning_candidates(input_dir: &Path) -> [PathBuf; 4] {
    [
        input_dir.join(DATASOURCE_PROVISIONING_FILENAME),
        input_dir.join("datasources.yml"),
        input_dir
            .join(DATASOURCE_PROVISIONING_SUBDIR)
            .join(DATASOURCE_PROVISIONING_FILENAME),
        input_dir
            .join(DATASOURCE_PROVISIONING_SUBDIR)
            .join("datasources.yml"),
    ]
}

fn has_datasource_inventory_export(input_dir: &Path) -> bool {
    input_dir.join(EXPORT_METADATA_FILENAME).is_file()
}

fn has_datasource_provisioning_export(input_dir: &Path) -> bool {
    datasource_inspect_provisioning_candidates(input_dir)
        .iter()
        .any(|candidate| candidate.is_file())
}

fn datasource_inspect_uses_tty() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

fn resolve_datasource_workspace_input_dir(input_dir: &Path) -> Result<PathBuf> {
    resolve_datasource_export_root_dir(input_dir).map_err(|error| {
        message(
            error
                .to_string()
                .replace("Datasource import", "Datasource list"),
        )
    })
}

pub(crate) fn resolve_datasource_inspect_export_input_format(
    input_dir: &Path,
    input_type: Option<DatasourceImportInputFormat>,
) -> Result<Option<DatasourceImportInputFormat>> {
    let input_dir = resolve_datasource_workspace_input_dir(input_dir)?;
    if let Some(input_type) = input_type {
        return Ok(Some(input_type));
    }
    if input_dir.is_file() {
        return Ok(Some(DatasourceImportInputFormat::Provisioning));
    }
    let has_inventory = has_datasource_inventory_export(&input_dir);
    let has_provisioning = has_datasource_provisioning_export(&input_dir);
    match (has_inventory, has_provisioning) {
        (true, true) => Ok(Some(prompt_datasource_inspect_export_input_format(
            &input_dir,
        )?)),
        (true, false) => Ok(Some(DatasourceImportInputFormat::Inventory)),
        (false, true) => Ok(Some(DatasourceImportInputFormat::Provisioning)),
        (false, false) => Ok(None),
    }
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn build_datasource_inspect_export_browser_items(
    source: &DatasourceInspectExportSource,
) -> Vec<BrowserItem> {
    source
        .records
        .iter()
        .map(|record| {
            let name = string_field(record, "name", "");
            let datasource_type = string_field(record, "type", "");
            let uid = string_field(record, "uid", "");
            let url = string_field(record, "url", "");
            let org = string_field(record, "org", "");
            let org_id = string_field(record, "orgId", "");
            let is_default = string_field(record, "isDefault", "");
            BrowserItem {
                kind: "datasource".to_string(),
                title: name.clone(),
                meta: format!(
                    "type={} uid={} org={} ({}) default={}",
                    datasource_type, uid, org, org_id, is_default
                ),
                details: vec![
                    format!("Name: {name}"),
                    format!("Type: {datasource_type}"),
                    format!("UID: {uid}"),
                    format!("URL: {url}"),
                    format!("Default: {is_default}"),
                    format!("Org: {org} ({org_id})"),
                    format!("Input mode: {}", source.input_mode),
                    format!("Input path: {}", source.input_path),
                ],
            }
        })
        .collect()
}

fn datasource_inspect_export_record(record: &DatasourceImportRecord) -> Map<String, Value> {
    record.to_inventory_record()
}

pub(crate) fn load_datasource_inspect_export_source(
    input_dir: &Path,
    input_format: DatasourceImportInputFormat,
) -> Result<DatasourceInspectExportSource> {
    let input_dir = resolve_datasource_workspace_input_dir(input_dir)?;
    if input_format == DatasourceImportInputFormat::Provisioning && input_dir.is_file() {
        let extension = input_dir
            .as_path()
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if !matches!(extension, "yaml" | "yml") {
            return Err(message(format!(
                "Datasource list local input file must be YAML (.yaml or .yml): {}",
                input_dir.display()
            )));
        }
        let (metadata, records) =
            load_import_records(&input_dir, DatasourceImportInputFormat::Provisioning)?;
        return Ok(DatasourceInspectExportSource {
            input_mode: "provisioning",
            input_path: input_dir.display().to_string(),
            metadata: Some(build_datasource_inspect_export_metadata(Map::from_iter(
                vec![
                    (
                        "inputFile".to_string(),
                        Value::String(input_dir.display().to_string()),
                    ),
                    (
                        "datasourcesFile".to_string(),
                        Value::String(input_dir.display().to_string()),
                    ),
                    (
                        "schemaVersion".to_string(),
                        Value::Number(metadata.schema_version.into()),
                    ),
                    ("kind".to_string(), Value::String(metadata.kind)),
                    ("variant".to_string(), Value::String(metadata.variant)),
                    ("resource".to_string(), Value::String(metadata.resource)),
                ],
            ))),
            records: records
                .into_iter()
                .map(|record| datasource_inspect_export_record(&record))
                .collect(),
        });
    }

    let metadata_path = input_dir.join(EXPORT_METADATA_FILENAME);
    if input_format == DatasourceImportInputFormat::Inventory && metadata_path.is_file() {
        let metadata = load_json_object_file(&metadata_path, "Datasource export metadata")?;
        let (_, records) = load_datasource_inventory_records_from_export_root(&input_dir)?;
        return Ok(DatasourceInspectExportSource {
            input_mode: "inventory",
            input_path: input_dir.display().to_string(),
            metadata: Some(build_datasource_inspect_export_metadata(
                metadata.as_object().cloned().ok_or_else(|| {
                    message(format!(
                        "Datasource export metadata must be a JSON object: {}",
                        metadata_path.display()
                    ))
                })?,
            )),
            records: records
                .into_iter()
                .map(|record| datasource_inspect_export_record(&record))
                .collect(),
        });
    }

    let provisioning_candidates = datasource_inspect_provisioning_candidates(&input_dir);
    if provisioning_candidates
        .iter()
        .any(|candidate| candidate.is_file())
    {
        let (metadata, records) =
            load_import_records(&input_dir, DatasourceImportInputFormat::Provisioning)?;
        return Ok(DatasourceInspectExportSource {
            input_mode: "provisioning",
            input_path: input_dir.display().to_string(),
            metadata: Some(build_datasource_inspect_export_metadata(Map::from_iter(
                vec![
                    (
                        "inputDir".to_string(),
                        Value::String(input_dir.display().to_string()),
                    ),
                    (
                        "datasourcesFile".to_string(),
                        Value::String(metadata.datasources_file),
                    ),
                    (
                        "schemaVersion".to_string(),
                        Value::Number(metadata.schema_version.into()),
                    ),
                    ("kind".to_string(), Value::String(metadata.kind)),
                    ("variant".to_string(), Value::String(metadata.variant)),
                    ("resource".to_string(), Value::String(metadata.resource)),
                ],
            ))),
            records: records
                .into_iter()
                .map(|record| datasource_inspect_export_record(&record))
                .collect(),
        });
    }

    Err(message(format!(
        "Datasource list could not find export-metadata.json or provisioning/datasources.yaml under {}.",
        input_dir.display()
    )))
}

#[cfg(feature = "tui")]
fn datasource_centered_popup_rect(area: Rect, width: u16, height: u16) -> Rect {
    let popup_width = area.width.saturating_sub(8).min(width).max(72);
    let popup_height = area.height.saturating_sub(4).min(height).max(12);
    Rect {
        x: area.x + area.width.saturating_sub(popup_width) / 2,
        y: area.y + area.height.saturating_sub(popup_height) / 2,
        width: popup_width,
        height: popup_height,
    }
}

#[cfg(feature = "tui")]
fn run_datasource_inspect_input_selector(input_dir: &Path) -> Result<DatasourceImportInputFormat> {
    let mut session = TerminalSession::enter()?;
    let options = [
        (
            DatasourceImportInputFormat::Inventory,
            "inventory",
            "Inspect datasource inventory export records",
        ),
        (
            DatasourceImportInputFormat::Provisioning,
            "provisioning",
            "Inspect provisioning datasource YAML",
        ),
    ];
    let mut selected = 0usize;
    loop {
        session.terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(Clear, area);
            let popup = datasource_centered_popup_rect(area, 88, 17);
            let inner = popup.inner(Margin {
                vertical: 1,
                horizontal: 3,
            });
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(7),
                    Constraint::Length(5),
                    Constraint::Min(1),
                    Constraint::Length(3),
                ])
                .split(inner);
            frame.render_widget(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Datasource list input")
                    .border_style(Style::default().fg(Color::LightBlue))
                    .style(Style::default().bg(Color::Black)),
                popup,
            );
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(vec![
                        tui_shell::label("Title "),
                        tui_shell::accent("Choose datasource local input mode", Color::Cyan),
                    ]),
                    Line::from(vec![
                        tui_shell::label("Input "),
                        tui_shell::plain(input_dir.display().to_string()),
                    ]),
                    Line::from(""),
                    Line::from(
                        "This path contains both datasource inventory and provisioning artifacts.",
                    ),
                    Line::from("Select one input mode before continuing into the browser."),
                ])
                .wrap(Wrap { trim: false }),
                chunks[0],
            );
            let items = options
                .iter()
                .enumerate()
                .map(|(index, (_, label, detail))| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{}. {label}", index + 1),
                            Style::default()
                                .fg(Color::LightCyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(format!("({detail})"), Style::default().fg(Color::White)),
                    ]))
                })
                .collect::<Vec<ListItem>>();
            let mut state = ListState::default().with_selected(Some(selected));
            frame.render_stateful_widget(
                List::new(items)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Options")
                            .border_style(Style::default().fg(Color::Gray)),
                    )
                    .highlight_symbol("   ")
                    .highlight_style(Style::default().bg(Color::Blue).fg(Color::Black)),
                chunks[1],
                &mut state,
            );
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(vec![
                        tui_shell::label("Choice "),
                        tui_shell::plain(format!("{}. {}", selected + 1, options[selected].1)),
                    ]),
                    Line::from(vec![
                        tui_shell::key_chip("Up/Down", Color::Blue),
                        Span::raw(" move  "),
                        tui_shell::key_chip("Enter", Color::Green),
                        Span::raw(" confirm  "),
                        tui_shell::key_chip("Esc/q", Color::DarkGray),
                        Span::raw(" cancel"),
                    ]),
                ]),
                chunks[3],
            );
        })?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                selected = selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                selected = (selected + 1).min(options.len().saturating_sub(1));
            }
            KeyCode::Enter => return Ok(options[selected].0),
            KeyCode::Esc | KeyCode::Char('q') => {
                return Err(message("Datasource list input selection cancelled."));
            }
            _ => {}
        }
    }
}

#[cfg(feature = "tui")]
pub(crate) fn prompt_datasource_inspect_export_input_format(
    input_dir: &Path,
) -> Result<DatasourceImportInputFormat> {
    if !datasource_inspect_uses_tty() {
        return Err(message(format!(
            "Datasource list found both inventory and provisioning artifacts under {}. Re-run with --input-format inventory or --input-format provisioning.",
            input_dir.display()
        )));
    }
    run_datasource_inspect_input_selector(input_dir)
}

#[cfg(not(feature = "tui"))]
pub(crate) fn prompt_datasource_inspect_export_input_format(
    input_dir: &Path,
) -> Result<DatasourceImportInputFormat> {
    if !datasource_inspect_uses_tty() {
        return Err(message(format!(
            "Datasource list found both inventory and provisioning artifacts under {}. Re-run with --input-format inventory or --input-format provisioning.",
            input_dir.display()
        )));
    }
    loop {
        println!("Title: Choose datasource local input mode");
        println!("Input: {}", input_dir.display());
        println!();
        println!("1. inventory (Inspect datasource inventory export records)");
        println!("2. provisioning (Inspect provisioning datasource YAML)");
        print!("Choice [1-2/inventory/provisioning]: ");
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let input_type = match line.trim().to_ascii_lowercase().as_str() {
            "1" | "inventory" | "i" => Some(DatasourceImportInputFormat::Inventory),
            "2" | "provisioning" | "p" | "yaml" | "yml" => {
                Some(DatasourceImportInputFormat::Provisioning)
            }
            _ => None,
        };
        if let Some(input_type) = input_type {
            return Ok(input_type);
        }
        eprintln!("Enter 1, 2, inventory, or provisioning.");
    }
}

fn build_datasource_inspect_export_document(source: &DatasourceInspectExportSource) -> Value {
    let mut document = Map::from_iter(vec![
        (
            "inputPath".to_string(),
            Value::String(source.input_path.clone()),
        ),
        (
            "inputMode".to_string(),
            Value::String(source.input_mode.to_string()),
        ),
        (
            "datasourceCount".to_string(),
            Value::Number((source.records.len() as i64).into()),
        ),
        (
            "datasources".to_string(),
            Value::Array(source.records.iter().cloned().map(Value::Object).collect()),
        ),
    ]);
    if let Some(metadata) = &source.metadata {
        document.insert("metadata".to_string(), metadata.clone());
    }
    Value::Object(document)
}

pub(crate) fn render_datasource_inspect_export_output(
    source: &DatasourceInspectExportSource,
    format: DatasourceInspectExportRenderFormat,
    selected_columns: Option<&[String]>,
) -> Result<String> {
    let mut output = String::new();
    let document = build_datasource_inspect_export_document(source);
    match format {
        DatasourceInspectExportRenderFormat::Text => {
            output.push_str(&format!("Datasource list: {}\n", source.input_path));
            output.push_str(&format!(
                "Layer: {}\n",
                datasource_inspect_export_output_layer(format)
            ));
            output.push_str(&format!("Mode: {}\n", source.input_mode));
            if let Some(metadata) = source.metadata.as_ref().and_then(Value::as_object) {
                if metadata
                    .get("masked")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    output.push_str("Bundle: recovery-capable masked export\n");
                }
                if let Some(kind) = metadata.get("kind").and_then(Value::as_str) {
                    output.push_str(&format!("Kind: {kind}\n"));
                }
                if let Some(variant) = metadata.get("variant").and_then(Value::as_str) {
                    output.push_str(&format!("Variant: {variant}\n"));
                }
                if let Some(resource) = metadata.get("resource").and_then(Value::as_str) {
                    output.push_str(&format!("Resource: {resource}\n"));
                }
                if let Some(datasources_file) =
                    metadata.get("datasourcesFile").and_then(Value::as_str)
                {
                    output.push_str(&format!("Datasources file: {datasources_file}\n"));
                }
            }
            output.push_str(&format!("Datasource count: {}\n", source.records.len()));
            output.push('\n');
            for line in render_data_source_table(&source.records, true, selected_columns) {
                output.push_str(&line);
                output.push('\n');
            }
        }
        DatasourceInspectExportRenderFormat::Table => {
            output.push_str(&format!("Datasource list: {}\n", source.input_path));
            output.push_str(&format!(
                "Layer: {}\n",
                datasource_inspect_export_output_layer(format)
            ));
            output.push_str(&format!("Mode: {}\n\n", source.input_mode));
            for line in render_data_source_table(&source.records, true, selected_columns) {
                output.push_str(&line);
                output.push('\n');
            }
        }
        DatasourceInspectExportRenderFormat::Csv => {
            for line in render_data_source_csv(&source.records, selected_columns) {
                output.push_str(&line);
                output.push('\n');
            }
        }
        DatasourceInspectExportRenderFormat::Json => {
            output.push_str(&render_json_value(&document)?);
        }
        DatasourceInspectExportRenderFormat::Yaml => {
            output.push_str(&render_yaml(&document)?);
        }
    }
    Ok(output)
}

fn datasource_inspect_export_output_layer(
    format: DatasourceInspectExportRenderFormat,
) -> &'static str {
    match format {
        DatasourceInspectExportRenderFormat::Text
        | DatasourceInspectExportRenderFormat::Table
        | DatasourceInspectExportRenderFormat::Csv => "operator-summary",
        DatasourceInspectExportRenderFormat::Json | DatasourceInspectExportRenderFormat::Yaml => {
            "full-contract"
        }
    }
}
