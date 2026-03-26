#![cfg(feature = "tui")]
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::time::Duration;

use crate::common::Result;

use super::inspect_workbench_render::render_frame;
use super::inspect_workbench_state::{
    handle_search_key, InspectPane, InspectWorkbenchState, SearchDirection, SearchState,
};
use super::inspect_workbench_support::InspectWorkbenchDocument;

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

pub(crate) fn run_inspect_workbench(document: InspectWorkbenchDocument) -> Result<()> {
    let mut state = InspectWorkbenchState::new(document);
    let mut session = TerminalSession::enter()?;

    loop {
        session
            .terminal
            .draw(|frame| render_frame(frame, &mut state))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if state.full_detail_open {
                match key.code {
                    KeyCode::Up => state.move_full_detail_focus(-1),
                    KeyCode::Down => state.move_full_detail_focus(1),
                    KeyCode::Home => state.set_full_detail_focus(0),
                    KeyCode::End => {
                        let count = state.current_full_detail_lines().len();
                        state.set_full_detail_focus(count.saturating_sub(1));
                    }
                    KeyCode::PageUp => state.move_full_detail_focus(-10),
                    KeyCode::PageDown => state.move_full_detail_focus(10),
                    KeyCode::Char('w') => state.toggle_full_detail_wrap(),
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                        state.close_full_detail();
                    }
                    _ => {}
                }
                continue;
            }
            if state.pending_search.is_some() {
                handle_search_key(&mut state, &key);
                continue;
            }
            match key.code {
                KeyCode::BackTab => {
                    state.focus_previous();
                    state.status = format!("Focused {:?} pane.", state.focus);
                }
                KeyCode::Tab => {
                    state.focus_next();
                    state.status = format!("Focused {:?} pane.", state.focus);
                }
                KeyCode::Up => match state.focus {
                    InspectPane::Groups => state.move_group_selection(-1),
                    InspectPane::Items => state.move_item_selection(-1),
                    InspectPane::Facts => state.move_detail_cursor(-1),
                },
                KeyCode::Down => match state.focus {
                    InspectPane::Groups => state.move_group_selection(1),
                    InspectPane::Items => state.move_item_selection(1),
                    InspectPane::Facts => state.move_detail_cursor(1),
                },
                KeyCode::Left => {
                    if state.focus == InspectPane::Items {
                        state.move_item_horizontal_offset(-4);
                        state.status =
                            "Panned item rows left. Use Left/Right to inspect long item lines."
                                .to_string();
                    }
                }
                KeyCode::Right => {
                    if state.focus == InspectPane::Items {
                        state.move_item_horizontal_offset(4);
                        state.status =
                            "Panned item rows right. Use Left/Right to inspect long item lines."
                                .to_string();
                    }
                }
                KeyCode::Home => match state.focus {
                    InspectPane::Groups => {
                        state
                            .group_state
                            .select((!state.document.groups.is_empty()).then_some(0));
                        state.reset_items();
                    }
                    InspectPane::Items => {
                        state
                            .item_state
                            .select((!state.current_items().is_empty()).then_some(0));
                        state.set_detail_cursor(0);
                    }
                    InspectPane::Facts => state.set_detail_cursor(0),
                },
                KeyCode::End => match state.focus {
                    InspectPane::Groups => {
                        state
                            .group_state
                            .select(state.document.groups.len().checked_sub(1));
                        state.reset_items();
                    }
                    InspectPane::Items => {
                        state
                            .item_state
                            .select(state.current_items().len().checked_sub(1));
                        state.set_detail_cursor(0);
                    }
                    InspectPane::Facts => {
                        let count = state.current_detail_lines().len();
                        state.set_detail_cursor(count.saturating_sub(1));
                    }
                },
                KeyCode::PageUp => {
                    if state.focus == InspectPane::Facts {
                        state.move_detail_cursor(-10);
                    }
                }
                KeyCode::PageDown => {
                    if state.focus == InspectPane::Facts {
                        state.move_detail_cursor(10);
                    }
                }
                KeyCode::Enter => {
                    if matches!(state.focus, InspectPane::Items | InspectPane::Facts) {
                        state.open_full_detail();
                    } else {
                        state.set_detail_cursor(0);
                        state.status = "Reset facts cursor to top.".to_string();
                    }
                }
                KeyCode::Char('/') => state.start_search(SearchDirection::Forward),
                KeyCode::Char('?') => state.start_search(SearchDirection::Backward),
                KeyCode::Char('n') => {
                    if let Some(index) = state.repeat_last_search() {
                        state.item_state.select(Some(index));
                        state.detail_cursor = 0;
                        if let Some(SearchState { query, .. }) = state.last_search.as_ref() {
                            state.status = format!("Matched next inspect row for {query}.");
                        }
                    } else {
                        state.status = "No further inspect search match.".to_string();
                    }
                }
                KeyCode::Char('g') => state.cycle_group(),
                KeyCode::Char('v') => state.cycle_group_view(),
                KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                _ => {}
            }
        }
    }
}
