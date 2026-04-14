//! Inspection path for Dashboard resources: analysis, extraction, and report shaping.

#[path = "inspect_orchestration_input.rs"]
mod inspect_orchestration_input;

#[cfg(not(feature = "tui"))]
use std::io::Write;
use std::io::{self, IsTerminal};
use std::path::Path;

use crate::common::{message, Result};
#[cfg(feature = "tui")]
use crate::tui_shell;

#[cfg(feature = "tui")]
use super::super::browse_terminal::TerminalSession;
use super::super::cli_defs::{
    DashboardImportInputFormat, InspectExportArgs, InspectExportInputType,
    InspectExportReportFormat, InspectOutputFormat,
};
use super::super::files::DashboardSourceKind;
#[cfg(feature = "tui")]
use super::super::inspect_governance::build_export_inspection_governance_document;
use super::super::inspect_live::TempInspectDir;
use super::super::inspect_report::{
    refresh_filtered_query_report_summary, report_format_supports_columns,
    resolve_report_column_ids_for_format, ExportInspectionQueryReport,
};
#[cfg(feature = "tui")]
use super::super::inspect_workbench::run_inspect_workbench;
#[cfg(feature = "tui")]
use super::super::inspect_workbench_support::build_inspect_workbench_document;
use super::super::RAW_EXPORT_SUBDIR;
use super::inspect_output::{
    render_export_inspection_report_output, render_export_inspection_summary_output,
};
use super::inspect_query_report::build_export_inspection_query_report_for_variant;
use super::write_inspect_output;
pub(crate) use inspect_orchestration_input::ResolvedInspectExportInput;

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

fn map_output_format_to_report(
    output_format: InspectOutputFormat,
) -> Option<InspectExportReportFormat> {
    match output_format {
        InspectOutputFormat::Text
        | InspectOutputFormat::Table
        | InspectOutputFormat::Csv
        | InspectOutputFormat::Json
        | InspectOutputFormat::Yaml => None,
        InspectOutputFormat::Tree => Some(InspectExportReportFormat::Tree),
        InspectOutputFormat::TreeTable => Some(InspectExportReportFormat::TreeTable),
        InspectOutputFormat::Dependency => Some(InspectExportReportFormat::Dependency),
        InspectOutputFormat::DependencyJson => Some(InspectExportReportFormat::DependencyJson),
        InspectOutputFormat::Governance => Some(InspectExportReportFormat::Governance),
        InspectOutputFormat::GovernanceJson => Some(InspectExportReportFormat::GovernanceJson),
        InspectOutputFormat::QueriesJson => Some(InspectExportReportFormat::QueriesJson),
    }
}

pub(crate) fn effective_inspect_report_format(
    args: &InspectExportArgs,
) -> Option<InspectExportReportFormat> {
    args.output_format.and_then(map_output_format_to_report)
}

pub(crate) fn effective_inspect_output_format(args: &InspectExportArgs) -> InspectOutputFormat {
    args.output_format.unwrap_or({
        if args.text {
            InspectOutputFormat::Text
        } else if args.table {
            InspectOutputFormat::Table
        } else if args.csv {
            InspectOutputFormat::Csv
        } else if args.json {
            InspectOutputFormat::Json
        } else if args.yaml {
            InspectOutputFormat::Yaml
        } else {
            InspectOutputFormat::Text
        }
    })
}

pub(crate) fn resolve_inspect_export_import_dir(
    temp_root: &Path,
    input_dir: &Path,
    input_format: DashboardImportInputFormat,
    input_type: Option<InspectExportInputType>,
    interactive: bool,
) -> Result<ResolvedInspectExportInput> {
    inspect_orchestration_input::resolve_inspect_export_import_dir_with_prompt(
        temp_root,
        input_dir,
        input_format,
        input_type,
        interactive,
        prompt_interactive_input_type,
    )
}

#[cfg(any(test, not(feature = "tui")))]
fn parse_interactive_input_type_answer(answer: &str) -> Option<InspectExportInputType> {
    match answer.trim().to_ascii_lowercase().as_str() {
        "1" | "raw" | "r" => Some(InspectExportInputType::Raw),
        "2" | "source" | "s" | "prompt" | "p" => Some(InspectExportInputType::Source),
        _ => None,
    }
}

#[cfg(feature = "tui")]
fn centered_popup_rect(area: Rect, width: u16, height: u16) -> Rect {
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
fn render_interactive_loading_frame(
    frame: &mut ratatui::Frame<'_>,
    input_dir: &Path,
    expected_variant: &str,
    source_kind: Option<DashboardSourceKind>,
    active_step: usize,
) {
    let area = frame.area();
    frame.render_widget(Clear, area);
    let popup = centered_popup_rect(area, 88, 16);
    let inner = popup.inner(Margin {
        vertical: 1,
        horizontal: 2,
    });
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(7),
            Constraint::Length(3),
        ])
        .split(inner);

    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title("Inspect Export")
            .border_style(Style::default().fg(Color::LightBlue))
            .style(Style::default().bg(Color::Rgb(8, 12, 18))),
        popup,
    );

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                tui_shell::label("Stage "),
                tui_shell::accent("Preparing interactive workbench", Color::Cyan),
            ]),
            Line::from(vec![
                tui_shell::label("Source "),
                tui_shell::plain(input_dir.display().to_string()),
            ]),
            Line::from(vec![
                tui_shell::label("Variant "),
                match source_kind {
                    Some(DashboardSourceKind::RawExport) => {
                        tui_shell::key_chip("RAW", Color::Rgb(78, 161, 255))
                    }
                    Some(DashboardSourceKind::ProvisioningExport) => {
                        tui_shell::key_chip("PROVISIONING", Color::Rgb(73, 182, 133))
                    }
                    _ if expected_variant == RAW_EXPORT_SUBDIR => {
                        tui_shell::key_chip("RAW", Color::Rgb(78, 161, 255))
                    }
                    _ => tui_shell::key_chip("SOURCE", Color::Rgb(73, 182, 133)),
                },
            ]),
            Line::from("Building inspection artifacts before opening the interactive browser."),
        ])
        .wrap(Wrap { trim: false }),
        chunks[0],
    );

    let steps = [
        "Build summary",
        "Build query report",
        "Build governance review",
        "Launch inspect workbench",
    ];
    let items = steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let (marker, style, text_color) = if index < active_step {
                (
                    " DONE ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                    Color::White,
                )
            } else if index == active_step {
                (
                    " NOW  ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                    Color::White,
                )
            } else {
                (
                    " WAIT ",
                    Style::default().fg(Color::Black).bg(Color::DarkGray),
                    Color::Gray,
                )
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {marker} "), style),
                Span::raw(" "),
                Span::styled(
                    (*step).to_string(),
                    Style::default()
                        .fg(text_color)
                        .add_modifier(if index == active_step {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
            ]))
        })
        .collect::<Vec<ListItem>>();
    frame.render_widget(List::new(items), chunks[1]);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            tui_shell::label("Status "),
            tui_shell::plain(
                "Loading is automatic. The workbench opens when preparation completes.",
            ),
        ])),
        chunks[2],
    );
}

#[cfg(feature = "tui")]
fn draw_interactive_loading_step(
    session: &mut TerminalSession,
    input_dir: &Path,
    expected_variant: &str,
    source_kind: Option<DashboardSourceKind>,
    active_step: usize,
) -> Result<()> {
    session.terminal.draw(|frame| {
        render_interactive_loading_frame(
            frame,
            input_dir,
            expected_variant,
            source_kind,
            active_step,
        )
    })?;
    Ok(())
}

#[cfg(feature = "tui")]
// Interactive selector for dual input variant (raw/source) before opening inspect workbench.
fn run_interactive_input_type_selector(input_dir: &Path) -> Result<InspectExportInputType> {
    let mut session = TerminalSession::enter()?;
    let options = [
        (
            InspectExportInputType::Raw,
            "raw",
            "Inspect API-safe raw export artifacts",
        ),
        (
            InspectExportInputType::Source,
            "source",
            "Inspect prompt/source export artifacts",
        ),
    ];
    let mut selected = 0usize;

    loop {
        session.terminal.draw(|frame| {
            let area = frame.area();
            frame.render_widget(Clear, area);
            let popup = centered_popup_rect(area, 88, 17);
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
                    .title("Inspect export input")
                    .border_style(Style::default().fg(Color::LightBlue))
                    .style(Style::default().bg(Color::Black)),
                popup,
            );

            frame.render_widget(
                Paragraph::new(vec![
                    Line::from(vec![
                        tui_shell::label("Title "),
                        tui_shell::accent("Choose dashboard export variant", Color::Cyan),
                    ]),
                    Line::from(vec![
                        tui_shell::label("Import "),
                        tui_shell::plain(input_dir.display().to_string()),
                    ]),
                    Line::from(""),
                    Line::from(
                        "This dashboard export root contains both raw/ and prompt/ variants.",
                    ),
                    Line::from("Select one variant before continuing into the inspect workbench."),
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
                return Err(message("Interactive inspect selection cancelled."));
            }
            _ => {}
        }
    }
}

#[cfg(feature = "tui")]
fn prompt_interactive_input_type(input_dir: &Path) -> Result<InspectExportInputType> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(message(format!(
            "Import path {} contains both raw/ and prompt/ dashboard variants. Re-run with --input-type raw or --input-type source.",
            input_dir.display()
        )));
    }
    run_interactive_input_type_selector(input_dir)
}

#[cfg(not(feature = "tui"))]
fn prompt_interactive_input_type(input_dir: &Path) -> Result<InspectExportInputType> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(message(format!(
            "Import path {} contains both raw/ and prompt/ dashboard variants. Re-run with --input-type raw or --input-type source.",
            input_dir.display()
        )));
    }
    loop {
        println!("Title: Choose dashboard export variant");
        println!("Import: {}", input_dir.display());
        println!();
        println!("1. raw (Inspect API-safe raw export artifacts)");
        println!("2. source (Inspect prompt/source export artifacts)");
        print!("Choice [1-2/raw/source]: ");
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        if let Some(input_type) = parse_interactive_input_type_answer(&line) {
            return Ok(input_type);
        }
        eprintln!("Enter 1, 2, raw, or source.");
    }
}

pub(crate) fn apply_query_report_filters(
    mut report: ExportInspectionQueryReport,
    datasource_filter: Option<&str>,
    panel_id_filter: Option<&str>,
) -> ExportInspectionQueryReport {
    let datasource_filter = datasource_filter
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let panel_id_filter = panel_id_filter
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if datasource_filter.is_none() && panel_id_filter.is_none() {
        return report;
    }
    report.queries.retain(|row| {
        let datasource_match = datasource_filter
            .map(|value| {
                row.datasource == value
                    || row.datasource_uid == value
                    || row.datasource_type == value
                    || row.datasource_family == value
            })
            .unwrap_or(true);
        let panel_match = panel_id_filter
            .map(|value| row.panel_id == value)
            .unwrap_or(true);
        datasource_match && panel_match
    });
    refresh_filtered_query_report_summary(&mut report);
    report
}

pub(crate) fn validate_inspect_export_report_args(args: &InspectExportArgs) -> Result<()> {
    let report_format = effective_inspect_report_format(args);
    if report_format.is_none() {
        if !args.report_columns.is_empty() {
            return Err(message(
                "--report-columns is only supported together with table, csv, tree-table, or queries-json output.",
            ));
        }
        if args.report_filter_datasource.is_some() {
            return Err(message(
                "--report-filter-datasource is only supported together with table, csv, tree-table, dependency, dependency-json, governance, governance-json, or queries-json output.",
            ));
        }
        if args.report_filter_panel_id.is_some() {
            return Err(message(
                "--report-filter-panel-id is only supported together with table, csv, tree-table, dependency, dependency-json, governance, governance-json, or queries-json output.",
            ));
        }
        return Ok(());
    }
    if report_format
        .map(|format| !report_format_supports_columns(format))
        .unwrap_or(false)
        && !args.report_columns.is_empty()
    {
        return Err(message(
            "--report-columns is only supported with table, csv, or tree-table output.",
        ));
    }
    let _ = resolve_report_column_ids_for_format(report_format, &args.report_columns)?;
    Ok(())
}

fn analyze_export_dir_at_path(
    args: &InspectExportArgs,
    input_dir: &Path,
    expected_variant: &str,
    source_kind: Option<DashboardSourceKind>,
) -> Result<usize> {
    if args.interactive {
        return run_interactive_export_workbench(input_dir, expected_variant, source_kind);
    }
    let write_output = |output: &str| -> Result<()> {
        write_inspect_output(output, args.output_file.as_ref(), args.also_stdout)
    };

    if let Some(report_format) = effective_inspect_report_format(args) {
        let report = apply_query_report_filters(
            build_export_inspection_query_report_for_variant(input_dir, expected_variant)?,
            args.report_filter_datasource.as_deref(),
            args.report_filter_panel_id.as_deref(),
        );
        let rendered = render_export_inspection_report_output(
            args,
            input_dir,
            expected_variant,
            report_format,
            &report,
        )?;
        write_output(&rendered.output)?;
        return Ok(rendered.dashboard_count);
    }

    let summary =
        super::super::build_export_inspection_summary_for_variant(input_dir, expected_variant)?;
    let output = render_export_inspection_summary_output(args, &summary)?;
    write_output(&output)?;
    Ok(summary.dashboard_count)
}

#[cfg(feature = "tui")]
// Render export inspection in an interactive workbench; shared with non-interactive
// and local-mode call-sites via the same dashboard-count return contract.
fn run_interactive_export_workbench(
    input_dir: &Path,
    expected_variant: &str,
    source_kind: Option<DashboardSourceKind>,
) -> Result<usize> {
    let mut session = TerminalSession::enter()?;
    draw_interactive_loading_step(&mut session, input_dir, expected_variant, source_kind, 0)?;
    let summary =
        super::super::build_export_inspection_summary_for_variant(input_dir, expected_variant)?;
    draw_interactive_loading_step(&mut session, input_dir, expected_variant, source_kind, 1)?;
    let report = build_export_inspection_query_report_for_variant(input_dir, expected_variant)?;
    draw_interactive_loading_step(&mut session, input_dir, expected_variant, source_kind, 2)?;
    let governance = build_export_inspection_governance_document(&summary, &report);
    draw_interactive_loading_step(&mut session, input_dir, expected_variant, source_kind, 3)?;
    let document =
        build_inspect_workbench_document("export artifacts", &summary, &governance, &report);
    drop(session);
    run_inspect_workbench(document)?;
    Ok(summary.dashboard_count)
}

#[cfg(not(feature = "tui"))]
// Non-TUI path preserves signature by returning a feature-missing error.
fn run_interactive_export_workbench(
    _import_dir: &Path,
    _expected_variant: &str,
    _source_kind: Option<DashboardSourceKind>,
) -> Result<usize> {
    super::super::tui_not_built("summary-export --interactive")
}

pub(crate) fn analyze_export_dir(args: &InspectExportArgs) -> Result<usize> {
    validate_inspect_export_report_args(args)?;
    let temp_dir = TempInspectDir::new("summary-export")?;
    let resolved = resolve_inspect_export_import_dir(
        &temp_dir.path,
        &args.input_dir,
        args.input_format,
        args.input_type,
        args.interactive,
    )?;
    analyze_export_dir_at_path(
        args,
        &resolved.input_dir,
        resolved.expected_variant,
        resolved.source_kind,
    )
}

#[cfg(test)]
mod tests {
    use super::{parse_interactive_input_type_answer, InspectExportInputType};

    #[test]
    fn parse_interactive_input_type_answer_accepts_expected_aliases() {
        assert_eq!(
            parse_interactive_input_type_answer("raw"),
            Some(InspectExportInputType::Raw)
        );
        assert_eq!(
            parse_interactive_input_type_answer("r"),
            Some(InspectExportInputType::Raw)
        );
        assert_eq!(
            parse_interactive_input_type_answer("source"),
            Some(InspectExportInputType::Source)
        );
        assert_eq!(
            parse_interactive_input_type_answer("prompt"),
            Some(InspectExportInputType::Source)
        );
        assert_eq!(
            parse_interactive_input_type_answer("s"),
            Some(InspectExportInputType::Source)
        );
        assert_eq!(parse_interactive_input_type_answer(""), None);
    }
}
