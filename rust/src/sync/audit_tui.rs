//! Specialized interactive TUI for sync audit drift triage.
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
use serde_json::Value;
use std::io::{self, Stdout};
use std::time::Duration;

use crate::common::{message, Result};
use crate::interactive_browser::BrowserItem;

use super::{
    sync_audit_drift_cmp, sync_audit_drift_details, sync_audit_drift_meta, sync_audit_drift_title,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AuditPane {
    Groups,
    Rows,
    Detail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuditGroup {
    pub(crate) label: String,
    pub(crate) status: String,
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

fn triage_color(status: &str) -> Color {
    match status {
        "missing-live" => Color::Red,
        "missing-lock" => Color::Yellow,
        "drift-detected" => Color::LightRed,
        "in-sync" => Color::Green,
        _ => Color::Gray,
    }
}

pub(crate) fn build_sync_audit_tui_groups(audit: &Value) -> Result<Vec<AuditGroup>> {
    let summary = audit
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Sync audit document is missing summary."))?;
    Ok(vec![
        AuditGroup {
            label: "All".to_string(),
            status: "all".to_string(),
            count: summary
                .get("driftCount")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize
                + summary
                    .get("missingLockCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize
                + summary
                    .get("missingLiveCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize,
            subtitle: "All triage rows".to_string(),
        },
        AuditGroup {
            label: "Missing Live".to_string(),
            status: "missing-live".to_string(),
            count: summary
                .get("missingLiveCount")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            subtitle: "Managed resources absent from live Grafana".to_string(),
        },
        AuditGroup {
            label: "Missing Lock".to_string(),
            status: "missing-lock".to_string(),
            count: summary
                .get("missingLockCount")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            subtitle: "Live resources outside the baseline lock".to_string(),
        },
        AuditGroup {
            label: "Drift".to_string(),
            status: "drift-detected".to_string(),
            count: summary
                .get("driftCount")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize,
            subtitle: "Managed resources changed since baseline".to_string(),
        },
    ])
}

pub(crate) fn build_sync_audit_tui_rows(audit: &Value, status: &str) -> Result<Vec<BrowserItem>> {
    let drifts = audit
        .get("drifts")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Sync audit document is missing drifts."))?;
    let mut rows = drifts.iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| sync_audit_drift_cmp(left, right));
    Ok(rows
        .into_iter()
        .filter(|row| status == "all" || row.get("status").and_then(Value::as_str) == Some(status))
        .map(|drift| BrowserItem {
            kind: drift
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("drift")
                .to_string(),
            title: sync_audit_drift_title(drift),
            meta: sync_audit_drift_meta(drift),
            details: sync_audit_drift_details(drift),
        })
        .collect())
}

pub(crate) fn run_sync_audit_interactive(audit: &Value) -> Result<()> {
    let summary = audit
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Sync audit document is missing summary."))?;
    let groups = build_sync_audit_tui_groups(audit)?;
    let mut group_state = ListState::default();
    group_state.select(Some(0));
    let mut row_state = ListState::default();
    let mut rows = build_sync_audit_tui_rows(audit, &groups[0].status)?;
    row_state.select((!rows.is_empty()).then_some(0));
    let mut detail_scroll = 0u16;
    let mut active_pane = AuditPane::Groups;
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
                    "Scope: managed={} baseline={} present={} missing={}",
                    summary.get("managedCount").and_then(Value::as_i64).unwrap_or(0),
                    summary.get("baselineCount").and_then(Value::as_i64).unwrap_or(0),
                    summary
                        .get("currentPresentCount")
                        .and_then(Value::as_i64)
                        .unwrap_or(0),
                    summary
                        .get("currentMissingCount")
                        .and_then(Value::as_i64)
                        .unwrap_or(0)
                )),
                Line::from(format!(
                    "Triage: drift={} in-sync={} missing-lock={} missing-live={}",
                    summary.get("driftCount").and_then(Value::as_i64).unwrap_or(0),
                    summary.get("inSyncCount").and_then(Value::as_i64).unwrap_or(0),
                    summary
                        .get("missingLockCount")
                        .and_then(Value::as_i64)
                        .unwrap_or(0),
                    summary
                        .get("missingLiveCount")
                        .and_then(Value::as_i64)
                        .unwrap_or(0)
                )),
                Line::from("Use triage groups to focus missing-live, missing-lock, and drift-detected rows."),
            ];
            let summary_widget = Paragraph::new(summary_lines)
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL).title("Sync Audit"));
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
                                    .fg(triage_color(&group.status))
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    })
                    .collect::<Vec<_>>(),
            )
            .block(pane_block("Triage Groups", active_pane == AuditPane::Groups))
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
            let row_title = groups
                .get(selected_group)
                .map(|group| format!("Rows {}/{}  {}", rows.len(), groups[0].count, group.label))
                .unwrap_or_else(|| "Rows".to_string());
            let row_list = List::new(
                rows.iter()
                    .enumerate()
                    .map(|(index, row)| {
                        ListItem::new(Line::from(vec![
                            Span::styled(
                                format!("{:>2}. ", index + 1),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled(
                                format!("[{}]", row.kind.to_uppercase()),
                                Style::default()
                                    .fg(triage_color(&row.kind))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(format!(" {}", row.title)),
                        ]))
                    })
                    .collect::<Vec<_>>(),
            )
            .block(pane_block(&row_title, active_pane == AuditPane::Rows))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_stateful_widget(row_list, panes[1], &mut row_state);

            let selected_row = row_state.selected().and_then(|index| rows.get(index));
            let detail_text = selected_row
                .map(|row| row.details.join("\n"))
                .unwrap_or_else(|| "No drift rows in this triage group.".to_string());
            let detail_total_lines = selected_row
                .map(|row| row.details.len().max(1))
                .unwrap_or(1);
            let detail_title = selected_row
                .map(|row| {
                    format!(
                        "Detail [{}/{}] {}  line {}/{}",
                        row_state.selected().map(|index| index + 1).unwrap_or(0),
                        rows.len(),
                        row.title,
                        (detail_scroll as usize + 1).min(detail_total_lines),
                        detail_total_lines
                    )
                })
                .unwrap_or_else(|| "Detail".to_string());
            let detail_widget = Paragraph::new(detail_text)
                .scroll((detail_scroll, 0))
                .wrap(Wrap { trim: false })
                .block(pane_block(&detail_title, active_pane == AuditPane::Detail));
            frame.render_widget(detail_widget, panes[2]);

            let footer = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        match active_pane {
                            AuditPane::Groups => "Focus: groups",
                            AuditPane::Rows => "Focus: rows",
                            AuditPane::Detail => "Focus: detail",
                        },
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("   "),
                    Span::raw(format!(
                        "Group {}/{}   Row {}/{}",
                        group_state.selected().map(|index| index + 1).unwrap_or(0),
                        groups.len(),
                        row_state.selected().map(|index| index + 1).unwrap_or(0),
                        rows.len()
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
                        AuditPane::Groups => AuditPane::Rows,
                        AuditPane::Rows => AuditPane::Detail,
                        AuditPane::Detail => AuditPane::Groups,
                    };
                }
                KeyCode::Up => match active_pane {
                    AuditPane::Groups => {
                        let selected = group_state.selected().unwrap_or(0).saturating_sub(1);
                        group_state.select(Some(selected));
                        rows = build_sync_audit_tui_rows(audit, &groups[selected].status)?;
                        row_state.select((!rows.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    AuditPane::Rows => {
                        let selected = row_state.selected().unwrap_or(0).saturating_sub(1);
                        row_state.select((!rows.is_empty()).then_some(selected));
                        detail_scroll = 0;
                    }
                    AuditPane::Detail => detail_scroll = detail_scroll.saturating_sub(1),
                },
                KeyCode::Down => match active_pane {
                    AuditPane::Groups => {
                        let selected = (group_state.selected().unwrap_or(0) + 1)
                            .min(groups.len().saturating_sub(1));
                        group_state.select(Some(selected));
                        rows = build_sync_audit_tui_rows(audit, &groups[selected].status)?;
                        row_state.select((!rows.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    AuditPane::Rows => {
                        let selected = (row_state.selected().unwrap_or(0) + 1)
                            .min(rows.len().saturating_sub(1));
                        row_state.select((!rows.is_empty()).then_some(selected));
                        detail_scroll = 0;
                    }
                    AuditPane::Detail => detail_scroll = detail_scroll.saturating_add(1),
                },
                KeyCode::PageUp => {
                    if active_pane == AuditPane::Detail {
                        detail_scroll = detail_scroll.saturating_sub(10);
                    }
                }
                KeyCode::PageDown => {
                    if active_pane == AuditPane::Detail {
                        detail_scroll = detail_scroll.saturating_add(10);
                    }
                }
                KeyCode::Home => match active_pane {
                    AuditPane::Groups => {
                        group_state.select(Some(0));
                        rows = build_sync_audit_tui_rows(audit, &groups[0].status)?;
                        row_state.select((!rows.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    AuditPane::Rows => {
                        row_state.select((!rows.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    AuditPane::Detail => detail_scroll = 0,
                },
                KeyCode::End => match active_pane {
                    AuditPane::Groups => {
                        let selected = groups.len().saturating_sub(1);
                        group_state.select(Some(selected));
                        rows = build_sync_audit_tui_rows(audit, &groups[selected].status)?;
                        row_state.select((!rows.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    AuditPane::Rows => {
                        row_state
                            .select((!rows.is_empty()).then_some(rows.len().saturating_sub(1)));
                        detail_scroll = 0;
                    }
                    AuditPane::Detail => {
                        detail_scroll = row_state
                            .selected()
                            .and_then(|index| rows.get(index))
                            .map(|row| row.details.len().saturating_sub(1) as u16)
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
