#![cfg(feature = "tui")]
use crate::tui_shell;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use super::browse_state::{BrowserState, PaneFocus, SearchDirection};
use super::browse_support::{DashboardBrowseNode, DashboardBrowseNodeKind};
use super::delete_render::render_delete_dry_run_text;

pub(crate) fn render_dashboard_browser_frame(frame: &mut ratatui::Frame, state: &mut BrowserState) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(frame.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(46), Constraint::Percentage(54)])
        .split(outer[1]);

    let header = tui_shell::build_header(
        "Dashboard Browser",
        render_summary_lines(state)
            .into_iter()
            .map(Line::from)
            .collect(),
    );
    frame.render_widget(header, outer[0]);

    let list = List::new(build_tree_items(&state.document.nodes))
        .block(
            pane_block(
                "Tree",
                state.focus == PaneFocus::Tree,
                Color::LightBlue,
                Color::Rgb(14, 20, 27),
            )
            .title(format!(
                "Tree  {} org(s) / {} folder(s) / {} dashboard(s)",
                state.document.summary.org_count,
                state.document.summary.folder_count,
                state.document.summary.dashboard_count
            )),
        )
        .highlight_symbol("▌ ")
        .repeat_highlight_symbol(true)
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, panes[0], &mut state.list_state);

    render_detail_panel(frame, panes[1], state);

    let footer = tui_shell::build_footer(
        control_lines(state.pending_delete.is_some(), state.pending_edit.is_some()),
        state.status.clone(),
    );
    frame.render_widget(footer, outer[2]);

    if let Some(plan) = state.pending_delete.as_ref() {
        tui_shell::render_overlay(
            frame,
            "Delete Preview",
            render_delete_dry_run_text(plan)
                .into_iter()
                .map(Line::from)
                .collect(),
            Color::Red,
        );
    }
    if let Some(edit_state) = state.pending_edit.as_ref() {
        edit_state.render(frame);
    }
    if let Some(history_state) = state.pending_history.as_ref() {
        history_state.render(frame);
    }
    if let Some(search_state) = state.pending_search.as_ref() {
        render_search_prompt(frame, search_state.direction, &search_state.query);
    }
}

fn build_tree_items(nodes: &[DashboardBrowseNode]) -> Vec<ListItem<'_>> {
    let mut rendered = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if node.kind == DashboardBrowseNodeKind::Org {
            let divider = Line::from(vec![
                Span::styled("──── ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    node.org_name.clone(),
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " ─────────────────────",
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            let line = Line::from(vec![
                Span::styled(
                    " ORG ",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(46, 66, 98))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    format!("{} ", node.title),
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("│ id={} │ {}", node.org_id, node.meta),
                    Style::default().fg(Color::Gray),
                ),
            ]);
            if index > 0 {
                rendered.push(ListItem::new(vec![
                    Line::from(Span::raw(" ")),
                    divider,
                    line,
                ]));
            } else {
                rendered.push(ListItem::new(vec![divider, line]));
            }
            continue;
        }

        let prefix = match node.kind {
            DashboardBrowseNodeKind::Folder => "+",
            DashboardBrowseNodeKind::Dashboard => "-",
            DashboardBrowseNodeKind::Org => "",
        };
        let line = Line::from(vec![
            Span::styled("     ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{}{} ", "  ".repeat(node.depth), prefix)),
            Span::styled(
                node.title.clone(),
                Style::default()
                    .fg(node_color(node))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  │  {}", node.meta),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        rendered.push(ListItem::new(line));
    }
    rendered
}

fn render_detail_panel(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &BrowserState,
) {
    let Some(node) = state.selected_node() else {
        let empty = Paragraph::new("No item selected.")
            .block(Block::default().borders(Borders::ALL).title("Detail"));
        frame.render_widget(empty, area);
        return;
    };

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(6),
            Constraint::Length(4),
        ])
        .split(area);

    let kind_color = match node.kind {
        DashboardBrowseNodeKind::Org => Color::Rgb(53, 79, 122),
        DashboardBrowseNodeKind::Folder => Color::Rgb(16, 92, 122),
        DashboardBrowseNodeKind::Dashboard => Color::Rgb(110, 78, 22),
    };
    let kind_label = match node.kind {
        DashboardBrowseNodeKind::Org => " ORG ",
        DashboardBrowseNodeKind::Folder => " FOLDER ",
        DashboardBrowseNodeKind::Dashboard => " DASHBOARD ",
    };
    let hero_lines = vec![
        Line::from(vec![
            Span::styled(
                kind_label,
                Style::default()
                    .fg(Color::White)
                    .bg(kind_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                node.title.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            match node.kind {
                DashboardBrowseNodeKind::Org => format!("Org {} ({})", node.org_name, node.org_id),
                _ => node.path.clone(),
            },
            Style::default().fg(Color::Cyan),
        )),
        Line::from(vec![
            muted("UID "),
            tui_shell::plain(
                node.uid
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("-"),
            ),
            Span::raw("   "),
            muted("META "),
            plain_boxed(&node.meta, Color::Rgb(40, 49, 61)),
        ]),
    ];
    render_focusable_lines(
        frame,
        sections[0],
        hero_lines,
        pane_block("Overview", false, Color::LightBlue, Color::Rgb(18, 24, 33)),
        false,
        state.detail_scroll,
    );

    let detail_lines = detail_lines_for_node(node, &state.live_view_cache);
    render_focusable_lines(
        frame,
        sections[1],
        build_info_lines(&detail_lines),
        pane_block(
            "Facts",
            state.focus == PaneFocus::Facts,
            Color::LightCyan,
            Color::Rgb(16, 20, 27),
        ),
        state.focus == PaneFocus::Facts,
        state.detail_scroll,
    );

    render_focusable_lines(
        frame,
        sections[2],
        detail_shortcut_lines(node),
        pane_block(
            "Actions",
            false,
            Color::LightMagenta,
            Color::Rgb(22, 18, 30),
        ),
        false,
        state.detail_scroll,
    );
}

fn build_info_lines(lines: &[String]) -> Vec<Line<'static>> {
    lines
        .iter()
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("Delete:"))
        .filter(|line| !line.starts_with("Delete folders:"))
        .filter(|line| !line.starts_with("Advanced edit:"))
        .filter(|line| !line.starts_with("View:"))
        .map(|line| {
            if line == "Live details:" {
                Line::from(vec![Span::styled(
                    "LIVE DETAILS",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                )])
            } else if let Some((label, value)) = line.split_once(':') {
                Line::from(vec![
                    Span::styled(
                        format!("{label:<18}: "),
                        Style::default()
                            .fg(Color::LightBlue)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(value.trim().to_string(), Style::default().fg(Color::White)),
                ])
            } else {
                Line::from(Span::styled(
                    line.clone(),
                    Style::default().fg(Color::White),
                ))
            }
        })
        .collect()
}

fn detail_shortcut_lines(node: &DashboardBrowseNode) -> Vec<Line<'static>> {
    match node.kind {
        DashboardBrowseNodeKind::Org => vec![
            Line::from(vec![
                tui_shell::key_chip("Up/Down", Color::Rgb(24, 78, 140)),
                tui_shell::plain(" select org, folder, or dashboard"),
            ]),
            Line::from(vec![
                tui_shell::key_chip("l", Color::Rgb(24, 78, 140)),
                tui_shell::plain(" refresh"),
                tui_shell::plain("   "),
                tui_shell::key_chip("/ ?", Color::Rgb(164, 116, 19)),
                tui_shell::plain(" search"),
                tui_shell::plain("   "),
                tui_shell::key_chip("e/d", Color::Rgb(90, 98, 107)),
                tui_shell::plain(" dashboard/folder rows only"),
            ]),
        ],
        DashboardBrowseNodeKind::Folder => vec![
            Line::from(vec![
                tui_shell::key_chip("d", Color::Rgb(150, 38, 46)),
                tui_shell::plain(" delete dashboards in subtree"),
            ]),
            Line::from(vec![
                tui_shell::key_chip("D", Color::Rgb(150, 38, 46)),
                tui_shell::plain(" delete subtree + folders"),
            ]),
        ],
        DashboardBrowseNodeKind::Dashboard => vec![
            Line::from(vec![
                tui_shell::key_chip("r", Color::Rgb(24, 106, 59)),
                tui_shell::plain(" rename"),
                tui_shell::plain("   "),
                tui_shell::key_chip("h", Color::Rgb(71, 55, 152)),
                tui_shell::plain(" history"),
                tui_shell::plain("   "),
                tui_shell::key_chip("m", Color::Rgb(24, 78, 140)),
                tui_shell::plain(" move folder"),
            ]),
            Line::from(vec![
                tui_shell::key_chip("e", Color::Rgb(71, 55, 152)),
                tui_shell::plain(" edit dialog"),
                tui_shell::plain("   "),
                tui_shell::key_chip("E", Color::Rgb(71, 55, 152)),
                tui_shell::plain(" raw json"),
                tui_shell::plain("   "),
                tui_shell::key_chip("d", Color::Rgb(150, 38, 46)),
                tui_shell::plain(" delete"),
            ]),
        ],
    }
}

fn detail_lines_for_node(
    node: &DashboardBrowseNode,
    live_view_cache: &std::collections::BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    if let Some(uid) = node.uid.as_ref() {
        if let Some(lines) = live_view_cache.get(&format!("{}::{uid}", node.org_id)) {
            return lines.clone();
        }
    }
    node.details.clone()
}

fn render_summary_lines(state: &BrowserState) -> Vec<String> {
    let document = &state.document;
    vec![
        if document.summary.org_count > 1 {
            format!(
                "Scope {}  orgs={}  folders={}  dashboards={}  root={}",
                document.summary.scope_label,
                document.summary.org_count,
                document.summary.folder_count,
                document.summary.dashboard_count,
                document
                    .summary
                    .root_path
                    .as_deref()
                    .unwrap_or("all folders")
            )
        } else {
            format!(
                "Folders: {}  Dashboards: {}  Root: {}",
                document.summary.folder_count,
                document.summary.dashboard_count,
                document
                    .summary
                    .root_path
                    .as_deref()
                    .unwrap_or("all folders")
            )
        },
        if state.pending_delete.is_some() {
            "Mode=confirm-delete   Confirm with y, cancel with Esc.".to_string()
        } else {
            "Mode=browse".to_string()
        },
    ]
}

fn control_lines(has_pending_delete: bool, has_pending_edit: bool) -> Vec<Line<'static>> {
    if has_pending_delete {
        vec![
            Line::from(vec![
                muted("Delete preview active. "),
                tui_shell::key_chip("y", Color::Rgb(150, 38, 46)),
                tui_shell::plain(" confirm"),
                tui_shell::plain("   "),
                tui_shell::key_chip("n", Color::Rgb(90, 98, 107)),
                tui_shell::plain(" cancel"),
                tui_shell::plain("   "),
                tui_shell::key_chip("Esc", Color::Rgb(90, 98, 107)),
                tui_shell::plain(" close"),
            ]),
            Line::from(vec![
                tui_shell::key_chip("l", Color::Rgb(24, 78, 140)),
                tui_shell::plain(" refresh"),
                tui_shell::plain("   "),
                tui_shell::key_chip("q", Color::Rgb(90, 98, 107)),
                tui_shell::plain(" exit"),
            ]),
        ]
    } else if has_pending_edit {
        vec![
            Line::from(vec![
                muted("Edit dialog active. "),
                tui_shell::key_chip("Ctrl+S", Color::Rgb(24, 106, 59)),
                tui_shell::plain(" save"),
                tui_shell::plain("   "),
                tui_shell::key_chip("Ctrl+X", Color::Rgb(90, 98, 107)),
                tui_shell::plain(" close"),
                tui_shell::plain("   "),
                tui_shell::key_chip("Esc", Color::Rgb(90, 98, 107)),
                tui_shell::plain(" cancel"),
            ]),
            Line::from(vec![
                tui_shell::key_chip("Tab", Color::Rgb(24, 78, 140)),
                tui_shell::plain(" next"),
                tui_shell::plain("   "),
                tui_shell::key_chip("Shift+Tab", Color::Rgb(24, 78, 140)),
                tui_shell::plain(" previous"),
                tui_shell::plain("   "),
                tui_shell::key_chip("Backspace", Color::Rgb(90, 98, 107)),
                tui_shell::plain(" delete char"),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                tui_shell::key_chip("Up/Down", Color::Rgb(24, 78, 140)),
                tui_shell::plain(" move"),
                tui_shell::plain("   "),
                tui_shell::key_chip("PgUp/PgDn", Color::Rgb(24, 78, 140)),
                tui_shell::plain(" detail"),
                tui_shell::plain("   "),
                tui_shell::key_chip("Home/End", Color::Rgb(24, 78, 140)),
                tui_shell::plain(" jump"),
                tui_shell::plain("   "),
                tui_shell::key_chip("Tab", Color::Rgb(164, 116, 19)),
                tui_shell::plain(" next pane"),
            ]),
            Line::from(vec![
                tui_shell::key_chip("Shift+Tab", Color::Rgb(164, 116, 19)),
                tui_shell::plain(" prev pane"),
                tui_shell::plain("   "),
                tui_shell::key_chip("/ ?", Color::Rgb(164, 116, 19)),
                tui_shell::plain(" search"),
                tui_shell::plain("   "),
                tui_shell::key_chip("n", Color::Rgb(164, 116, 19)),
                tui_shell::plain(" next match"),
                tui_shell::plain("   "),
                tui_shell::key_chip("r", Color::Rgb(24, 106, 59)),
                tui_shell::plain(" rename"),
                tui_shell::plain("   "),
                tui_shell::key_chip("m", Color::Rgb(24, 78, 140)),
                tui_shell::plain(" move folder"),
            ]),
            Line::from(vec![
                tui_shell::key_chip("d", Color::Rgb(150, 38, 46)),
                tui_shell::plain(" delete"),
                tui_shell::plain("   "),
                tui_shell::key_chip("D", Color::Rgb(150, 38, 46)),
                tui_shell::plain(" delete+folders"),
                tui_shell::plain("   "),
                tui_shell::key_chip("v", Color::Rgb(71, 55, 152)),
                tui_shell::plain(" live details"),
                tui_shell::plain("   "),
                tui_shell::key_chip("h", Color::Rgb(71, 55, 152)),
                tui_shell::plain(" history"),
                tui_shell::plain("   "),
                tui_shell::key_chip("e", Color::Rgb(71, 55, 152)),
                tui_shell::plain(" edit dialog"),
                tui_shell::plain("   "),
                tui_shell::key_chip("E", Color::Rgb(71, 55, 152)),
                tui_shell::plain(" raw json"),
                tui_shell::plain("   "),
                tui_shell::key_chip("l", Color::Rgb(24, 78, 140)),
                tui_shell::plain(" refresh"),
                tui_shell::plain("   "),
                tui_shell::key_chip("q", Color::Rgb(90, 98, 107)),
                tui_shell::plain(" exit"),
            ]),
        ]
    }
}

fn muted(text: &'static str) -> Span<'static> {
    Span::styled(text, Style::default().fg(Color::Gray))
}

fn plain_boxed(text: &str, bg: Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", text),
        Style::default().fg(Color::White).bg(bg),
    )
}

fn node_color(node: &DashboardBrowseNode) -> Color {
    match node.kind {
        DashboardBrowseNodeKind::Org => Color::LightCyan,
        DashboardBrowseNodeKind::Folder => Color::Cyan,
        DashboardBrowseNodeKind::Dashboard => Color::Yellow,
    }
}

fn pane_block(title: &str, focused: bool, accent: Color, bg: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(if focused {
            format!("{title} [Focused]")
        } else {
            title.to_string()
        })
        .style(Style::default().bg(bg))
        .border_style(Style::default().fg(if focused { accent } else { Color::Gray }))
        .title_style(
            Style::default()
                .fg(Color::White)
                .bg(bg)
                .add_modifier(if focused {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        )
}

fn render_focusable_lines(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    lines: Vec<Line<'static>>,
    block: Block<'static>,
    focused: bool,
    scroll: u16,
) {
    let lines = if lines.is_empty() {
        vec![Line::from("-")]
    } else {
        lines
    };
    let items = lines.into_iter().map(ListItem::new).collect::<Vec<_>>();
    if focused {
        let mut state = ratatui::widgets::ListState::default();
        state.select(Some((scroll as usize).min(items.len().saturating_sub(1))));
        let list = List::new(items)
            .block(block)
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(list, area, &mut state);
    } else {
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
    }
}

fn render_search_prompt(frame: &mut ratatui::Frame, direction: SearchDirection, query: &str) {
    let area = ratatui::layout::Rect {
        x: frame.area().x + 6,
        y: frame.area().y + frame.area().height.saturating_sub(5),
        width: frame.area().width.saturating_sub(12).min(78),
        height: 3,
    };
    frame.render_widget(Clear, area);
    let prefix = match direction {
        SearchDirection::Forward => "/",
        SearchDirection::Backward => "?",
    };
    let prompt = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", prefix),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(164, 116, 19))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(query.to_string(), Style::default().fg(Color::White)),
        ]),
        Line::from(Span::styled(
            "Enter search   Esc cancel   n repeat last search",
            Style::default().fg(Color::Gray),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Search")
            .style(Style::default().bg(Color::Rgb(18, 20, 26)))
            .border_style(Style::default().fg(Color::Yellow)),
    )
    .style(Style::default().bg(Color::Rgb(18, 20, 26)));
    frame.render_widget(prompt, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::delete_support::DeletePlan;

    fn empty_document() -> super::super::browse_support::DashboardBrowseDocument {
        super::super::browse_support::DashboardBrowseDocument {
            summary: super::super::browse_support::DashboardBrowseSummary {
                root_path: None,
                dashboard_count: 0,
                folder_count: 0,
                org_count: 1,
                scope_label: "current-org".to_string(),
            },
            nodes: Vec::new(),
        }
    }

    #[test]
    fn summary_lines_move_status_out_of_the_header() {
        let state = BrowserState::new(empty_document());
        let lines = render_summary_lines(&state);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("Folders: 0"));
        assert!(lines[0].contains("Dashboards: 0"));
        assert!(lines[1].contains("Mode=browse"));
        assert!(!lines
            .iter()
            .any(|line| line.contains("Loaded dashboard tree")));
    }

    #[test]
    fn summary_lines_surface_pending_delete_mode() {
        let mut state = BrowserState::new(empty_document());
        state.pending_delete = Some(DeletePlan {
            selector_uid: None,
            selector_path: None,
            delete_folders: false,
            dashboards: Vec::new(),
            folders: Vec::new(),
        });
        let lines = render_summary_lines(&state);
        assert!(lines[1].contains("Mode=confirm-delete"));
    }
}
