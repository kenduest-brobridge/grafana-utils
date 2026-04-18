//! Interactive sync review TUI.
//! Allows operators to keep or drop actionable sync operations before the plan is marked reviewed.
#[cfg(feature = "tui")]
use crate::common::message;
#[cfg(not(feature = "tui"))]
use crate::common::tui;
use crate::common::Result;

#[cfg(feature = "tui")]
use crate::tui_shell;
#[cfg(feature = "tui")]
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
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
use ratatui::text::Line;
#[cfg(feature = "tui")]
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
#[cfg(feature = "tui")]
use ratatui::Terminal;
use serde_json::Value;
#[cfg(any(feature = "tui", test))]
use std::collections::BTreeSet;
#[cfg(feature = "tui")]
use std::io::{self, Stdout};
#[cfg(feature = "tui")]
use std::time::Duration;

#[path = "review_tui_helpers.rs"]
mod review_tui_helpers;

#[allow(unused_imports)]
#[cfg(feature = "tui")]
pub(crate) use review_tui_helpers::{
    build_checklist_line, build_diff_controls_lines, operation_badge_color, operation_row_color,
    render_diff_items,
};
#[allow(unused_imports)]
#[cfg(any(feature = "tui", test))]
pub(crate) use review_tui_helpers::{
    build_review_operation_diff_model, clip_text_window, collect_reviewable_operations,
    diff_pane_title, diff_scroll_max, filter_review_plan_operations, operation_changed_count,
    operation_detail_line_count, operation_preview, selection_title_with_position,
    wrap_text_chunks, DiffControlsState, DiffPaneFocus, ReviewDiffLine, ReviewDiffModel,
    ReviewableOperation,
};

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
pub(crate) fn build_review_header_lines(
    item_count: usize,
    selected_count: usize,
    diff_mode: bool,
    diff_focus: DiffPaneFocus,
) -> Vec<Line<'static>> {
    vec![
        Line::from(format!(
            "Reviewable staged operations={}   selected={}   pending-drop={}",
            item_count,
            selected_count,
            item_count.saturating_sub(selected_count)
        )),
        Line::from(format!(
            "Mode={}   active-pane={}",
            if diff_mode { "diff" } else { "checklist" },
            match diff_focus {
                DiffPaneFocus::Live if diff_mode => "live",
                DiffPaneFocus::Desired if diff_mode => "desired",
                _ => "operations",
            }
        )),
        Line::from(
            "Keep the staged plan primary. Review operations first, then confirm the staged selection."
                .to_string(),
        ),
    ]
}

#[cfg(feature = "tui")]
pub(crate) fn review_status(diff_mode: bool) -> String {
    if diff_mode {
        "Diff mode active. Tab switches pane, Esc returns to the checklist, c confirms the staged selection.".to_string()
    } else {
        "Checklist mode active. Space keeps or drops staged operations, Enter opens the diff view, c confirms the staged selection.".to_string()
    }
}

#[cfg(feature = "tui")]
pub(crate) fn run_sync_review_tui(plan: &Value) -> Result<Value> {
    let items = collect_reviewable_operations(plan)?;
    if items.is_empty() {
        return Ok(plan.clone());
    }
    let mut session = TerminalSession::enter()?;
    let mut selected_keys = items
        .iter()
        .map(|item| item.key.clone())
        .collect::<BTreeSet<_>>();
    let mut state = ListState::default();
    state.select(Some(0));
    let mut diff_mode = false;
    let mut diff_focus = DiffPaneFocus::Live;
    let mut live_diff_cursor = 0usize;
    let mut desired_diff_cursor = 0usize;
    let mut live_horizontal_offset = 0usize;
    let mut desired_horizontal_offset = 0usize;
    let mut live_wrap_lines = false;
    let mut desired_wrap_lines = false;

    loop {
        session.terminal.draw(|frame| {
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(5),
                    Constraint::Min(1),
                    Constraint::Length(if diff_mode { 0 } else { 4 }),
                    Constraint::Length(4),
                ])
                .split(frame.area());
            let selected = state.selected().unwrap_or(0);
            let selected_item = items.get(selected);
            let selected_count = selected_keys.len();
            frame.render_widget(
                tui_shell::build_header(
                    "Sync Review",
                    build_review_header_lines(items.len(), selected_count, diff_mode, diff_focus),
                ),
                outer[0],
            );
            if diff_mode {
                let model = selected_item
                    .and_then(|item| build_review_operation_diff_model(&item.operation).ok());
                let panes = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(outer[1]);
                if let Some(model) = model {
                    let action_color = operation_badge_color(&model.action);
                    let mut live_state = ListState::default();
                    live_state.select(Some(
                        live_diff_cursor.min(model.live_lines.len().saturating_sub(1)),
                    ));
                    let live = List::new(render_diff_items(
                        &model.live_lines,
                        Color::Red,
                        panes[0].width.saturating_sub(5) as usize,
                        live_wrap_lines,
                        live_horizontal_offset,
                    ))
                    .block(
                        Block::default()
                            .title(diff_pane_title(
                                "Live",
                                &model.action,
                                &model.title,
                                selected,
                                items.len(),
                            ))
                            .border_style(Style::default().fg(
                                if diff_focus == DiffPaneFocus::Live {
                                    Color::Cyan
                                } else {
                                    action_color
                                },
                            ))
                            .borders(Borders::ALL),
                    )
                    .highlight_symbol("▌ ")
                    .repeat_highlight_symbol(true)
                    .highlight_style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                    );
                    let mut desired_state = ListState::default();
                    desired_state.select(Some(
                        desired_diff_cursor.min(model.desired_lines.len().saturating_sub(1)),
                    ));
                    let desired = List::new(render_diff_items(
                        &model.desired_lines,
                        Color::Green,
                        panes[1].width.saturating_sub(5) as usize,
                        desired_wrap_lines,
                        desired_horizontal_offset,
                    ))
                    .block(
                        Block::default()
                            .title(diff_pane_title(
                                "Desired",
                                &model.action,
                                &model.title,
                                selected,
                                items.len(),
                            ))
                            .border_style(Style::default().fg(
                                if diff_focus == DiffPaneFocus::Desired {
                                    Color::Cyan
                                } else {
                                    action_color
                                },
                            ))
                            .borders(Borders::ALL),
                    )
                    .highlight_symbol("▌ ")
                    .repeat_highlight_symbol(true)
                    .highlight_style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                    );
                    frame.render_stateful_widget(live, panes[0], &mut live_state);
                    frame.render_stateful_widget(desired, panes[1], &mut desired_state);
                }
                frame.render_widget(
                    tui_shell::build_footer(
                        build_diff_controls_lines(&DiffControlsState {
                            selected,
                            total: items.len(),
                            diff_focus,
                            live_wrap_lines,
                            desired_wrap_lines,
                            live_diff_cursor,
                            live_horizontal_offset,
                            desired_diff_cursor,
                            desired_horizontal_offset,
                        }),
                        selected_item
                            .map(|item| {
                                let preview = operation_preview(item).join("   ");
                                format!("{}   {}", review_status(true), preview)
                            })
                            .unwrap_or_else(|| review_status(true)),
                    ),
                    outer[2],
                );
            } else {
                let list_items = items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        let action = item
                            .operation
                            .get("action")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown");
                        ListItem::new(build_checklist_line(
                            item,
                            index,
                            selected_keys.contains(&item.key),
                            outer[1].width.saturating_sub(6) as usize,
                        ))
                        .style(Style::default().fg(operation_row_color(action)))
                    })
                    .collect::<Vec<_>>();
                let list = List::new(list_items)
                    .block(
                        Block::default()
                            .title("Sync Review Checklist")
                            .borders(Borders::ALL),
                    )
                    .highlight_symbol("▌ ")
                    .repeat_highlight_symbol(true)
                    .highlight_style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Cyan)
                            .add_modifier(Modifier::BOLD | Modifier::REVERSED),
                    );
                frame.render_stateful_widget(list, outer[1], &mut state);
                let preview = Paragraph::new(
                    selected_item
                        .map(operation_preview)
                        .unwrap_or_else(|| vec!["No operation selected".to_string()])
                        .join("\n"),
                )
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(selection_title_with_position(
                            selected_item,
                            state.selected(),
                            Some(items.len()),
                        ))
                        .border_style(
                            Style::default().fg(selected_item
                                .and_then(|item| {
                                    item.operation
                                        .get("action")
                                        .and_then(Value::as_str)
                                        .map(operation_badge_color)
                                })
                                .unwrap_or(Color::Gray)),
                        ),
                );
                frame.render_widget(preview, outer[2]);
                frame.render_widget(
                    tui_shell::build_footer(
                        vec![
                            tui_shell::control_line(&[
                                ("Up/Down", Color::Blue, "move"),
                                ("Space", Color::Yellow, "keep/drop"),
                                ("a", Color::Cyan, "select-all"),
                                ("n", Color::Cyan, "select-none"),
                                ("Enter", Color::Blue, "open diff"),
                                ("c", Color::Green, "confirm staged selection"),
                            ]),
                            tui_shell::control_line(&[
                                ("q", Color::Gray, "cancel"),
                                ("Esc", Color::Gray, "cancel"),
                            ]),
                        ],
                        review_status(false),
                    ),
                    outer[3],
                );
            }
        })?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            let selected = state.selected().unwrap_or(0);
            match key.code {
                KeyCode::Up if diff_mode => match diff_focus {
                    DiffPaneFocus::Live => {
                        live_diff_cursor = live_diff_cursor.saturating_sub(1);
                    }
                    DiffPaneFocus::Desired => {
                        desired_diff_cursor = desired_diff_cursor.saturating_sub(1);
                    }
                },
                KeyCode::Up => {
                    let next = selected.saturating_sub(1);
                    state.select(Some(next));
                }
                KeyCode::Down if diff_mode => {
                    if let Some(item) = items.get(selected) {
                        if let Ok(model) = build_review_operation_diff_model(&item.operation) {
                            match diff_focus {
                                DiffPaneFocus::Live => {
                                    live_diff_cursor = (live_diff_cursor + 1)
                                        .min(diff_scroll_max(&model, DiffPaneFocus::Live));
                                }
                                DiffPaneFocus::Desired => {
                                    desired_diff_cursor = (desired_diff_cursor + 1)
                                        .min(diff_scroll_max(&model, DiffPaneFocus::Desired));
                                }
                            }
                        }
                    }
                }
                KeyCode::Down => {
                    let next = (selected + 1).min(items.len().saturating_sub(1));
                    state.select(Some(next));
                }
                KeyCode::Left if diff_mode => match diff_focus {
                    DiffPaneFocus::Live if !live_wrap_lines => {
                        live_horizontal_offset = live_horizontal_offset.saturating_sub(4);
                    }
                    DiffPaneFocus::Desired if !desired_wrap_lines => {
                        desired_horizontal_offset = desired_horizontal_offset.saturating_sub(4);
                    }
                    _ => {}
                },
                KeyCode::Right if diff_mode => match diff_focus {
                    DiffPaneFocus::Live if !live_wrap_lines => live_horizontal_offset += 4,
                    DiffPaneFocus::Desired if !desired_wrap_lines => desired_horizontal_offset += 4,
                    _ => {}
                },
                KeyCode::Char('[') if diff_mode => {
                    let next = selected.saturating_sub(1);
                    state.select(Some(next));
                    live_diff_cursor = 0;
                    desired_diff_cursor = 0;
                    live_horizontal_offset = 0;
                    desired_horizontal_offset = 0;
                }
                KeyCode::Char(']') if diff_mode => {
                    let next = (selected + 1).min(items.len().saturating_sub(1));
                    state.select(Some(next));
                    live_diff_cursor = 0;
                    desired_diff_cursor = 0;
                    live_horizontal_offset = 0;
                    desired_horizontal_offset = 0;
                }
                KeyCode::Tab if diff_mode => {
                    diff_focus = match diff_focus {
                        DiffPaneFocus::Live => DiffPaneFocus::Desired,
                        DiffPaneFocus::Desired => DiffPaneFocus::Live,
                    };
                }
                KeyCode::Char('w') | KeyCode::Char('W') if diff_mode => {
                    let apply_both = key.modifiers.contains(KeyModifiers::SHIFT)
                        || matches!(key.code, KeyCode::Char('W'));
                    if apply_both {
                        let next_wrap = !(live_wrap_lines && desired_wrap_lines);
                        live_wrap_lines = next_wrap;
                        desired_wrap_lines = next_wrap;
                        if next_wrap {
                            live_horizontal_offset = 0;
                            desired_horizontal_offset = 0;
                        }
                    } else {
                        match diff_focus {
                            DiffPaneFocus::Live => {
                                live_wrap_lines = !live_wrap_lines;
                                if live_wrap_lines {
                                    live_horizontal_offset = 0;
                                }
                            }
                            DiffPaneFocus::Desired => {
                                desired_wrap_lines = !desired_wrap_lines;
                                if desired_wrap_lines {
                                    desired_horizontal_offset = 0;
                                }
                            }
                        }
                    }
                }
                KeyCode::PageUp if diff_mode => match diff_focus {
                    DiffPaneFocus::Live => {
                        live_diff_cursor = live_diff_cursor.saturating_sub(10);
                    }
                    DiffPaneFocus::Desired => {
                        desired_diff_cursor = desired_diff_cursor.saturating_sub(10);
                    }
                },
                KeyCode::PageDown if diff_mode => {
                    if let Some(item) = items.get(selected) {
                        if let Ok(model) = build_review_operation_diff_model(&item.operation) {
                            match diff_focus {
                                DiffPaneFocus::Live => {
                                    live_diff_cursor = (live_diff_cursor + 10)
                                        .min(diff_scroll_max(&model, DiffPaneFocus::Live));
                                }
                                DiffPaneFocus::Desired => {
                                    desired_diff_cursor = (desired_diff_cursor + 10)
                                        .min(diff_scroll_max(&model, DiffPaneFocus::Desired));
                                }
                            }
                        }
                    }
                }
                KeyCode::Home if diff_mode => match diff_focus {
                    DiffPaneFocus::Live => live_diff_cursor = 0,
                    DiffPaneFocus::Desired => desired_diff_cursor = 0,
                },
                KeyCode::End if diff_mode => {
                    if let Some(item) = items.get(selected) {
                        if let Ok(model) = build_review_operation_diff_model(&item.operation) {
                            match diff_focus {
                                DiffPaneFocus::Live => {
                                    live_diff_cursor = diff_scroll_max(&model, DiffPaneFocus::Live);
                                }
                                DiffPaneFocus::Desired => {
                                    desired_diff_cursor =
                                        diff_scroll_max(&model, DiffPaneFocus::Desired);
                                }
                            }
                        }
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(item) = items.get(selected) {
                        if !selected_keys.insert(item.key.clone()) {
                            selected_keys.remove(&item.key);
                        }
                    }
                }
                KeyCode::Char('a') if !diff_mode => {
                    selected_keys = items.iter().map(|item| item.key.clone()).collect();
                }
                KeyCode::Char('n') if !diff_mode => {
                    selected_keys.clear();
                }
                KeyCode::Enter if !diff_mode => {
                    diff_mode = true;
                    diff_focus = DiffPaneFocus::Live;
                    live_diff_cursor = 0;
                    desired_diff_cursor = 0;
                    live_horizontal_offset = 0;
                    desired_horizontal_offset = 0;
                }
                KeyCode::Char('c') => return filter_review_plan_operations(plan, &selected_keys),
                KeyCode::Char('q') | KeyCode::Esc if diff_mode => {
                    diff_mode = false;
                    diff_focus = DiffPaneFocus::Live;
                    live_diff_cursor = 0;
                    desired_diff_cursor = 0;
                    live_horizontal_offset = 0;
                    desired_horizontal_offset = 0;
                    continue;
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    return Err(message("Interactive sync review cancelled."));
                }
                _ => {}
            }
        }
    }
}

#[cfg(not(feature = "tui"))]
pub(crate) fn run_sync_review_tui(plan: &Value) -> Result<Value> {
    let _ = plan;
    Err(tui(
        "Sync review interactive TUI requires the `tui` feature.",
    ))
}

#[cfg(not(feature = "tui"))]
#[test]
fn run_sync_review_tui_returns_tui_error_when_feature_disabled() {
    let plan = serde_json::json!({
        "kind": "grafana-utils-sync-plan",
        "operations": []
    });

    let error = run_sync_review_tui(&plan).expect_err("feature-disabled review should error");
    assert_eq!(
        error.to_string(),
        "Sync review interactive TUI requires the `tui` feature."
    );
}
