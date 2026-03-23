//! Shared read-only TUI browser for list/detail artifact inspection.
#[cfg(test)]
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
#[cfg(test)]
use crossterm::execute;
#[cfg(test)]
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
#[cfg(test)]
use ratatui::backend::CrosstermBackend;
#[cfg(test)]
use ratatui::layout::{Constraint, Direction, Layout};
#[cfg(test)]
use ratatui::style::{Color, Modifier, Style};
#[cfg(test)]
use ratatui::text::{Line, Span};
#[cfg(test)]
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
#[cfg(test)]
use ratatui::Terminal;
#[cfg(test)]
use std::io::{self, Stdout};
#[cfg(test)]
use std::time::Duration;

#[cfg(test)]
use crate::common::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserItem {
    pub(crate) kind: String,
    pub(crate) title: String,
    pub(crate) meta: String,
    pub(crate) details: Vec<String>,
}

#[cfg(test)]
struct TerminalSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

#[cfg(test)]
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

#[cfg(test)]
impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(test)]
fn item_color(kind: &str) -> Color {
    match kind {
        "dashboard" => Color::Yellow,
        "alert" | "alert-rule" => Color::Red,
        "datasource" => Color::Cyan,
        "warning" => Color::Yellow,
        "violation" => Color::LightRed,
        "drift" => Color::LightRed,
        "policy" => Color::Magenta,
        _ => Color::Gray,
    }
}

#[cfg(test)]
fn collect_kind_filters(items: &[BrowserItem]) -> Vec<String> {
    let mut filters = vec!["all".to_string()];
    for item in items {
        if !filters.iter().any(|kind| kind == &item.kind) {
            filters.push(item.kind.clone());
        }
    }
    filters
}

#[cfg(test)]
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

#[cfg(test)]
fn selected_detail_line_count(item: Option<&BrowserItem>) -> usize {
    item.map(|candidate| candidate.details.len().max(1))
        .unwrap_or(1)
}

#[cfg(test)]
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
                Block::default().borders(Borders::ALL).title(format!(
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

            let detail_text = selected_item
                .map(|item| item.details.join("\n"))
                .unwrap_or_else(|| "No item selected".to_string());
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
            let detail = Paragraph::new(detail_text)
                .scroll((detail_scroll, 0))
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL).title(detail_title));
            frame.render_widget(detail, panes[1]);

            let footer = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!(
                            "Selection {}/{}",
                            state.selected().map(|index| index + 1).unwrap_or(0),
                            visible_indexes.len()
                        ),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("   "),
                    Span::styled(
                        format!("Filter {}", kind_filters[active_filter]),
                        Style::default().fg(Color::Yellow),
                    ),
                ]),
                Line::from(
                    "Up/Down item  PgUp/PgDn detail  Home/End list  Enter reset detail  f next filter  F prev filter".to_string(),
                ),
                Line::from("q/Esc exit".to_string()),
            ])
            .block(Block::default().borders(Borders::ALL).title("Controls"));
            frame.render_widget(footer, outer[2]);
        })?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Up => {
                    let selected = state.selected().unwrap_or(0);
                    state.select(Some(selected.saturating_sub(1)));
                    detail_scroll = 0;
                }
                KeyCode::Down => {
                    let selected = state.selected().unwrap_or(0);
                    state.select(Some(
                        (selected + 1).min(visible_indexes.len().saturating_sub(1)),
                    ));
                    detail_scroll = 0;
                }
                KeyCode::PageUp => detail_scroll = detail_scroll.saturating_sub(10),
                KeyCode::PageDown => detail_scroll = detail_scroll.saturating_add(10),
                KeyCode::Home => {
                    state.select(Some(0));
                    detail_scroll = 0;
                }
                KeyCode::End => {
                    state.select(Some(visible_indexes.len().saturating_sub(1)));
                    detail_scroll = 0;
                }
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
