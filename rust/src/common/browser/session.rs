//! Shared read-only TUI browser for list/detail artifact inspection.
#![cfg_attr(not(test), allow(dead_code))]
#[cfg(not(feature = "tui"))]
use crate::common::tui;
use crate::common::Result;
#[cfg(feature = "tui")]
use crate::tui_shell;

#[cfg(feature = "tui")]
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
#[cfg(feature = "tui")]
use crossterm::execute;
#[cfg(feature = "tui")]
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
#[cfg(feature = "tui")]
use ratatui::backend::CrosstermBackend;
#[cfg(feature = "tui")]
use ratatui::layout::{Constraint, Direction, Layout};
#[cfg(feature = "tui")]
use ratatui::style::{Color, Modifier, Style};
#[cfg(feature = "tui")]
use ratatui::text::{Line, Span};
#[cfg(feature = "tui")]
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
#[cfg(feature = "tui")]
use ratatui::Terminal;
#[cfg(feature = "tui")]
use std::io::{self, Stdout};
#[cfg(feature = "tui")]
use std::time::Duration;

#[cfg(any(feature = "tui", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BrowserPane {
    Items,
    Detail,
}

#[cfg_attr(not(feature = "tui"), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserItem {
    pub(crate) kind: String,
    pub(crate) title: String,
    pub(crate) meta: String,
    pub(crate) details: Vec<String>,
}

#[cfg(feature = "tui")]
struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

#[cfg(feature = "tui")]
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

#[cfg(feature = "tui")]
impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(feature = "tui")]
fn item_color(kind: &str) -> Color {
    match kind {
        "dashboard" => Color::Yellow,
        "alert" | "alert-rule" => Color::Red,
        "datasource" => Color::Cyan,
        "user" => Color::Green,
        "team" => Color::LightMagenta,
        "warning" => Color::Yellow,
        "violation" => Color::LightRed,
        "drift" => Color::LightRed,
        "policy" => Color::Magenta,
        _ => Color::Gray,
    }
}

#[cfg(feature = "tui")]
fn collect_kind_filters(items: &[BrowserItem]) -> Vec<String> {
    let mut filters = vec!["all".to_string()];
    for item in items {
        if !filters.iter().any(|kind| kind == &item.kind) {
            filters.push(item.kind.clone());
        }
    }
    filters
}

#[cfg(feature = "tui")]
fn visible_item_indexes(items: &[BrowserItem], filter_kind: &str) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            if filter_kind == "all" || item.kind == filter_kind {
                Some(index)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(feature = "tui")]
fn selected_detail_line_count(item: Option<&BrowserItem>) -> usize {
    item.map(|candidate| candidate.details.len().max(1))
        .unwrap_or(1)
}

#[cfg(feature = "tui")]
pub(crate) fn run_interactive_browser(
    title: &str,
    summary_lines: &[String],
    items: &[BrowserItem],
) -> Result<()> {
    let mut session = TerminalSession::enter()?;
    let mut state = ListState::default();
    let kind_filters = collect_kind_filters(items);
    let mut active_filter = 0usize;
    let mut visible_indexes = visible_item_indexes(items, &kind_filters[active_filter]);
    state.select((!visible_indexes.is_empty()).then_some(0));
    let mut detail_scroll = 0u16;
    let mut pane_focus = BrowserPane::Items;

    loop {
        session.terminal.draw(|frame| {
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length((summary_lines.len().max(1) + 2) as u16),
                    Constraint::Min(1),
                    Constraint::Length(4),
                ])
                .split(frame.area());
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(outer[1]);
            let selected_visible = state.selected().unwrap_or(0);
            let selected_item = visible_indexes
                .get(selected_visible)
                .and_then(|index| items.get(*index));
            let total_detail_lines = selected_detail_line_count(selected_item);
            let detail_lines = selected_item
                .map(|item| {
                    if item.details.is_empty() {
                        vec!["No detail lines.".to_string()]
                    } else {
                        item.details.clone()
                    }
                })
                .unwrap_or_else(|| vec!["No item selected".to_string()]);
            let detail_selected =
                (detail_scroll as usize).min(detail_lines.len().saturating_sub(1));

            let summary = Paragraph::new(summary_lines.join("\n"))
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL).title(title));
            frame.render_widget(summary, outer[0]);

            let list = List::new(
                visible_indexes
                    .iter()
                    .enumerate()
                    .map(|(visible_index, item_index)| {
                        let item = &items[*item_index];
                        let line = Line::from(vec![
                            Span::styled(
                                format!("{:>2}. ", visible_index + 1),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled(
                                format!("[{}]", item.kind.to_uppercase()),
                                Style::default()
                                    .fg(item_color(&item.kind))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::raw(format!(" {}", item.title)),
                            Span::styled(
                                format!("  {}", item.meta),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]);
                        ListItem::new(line)
                    })
                    .collect::<Vec<_>>(),
            )
            .block(
                pane_block(
                    "Items",
                    pane_focus == BrowserPane::Items,
                    Color::Cyan,
                    Color::Black,
                )
                .title(format!(
                    "Items {}/{}  filter:{}",
                    visible_indexes.len(),
                    items.len(),
                    kind_filters[active_filter]
                )),
            )
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED),
            );
            frame.render_stateful_widget(list, panes[0], &mut state);

            let detail_title = selected_item
                .map(|item| {
                    let item_position = visible_indexes
                        .get(selected_visible)
                        .map(|index| index + 1)
                        .unwrap_or(0);
                    format!(
                        "Detail {}/{} [{}]  line {}/{}",
                        item_position,
                        items.len(),
                        item.kind,
                        (detail_scroll as usize + 1).min(total_detail_lines),
                        total_detail_lines
                    )
                })
                .unwrap_or_else(|| "Detail".to_string());
            let detail_items = detail_lines
                .iter()
                .map(|line| {
                    ListItem::new(Line::from(Span::styled(
                        line.clone(),
                        Style::default().fg(Color::White),
                    )))
                })
                .collect::<Vec<_>>();
            if pane_focus == BrowserPane::Detail {
                let mut detail_state = ListState::default();
                detail_state.select(Some(detail_selected));
                let detail = List::new(detail_items)
                    .block(
                        pane_block("Detail", true, Color::LightBlue, Color::Black)
                            .title(detail_title),
                    )
                    .highlight_symbol("▌ ")
                    .repeat_highlight_symbol(true)
                    .highlight_style(
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    );
                frame.render_stateful_widget(detail, panes[1], &mut detail_state);
            } else {
                let detail = List::new(detail_items).block(
                    pane_block("Detail", false, Color::LightBlue, Color::Black).title(detail_title),
                );
                frame.render_widget(detail, panes[1]);
            }

            frame.render_widget(
                tui_shell::build_footer_controls(vec![
                    Line::from(vec![
                        tui_shell::label("Selection "),
                        tui_shell::accent(
                            format!(
                                "{}/{}",
                                state.selected().map(|index| index + 1).unwrap_or(0),
                                visible_indexes.len()
                            ),
                            Color::White,
                        ),
                        Span::raw("  "),
                        tui_shell::label("Filter "),
                        tui_shell::accent(kind_filters[active_filter].to_string(), Color::Yellow),
                        Span::raw("  "),
                        tui_shell::focus_label("Focus "),
                        tui_shell::key_chip(
                            match pane_focus {
                                BrowserPane::Items => "Items",
                                BrowserPane::Detail => "Detail",
                            },
                            Color::Blue,
                        ),
                    ]),
                    tui_shell::control_line(&[
                        ("Tab", Color::Blue, "next pane"),
                        ("Shift+Tab", Color::Blue, "previous pane"),
                        ("Up/Down", Color::Blue, "move"),
                        ("PgUp/PgDn", Color::Blue, "scroll detail"),
                    ]),
                    tui_shell::control_line(&[
                        ("f/F", Color::Yellow, "change filter"),
                        ("Home/End", Color::Blue, "jump"),
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
            let selected_visible = state.selected().unwrap_or(0);
            let selected_item = visible_indexes
                .get(selected_visible)
                .and_then(|index| items.get(*index));
            let total_detail_lines = selected_detail_line_count(selected_item);
            match key.code {
                KeyCode::BackTab => {
                    pane_focus = match pane_focus {
                        BrowserPane::Items => BrowserPane::Detail,
                        BrowserPane::Detail => BrowserPane::Items,
                    };
                }
                KeyCode::Tab => {
                    pane_focus = match pane_focus {
                        BrowserPane::Items => BrowserPane::Detail,
                        BrowserPane::Detail => BrowserPane::Items,
                    };
                }
                KeyCode::Up => match pane_focus {
                    BrowserPane::Items => {
                        let selected = state.selected().unwrap_or(0);
                        state.select(Some(selected.saturating_sub(1)));
                        detail_scroll = 0;
                    }
                    BrowserPane::Detail => {
                        detail_scroll = detail_scroll.saturating_sub(1);
                    }
                },
                KeyCode::Down => match pane_focus {
                    BrowserPane::Items => {
                        let selected = state.selected().unwrap_or(0);
                        state.select(Some(
                            (selected + 1).min(visible_indexes.len().saturating_sub(1)),
                        ));
                        detail_scroll = 0;
                    }
                    BrowserPane::Detail => {
                        detail_scroll = detail_scroll
                            .saturating_add(1)
                            .min(total_detail_lines.saturating_sub(1) as u16);
                    }
                },
                KeyCode::PageUp => {
                    detail_scroll = detail_scroll.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    detail_scroll = detail_scroll
                        .saturating_add(10)
                        .min(total_detail_lines.saturating_sub(1) as u16);
                }
                KeyCode::Home => match pane_focus {
                    BrowserPane::Items => {
                        state.select(Some(0));
                        detail_scroll = 0;
                    }
                    BrowserPane::Detail => detail_scroll = 0,
                },
                KeyCode::End => match pane_focus {
                    BrowserPane::Items => {
                        state.select(Some(visible_indexes.len().saturating_sub(1)));
                        detail_scroll = 0;
                    }
                    BrowserPane::Detail => {
                        detail_scroll = total_detail_lines.saturating_sub(1) as u16;
                    }
                },
                KeyCode::Enter => detail_scroll = 0,
                KeyCode::Char('f') => {
                    active_filter = (active_filter + 1) % kind_filters.len();
                    visible_indexes = visible_item_indexes(items, &kind_filters[active_filter]);
                    state.select((!visible_indexes.is_empty()).then_some(0));
                    detail_scroll = 0;
                }
                KeyCode::Char('F') => {
                    active_filter = if active_filter == 0 {
                        kind_filters.len().saturating_sub(1)
                    } else {
                        active_filter - 1
                    };
                    visible_indexes = visible_item_indexes(items, &kind_filters[active_filter]);
                    state.select((!visible_indexes.is_empty()).then_some(0));
                    detail_scroll = 0;
                }
                KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                _ => {}
            }
        }
    }
}

#[cfg(feature = "tui")]
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
                .add_modifier(Modifier::BOLD),
        )
}

#[cfg(not(feature = "tui"))]
pub(crate) fn run_interactive_browser(
    _title: &str,
    _summary_lines: &[String],
    _items: &[BrowserItem],
) -> Result<()> {
    Err(tui(
        "Shared interactive browser requires the `tui` feature.",
    ))
}

#[cfg(all(test, not(feature = "tui")))]
#[test]
fn run_interactive_browser_returns_tui_error_when_feature_disabled() {
    let error = run_interactive_browser(
        "Test",
        &[],
        &[BrowserItem {
            kind: "dashboard".to_string(),
            title: "Example".to_string(),
            meta: "meta".to_string(),
            details: vec!["detail".to_string()],
        }],
    )
    .expect_err("feature-disabled browser should return an error");

    assert_eq!(
        error.to_string(),
        "Shared interactive browser requires the `tui` feature."
    );
}
