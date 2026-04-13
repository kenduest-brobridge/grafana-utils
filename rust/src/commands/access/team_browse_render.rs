//! Interactive browse workflows and terminal-driven state flow for Access entities.

use crate::tui_shell;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use serde_json::{Map, Value};

use super::team_browse_dialog::{
    render_delete_prompt, render_member_remove_prompt, render_search_prompt,
};
use super::team_browse_state::{row_kind, BrowserState, PaneFocus};
use super::TeamBrowseArgs;
use crate::access::render::map_get_text;

pub(super) fn render_frame(
    frame: &mut ratatui::Frame,
    state: &mut BrowserState,
    args: &TeamBrowseArgs,
) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(tui_shell::footer_height(3)),
        ])
        .split(frame.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(outer[1]);

    frame.render_widget(
        tui_shell::build_header(
            "Team Browser",
            vec![
                tui_shell::summary_line(&[
                    tui_shell::summary_cell(
                        "Teams",
                        state.team_rows.len().to_string(),
                        Color::White,
                    ),
                    tui_shell::summary_cell(
                        "Expanded",
                        state.expanded_team_ids.len().to_string(),
                        Color::White,
                    ),
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

    frame.render_stateful_widget(
        List::new(build_list_items(&state.rows, state.show_numbers))
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
            ),
        panes[0],
        &mut state.list_state,
    );

    render_detail_panel(frame, panes[1], state);

    frame.render_widget(
        tui_shell::build_footer(control_lines(args), state.status.clone()),
        outer[2],
    );

    if let Some(edit) = state.pending_edit.as_ref() {
        edit.render(frame);
    }
    if state.pending_delete {
        render_delete_prompt(frame, state.selected_row());
    }
    if state.pending_member_remove {
        render_member_remove_prompt(frame, state.selected_member_row());
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
                "member" => {
                    spans.extend([
                        Span::raw("  "),
                        Span::styled("└─ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(
                            blank_dash(&map_get_text(row, "memberIdentity")).to_string(),
                            Style::default().fg(Color::LightCyan),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!("[{}]", blank_dash(&map_get_text(row, "memberRole"))),
                            Style::default().fg(Color::Yellow),
                        ),
                    ]);
                }
                _ => {
                    let expanded = map_get_text(row, "expanded") == "true";
                    spans.extend([
                        Span::styled(
                            if expanded { "▼ " } else { "▶ " },
                            Style::default().fg(Color::LightBlue),
                        ),
                        Span::styled(
                            blank_dash(&map_get_text(row, "name")).to_string(),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            format!(
                                "id={} members={}",
                                blank_dash(&map_get_text(row, "id")),
                                blank_dash(&map_get_text(row, "memberCount"))
                            ),
                            Style::default().fg(Color::Gray),
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
            Paragraph::new("No team selected.")
                .block(Block::default().borders(Borders::ALL).title("Detail")),
            area,
        );
        return;
    };
    if row_kind(row) == "member" {
        render_member_detail_panel(frame, area, state, row);
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
                    " TEAM ",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(98, 46, 122))
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
                    "id={}   members={}",
                    blank_dash(&map_get_text(row, "id")),
                    blank_dash(&map_get_text(row, "memberCount"))
                ),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(vec![
                Span::styled("EMAIL ", Style::default().fg(Color::Gray)),
                Span::styled(
                    blank_dash(&map_get_text(row, "email")).to_string(),
                    Style::default().fg(Color::White),
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
                    Style::default().fg(Color::LightYellow),
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
        team_detail_lines(row),
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
                plain(" jump user browse"),
                plain("   "),
                key_chip("c", Color::Magenta),
                plain(" toggle all"),
            ]),
            Line::from(vec![
                key_chip("e", Color::Green),
                plain(" edit members/admins"),
            ]),
            Line::from(vec![
                key_chip("d", Color::Red),
                plain(" delete team"),
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

fn team_detail_lines(row: &Map<String, Value>) -> Vec<Line<'static>> {
    vec![
        detail_line("ID", &map_get_text(row, "id")),
        detail_line("Name", &map_get_text(row, "name")),
        detail_line("Email", &map_get_text(row, "email")),
        detail_line("Member Count", &map_get_text(row, "memberCount")),
        detail_line("Members", &map_get_text(row, "members")),
    ]
}

fn render_member_detail_panel(
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
                    " MEMBER ",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(42, 92, 122))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    blank_dash(&map_get_text(row, "memberIdentity")).to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                format!("team={}", blank_dash(&map_get_text(row, "parentTeamName"))),
                Style::default().fg(Color::Cyan),
            )),
            Line::from(Span::styled(
                "Child team-member row".to_string(),
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
            detail_line("Login", &map_get_text(row, "memberLogin")),
            detail_line("Email", &map_get_text(row, "memberEmail")),
            detail_line("Name", &map_get_text(row, "memberName")),
            detail_line("Role", &map_get_text(row, "memberRole")),
            detail_line("Team", &map_get_text(row, "parentTeamName")),
            detail_line("Team ID", &map_get_text(row, "parentTeamId")),
            detail_line("Row Kind", "team-member"),
        ],
        pane_block("Facts", state.focus == PaneFocus::Facts, Color::LightCyan),
        state.focus == PaneFocus::Facts,
        state.detail_cursor,
    );
    render_focusable_lines(
        frame,
        sections[2],
        member_action_lines(row),
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

fn member_action_lines(row: &Map<String, Value>) -> Vec<Line<'static>> {
    let is_admin = map_get_text(row, "memberRole").eq_ignore_ascii_case("admin");
    vec![
        Line::from(vec![
            key_chip("Left", Color::Blue),
            plain(" collapse parent"),
        ]),
        Line::from(vec![
            key_chip("e", Color::DarkGray),
            plain(" use access user browse"),
        ]),
        Line::from(vec![key_chip("r", Color::Red), plain(" remove membership")]),
        Line::from(vec![
            key_chip("a", Color::Magenta),
            plain(if is_admin {
                " revoke team admin"
            } else {
                " grant team admin"
            }),
        ]),
    ]
}

fn key_chip(label: &'static str, bg: Color) -> Span<'static> {
    tui_shell::key_chip(label, bg)
}

fn control_lines(args: &TeamBrowseArgs) -> Vec<Line<'static>> {
    tui_shell::control_grid(&[
        vec![
            ("Up/Down", Color::Blue, "move"),
            ("Tab", Color::Blue, "next pane"),
            ("Enter", Color::Blue, "expand"),
            ("Left", Color::Blue, "collapse"),
            (
                "r",
                Color::Red,
                if args.input_dir.is_some() {
                    "read-only"
                } else {
                    "remove member"
                },
            ),
            (
                "g",
                Color::Magenta,
                if args.input_dir.is_some() {
                    "live-only jump"
                } else {
                    "jump users"
                },
            ),
        ],
        vec![
            ("c", Color::Magenta, "toggle all"),
            (
                "a",
                Color::Magenta,
                if args.input_dir.is_some() {
                    "read-only"
                } else {
                    "admin toggle"
                },
            ),
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
            ("Shift+Tab", Color::Blue, "previous pane"),
        ],
        vec![
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
            ("q", Color::Gray, "exit"),
            ("Esc", Color::Gray, "exit"),
        ],
    ])
}

fn plain(text: impl Into<std::borrow::Cow<'static, str>>) -> Span<'static> {
    tui_shell::plain(text.into())
}

fn blank_dash(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "-"
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::CommonCliArgs;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use serde_json::{Map, Value};

    fn args() -> TeamBrowseArgs {
        TeamBrowseArgs {
            common: CommonCliArgs {
                profile: None,
                url: "http://127.0.0.1:3000".to_string(),
                api_token: Some("token".to_string()),
                username: None,
                password: None,
                prompt_password: false,
                prompt_token: false,
                org_id: None,
                timeout: 30,
                verify_ssl: false,
                insecure: false,
                ca_cert: None,
            },
            input_dir: None,
            query: None,
            name: None,
            with_members: true,
            page: 1,
            per_page: 100,
        }
    }

    fn state() -> BrowserState {
        BrowserState::new(vec![Map::from_iter(vec![
            ("id".to_string(), Value::String("7".to_string())),
            (
                "name".to_string(),
                Value::String("platform-ops".to_string()),
            ),
            (
                "email".to_string(),
                Value::String("platform@example.com".to_string()),
            ),
            ("memberCount".to_string(), Value::String("2".to_string())),
        ])])
    }

    fn state_with_selected_member() -> BrowserState {
        let mut state = BrowserState::new(vec![Map::from_iter(vec![
            ("id".to_string(), Value::String("7".to_string())),
            (
                "name".to_string(),
                Value::String("platform-ops".to_string()),
            ),
            (
                "memberRows".to_string(),
                Value::Array(vec![Value::Object(Map::from_iter(vec![
                    (
                        "memberIdentity".to_string(),
                        Value::String("alice".to_string()),
                    ),
                    (
                        "memberLogin".to_string(),
                        Value::String("alice".to_string()),
                    ),
                    (
                        "memberEmail".to_string(),
                        Value::String("alice@example.com".to_string()),
                    ),
                    (
                        "memberRole".to_string(),
                        Value::String("Member".to_string()),
                    ),
                ]))]),
            ),
        ])]);
        state.expand_selected();
        state.select_index(1);
        state
    }

    #[test]
    fn team_delete_confirmation_renders_as_dialog_overlay() {
        let mut state = state();
        state.pending_delete = true;
        let args = args();
        let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();

        terminal
            .draw(|frame| render_frame(frame, &mut state, &args))
            .unwrap();

        let screen = format!("{}", terminal.backend());
        assert!(screen.contains("Delete team"));
        assert!(screen.contains("Press y to confirm delete."));
        assert!(!screen.contains("Delete Preview"));
    }

    #[test]
    fn member_remove_confirmation_renders_as_dialog_overlay() {
        let mut state = state_with_selected_member();
        state.pending_member_remove = true;
        let args = args();
        let mut terminal = Terminal::new(TestBackend::new(140, 40)).unwrap();

        terminal
            .draw(|frame| render_frame(frame, &mut state, &args))
            .unwrap();

        let screen = format!("{}", terminal.backend());
        assert!(screen.contains("Remove membership"));
        assert!(screen.contains("Remove member alice from team platform-ops"));
        assert!(screen.contains("Press y to confirm removal."));
    }
}
