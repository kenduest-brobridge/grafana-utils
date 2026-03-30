#![cfg(feature = "tui")]
// Specialized interactive TUI for dashboard governance findings review.
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
use crate::interactive_browser::BrowserItem;
use crate::tui_shell;

use super::governance_gate::{build_browser_item, finding_sort_key, DashboardGovernanceGateResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FindingsPane {
    Groups,
    Findings,
    Detail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GovernanceGateGroup {
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

fn finding_color(kind: &str) -> Color {
    match kind {
        "violation" => Color::LightRed,
        "warning" => Color::Yellow,
        _ => Color::Gray,
    }
}

pub(crate) fn build_governance_gate_tui_groups(
    result: &DashboardGovernanceGateResult,
) -> Vec<GovernanceGateGroup> {
    vec![
        GovernanceGateGroup {
            label: "All".to_string(),
            kind: "all".to_string(),
            count: result.violations.len() + result.warnings.len(),
            subtitle: "All findings".to_string(),
        },
        GovernanceGateGroup {
            label: "Violations".to_string(),
            kind: "violation".to_string(),
            count: result.violations.len(),
            subtitle: "Policy-breaking findings".to_string(),
        },
        GovernanceGateGroup {
            label: "Warnings".to_string(),
            kind: "warning".to_string(),
            count: result.warnings.len(),
            subtitle: "Advisory governance findings".to_string(),
        },
    ]
}

pub(crate) fn build_governance_gate_tui_items(
    result: &DashboardGovernanceGateResult,
    kind: &str,
) -> Vec<BrowserItem> {
    let mut violations = result.violations.iter().collect::<Vec<_>>();
    violations.sort_by_key(|record| finding_sort_key(record));
    let mut warnings = result.warnings.iter().collect::<Vec<_>>();
    warnings.sort_by_key(|record| finding_sort_key(record));

    let mut items = Vec::new();
    if kind == "all" || kind == "violation" {
        items.extend(
            violations
                .into_iter()
                .map(|record| build_browser_item("violation", record)),
        );
    }
    if kind == "all" || kind == "warning" {
        items.extend(
            warnings
                .into_iter()
                .map(|record| build_browser_item("warning", record)),
        );
    }
    items
}

pub(crate) fn run_governance_gate_interactive(
    result: &DashboardGovernanceGateResult,
) -> Result<()> {
    let groups = build_governance_gate_tui_groups(result);
    let mut group_state = ListState::default();
    group_state.select(Some(0));
    let mut finding_state = ListState::default();
    let mut items = build_governance_gate_tui_items(result, &groups[0].kind);
    finding_state.select((!items.is_empty()).then_some(0));
    let mut detail_scroll = 0u16;
    let mut active_pane = FindingsPane::Groups;
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
                Line::from(vec![
                    Span::styled(
                        format!("Outcome {}", if result.ok { "OK" } else { "FAIL" }),
                        Style::default()
                            .fg(if result.ok { Color::Green } else { Color::LightRed })
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("   "),
                    Span::raw(format!(
                        "dashboards={} queries={}",
                        result.summary.dashboard_count, result.summary.query_record_count
                    )),
                ]),
                Line::from(format!(
                    "violations={} warnings={}",
                    result.summary.violation_count, result.summary.warning_count
                )),
                Line::from(
                    "Use finding groups to focus policy-breaking issues first, then inspect the full scope and reason.",
                ),
            ];
            let summary_widget = Paragraph::new(summary_lines)
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL).title("Governance Gate"));
            frame.render_widget(summary_widget, outer[0]);

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
                                    .fg(finding_color(&group.kind))
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    })
                    .collect::<Vec<_>>(),
            )
            .block(pane_block("Finding Groups", active_pane == FindingsPane::Groups))
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
            let findings_title = groups
                .get(selected_group)
                .map(|group| format!("Findings {}/{}  {}", items.len(), groups[0].count, group.label))
                .unwrap_or_else(|| "Findings".to_string());
            let findings_list = List::new(
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
                                    .fg(finding_color(&item.kind))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(format!(" {}", item.title)),
                        ]))
                    })
                    .collect::<Vec<_>>(),
            )
            .block(pane_block(&findings_title, active_pane == FindingsPane::Findings))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_stateful_widget(findings_list, panes[1], &mut finding_state);

            let selected_item = finding_state.selected().and_then(|index| items.get(index));
            let detail_text = selected_item
                .map(|item| item.details.join("\n"))
                .unwrap_or_else(|| "No findings in this group.".to_string());
            let detail_total_lines = selected_item
                .map(|item| item.details.len().max(1))
                .unwrap_or(1);
            let detail_title = selected_item
                .map(|item| {
                    format!(
                        "Detail [{}/{}] {}  line {}/{}",
                        finding_state.selected().map(|index| index + 1).unwrap_or(0),
                        items.len(),
                        item.title,
                        (detail_scroll as usize + 1).min(detail_total_lines),
                        detail_total_lines
                    )
                })
                .unwrap_or_else(|| "Detail".to_string());
            let detail_widget = Paragraph::new(detail_text)
                .scroll((detail_scroll, 0))
                .wrap(Wrap { trim: false })
                .block(pane_block(&detail_title, active_pane == FindingsPane::Detail));
            frame.render_widget(detail_widget, panes[2]);

            frame.render_widget(
                tui_shell::build_footer_controls(vec![
                    Line::from(vec![
                        tui_shell::label("Focus "),
                        tui_shell::key_chip(
                            match active_pane {
                                FindingsPane::Groups => "Groups",
                                FindingsPane::Findings => "Findings",
                                FindingsPane::Detail => "Detail",
                            },
                            Color::Blue,
                        ),
                        Span::raw("  "),
                        tui_shell::label("Selection "),
                        tui_shell::accent(
                            format!(
                                "group {}/{}  finding {}/{}",
                                group_state.selected().map(|index| index + 1).unwrap_or(0),
                                groups.len(),
                                finding_state.selected().map(|index| index + 1).unwrap_or(0),
                                items.len()
                            ),
                            Color::White,
                        ),
                    ]),
                    tui_shell::control_line(&[
                        ("Tab", Color::Blue, "next pane"),
                        ("Up/Down", Color::Blue, "move"),
                        ("PgUp/PgDn", Color::Blue, "scroll detail"),
                        ("Home/End", Color::Blue, "jump"),
                    ]),
                    tui_shell::control_line(&[
                        ("Enter", Color::Blue, "reset detail"),
                        ("q/Esc", Color::Gray, "exit"),
                    ]),
                ]),
                outer[2],
            );
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
                        FindingsPane::Groups => FindingsPane::Findings,
                        FindingsPane::Findings => FindingsPane::Detail,
                        FindingsPane::Detail => FindingsPane::Groups,
                    };
                }
                KeyCode::Up => match active_pane {
                    FindingsPane::Groups => {
                        let selected = group_state.selected().unwrap_or(0).saturating_sub(1);
                        group_state.select(Some(selected));
                        items = build_governance_gate_tui_items(result, &groups[selected].kind);
                        finding_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    FindingsPane::Findings => {
                        let selected = finding_state.selected().unwrap_or(0).saturating_sub(1);
                        finding_state.select((!items.is_empty()).then_some(selected));
                        detail_scroll = 0;
                    }
                    FindingsPane::Detail => detail_scroll = detail_scroll.saturating_sub(1),
                },
                KeyCode::Down => match active_pane {
                    FindingsPane::Groups => {
                        let selected = (group_state.selected().unwrap_or(0) + 1)
                            .min(groups.len().saturating_sub(1));
                        group_state.select(Some(selected));
                        items = build_governance_gate_tui_items(result, &groups[selected].kind);
                        finding_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    FindingsPane::Findings => {
                        let selected = (finding_state.selected().unwrap_or(0) + 1)
                            .min(items.len().saturating_sub(1));
                        finding_state.select((!items.is_empty()).then_some(selected));
                        detail_scroll = 0;
                    }
                    FindingsPane::Detail => detail_scroll = detail_scroll.saturating_add(1),
                },
                KeyCode::PageUp => {
                    if active_pane == FindingsPane::Detail {
                        detail_scroll = detail_scroll.saturating_sub(10);
                    }
                }
                KeyCode::PageDown => {
                    if active_pane == FindingsPane::Detail {
                        detail_scroll = detail_scroll.saturating_add(10);
                    }
                }
                KeyCode::Home => match active_pane {
                    FindingsPane::Groups => {
                        group_state.select(Some(0));
                        items = build_governance_gate_tui_items(result, &groups[0].kind);
                        finding_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    FindingsPane::Findings => {
                        finding_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    FindingsPane::Detail => detail_scroll = 0,
                },
                KeyCode::End => match active_pane {
                    FindingsPane::Groups => {
                        let selected = groups.len().saturating_sub(1);
                        group_state.select(Some(selected));
                        items = build_governance_gate_tui_items(result, &groups[selected].kind);
                        finding_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    FindingsPane::Findings => {
                        finding_state
                            .select((!items.is_empty()).then_some(items.len().saturating_sub(1)));
                        detail_scroll = 0;
                    }
                    FindingsPane::Detail => {
                        detail_scroll = finding_state
                            .selected()
                            .and_then(|index| items.get(index))
                            .map(|item| item.details.len().saturating_sub(1) as u16)
                            .unwrap_or(0);
                    }
                },
                KeyCode::Enter => detail_scroll = 0,
                KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                _ => {}
            }
        }
    }
}
