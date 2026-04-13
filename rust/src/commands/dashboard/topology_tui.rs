#![cfg(feature = "tui")]
// Specialized interactive TUI for dashboard topology exploration.
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

use super::topology::{build_topology_browser_items, TopologyDocument};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TopologyPane {
    Groups,
    Nodes,
    Detail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TopologyGroup {
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
        .title(pane_title(label, active))
        .title_style(
            Style::default()
                .fg(if active { Color::Black } else { Color::White })
                .bg(if active { Color::Cyan } else { Color::Reset })
                .add_modifier(Modifier::BOLD),
        );
    if active {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }
    block
}

fn node_kind_color(kind: &str) -> Color {
    match kind {
        "datasource" => Color::Cyan,
        "dashboard" => Color::Yellow,
        "panel" => Color::Blue,
        "variable" => Color::Green,
        "alert-rule" => Color::LightRed,
        "contact-point" => Color::LightGreen,
        "mute-timing" => Color::LightMagenta,
        "notification-policy" => Color::Magenta,
        "template" => Color::LightCyan,
        _ => Color::Gray,
    }
}

pub(crate) fn build_topology_tui_groups(document: &TopologyDocument) -> Vec<TopologyGroup> {
    vec![
        TopologyGroup {
            label: "All".to_string(),
            kind: "all".to_string(),
            count: document.summary.node_count,
            subtitle: "Full graph".to_string(),
        },
        TopologyGroup {
            label: "Datasources".to_string(),
            kind: "datasource".to_string(),
            count: document.summary.datasource_count,
            subtitle: "Datasource entry points".to_string(),
        },
        TopologyGroup {
            label: "Dashboards".to_string(),
            kind: "dashboard".to_string(),
            count: document.summary.dashboard_count,
            subtitle: "Dashboard owners of graph branches".to_string(),
        },
        TopologyGroup {
            label: "Panels".to_string(),
            kind: "panel".to_string(),
            count: document.summary.panel_count,
            subtitle: "Panel-level render surfaces".to_string(),
        },
        TopologyGroup {
            label: "Variables".to_string(),
            kind: "variable".to_string(),
            count: document.summary.variable_count,
            subtitle: "Templating pivots".to_string(),
        },
        TopologyGroup {
            label: "Alert Rules".to_string(),
            kind: "alert-rule".to_string(),
            count: document.summary.alert_rule_count,
            subtitle: "Alert rule nodes".to_string(),
        },
        TopologyGroup {
            label: "Contact Points".to_string(),
            kind: "contact-point".to_string(),
            count: document.summary.contact_point_count,
            subtitle: "Downstream notification endpoints".to_string(),
        },
        TopologyGroup {
            label: "Mute Timings".to_string(),
            kind: "mute-timing".to_string(),
            count: document.summary.mute_timing_count,
            subtitle: "Mute timing resources".to_string(),
        },
        TopologyGroup {
            label: "Policies".to_string(),
            kind: "notification-policy".to_string(),
            count: document.summary.notification_policy_count,
            subtitle: "Routing policy nodes".to_string(),
        },
        TopologyGroup {
            label: "Templates".to_string(),
            kind: "template".to_string(),
            count: document.summary.template_count,
            subtitle: "Notification templates".to_string(),
        },
        TopologyGroup {
            label: "Alert Resources".to_string(),
            kind: "alert-resource".to_string(),
            count: document.summary.alert_resource_count.saturating_sub(
                document.summary.alert_rule_count
                    + document.summary.contact_point_count
                    + document.summary.mute_timing_count
                    + document.summary.notification_policy_count
                    + document.summary.template_count,
            ),
            subtitle: "Other alert-plane resources".to_string(),
        },
    ]
}

pub(crate) fn filter_topology_tui_items(
    document: &TopologyDocument,
    group_kind: &str,
) -> Vec<BrowserItem> {
    build_topology_browser_items(document)
        .into_iter()
        .filter(|item| group_kind == "all" || item.kind == group_kind)
        .collect()
}

pub(crate) fn run_topology_interactive(document: &TopologyDocument) -> Result<()> {
    let groups = build_topology_tui_groups(document);
    let mut group_state = ListState::default();
    group_state.select(Some(0));
    let mut node_state = ListState::default();
    let mut items = filter_topology_tui_items(document, &groups[0].kind);
    node_state.select((!items.is_empty()).then_some(0));
    let mut detail_scroll = 0u16;
    let mut active_pane = TopologyPane::Groups;
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
                    "Nodes={} edges={} dashboards={} datasources={} panels={} variables={}",
                    document.summary.node_count,
                    document.summary.edge_count,
                    document.summary.dashboard_count,
                    document.summary.datasource_count,
                    document.summary.panel_count,
                    document.summary.variable_count
                )),
                Line::from(format!(
                    "alert-rules={} contact-points={} mute-timings={} policies={} templates={}",
                    document.summary.alert_rule_count,
                    document.summary.contact_point_count,
                    document.summary.mute_timing_count,
                    document.summary.notification_policy_count,
                    document.summary.template_count
                )),
                Line::from(
                    "Use node groups to walk the graph and inspect each node's inbound and outbound edges.",
                ),
            ];
            frame.render_widget(
                Paragraph::new(summary_lines)
                    .wrap(Wrap { trim: false })
                    .block(Block::default().borders(Borders::ALL).title("Topology Summary")),
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
                                    .fg(node_kind_color(&group.kind))
                                    .add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    })
                    .collect::<Vec<_>>(),
            )
            .block(pane_block("Node Groups", active_pane == TopologyPane::Groups))
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
                format!("Nodes {}/{}  {}", items.len(), document.summary.node_count, group.label)
            } else {
                "Nodes".to_string()
            };
            let node_list = List::new(
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
                                    .fg(node_kind_color(&item.kind))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(format!(" {}", item.title)),
                        ]))
                    })
                    .collect::<Vec<_>>(),
            )
            .block(pane_block(&item_title, active_pane == TopologyPane::Nodes))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_stateful_widget(node_list, panes[1], &mut node_state);

            let selected_item = node_state.selected().and_then(|index| items.get(index));
            let detail_text = selected_item
                .map(|item| item.details.join("\n"))
                .unwrap_or_else(|| "No node in this group.".to_string());
            let detail_total_lines = selected_item
                .map(|item| item.details.len().max(1))
                .unwrap_or(1);
            let detail_title = selected_item
                .map(|item| {
                    format!(
                        "Detail [{}/{}] {}  line {}/{}",
                        node_state.selected().map(|index| index + 1).unwrap_or(0),
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
                    .block(pane_block(&detail_title, active_pane == TopologyPane::Detail)),
                panes[2],
            );

            frame.render_widget(
                tui_shell::build_footer_controls(vec![
                    Line::from(vec![
                        tui_shell::focus_label("Focus "),
                        tui_shell::key_chip(
                            match active_pane {
                                TopologyPane::Groups => "Groups",
                                TopologyPane::Nodes => "Nodes",
                                TopologyPane::Detail => "Detail",
                            },
                            Color::Blue,
                        ),
                        Span::raw("  "),
                        tui_shell::label("Selection "),
                        tui_shell::accent(
                            format!(
                                "group {}/{}  node {}/{}",
                                group_state.selected().map(|index| index + 1).unwrap_or(0),
                                groups.len(),
                                node_state.selected().map(|index| index + 1).unwrap_or(0),
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
                        TopologyPane::Groups => TopologyPane::Nodes,
                        TopologyPane::Nodes => TopologyPane::Detail,
                        TopologyPane::Detail => TopologyPane::Groups,
                    };
                }
                KeyCode::Up => match active_pane {
                    TopologyPane::Groups => {
                        let selected = group_state.selected().unwrap_or(0).saturating_sub(1);
                        group_state.select(Some(selected));
                        items = filter_topology_tui_items(document, &groups[selected].kind);
                        node_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    TopologyPane::Nodes => {
                        let selected = node_state.selected().unwrap_or(0).saturating_sub(1);
                        node_state.select((!items.is_empty()).then_some(selected));
                        detail_scroll = 0;
                    }
                    TopologyPane::Detail => detail_scroll = detail_scroll.saturating_sub(1),
                },
                KeyCode::Down => match active_pane {
                    TopologyPane::Groups => {
                        let selected = (group_state.selected().unwrap_or(0) + 1)
                            .min(groups.len().saturating_sub(1));
                        group_state.select(Some(selected));
                        items = filter_topology_tui_items(document, &groups[selected].kind);
                        node_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    TopologyPane::Nodes => {
                        let selected = (node_state.selected().unwrap_or(0) + 1)
                            .min(items.len().saturating_sub(1));
                        node_state.select((!items.is_empty()).then_some(selected));
                        detail_scroll = 0;
                    }
                    TopologyPane::Detail => detail_scroll = detail_scroll.saturating_add(1),
                },
                KeyCode::PageUp => {
                    if active_pane == TopologyPane::Detail {
                        detail_scroll = detail_scroll.saturating_sub(10);
                    }
                }
                KeyCode::PageDown => {
                    if active_pane == TopologyPane::Detail {
                        detail_scroll = detail_scroll.saturating_add(10);
                    }
                }
                KeyCode::Home => match active_pane {
                    TopologyPane::Groups => {
                        group_state.select(Some(0));
                        items = filter_topology_tui_items(document, &groups[0].kind);
                        node_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    TopologyPane::Nodes => {
                        node_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    TopologyPane::Detail => detail_scroll = 0,
                },
                KeyCode::End => match active_pane {
                    TopologyPane::Groups => {
                        let selected = groups.len().saturating_sub(1);
                        group_state.select(Some(selected));
                        items = filter_topology_tui_items(document, &groups[selected].kind);
                        node_state.select((!items.is_empty()).then_some(0));
                        detail_scroll = 0;
                    }
                    TopologyPane::Nodes => {
                        node_state
                            .select((!items.is_empty()).then_some(items.len().saturating_sub(1)));
                        detail_scroll = 0;
                    }
                    TopologyPane::Detail => {
                        detail_scroll =
                            selected_item_max_scroll(&items, node_state.selected()) as u16;
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
