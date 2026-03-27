#![cfg(feature = "tui")]

use crate::tui_shell;
use crate::tui_shell::pane_block;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use super::datasource_browse_state::{BrowserState, PaneFocus};
use super::datasource_browse_support::{detail_lines, DatasourceBrowseItem};

pub(crate) fn render_datasource_browser_frame(
    frame: &mut ratatui::Frame,
    state: &mut BrowserState,
) {
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
        .constraints([Constraint::Percentage(44), Constraint::Percentage(56)])
        .split(outer[1]);

    let header = tui_shell::build_header(
        "Datasource Browser",
        summary_lines(state)
            .into_iter()
            .map(Line::from)
            .collect::<Vec<_>>(),
    );
    frame.render_widget(header, outer[0]);

    let list = List::new(build_list_items(&state.document.items))
        .block(
            pane_block(
                "List",
                state.focus == PaneFocus::List,
                Color::LightBlue,
                Color::Rgb(14, 20, 27),
            )
            .title(format!(
                "List  {} org(s) / {} datasource(s)",
                state.document.org_count, state.document.datasource_count
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

    if let Some(edit_state) = state.pending_edit.as_ref() {
        edit_state.render(frame);
    }
    if state.pending_delete.is_some() {
        tui_shell::render_overlay(
            frame,
            &detail_title(state),
            detail_text(state)
                .lines()
                .map(|line| Line::from(line.to_string()))
                .collect(),
            Color::Red,
        );
    }
    if let Some(search_state) = state.pending_search.as_ref() {
        render_search_prompt(frame, search_state.direction, &search_state.query);
    }
}

fn summary_lines(state: &BrowserState) -> Vec<String> {
    let document = &state.document;
    vec![
        if document.org_count > 1 {
            format!(
                "Scope {}  orgs={}  datasources={}",
                blank_dash(&document.scope_label),
                document.org_count,
                document.datasource_count
            )
        } else {
            format!(
                "Org {} (id={})  datasources={}",
                blank_dash(&document.org),
                blank_dash(&document.org_id),
                document.datasource_count
            )
        },
        format!(
            "Active={}   mode={}",
            state.focus_label(),
            if state.pending_delete.is_some() {
                "confirm-delete"
            } else if state.pending_edit.is_some() {
                "edit"
            } else if state.pending_search.is_some() {
                "search"
            } else {
                "browse"
            }
        ),
    ]
}

fn build_list_items(items: &[DatasourceBrowseItem]) -> Vec<ListItem<'_>> {
    let mut rendered = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if item.is_org_row() {
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
                    format!("{} ", item.org),
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        "│ id={} │ {} datasource(s)",
                        item.org_id, item.datasource_count
                    ),
                    Style::default().fg(Color::Gray),
                ),
            ]);
            if index > 0 {
                rendered.push(ListItem::new(vec![Line::from(Span::raw(" ")), line]));
            } else {
                rendered.push(ListItem::new(line));
            }
            continue;
        }
        let badge_color = if item.is_default {
            Color::Green
        } else {
            Color::DarkGray
        };
        let branch = datasource_tree_branch(items, index);
        let line = Line::from(vec![
            Span::styled("     ", Style::default().fg(Color::DarkGray)),
            Span::styled(branch, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(
                if item.is_default { "[*]" } else { "[ ]" },
                Style::default().fg(Color::White).bg(badge_color),
            ),
            Span::raw(" "),
            Span::styled(
                item.name.clone(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "  │  {}  │  {}",
                    blank_dash(&item.datasource_type),
                    blank_dash(&item.uid)
                ),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        rendered.push(ListItem::new(line));
    }
    rendered
}

fn datasource_tree_branch(items: &[DatasourceBrowseItem], index: usize) -> &'static str {
    let has_next_datasource_in_same_group = items
        .get(index + 1)
        .map(|next| !next.is_org_row())
        .unwrap_or(false);
    if has_next_datasource_in_same_group {
        "├─"
    } else {
        "└─"
    }
}

fn detail_title(state: &BrowserState) -> String {
    if let Some(pending_delete) = state.pending_delete.as_ref() {
        format!("Delete {} ({})", pending_delete.name, pending_delete.uid)
    } else {
        "Detail".to_string()
    }
}

fn detail_text(state: &BrowserState) -> String {
    if let Some(pending_delete) = state.pending_delete.as_ref() {
        return [
            format!("Delete datasource {}", blank_dash(&pending_delete.name)),
            format!("UID: {}", blank_dash(&pending_delete.uid)),
            format!("ID: {}", pending_delete.id),
            String::new(),
            "Press y to confirm delete.".to_string(),
            "Press n, Esc, or q to cancel.".to_string(),
        ]
        .join("\n");
    }
    state
        .selected_item()
        .map(|item| {
            if item.is_org_row() {
                return [
                    format!("Org: {}", blank_dash(&item.org)),
                    format!("Org ID: {}", blank_dash(&item.org_id)),
                    format!("Datasources: {}", item.datasource_count),
                    String::new(),
                    "Org rows are scope headers for all-org browsing.".to_string(),
                    "Select a datasource row to edit or delete.".to_string(),
                ]
                .join("\n");
            }
            let mut lines = vec![
                format!("Name: {}", blank_dash(&item.name)),
                format!("Type: {}", blank_dash(&item.datasource_type)),
                format!("UID: {}", blank_dash(&item.uid)),
                format!(
                    "Org: {} ({})",
                    blank_dash(&item.org),
                    blank_dash(&item.org_id)
                ),
                String::new(),
            ];
            lines.extend(detail_lines(item));
            lines.join("\n")
        })
        .unwrap_or_else(|| "No datasource selected.".to_string())
}

fn render_detail_panel(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &BrowserState,
) {
    let Some(item) = state.selected_item() else {
        let empty = Paragraph::new("No datasource selected.")
            .block(Block::default().borders(Borders::ALL).title("Detail"));
        frame.render_widget(empty, area);
        return;
    };

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(9),
            Constraint::Length(4),
        ])
        .split(area);

    let hero_lines = if item.is_org_row() {
        vec![
            Line::from(vec![
                Span::styled(
                    " ORG ",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(53, 79, 122))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    item.org.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!(
                    "Org {}   {} datasource(s)",
                    blank_dash(&item.org_id),
                    item.datasource_count
                ),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(vec![
                muted("SCOPE "),
                plain_boxed("all-org browse header", Color::Rgb(40, 49, 61)),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::styled(
                    if item.is_default {
                        " DEFAULT "
                    } else {
                        " DATASOURCE "
                    },
                    Style::default()
                        .fg(Color::White)
                        .bg(if item.is_default {
                            Color::Rgb(18, 110, 52)
                        } else {
                            Color::Rgb(16, 92, 122)
                        })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    item.name.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!(
                    "{}   {}",
                    blank_dash(&item.datasource_type),
                    blank_dash(&item.uid)
                ),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(vec![
                muted("URL "),
                plain_boxed(&item.url, Color::Rgb(40, 49, 61)),
                Span::raw("   "),
                muted("ORG "),
                tui_shell::plain(format!(
                    "{} ({})",
                    blank_dash(&item.org),
                    blank_dash(&item.org_id)
                )),
            ]),
        ]
    };
    render_focusable_lines(
        frame,
        sections[0],
        hero_lines,
        pane_block("Overview", false, Color::LightBlue, Color::Rgb(18, 24, 33)).title("Overview"),
        false,
        state.detail_scroll,
    );

    render_focusable_lines(
        frame,
        sections[1],
        build_info_lines(&detail_lines(item)),
        pane_block(
            "Facts",
            state.focus == PaneFocus::Facts,
            Color::LightCyan,
            Color::Rgb(16, 20, 27),
        ),
        state.focus == PaneFocus::Facts,
        state.detail_scroll,
    );

    let shortcut_lines = if item.is_org_row() {
        vec![
            Line::from(vec![
                tui_shell::key_chip("Up/Down", Color::Blue),
                tui_shell::plain(" select org or datasource row"),
            ]),
            Line::from(vec![
                tui_shell::key_chip("l", Color::Cyan),
                tui_shell::plain(" refresh all visible orgs"),
                Span::raw("   "),
                tui_shell::key_chip("e/d", Color::DarkGray),
                tui_shell::plain(" datasource rows only"),
            ]),
        ]
    } else {
        vec![
            Line::from(vec![
                tui_shell::key_chip("e", Color::Green),
                tui_shell::plain(" edit datasource"),
            ]),
            Line::from(vec![
                tui_shell::key_chip("d", Color::Red),
                tui_shell::plain(" delete datasource"),
                Span::raw("   "),
                tui_shell::key_chip("l", Color::Cyan),
                tui_shell::plain(" refresh live data"),
            ]),
        ]
    };
    render_focusable_lines(
        frame,
        sections[2],
        shortcut_lines,
        pane_block(
            "Actions",
            false,
            Color::LightMagenta,
            Color::Rgb(22, 18, 30),
        )
        .title("Actions"),
        false,
        state.detail_scroll,
    );
}

fn build_info_lines(lines: &[String]) -> Vec<Line<'static>> {
    lines
        .iter()
        .filter(|line| !line.is_empty())
        .map(|line| {
            if let Some((label, value)) = line.split_once(':') {
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

fn control_lines(has_pending_delete: bool, has_pending_edit: bool) -> Vec<Line<'static>> {
    if has_pending_delete {
        return vec![control_line(&[
            ("y", Color::Red, "confirm delete"),
            ("n", Color::Gray, "cancel"),
            ("Esc", Color::Gray, "cancel"),
            ("q", Color::Gray, "cancel"),
        ])];
    }
    if has_pending_edit {
        return vec![control_line(&[
            ("Ctrl+S", Color::Green, "save"),
            ("Esc", Color::Gray, "cancel"),
            ("Ctrl+X", Color::Gray, "close"),
            ("Tab", Color::Blue, "next field"),
        ])];
    }
    vec![
        control_line(&[
            ("Up/Down", Color::Blue, "select"),
            ("PgUp/PgDn", Color::Blue, "scroll detail"),
            ("Tab", Color::Blue, "next pane"),
            ("e", Color::Green, "edit"),
            ("d", Color::Red, "delete"),
        ]),
        control_line(&[
            ("Shift+Tab", Color::Blue, "prev pane"),
            ("/ ?", Color::Yellow, "search"),
            ("n", Color::Yellow, "next match"),
            ("l", Color::Cyan, "refresh"),
            ("Home/End", Color::Blue, "jump"),
        ]),
        control_line(&[("q", Color::Gray, "exit"), ("Esc", Color::Gray, "exit")]),
    ]
}

fn control_line(segments: &[(&'static str, Color, &'static str)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (index, (key, color, label)) in segments.iter().enumerate() {
        if index > 0 {
            spans.push(tui_shell::plain("  "));
        }
        spans.push(tui_shell::key_chip(key, *color));
        spans.push(tui_shell::plain(format!(" {:<14}", label)));
    }
    Line::from(spans)
}

fn plain_boxed(text: &str, bg: Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", blank_dash(text)),
        Style::default().fg(Color::White).bg(bg),
    )
}

fn muted(text: &'static str) -> Span<'static> {
    Span::styled(text, Style::default().fg(Color::Gray))
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

fn blank_dash(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "-"
    } else {
        trimmed
    }
}

fn render_search_prompt(
    frame: &mut ratatui::Frame,
    direction: super::datasource_browse_state::SearchDirection,
    query: &str,
) {
    let area = ratatui::layout::Rect {
        x: frame.area().x + 6,
        y: frame.area().y + frame.area().height.saturating_sub(5),
        width: frame.area().width.saturating_sub(12).min(70),
        height: 3,
    };
    frame.render_widget(Clear, area);
    let prefix = match direction {
        super::datasource_browse_state::SearchDirection::Forward => "/",
        super::datasource_browse_state::SearchDirection::Backward => "?",
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
    use super::super::datasource_browse_support::DatasourceBrowseDocument;
    use super::*;

    fn empty_document() -> DatasourceBrowseDocument {
        DatasourceBrowseDocument {
            org: "Main Org.".to_string(),
            org_id: "1".to_string(),
            org_count: 1,
            datasource_count: 0,
            scope_label: "current-org".to_string(),
            items: Vec::new(),
        }
    }

    #[test]
    fn summary_lines_surface_focus_and_mode() {
        let state = BrowserState::new(empty_document());
        let lines = summary_lines(&state);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].contains("Active=list"));
        assert!(lines[1].contains("mode=browse"));
        assert!(!lines.iter().any(|line| line.contains("default datasource")));
    }

    #[test]
    fn summary_lines_surface_pending_delete_mode() {
        let mut state = BrowserState::new(empty_document());
        state.pending_delete = Some(super::super::datasource_browse_state::PendingDelete {
            uid: "uid-1".to_string(),
            name: "Prom".to_string(),
            id: 7,
        });
        let lines = summary_lines(&state);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].contains("mode=confirm-delete"));
    }
}
