//! Interactive browse workflows and terminal-driven state flow for Access entities.

use crate::tui_shell;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use serde_json::{Map, Value};

use crate::access::render::{map_get_text, user_scope_text};

use super::user_browse_dialog::{delete_lines, render_search_prompt};
use super::user_browse_state::{row_kind, BrowserState, DisplayMode, PaneFocus};
use super::UserBrowseArgs;

fn team_count(row: &Map<String, Value>) -> usize {
    match row.get("teams") {
        Some(Value::Array(values)) => values.iter().filter_map(Value::as_str).count(),
        Some(Value::String(text)) if !text.trim().is_empty() => text.split(',').count(),
        _ => 0,
    }
}

pub(super) fn render_frame(
    frame: &mut ratatui::Frame,
    state: &mut BrowserState,
    args: &UserBrowseArgs,
) {
    let footer_controls = control_lines(args);
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(tui_shell::footer_height(footer_controls.len())),
        ])
        .split(frame.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(outer[1]);

    frame.render_widget(
        tui_shell::build_header(
            "User Browser",
            vec![
                tui_shell::summary_line(&[
                    tui_shell::summary_cell(
                        "Scope",
                        user_scope_text(&args.scope),
                        Color::LightBlue,
                    ),
                    tui_shell::summary_cell(
                        "Mode",
                        match state.display_mode {
                            DisplayMode::GlobalAccounts => "global-accounts",
                            DisplayMode::OrgMemberships => "org-memberships",
                        },
                        Color::White,
                    ),
                    tui_shell::summary_cell("Rows", state.rows.len().to_string(), Color::White),
                ]),
                Line::from(vec![
                    tui_shell::focus_label("Focus "),
                    tui_shell::key_chip(
                        match state.focus {
                            PaneFocus::List => "List",
                            PaneFocus::Facts => "Facts",
                        },
                        Color::Blue,
                    ),
                    Span::raw("  "),
                    tui_shell::label(if args.input_dir.is_some() {
                        "BUNDLE "
                    } else {
                        "URL "
                    }),
                    tui_shell::accent(
                        args.input_dir
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_else(|| args.common.url.clone()),
                        Color::White,
                    ),
                ]),
            ],
        ),
        outer[0],
    );

    let list = List::new(build_list_items(&state.rows, state.show_numbers))
        .block(pane_block(
            "List",
            state.focus == PaneFocus::List,
            Color::LightBlue,
        ))
        .highlight_symbol("▌ ")
        .repeat_highlight_symbol(true)
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(list, panes[0], &mut state.list_state);

    if state.pending_delete {
        render_focusable_lines(
            frame,
            panes[1],
            delete_lines(state.selected_row()),
            pane_block(
                "Delete Preview",
                state.focus == PaneFocus::Facts,
                Color::Red,
            ),
            state.focus == PaneFocus::Facts,
            state.detail_cursor,
        );
    } else {
        render_detail_panel(frame, panes[1], state);
    }

    frame.render_widget(
        tui_shell::build_footer(footer_controls, state.status.clone()),
        outer[2],
    );

    if let Some(edit) = state.pending_edit.as_ref() {
        edit.render(frame);
    }
    if let Some(search) = state.pending_search.as_ref() {
        render_search_prompt(frame, search);
    }
}

fn build_list_items(rows: &[Map<String, Value>], show_numbers: bool) -> Vec<ListItem<'static>> {
    rows.iter()
        .enumerate()
        .map(|(index, row)| {
            let mut spans = Vec::new();
            if show_numbers {
                spans.push(Span::styled(
                    format!("{:>2}. ", index + 1),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            match row_kind(row) {
                "org" => {
                    spans.extend([
                        Span::styled(
                            "ORG ".to_string(),
                            Style::default()
                                .fg(Color::White)
                                .bg(Color::Rgb(46, 78, 122))
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            blank_dash(&map_get_text(row, "orgName")).to_string(),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!("users={}", blank_dash(&map_get_text(row, "memberCount"))),
                            Style::default().fg(Color::Gray),
                        ),
                    ]);
                }
                "team" => {
                    spans.extend([
                        Span::raw("  "),
                        Span::styled("└─ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            "TEAM ".to_string(),
                            Style::default()
                                .fg(Color::White)
                                .bg(Color::Rgb(42, 92, 122))
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            blank_dash(&map_get_text(row, "teamName")).to_string(),
                            Style::default().fg(Color::LightCyan),
                        ),
                    ]);
                }
                "member" => {
                    spans.extend([
                        Span::raw("  "),
                        Span::styled(
                            blank_dash(&map_get_text(row, "login")).to_string(),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!("[{}]", blank_dash(&map_get_text(row, "orgRole"))),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!("teams={}", team_count(row)),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            "SHARED".to_string(),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::LightYellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]);
                }
                _ => {
                    let expanded = map_get_text(row, "expanded") == "true";
                    let role_summary = {
                        let summary = map_get_text(row, "roleSummary");
                        if summary.is_empty() {
                            map_get_text(row, "orgRole")
                        } else {
                            summary
                        }
                    };
                    let is_server_admin = map_get_text(row, "grafanaAdmin") == "true";
                    spans.extend([
                        Span::styled(
                            if expanded { "▼ " } else { "▶ " },
                            Style::default().fg(Color::LightBlue),
                        ),
                        Span::styled(
                            blank_dash(&map_get_text(row, "login")).to_string(),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!("[{}]", blank_dash(&role_summary)),
                            Style::default().fg(Color::Cyan),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!(
                                "orgs={}",
                                blank_dash(&map_get_text(row, "orgMembershipCount"))
                            ),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!("teams={}", team_count(row)),
                            Style::default().fg(Color::Gray),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            if is_server_admin {
                                "SERVER_ADMIN".to_string()
                            } else {
                                "ORG_USER".to_string()
                            },
                            Style::default()
                                .fg(Color::Black)
                                .bg(if is_server_admin {
                                    Color::LightRed
                                } else {
                                    Color::LightGreen
                                })
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            "SHARED".to_string(),
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::LightYellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]);
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect()
}

fn render_detail_panel(frame: &mut ratatui::Frame, area: Rect, state: &BrowserState) {
    let Some(row) = state.selected_row() else {
        frame.render_widget(
            Paragraph::new("No user selected.")
                .block(Block::default().borders(Borders::ALL).title("Detail")),
            area,
        );
        return;
    };
    if row_kind(row) == "org" {
        render_org_detail_panel(frame, area, state, row);
        return;
    }
    if row_kind(row) == "team" {
        render_team_detail_panel(frame, area, state, row);
        return;
    }
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .split(area);
    render_focusable_lines(
        frame,
        sections[0],
        vec![
            Line::from(vec![
                Span::styled(
                    " USER ",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(18, 110, 52))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    blank_dash(&map_get_text(row, "name")).to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!(
                    "{}   {}",
                    blank_dash(&map_get_text(row, "login")),
                    blank_dash(&map_get_text(row, "email"))
                ),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(vec![
                Span::styled("SCOPE ", Style::default().fg(Color::Gray)),
                Span::styled(
                    blank_dash(&map_get_text(row, "scope")).to_string(),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled("IDENTITY ", Style::default().fg(Color::Gray)),
                Span::styled(
                    blank_dash(&map_get_text(row, "accountScope")).to_string(),
                    Style::default().fg(Color::LightYellow),
                ),
            ]),
            Line::from(vec![
                Span::styled("TREE ", Style::default().fg(Color::Gray)),
                Span::styled(
                    if map_get_text(row, "expanded") == "true" {
                        "expanded".to_string()
                    } else {
                        "collapsed".to_string()
                    },
                    Style::default().fg(Color::LightBlue),
                ),
            ]),
        ],
        pane_block("Overview", false, Color::LightBlue),
        false,
        state.detail_cursor,
    );
    render_focusable_lines(
        frame,
        sections[1],
        user_detail_lines(row),
        pane_block("Facts", state.focus == PaneFocus::Facts, Color::LightCyan),
        state.focus == PaneFocus::Facts,
        state.detail_cursor,
    );
    render_focusable_lines(
        frame,
        sections[2],
        vec![
            Line::from(vec![
                key_chip("Enter", Color::Blue),
                plain(" expand teams"),
                plain("   "),
                key_chip("Left", Color::Blue),
                plain(" collapse"),
            ]),
            Line::from(vec![key_chip("e", Color::Green), plain(" edit user")]),
            Line::from(vec![
                key_chip("d", Color::Red),
                plain(" delete user"),
                plain("   "),
                key_chip("l", Color::Cyan),
                plain(" refresh"),
            ]),
        ],
        pane_block("Actions", false, Color::LightMagenta),
        false,
        state.detail_cursor,
    );
}

fn user_detail_lines(row: &Map<String, Value>) -> Vec<Line<'static>> {
    let user_id = {
        let value = map_get_text(row, "userId");
        if value.is_empty() {
            map_get_text(row, "id")
        } else {
            value
        }
    };
    vec![
        detail_line("ID", &user_id),
        detail_line("Login", &map_get_text(row, "login")),
        detail_line("Email", &map_get_text(row, "email")),
        detail_line("Name", &map_get_text(row, "name")),
        detail_line("Org", &map_get_text(row, "orgName")),
        detail_line("Org Role", &map_get_text(row, "orgRole")),
        detail_line("Role Summary", &map_get_text(row, "roleSummary")),
        detail_line("Grafana Admin", &map_get_text(row, "grafanaAdmin")),
        detail_line("Org Memberships", &map_get_text(row, "orgMembershipCount")),
        detail_line("Scope", &map_get_text(row, "scope")),
        detail_line("Identity Scope", &map_get_text(row, "accountScope")),
        detail_line("Cross Orgs", &map_get_text(row, "crossOrgMemberships")),
        detail_line("Teams", &map_get_text(row, "teams")),
    ]
}

fn render_org_detail_panel(
    frame: &mut ratatui::Frame,
    area: Rect,
    state: &BrowserState,
    row: &Map<String, Value>,
) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .split(area);
    render_focusable_lines(
        frame,
        sections[0],
        vec![
            Line::from(vec![
                Span::styled(
                    " ORG ",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(46, 78, 122))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    blank_dash(&map_get_text(row, "orgName")).to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!(
                    "id={}   users={}",
                    map_get_text(row, "orgId"),
                    map_get_text(row, "memberCount")
                ),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(Span::styled(
                "Grouped membership header".to_string(),
                Style::default().fg(Color::Gray),
            )),
        ],
        pane_block("Overview", false, Color::LightBlue),
        false,
        state.detail_cursor,
    );
    render_focusable_lines(
        frame,
        sections[1],
        vec![
            detail_line("Org Name", &map_get_text(row, "orgName")),
            detail_line("Org ID", &map_get_text(row, "orgId")),
            detail_line("Users", &map_get_text(row, "memberCount")),
            detail_line("Mode", "org-grouped memberships"),
        ],
        pane_block("Facts", state.focus == PaneFocus::Facts, Color::LightCyan),
        state.focus == PaneFocus::Facts,
        state.detail_cursor,
    );
    render_focusable_lines(
        frame,
        sections[2],
        vec![
            Line::from(vec![
                key_chip("g", Color::Magenta),
                plain(" jump team browse"),
                plain("   "),
                key_chip("v", Color::Magenta),
                plain(" switch view"),
            ]),
            Line::from(vec![
                key_chip("c", Color::Magenta),
                plain(" toggle all teams"),
            ]),
            Line::from(vec![
                key_chip("l", Color::Cyan),
                plain(" refresh"),
                plain("   "),
                key_chip("/", Color::Yellow),
                plain(" search"),
            ]),
        ],
        pane_block("Actions", false, Color::LightMagenta),
        false,
        state.detail_cursor,
    );
}

fn render_team_detail_panel(
    frame: &mut ratatui::Frame,
    area: Rect,
    state: &BrowserState,
    row: &Map<String, Value>,
) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .split(area);
    render_focusable_lines(
        frame,
        sections[0],
        vec![
            Line::from(vec![
                Span::styled(
                    " TEAM ",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(42, 92, 122))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    blank_dash(&map_get_text(row, "teamName")).to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!("user={}", blank_dash(&map_get_text(row, "parentLogin"))),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(Span::styled(
                "Child user-team row".to_string(),
                Style::default().fg(Color::Gray),
            )),
        ],
        pane_block("Overview", false, Color::LightBlue),
        false,
        state.detail_cursor,
    );
    render_focusable_lines(
        frame,
        sections[1],
        vec![
            detail_line("Team/Group", &map_get_text(row, "teamName")),
            detail_line("User", &map_get_text(row, "parentLogin")),
            detail_line("User ID", &map_get_text(row, "parentUserId")),
            detail_line("Row Kind", "team-group membership"),
        ],
        pane_block("Facts", state.focus == PaneFocus::Facts, Color::LightCyan),
        state.focus == PaneFocus::Facts,
        state.detail_cursor,
    );
    render_focusable_lines(
        frame,
        sections[2],
        vec![
            Line::from(vec![
                key_chip("Left", Color::Blue),
                plain(" collapse parent"),
            ]),
            Line::from(vec![
                key_chip("e", Color::DarkGray),
                plain(" user row only"),
                plain("   "),
                key_chip("d", Color::DarkGray),
                plain(" user row only"),
            ]),
        ],
        pane_block("Actions", false, Color::LightMagenta),
        false,
        state.detail_cursor,
    );
}

fn pane_block(title: &str, focused: bool, accent: Color) -> Block<'static> {
    tui_shell::pane_block(title, focused, accent, Color::Reset)
}

fn render_focusable_lines(
    frame: &mut ratatui::Frame,
    area: Rect,
    lines: Vec<Line<'static>>,
    block: Block<'static>,
    focused: bool,
    selected_index: usize,
) {
    let items = if lines.is_empty() {
        vec![ListItem::new(Line::from("-"))]
    } else {
        lines.into_iter().map(ListItem::new).collect::<Vec<_>>()
    };
    if focused {
        let mut state = ListState::default();
        state.select(Some(selected_index.min(items.len().saturating_sub(1))));
        frame.render_stateful_widget(
            List::new(items)
                .block(block)
                .highlight_symbol("▌ ")
                .repeat_highlight_symbol(true)
                .highlight_style(
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                ),
            area,
            &mut state,
        );
    } else {
        frame.render_widget(List::new(items).block(block), area);
    }
}

fn detail_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label:<18}: "),
            Style::default()
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            blank_dash(value).to_string(),
            Style::default().fg(Color::White),
        ),
    ])
}

fn key_chip(label: &'static str, bg: Color) -> Span<'static> {
    tui_shell::key_chip(label, bg)
}

fn control_lines(args: &UserBrowseArgs) -> Vec<Line<'static>> {
    tui_shell::control_grid(&[
        vec![
            ("Up/Down", Color::Blue, "move"),
            ("Tab", Color::Blue, "next pane"),
            (
                "g",
                Color::Magenta,
                if args.input_dir.is_some() {
                    "live-only jump"
                } else {
                    "jump teams"
                },
            ),
            (
                "v",
                Color::Magenta,
                if args.input_dir.is_some() {
                    "live-only view"
                } else {
                    "view"
                },
            ),
            ("c", Color::Magenta, "toggle all"),
            (
                "e",
                Color::Green,
                if args.input_dir.is_some() {
                    "read-only"
                } else {
                    "edit"
                },
            ),
            (
                "d",
                Color::Red,
                if args.input_dir.is_some() {
                    "read-only"
                } else {
                    "delete"
                },
            ),
        ],
        vec![
            ("Shift+Tab", Color::Blue, "previous pane"),
            ("/ ?", Color::Yellow, "search"),
            ("n", Color::Yellow, "next match"),
            ("Home/End", Color::Blue, "jump"),
            ("PgUp/PgDn", Color::Blue, "scroll detail"),
            (
                "l",
                Color::Cyan,
                if args.input_dir.is_some() {
                    "reload bundle"
                } else {
                    "refresh"
                },
            ),
            ("i", Color::Magenta, "numbers"),
        ],
        vec![("q", Color::Gray, "exit"), ("Esc", Color::Gray, "exit")],
    ])
}

fn plain(text: impl Into<std::borrow::Cow<'static, str>>) -> Span<'static> {
    tui_shell::plain(text.into())
}

fn blank_dash(value: &str) -> &str {
    if value.is_empty() {
        "-"
    } else {
        value
    }
}
