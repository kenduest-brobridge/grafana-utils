use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use super::history::DashboardHistoryVersion;

#[derive(Clone, Debug)]
pub(crate) struct HistoryDialogState {
    pub(crate) dashboard_uid: String,
    pub(crate) dashboard_title: String,
    versions: Vec<DashboardHistoryVersion>,
    selected_index: usize,
    pending_restore: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HistoryDialogAction {
    Continue,
    Close,
    Restore { uid: String, version: i64 },
}

impl HistoryDialogState {
    pub(crate) fn new(
        dashboard_uid: String,
        dashboard_title: String,
        versions: Vec<DashboardHistoryVersion>,
    ) -> Self {
        Self {
            dashboard_uid,
            dashboard_title,
            versions,
            selected_index: 0,
            pending_restore: false,
        }
    }

    pub(crate) fn handle_key(&mut self, key: &KeyEvent) -> HistoryDialogAction {
        if self.pending_restore {
            return match key.code {
                KeyCode::Esc | KeyCode::Char('n') => {
                    self.pending_restore = false;
                    HistoryDialogAction::Continue
                }
                KeyCode::Char('y') => HistoryDialogAction::Restore {
                    uid: self.dashboard_uid.clone(),
                    version: self
                        .selected_version()
                        .map(|item| item.version)
                        .unwrap_or_default(),
                },
                _ => HistoryDialogAction::Continue,
            };
        }
        match key.code {
            KeyCode::Esc => HistoryDialogAction::Close,
            KeyCode::Char('q') => HistoryDialogAction::Close,
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                HistoryDialogAction::Close
            }
            KeyCode::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
                HistoryDialogAction::Continue
            }
            KeyCode::Down => {
                if self.selected_index + 1 < self.versions.len() {
                    self.selected_index += 1;
                }
                HistoryDialogAction::Continue
            }
            KeyCode::Char('r') => {
                if !self.versions.is_empty() {
                    self.pending_restore = true;
                }
                HistoryDialogAction::Continue
            }
            _ => HistoryDialogAction::Continue,
        }
    }

    pub(crate) fn render(&self, frame: &mut ratatui::Frame) {
        let area = centered_rect(74, 20, frame.area());
        frame.render_widget(Clear, area);
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(7),
                Constraint::Length(5),
                Constraint::Length(5),
            ])
            .margin(1)
            .split(area);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(14, 20, 28)))
                .border_style(Style::default().fg(Color::LightMagenta)),
            area,
        );
        let header = Paragraph::new(vec![
            Line::from(Span::styled(
                format!("History {}", self.dashboard_title),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(71, 55, 152))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Up/Down select  r restore selected  Ctrl+X/Esc close",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(71, 55, 152))
                    .add_modifier(Modifier::BOLD),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(71, 55, 152))),
        );
        frame.render_widget(header, sections[0]);

        let items = self
            .versions
            .iter()
            .map(|item| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" v{} ", item.version),
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Rgb(24, 78, 140))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(" {} ", item.created)),
                    Span::styled(
                        item.created_by.clone(),
                        Style::default().fg(Color::LightCyan),
                    ),
                    Span::raw(if item.message.is_empty() {
                        "".to_string()
                    } else {
                        format!("  {}", item.message)
                    }),
                ]))
            })
            .collect::<Vec<_>>();
        let mut list_state = ListState::default();
        list_state.select((!self.versions.is_empty()).then_some(self.selected_index));
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Versions")
                    .style(Style::default().bg(Color::Rgb(16, 20, 27))),
            )
            .highlight_symbol(">> ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(list, sections[1], &mut list_state);

        let summary = if self.pending_restore {
            vec![
                Line::from(vec![
                    Span::styled(
                        "RESTORE",
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Rgb(150, 38, 46))
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(format!(
                        " restore {} to version {} ?",
                        self.dashboard_title,
                        self.selected_version()
                            .map(|item| item.version)
                            .unwrap_or_default()
                    )),
                ]),
                Line::from("y confirm  n cancel  Esc cancel"),
            ]
        } else if let Some(item) = self.selected_version() {
            vec![
                Line::from(format!("Version: {}", item.version)),
                Line::from(format!("Created: {}", item.created)),
                Line::from(format!("Author: {}", item.created_by)),
                Line::from(format!(
                    "Message: {}",
                    if item.message.is_empty() {
                        "-".to_string()
                    } else {
                        item.message.clone()
                    }
                )),
            ]
        } else {
            vec![Line::from("No history versions available.")]
        };
        let summary = Paragraph::new(summary).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(if self.pending_restore {
                    "Confirm Restore"
                } else {
                    "Selected Version"
                }),
        );
        frame.render_widget(summary, sections[2]);

        let footer = Paragraph::new(if self.pending_restore {
            vec![Line::from(vec![
                hotkey("y", Color::Rgb(24, 106, 59)),
                plain(" confirm restore"),
                plain("   "),
                hotkey("n", Color::Rgb(90, 98, 107)),
                plain(" cancel restore"),
                plain("   "),
                hotkey("Esc", Color::Rgb(90, 98, 107)),
                plain(" close"),
                plain("   "),
                hotkey("q", Color::Rgb(90, 98, 107)),
                plain(" close"),
            ])]
        } else {
            vec![
                Line::from(vec![
                    hotkey("Up/Down", Color::Rgb(24, 78, 140)),
                    plain(" select version"),
                    plain("   "),
                    hotkey("r", Color::Rgb(150, 38, 46)),
                    plain(" restore selected"),
                    plain("   "),
                    hotkey("Esc", Color::Rgb(90, 98, 107)),
                    plain(" close"),
                    plain("   "),
                    hotkey("q", Color::Rgb(90, 98, 107)),
                    plain(" close"),
                ]),
                Line::from(vec![
                    hotkey("Ctrl+X", Color::Rgb(90, 98, 107)),
                    plain(" close history"),
                ]),
            ]
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Hotkeys")
                .style(Style::default().bg(Color::Rgb(22, 18, 30)))
                .border_style(Style::default().fg(Color::LightMagenta)),
        )
        .style(Style::default().bg(Color::Rgb(22, 18, 30)));
        frame.render_widget(footer, sections[3]);
    }

    fn selected_version(&self) -> Option<&DashboardHistoryVersion> {
        self.versions.get(self.selected_index)
    }
}

fn centered_rect(width_percent: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(height),
            Constraint::Min(1),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100u16.saturating_sub(width_percent)) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100u16.saturating_sub(width_percent)) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}

fn hotkey(label: &'static str, bg: Color) -> Span<'static> {
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
