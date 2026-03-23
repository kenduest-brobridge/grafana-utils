//! Specialized interactive TUI for live dashboard inspection review.
#![cfg_attr(test, allow(dead_code))]
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::time::Duration;

use crate::common::Result;
use crate::dashboard::inspect_report::ExportInspectionQueryReport;
use crate::interactive_browser::BrowserItem;

use super::inspect_governance::ExportInspectionGovernanceDocument;
use super::ExportInspectionSummary;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectPane {
    Groups,
    Items,
    Detail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InspectLiveGroup {
    pub(crate) label: String,
    pub(crate) kind: String,
    pub(crate) count: usize,
    pub(crate) subtitle: String,
}

struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn pane_title(label: &str, active: bool) -> String {
    if active {
        format!("{label} [active]")
    } else {
        label.to_string()
    }
}

fn pane_block(label: &str, active: bool) -> Block<'static> {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .title(pane_title(label, active));
    if active {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }
    block
}

fn inspect_item_color(kind: &str) -> Color {
    match kind {
        "dashboard" | "dashboards" => Color::Yellow,
        "query" | "queries" => Color::Cyan,
        "risk" | "risks" | "dashboard-risk" | "risk-record" | "query-audit" => Color::LightRed,
        _ => Color::Gray,
    }
}

pub(crate) fn build_inspect_live_tui_groups(
    summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
    _report: &ExportInspectionQueryReport,
) -> Vec<InspectLiveGroup> {
    vec![
        InspectLiveGroup {
            label: "All".to_string(),
            kind: "all".to_string(),
            count: governance.dashboard_governance.len()
                + summary.query_count
                + governance
                    .dashboard_governance
                    .iter()
                    .filter(|row| row.risk_count != 0)
                    .count()
                + governance.risk_records.len()
                + governance.query_audits.len(),
            subtitle: "Dashboards, queries, and risks".to_string(),
        },
        InspectLiveGroup {
            label: "Dashboards".to_string(),
            kind: "dashboards".to_string(),
            count: governance.dashboard_governance.len(),
            subtitle: "Dashboard governance rollup".to_string(),
        },
        InspectLiveGroup {
            label: "Queries".to_string(),
            kind: "queries".to_string(),
            count: report_query_count(summary),
            subtitle: "Extracted query rows".to_string(),
        },
        InspectLiveGroup {
            label: "Risks".to_string(),
            kind: "risks".to_string(),
            count: governance
                .dashboard_governance
                .iter()
                .filter(|row| row.risk_count != 0)
                .count()
                + governance.risk_records.len()
                + governance.query_audits.len(),
            subtitle: "Governance risks requiring triage".to_string(),
        },
    ]
}

fn report_query_count(summary: &ExportInspectionSummary) -> usize {
    summary.query_count
}

pub(crate) fn filter_inspect_live_tui_items(
    _summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
    report: &ExportInspectionQueryReport,
    group_kind: &str,
) -> Vec<BrowserItem> {
    let mut items = Vec::new();
    if matches!(group_kind, "all" | "dashboards") {
        items.extend(
            governance
                .dashboard_governance
                .iter()
                .map(|row| BrowserItem {
                    kind: "dashboard".to_string(),
                    title: row.dashboard_title.clone(),
                    meta: format!(
                        "uid={} risks={} ds-families={}",
                        row.dashboard_uid, row.risk_count, row.datasource_family_count
                    ),
                    details: vec![
                        format!("Dashboard UID: {}", row.dashboard_uid),
                        format!("Title: {}", row.dashboard_title),
                        format!("Folder: {}", row.folder_path),
                        format!("Panels: {}", row.panel_count),
                        format!("Queries: {}", row.query_count),
                        format!("Datasources: {}", row.datasources.join(", ")),
                        format!("Families: {}", row.datasource_families.join(", ")),
                        format!("Mixed datasource: {}", row.mixed_datasource),
                        format!("Risk count: {}", row.risk_count),
                        format!("Risk kinds: {}", row.risk_kinds.join(", ")),
                    ],
                }),
        );
    }
    if matches!(group_kind, "all" | "queries") {
        items.extend(report.queries.iter().map(|row| BrowserItem {
            kind: "query".to_string(),
            title: format!("{} / {}", row.dashboard_title, row.panel_title),
            meta: format!(
                "{} {} panel={} metrics={}",
                row.datasource_family,
                row.ref_id,
                row.panel_id,
                row.metrics.len()
            ),
            details: vec![
                format!("Dashboard UID: {}", row.dashboard_uid),
                format!("Dashboard title: {}", row.dashboard_title),
                format!("Folder: {}", row.folder_path),
                format!("Panel ID: {}", row.panel_id),
                format!("Panel title: {}", row.panel_title),
                format!("Panel type: {}", row.panel_type),
                format!("Ref ID: {}", row.ref_id),
                format!("Datasource: {}", row.datasource_name),
                format!("Datasource UID: {}", row.datasource_uid),
                format!("Datasource family: {}", row.datasource_family),
                format!("Query field: {}", row.query_field),
                format!("Metrics: {}", row.metrics.join(", ")),
                format!("Functions: {}", row.functions.join(", ")),
                format!("Measurements: {}", row.measurements.join(", ")),
                format!("Buckets: {}", row.buckets.join(", ")),
                format!("Variables: {}", row.query_variables.join(", ")),
                String::new(),
                format!("Query: {}", row.query_text),
            ],
        }));
    }
    if matches!(group_kind, "all" | "risks") {
        items.extend(
            governance
                .dashboard_governance
                .iter()
                .filter(|row| row.risk_count != 0)
                .map(|row| BrowserItem {
                    kind: "dashboard-risk".to_string(),
                    title: row.dashboard_title.clone(),
                    meta: format!("uid={} risks={}", row.dashboard_uid, row.risk_count),
                    details: vec![
                        format!("Dashboard UID: {}", row.dashboard_uid),
                        format!("Title: {}", row.dashboard_title),
                        format!("Folder: {}", row.folder_path),
                        format!("Risk count: {}", row.risk_count),
                        format!("Risk kinds: {}", row.risk_kinds.join(", ")),
                    ],
                }),
        );
        let mut risks = governance.risk_records.clone();
        risks.sort_by(|left, right| {
            right
                .severity
                .cmp(&left.severity)
                .then_with(|| left.dashboard_uid.cmp(&right.dashboard_uid))
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.panel_id.cmp(&right.panel_id))
        });
        items.extend(risks.into_iter().map(|risk| BrowserItem {
            kind: "risk-record".to_string(),
            title: format!("{} / {}", risk.dashboard_uid, risk.kind),
            meta: format!("severity={} panel={}", risk.severity, risk.panel_id),
            details: vec![
                format!("Kind: {}", risk.kind),
                format!("Severity: {}", risk.severity),
                format!("Category: {}", risk.category),
                format!("Dashboard UID: {}", risk.dashboard_uid),
                format!("Panel ID: {}", risk.panel_id),
                format!("Datasource: {}", risk.datasource),
                format!("Detail: {}", risk.detail),
                format!("Recommendation: {}", risk.recommendation),
            ],
        }));
        items.extend(governance.query_audits.iter().map(|audit| BrowserItem {
            kind: "query-audit".to_string(),
            title: format!(
                "{} / {} / {}",
                audit.dashboard_title, audit.panel_title, audit.ref_id
            ),
            meta: format!("severity={} score={}", audit.severity, audit.score),
            details: vec![
                format!("Dashboard UID: {}", audit.dashboard_uid),
                format!("Dashboard title: {}", audit.dashboard_title),
                format!("Panel ID: {}", audit.panel_id),
                format!("Panel title: {}", audit.panel_title),
                format!("Ref ID: {}", audit.ref_id),
                format!("Datasource: {}", audit.datasource),
                format!("Datasource UID: {}", audit.datasource_uid),
                format!("Datasource family: {}", audit.datasource_family),
                format!("Aggregation depth: {}", audit.aggregation_depth),
                format!("Regex matcher count: {}", audit.regex_matcher_count),
                format!("Estimated series risk: {}", audit.estimated_series_risk),
                format!("Query cost score: {}", audit.query_cost_score),
                format!("Score: {}", audit.score),
                format!("Severity: {}", audit.severity),
                format!("Reasons: {}", audit.reasons.join(", ")),
                format!("Recommendations: {}", audit.recommendations.join(", ")),
            ],
        }));
    }
    items
}

pub(crate) fn run_inspect_live_interactive(
    summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
    report: &ExportInspectionQueryReport,
) -> Result<()> {
    let groups = build_inspect_live_tui_groups(summary, governance, report);
    let mut group_state = ListState::default();
    group_state.select(Some(0));
    let mut item_state = ListState::default();
    let mut items = filter_inspect_live_tui_items(summary, governance, report, &groups[0].kind);
    item_state.select((!items.is_empty()).then_some(0));
    let mut detail_scroll = 0u16;
    let mut active_pane = InspectPane::Groups;
    let mut session = TerminalSession::enter()?;

    loop {
        session.terminal.draw(|frame| {
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(5), Constraint::Min(1), Constraint::Length(4)])
                .split(frame.area());
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(22),
                    Constraint::Percentage(33),
                    Constraint::Percentage(45),
                ])
                .split(outer[1]);

            let summary_lines = vec![
                Line::from(format!(
                    "Dashboards={} panels={} queries={} datasource-families={} risk-records={}",
                    summary.dashboard_count,
                    summary.panel_count,
                    summary.query_count,
                    governance.summary.datasource_family_count,
                    governance.summary.risk_record_count
                )),
                Line::from(format!(
                    "dashboard-risk-coverage={} datasource-risk-coverage={}",
                    governance.summary.dashboard_risk_coverage_count,
                    governance.summary.datasource_risk_coverage_count
                )),
                Line::from(
                    "Use groups to pivot between dashboard rollups, extracted queries, and governance risks.",
                ),
            ];
            frame.render_widget(
                Paragraph::new(summary_lines)
                    .wrap(Wrap { trim: false })
                    .block(Block::default().borders(Borders::ALL).title("Inspect Live Summary")),
                outer[0],
            );

            let group_list = List::new(
                groups
                    .iter()
                    .map(|group| {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("{:>2} ", group.count),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled(
                                group.label.clone(),
                                Style::default()
                                    .fg(inspect_item_color(&group.kind))
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    })
                    .collect::<Vec<_>>(),
            )
            .block(pane_block("Groups", active_pane == InspectPane::Groups))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_stateful_widget(group_list, panes[0], &mut group_state);

            let selected_group = group_state.selected().unwrap_or(0);
            let item_title = if let Some(group) = groups.get(selected_group) {
                format!("Items {}/{}  {}", items.len(), group.count, group.label)
            } else {
                "Items".to_string()
            };
            let item_list = List::new(
                items.iter()
                    .enumerate()
                    .map(|(index, item)| {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("{:>2}. ", index + 1),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled(
                                format!("[{}]", item.kind.to_uppercase()),
                                Style::default()
                                    .fg(inspect_item_color(&item.kind))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(format!(" {}", item.title)),
                        ]))
                    })
                    .collect::<Vec<_>>(),
            )
            .block(pane_block(&item_title, active_pane == InspectPane::Items))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_stateful_widget(item_list, panes[1], &mut item_state);

            let selected_item = item_state.selected().and_then(|index| items.get(index));
            let detail_text = selected_item
                .map(|item| item.details.join("\n"))
                .unwrap_or_else(|| "No item in this group.".to_string());
            let detail_total_lines = selected_item
                .map(|item| item.details.len().max(1))
                .unwrap_or(1);
            let detail_title = selected_item
                .map(|item| {
                    format!(
                        "Detail [{}/{}] {}  line {}/{}",
                        item_state.selected().map(|index| index + 1).unwrap_or(0),
                        items.len(),
                        item.title,
                        (detail_scroll as usize + 1).min(detail_total_lines),
                        detail_total_lines
                    )
                })
                .unwrap_or_else(|| "Detail".to_string());
            frame.render_widget(
                Paragraph::new(detail_text)
                    .scroll((detail_scroll, 0))
                    .wrap(Wrap { trim: false })
                    .block(pane_block(&detail_title, active_pane == InspectPane::Detail)),
                panes[2],
            );

            let footer = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        match active_pane {
                            InspectPane::Groups => "Focus: groups",
                            InspectPane::Items => "Focus: items",
                            InspectPane::Detail => "Focus: detail",
                        },
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("   "),
                    Span::raw(format!(
                        "Group {}/{}   Item {}/{}",
                        group_state.selected().map(|index| index + 1).unwrap_or(0),
                        groups.len(),
                        item_state.selected().map(|index| index + 1).unwrap_or(0),
                        items.len()
                    )),
                ]),
                Line::from(
                    "Tab next pane  Up/Down move active pane  PgUp/PgDn detail jump  Home/End bounds"
                        .to_string(),
                ),
                Line::from("Enter reset detail  q/Esc exit".to_string()),
            ])
            .block(Block::default().borders(Borders::ALL).title("Controls"));
            frame.render_widget(footer, outer[2]);
        })?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Tab => {
                    active_pane = match active_pane {
                        InspectPane::Groups => InspectPane::Items,
                        InspectPane::Items => InspectPane::Detail,
                        InspectPane::Detail => InspectPane::Groups,
                    };
                }
                KeyCode::Up => match active_pane {
                    InspectPane::Groups => {
                        let selected = group_state.selected().unwrap_or(0).saturating_sub(1);
                        group_state.select(Some(selected));
                        items = filter_inspect_live_tui_items(
                            summary,
                            governance,
                            report,
                            &groups[selected].kind,
                        );
                        item_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    InspectPane::Items => {
                        let selected = item_state.selected().unwrap_or(0).saturating_sub(1);
                        item_state.select((!items.is_empty()).then_some(selected));
                        detail_scroll = 0;
                    }
                    InspectPane::Detail => detail_scroll = detail_scroll.saturating_sub(1),
                },
                KeyCode::Down => match active_pane {
                    InspectPane::Groups => {
                        let selected = (group_state.selected().unwrap_or(0) + 1)
                            .min(groups.len().saturating_sub(1));
                        group_state.select(Some(selected));
                        items = filter_inspect_live_tui_items(
                            summary,
                            governance,
                            report,
                            &groups[selected].kind,
                        );
                        item_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    InspectPane::Items => {
                        let selected = (item_state.selected().unwrap_or(0) + 1)
                            .min(items.len().saturating_sub(1));
                        item_state.select((!items.is_empty()).then_some(selected));
                        detail_scroll = 0;
                    }
                    InspectPane::Detail => detail_scroll = detail_scroll.saturating_add(1),
                },
                KeyCode::PageUp => {
                    if active_pane == InspectPane::Detail {
                        detail_scroll = detail_scroll.saturating_sub(10);
                    }
                }
                KeyCode::PageDown => {
                    if active_pane == InspectPane::Detail {
                        detail_scroll = detail_scroll.saturating_add(10);
                    }
                }
                KeyCode::Home => match active_pane {
                    InspectPane::Groups => {
                        group_state.select(Some(0));
                        items = filter_inspect_live_tui_items(
                            summary,
                            governance,
                            report,
                            &groups[0].kind,
                        );
                        item_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    InspectPane::Items => {
                        item_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    InspectPane::Detail => detail_scroll = 0,
                },
                KeyCode::End => match active_pane {
                    InspectPane::Groups => {
                        let selected = groups.len().saturating_sub(1);
                        group_state.select(Some(selected));
                        items = filter_inspect_live_tui_items(
                            summary,
                            governance,
                            report,
                            &groups[selected].kind,
                        );
                        item_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    InspectPane::Items => {
                        item_state
                            .select((!items.is_empty()).then_some(items.len().saturating_sub(1)));
                        detail_scroll = 0;
                    }
                    InspectPane::Detail => {
                        detail_scroll =
                            selected_item_max_scroll(&items, item_state.selected()) as u16;
                    }
                },
                KeyCode::Enter => detail_scroll = 0,
                KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                _ => {}
            }
        }
    }
}

fn selected_item_max_scroll(items: &[BrowserItem], selected: Option<usize>) -> usize {
    selected
        .and_then(|index| items.get(index))
        .map(|item| item.details.len().saturating_sub(1))
        .unwrap_or(0)
}
