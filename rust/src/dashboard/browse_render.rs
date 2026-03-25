use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use super::browse_state::BrowserState;
use super::browse_support::{
    DashboardBrowseDocument, DashboardBrowseNode, DashboardBrowseNodeKind,
};
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

    let header = Paragraph::new(render_summary_lines(&state.document, &state.status).join("\n"))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Dashboard Browser"),
        );
    frame.render_widget(header, outer[0]);

    let list = List::new(build_tree_items(&state.document.nodes))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Tree {} item(s)", state.document.nodes.len())),
        )
        .highlight_symbol(">> ")
        .repeat_highlight_symbol(true)
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, panes[0], &mut state.list_state);

    if state.pending_delete.is_some() {
        let detail = Paragraph::new(build_detail_lines(state).join("\n"))
            .scroll((state.detail_scroll, 0))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Delete Preview"),
            );
        frame.render_widget(detail, panes[1]);
    } else {
        render_detail_panel(frame, panes[1], state);
    }

    let footer = Paragraph::new(control_lines(
        state.pending_delete.is_some(),
        state.pending_edit.is_some(),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Controls")
            .style(Style::default().bg(Color::Rgb(16, 22, 30)))
            .border_style(Style::default().fg(Color::LightBlue))
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(16, 22, 30))
                    .add_modifier(Modifier::BOLD),
            ),
    )
    .style(Style::default().bg(Color::Rgb(16, 22, 30)).fg(Color::White));
    frame.render_widget(footer, outer[2]);

    if let Some(edit_state) = state.pending_edit.as_ref() {
        edit_state.render(frame);
    }
    if let Some(history_state) = state.pending_history.as_ref() {
        history_state.render(frame);
    }
}

fn build_tree_items(nodes: &[DashboardBrowseNode]) -> Vec<ListItem<'_>> {
    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let prefix = match node.kind {
                DashboardBrowseNodeKind::Folder => "+",
                DashboardBrowseNodeKind::Dashboard => "-",
            };
            let line = Line::from(vec![
                Span::styled(
                    format!("{:>3} ", index + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(format!("{}{} ", "  ".repeat(node.depth), prefix)),
                Span::styled(
                    node.title.clone(),
                    Style::default()
                        .fg(node_color(node))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {}", node.meta),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            ListItem::new(line)
        })
        .collect()
}

fn build_detail_lines(state: &BrowserState) -> Vec<String> {
    match state.pending_delete.as_ref() {
        Some(plan) => render_delete_dry_run_text(plan),
        None => state
            .selected_node()
            .map(|node| detail_lines_for_node(node, &state.live_view_cache))
            .unwrap_or_else(|| vec!["No item selected.".to_string()]),
    }
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
        DashboardBrowseNodeKind::Folder => Color::Rgb(16, 92, 122),
        DashboardBrowseNodeKind::Dashboard => Color::Rgb(110, 78, 22),
    };
    let kind_label = match node.kind {
        DashboardBrowseNodeKind::Folder => " FOLDER ",
        DashboardBrowseNodeKind::Dashboard => " DASHBOARD ",
    };
    let hero = Paragraph::new(vec![
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
            node.path.clone(),
            Style::default().fg(Color::Cyan),
        )),
        Line::from(vec![
            muted("UID "),
            plain_owned(
                node.uid
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .unwrap_or("-"),
            ),
            Span::raw("   "),
            muted("META "),
            plain_boxed(&node.meta, Color::Rgb(40, 49, 61)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Overview")
            .style(Style::default().bg(Color::Rgb(18, 24, 33)))
            .border_style(Style::default().fg(Color::LightBlue))
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
    )
    .style(Style::default().bg(Color::Rgb(18, 24, 33)));
    frame.render_widget(hero, sections[0]);

    let detail_lines = detail_lines_for_node(node, &state.live_view_cache);
    let info = Paragraph::new(build_info_lines(&detail_lines))
        .scroll((state.detail_scroll, 0))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Facts")
                .style(Style::default().bg(Color::Rgb(16, 20, 27)))
                .border_style(Style::default().fg(Color::Gray))
                .title_style(
                    Style::default()
                        .fg(Color::LightCyan)
                        .bg(Color::Rgb(16, 20, 27))
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .style(Style::default().bg(Color::Rgb(16, 20, 27)));
    frame.render_widget(info, sections[1]);

    let shortcuts = Paragraph::new(detail_shortcut_lines(node))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Actions")
                .style(Style::default().bg(Color::Rgb(22, 18, 30)))
                .border_style(Style::default().fg(Color::LightMagenta))
                .title_style(
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(22, 18, 30))
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .style(Style::default().bg(Color::Rgb(22, 18, 30)));
    frame.render_widget(shortcuts, sections[2]);
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
                        format!("{label}: "),
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
        DashboardBrowseNodeKind::Folder => vec![
            Line::from(vec![
                key_chip("d", Color::Rgb(150, 38, 46)),
                plain(" delete dashboards in subtree"),
            ]),
            Line::from(vec![
                key_chip("D", Color::Rgb(150, 38, 46)),
                plain(" delete subtree + folders"),
            ]),
        ],
        DashboardBrowseNodeKind::Dashboard => vec![
            Line::from(vec![
                key_chip("r", Color::Rgb(24, 106, 59)),
                plain(" rename"),
                plain("   "),
                key_chip("h", Color::Rgb(71, 55, 152)),
                plain(" history"),
                plain("   "),
                key_chip("m", Color::Rgb(24, 78, 140)),
                plain(" move folder"),
            ]),
            Line::from(vec![
                key_chip("e", Color::Rgb(71, 55, 152)),
                plain(" edit dialog"),
                plain("   "),
                key_chip("E", Color::Rgb(71, 55, 152)),
                plain(" raw json"),
                plain("   "),
                key_chip("d", Color::Rgb(150, 38, 46)),
                plain(" delete"),
            ]),
        ],
    }
}

fn detail_lines_for_node(
    node: &DashboardBrowseNode,
    live_view_cache: &std::collections::BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    if let Some(uid) = node.uid.as_ref() {
        if let Some(lines) = live_view_cache.get(uid) {
            return lines.clone();
        }
    }
    node.details.clone()
}

fn render_summary_lines(document: &DashboardBrowseDocument, status: &str) -> Vec<String> {
    vec![
        format!(
            "Folders: {}  Dashboards: {}  Root: {}",
            document.summary.folder_count,
            document.summary.dashboard_count,
            document
                .summary
                .root_path
                .as_deref()
                .unwrap_or("all folders")
        ),
        status.to_string(),
    ]
}

fn control_lines(has_pending_delete: bool, has_pending_edit: bool) -> Vec<Line<'static>> {
    if has_pending_delete {
        vec![
            Line::from(vec![
                muted("Delete preview active. "),
                key_chip("y", Color::Rgb(150, 38, 46)),
                plain(" confirm"),
                plain("   "),
                key_chip("n", Color::Rgb(90, 98, 107)),
                plain(" cancel"),
                plain("   "),
                key_chip("Esc", Color::Rgb(90, 98, 107)),
                plain(" close"),
            ]),
            Line::from(vec![
                key_chip("l", Color::Rgb(24, 78, 140)),
                plain(" refresh"),
                plain("   "),
                key_chip("q", Color::Rgb(90, 98, 107)),
                plain(" exit"),
            ]),
        ]
    } else if has_pending_edit {
        vec![
            Line::from(vec![
                muted("Edit dialog active. "),
                key_chip("Ctrl+S", Color::Rgb(24, 106, 59)),
                plain(" save"),
                plain("   "),
                key_chip("Ctrl+X", Color::Rgb(90, 98, 107)),
                plain(" close"),
                plain("   "),
                key_chip("Esc", Color::Rgb(90, 98, 107)),
                plain(" cancel"),
            ]),
            Line::from(vec![
                key_chip("Tab", Color::Rgb(24, 78, 140)),
                plain(" next"),
                plain("   "),
                key_chip("Shift+Tab", Color::Rgb(24, 78, 140)),
                plain(" previous"),
                plain("   "),
                key_chip("Backspace", Color::Rgb(90, 98, 107)),
                plain(" delete char"),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                key_chip("Up/Down", Color::Rgb(24, 78, 140)),
                plain(" move"),
                plain("   "),
                key_chip("PgUp/PgDn", Color::Rgb(24, 78, 140)),
                plain(" detail"),
                plain("   "),
                key_chip("Home/End", Color::Rgb(24, 78, 140)),
                plain(" jump"),
            ]),
            Line::from(vec![
                key_chip("r", Color::Rgb(24, 106, 59)),
                plain(" rename"),
                plain("   "),
                key_chip("m", Color::Rgb(24, 78, 140)),
                plain(" move folder"),
                plain("   "),
                key_chip("d", Color::Rgb(150, 38, 46)),
                plain(" delete"),
                plain("   "),
                key_chip("D", Color::Rgb(150, 38, 46)),
                plain(" delete+folders"),
            ]),
            Line::from(vec![
                key_chip("v", Color::Rgb(71, 55, 152)),
                plain(" live details"),
                plain("   "),
                key_chip("h", Color::Rgb(71, 55, 152)),
                plain(" history"),
                plain("   "),
                key_chip("e", Color::Rgb(71, 55, 152)),
                plain(" edit dialog"),
                plain("   "),
                key_chip("E", Color::Rgb(71, 55, 152)),
                plain(" raw json"),
                plain("   "),
                key_chip("l", Color::Rgb(24, 78, 140)),
                plain(" refresh"),
                plain("   "),
                key_chip("q", Color::Rgb(90, 98, 107)),
                plain(" exit"),
            ]),
        ]
    }
}

fn key_chip(label: &'static str, bg: Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", label),
        Style::default()
            .fg(Color::White)
            .bg(bg)
            .add_modifier(Modifier::BOLD),
    )
}

fn plain(text: &'static str) -> Span<'static> {
    Span::styled(text, Style::default().fg(Color::White))
}

fn plain_owned(text: &str) -> Span<'static> {
    Span::styled(text.to_string(), Style::default().fg(Color::White))
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
        DashboardBrowseNodeKind::Folder => Color::Cyan,
        DashboardBrowseNodeKind::Dashboard => Color::Yellow,
    }
}
