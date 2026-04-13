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

    let header = tui_shell::build_header("Dashboard Browser", render_summary_lines(state));
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
        control_lines(
            state.pending_delete.is_some(),
            state.pending_edit.is_some(),
            state.pending_external_edit.is_some(),
            state.local_mode,
        ),
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
    if let Some(external_edit_state) = state.pending_external_edit.as_ref() {
        external_edit_state.render(frame);
    }
    if let Some(external_edit_error_state) = state.pending_external_edit_error.as_ref() {
        external_edit_error_state.render(frame);
    }
    if let Some(history_state) = state.pending_history.as_ref() {
        history_state.render(frame);
    }
    if let Some(search_state) = state.pending_search.as_ref() {
        render_search_prompt(frame, search_state.direction, &search_state.query);
    }
    if let Some(notice) = state.completion_notice.as_ref() {
        tui_shell::render_overlay(
            frame,
            &notice.title,
            vec![
                Line::from(notice.body.clone()),
                Line::from(""),
                Line::from("Press any key to continue."),
            ],
            Color::Green,
        );
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
        detail_shortcut_lines(node, state.local_mode),
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

fn detail_shortcut_lines(node: &DashboardBrowseNode, local_mode: bool) -> Vec<Line<'static>> {
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
                if local_mode {
                    tui_shell::key_chip("local", Color::Rgb(90, 98, 107))
                } else {
                    tui_shell::key_chip("e/d", Color::Rgb(90, 98, 107))
                },
                tui_shell::plain(if local_mode {
                    " read-only tree"
                } else {
                    " dashboard/folder rows only"
                }),
            ]),
        ],
        DashboardBrowseNodeKind::Folder => vec![
            Line::from(vec![
                tui_shell::key_chip("d", Color::Rgb(150, 38, 46)),
                tui_shell::plain(if local_mode {
                    " local browse is read-only"
                } else {
                    " delete dashboards in subtree"
                }),
            ]),
            Line::from(vec![
                tui_shell::key_chip("D", Color::Rgb(150, 38, 46)),
                tui_shell::plain(if local_mode {
                    " live delete actions unavailable"
                } else {
                    " delete subtree + folders"
                }),
            ]),
        ],
        DashboardBrowseNodeKind::Dashboard => vec![
            Line::from(vec![
                tui_shell::key_chip("r", Color::Rgb(24, 106, 59)),
                tui_shell::plain(if local_mode {
                    " local browse is read-only"
                } else {
                    " rename"
                }),
                tui_shell::plain("   "),
                tui_shell::key_chip("h", Color::Rgb(71, 55, 152)),
                tui_shell::plain(if local_mode {
                    " local history unavailable"
                } else {
                    " history"
                }),
                tui_shell::plain("   "),
                tui_shell::key_chip("m", Color::Rgb(24, 78, 140)),
                tui_shell::plain(if local_mode {
                    " local browse is read-only"
                } else {
                    " move folder"
                }),
            ]),
            Line::from(vec![
                tui_shell::key_chip("e", Color::Rgb(71, 55, 152)),
                tui_shell::plain(if local_mode {
                    " local browse is read-only"
                } else {
                    " metadata edit dialog"
                }),
                tui_shell::plain("   "),
                tui_shell::key_chip("E", Color::Rgb(71, 55, 152)),
                tui_shell::plain(if local_mode {
                    " local browse is read-only"
                } else {
                    " raw JSON -> review/apply/save"
                }),
                tui_shell::plain("   "),
                tui_shell::key_chip("d", Color::Rgb(150, 38, 46)),
                tui_shell::plain(if local_mode {
                    " local browse is read-only"
                } else {
                    " delete"
                }),
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

fn render_summary_lines(state: &BrowserState) -> Vec<Line<'static>> {
    let document = &state.document;
    vec![
        if document.summary.org_count > 1 {
            tui_shell::summary_line(&[
                tui_shell::summary_cell(
                    "Scope",
                    document.summary.scope_label.clone(),
                    Color::LightBlue,
                ),
                tui_shell::summary_cell(
                    "Orgs",
                    document.summary.org_count.to_string(),
                    Color::White,
                ),
                tui_shell::summary_cell(
                    "Folders",
                    document.summary.folder_count.to_string(),
                    Color::White,
                ),
                tui_shell::summary_cell(
                    "Dashboards",
                    document.summary.dashboard_count.to_string(),
                    Color::White,
                ),
                tui_shell::summary_cell(
                    "Root",
                    document
                        .summary
                        .root_path
                        .as_deref()
                        .unwrap_or("all folders"),
                    Color::White,
                ),
            ])
        } else {
            tui_shell::summary_line(&[
                tui_shell::summary_cell(
                    "Folders",
                    document.summary.folder_count.to_string(),
                    Color::White,
                ),
                tui_shell::summary_cell(
                    "Dashboards",
                    document.summary.dashboard_count.to_string(),
                    Color::White,
                ),
                tui_shell::summary_cell(
                    "Root",
                    document
                        .summary
                        .root_path
                        .as_deref()
                        .unwrap_or("all folders"),
                    Color::White,
                ),
            ])
        },
        if state.pending_delete.is_some() {
            Line::from(vec![
                tui_shell::label("Mode "),
                tui_shell::accent("confirm-delete", Color::LightRed),
                Span::raw("  "),
                tui_shell::focus_label("Focus "),
                tui_shell::key_chip(
                    match state.focus {
                        PaneFocus::Tree => "Tree",
                        PaneFocus::Facts => "Facts",
                    },
                    Color::Blue,
                ),
                Span::raw("  "),
                tui_shell::label("Confirm "),
                tui_shell::accent("y / Esc / q", Color::Yellow),
            ])
        } else {
            Line::from(vec![
                tui_shell::label("Mode "),
                tui_shell::accent(
                    if state.local_mode {
                        "local-browse"
                    } else {
                        "browse"
                    },
                    Color::Green,
                ),
                Span::raw("  "),
                tui_shell::focus_label("Focus "),
                tui_shell::key_chip(
                    match state.focus {
                        PaneFocus::Tree => "Tree",
                        PaneFocus::Facts => "Facts",
                    },
                    Color::Blue,
                ),
            ])
        },
    ]
}

fn control_lines(
    has_pending_delete: bool,
    has_pending_edit: bool,
    has_pending_external_edit: bool,
    local_mode: bool,
) -> Vec<Line<'static>> {
    if local_mode && !has_pending_delete && !has_pending_edit && !has_pending_external_edit {
        return tui_shell::control_grid(&[
            vec![
                ("Up/Down", Color::Rgb(24, 78, 140), "move"),
                ("PgUp/PgDn", Color::Rgb(24, 78, 140), "scroll detail"),
                ("Tab", Color::Rgb(164, 116, 19), "next pane"),
                ("l", Color::Rgb(24, 78, 140), "refresh local tree"),
            ],
            vec![
                ("/ ?", Color::Rgb(164, 116, 19), "search"),
            ],
        ])
        .into_iter()
        .chain(std::iter::once(
            Line::from(vec![
                muted("Local browse is read-only. Live edit, move, delete, and history actions are unavailable."),
            ]),
        ))
        .collect();
    }
    if has_pending_delete {
        tui_shell::control_grid(&[
            vec![
                ("y", Color::Rgb(150, 38, 46), "confirm delete"),
                ("n", Color::Rgb(90, 98, 107), "cancel"),
                ("Esc", Color::Rgb(90, 98, 107), "cancel"),
                ("q", Color::Rgb(90, 98, 107), "cancel"),
            ],
            vec![("l", Color::Rgb(24, 78, 140), "refresh")],
        ])
    } else if has_pending_edit {
        tui_shell::control_grid(&[
            vec![
                ("Ctrl+S", Color::Rgb(24, 106, 59), "save"),
                ("Ctrl+X", Color::Rgb(90, 98, 107), "close"),
                ("Esc", Color::Rgb(90, 98, 107), "cancel"),
            ],
            vec![
                ("Tab", Color::Rgb(24, 78, 140), "next field"),
                ("Shift+Tab", Color::Rgb(24, 78, 140), "previous field"),
                ("Backspace", Color::Rgb(90, 98, 107), "delete char"),
            ],
        ])
    } else if has_pending_external_edit {
        tui_shell::control_grid(&[
            vec![
                ("a", Color::Rgb(24, 106, 59), "apply live"),
                ("w", Color::Rgb(164, 116, 19), "draft filename"),
                ("q", Color::Rgb(90, 98, 107), "discard"),
            ],
            vec![
                ("Enter", Color::Rgb(24, 106, 59), "apply live"),
                ("p", Color::Rgb(24, 78, 140), "refresh preview"),
            ],
        ])
    } else {
        tui_shell::control_grid(&[
            vec![
                ("Up/Down", Color::Rgb(24, 78, 140), "move"),
                ("PgUp/PgDn", Color::Rgb(24, 78, 140), "scroll detail"),
                ("Home/End", Color::Rgb(24, 78, 140), "jump"),
                ("Tab", Color::Rgb(164, 116, 19), "next pane"),
            ],
            vec![
                ("Shift+Tab", Color::Rgb(164, 116, 19), "previous pane"),
                ("/ ?", Color::Rgb(164, 116, 19), "search"),
                ("n", Color::Rgb(164, 116, 19), "next match"),
                ("r", Color::Rgb(24, 106, 59), "rename"),
                ("m", Color::Rgb(24, 78, 140), "move folder"),
            ],
            vec![
                ("d", Color::Rgb(150, 38, 46), "delete"),
                ("D", Color::Rgb(150, 38, 46), "delete+folders"),
                ("v", Color::Rgb(71, 55, 152), "live details"),
                ("h", Color::Rgb(71, 55, 152), "history"),
                ("e", Color::Rgb(71, 55, 152), "edit"),
                ("E", Color::Rgb(71, 55, 152), "raw json"),
                ("l", Color::Rgb(24, 78, 140), "refresh"),
                ("Esc/q", Color::Rgb(90, 98, 107), "exit"),
            ],
        ])
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
    let title_bg = if focused { accent } else { bg };
    let title_fg = if focused { Color::Black } else { Color::White };
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
                .fg(title_fg)
                .bg(title_bg)
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
        let lines = render_summary_lines(&state)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("Folders"));
        assert!(lines[0].contains('0'));
        assert!(lines[0].contains("Dashboards"));
        assert!(lines[1].contains("Mode"));
        assert!(lines[1].contains("browse"));
        assert!(lines[1].contains("Focus"));
        assert!(lines[1].contains("Tree"));
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
        let lines = render_summary_lines(&state)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        assert!(lines[1].contains("Mode"));
        assert!(lines[1].contains("confirm-delete"));
        assert!(lines[1].contains("Focus"));
        assert!(lines[1].contains("Tree"));
    }

    #[test]
    fn control_lines_use_consistent_pane_and_exit_labels() {
        let lines = control_lines(false, false, false, false)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        assert!(lines[0].contains("next pane"));
        assert!(lines[1].contains("previous pane"));
        assert!(lines[1].contains("search"));
        assert!(lines[2].contains("exit"));
        assert!(lines[2].contains("Esc/q"));
    }

    #[test]
    fn delete_control_lines_use_cancel_labels() {
        let lines = control_lines(true, false, false, false)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        assert!(lines[0].contains("confirm delete"));
        assert!(lines[0].contains("cancel"));
        assert!(lines[1].contains("refresh"));
        assert!(!lines.iter().any(|line| line.contains("exit")));
    }

    #[test]
    fn local_mode_summary_and_controls_mark_read_only_state() {
        let state = BrowserState::new_with_mode(empty_document(), true);
        let summary_lines = render_summary_lines(&state)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        assert!(summary_lines[1].contains("local-browse"));
        assert!(summary_lines[1].contains("Tree"));

        let lines = control_lines(false, false, false, true)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        assert!(lines[0].contains("refresh local tree"));
        assert!(lines[2].contains("read-only"));
    }

    #[test]
    fn external_edit_control_lines_show_preview_save_apply_actions() {
        let lines = control_lines(false, false, true, false)
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        assert!(lines[0].contains("apply live"));
        assert!(lines[0].contains("draft filename"));
        assert!(lines[0].contains("discard"));
        assert!(lines[1].contains("refresh preview"));
        assert!(!lines.iter().any(|line| line.contains("s ")));
    }
}
