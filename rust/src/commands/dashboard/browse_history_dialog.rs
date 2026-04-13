#![cfg(feature = "tui")]
use crate::tui_shell;
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
    editing_message: bool,
    restore_message: String,
    busy_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HistoryDialogAction {
    Continue,
    Close,
    Restore {
        uid: String,
        version: i64,
        message: String,
    },
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
            editing_message: false,
            restore_message: String::new(),
            busy_message: None,
        }
    }

    pub(crate) fn handle_key(&mut self, key: &KeyEvent) -> HistoryDialogAction {
        if self.busy_message.is_some() {
            return HistoryDialogAction::Continue;
        }
        if self.pending_restore {
            return match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    if self.editing_message {
                        self.editing_message = false;
                    } else {
                        self.pending_restore = false;
                    }
                    HistoryDialogAction::Continue
                }
                KeyCode::Char('n') => {
                    self.pending_restore = false;
                    self.editing_message = false;
                    HistoryDialogAction::Continue
                }
                KeyCode::Enter => HistoryDialogAction::Restore {
                    uid: self.dashboard_uid.clone(),
                    version: self
                        .selected_version()
                        .map(|item| item.version)
                        .unwrap_or_default(),
                    message: self.restore_message.clone(),
                },
                KeyCode::Char('e') => {
                    self.editing_message = true;
                    HistoryDialogAction::Continue
                }
                KeyCode::Backspace if self.editing_message => {
                    self.restore_message.pop();
                    HistoryDialogAction::Continue
                }
                KeyCode::Char(ch)
                    if self.editing_message && !key.modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    self.restore_message.push(ch);
                    HistoryDialogAction::Continue
                }
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
                    self.editing_message = false;
                    self.restore_message = self.default_restore_message();
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
                if self.pending_restore {
                    "Review restore before apply. Enter confirms. e edits the message."
                } else {
                    "Up/Down select a version. r opens a restore review."
                },
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
            let selected_version = self
                .selected_version()
                .map(|item| item.version)
                .unwrap_or_default();
            let selected_message = self
                .selected_version()
                .map(|item| item.message.clone())
                .filter(|message| !message.is_empty())
                .unwrap_or_else(|| "-".to_string());
            vec![
                Line::from(
                    self.busy_message
                        .clone()
                        .unwrap_or_else(|| "Ready to restore the selected version.".to_string()),
                ),
                Line::from("".to_string()),
                Line::from(format!("Dashboard: {}", self.dashboard_title)),
                Line::from(format!("UID: {}", self.dashboard_uid)),
                Line::from(format!("Restore to version: {}", selected_version)),
                Line::from(
                    "Result: Grafana will create a new latest revision. Existing history stays."
                        .to_string(),
                ),
                Line::from(format!("Selected version message: {}", selected_message)),
                Line::from("".to_string()),
                Line::from(Span::styled(
                    "Revision Message",
                    Style::default()
                        .fg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(if self.restore_message.trim().is_empty() {
                    if self.editing_message {
                        "> ".to_string()
                    } else {
                        "(empty)".to_string()
                    }
                } else if self.editing_message {
                    format!("> {}", self.restore_message)
                } else {
                    self.restore_message.clone()
                }),
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

        let footer = tui_shell::build_footer_controls(if self.pending_restore {
            if self.busy_message.is_some() {
                tui_shell::control_grid(&[
                    vec![("Working", Color::Rgb(24, 78, 140), "restoring revision")],
                    vec![("Esc/q", Color::Rgb(90, 98, 107), "wait for completion")],
                ])
            } else {
                tui_shell::control_grid(&[
                    vec![
                        ("Enter", Color::Rgb(24, 106, 59), "confirm restore"),
                        ("e", Color::Rgb(164, 116, 19), "edit message"),
                        ("n", Color::Rgb(90, 98, 107), "cancel review"),
                    ],
                    vec![
                        ("Esc", Color::Rgb(90, 98, 107), "back"),
                        ("q", Color::Rgb(90, 98, 107), "back"),
                    ],
                ])
            }
        } else {
            tui_shell::control_grid(&[
                vec![
                    ("Up/Down", Color::Rgb(24, 78, 140), "select version"),
                    ("r", Color::Rgb(150, 38, 46), "open restore review"),
                ],
                vec![
                    ("Esc", Color::Rgb(90, 98, 107), "close"),
                    ("q", Color::Rgb(90, 98, 107), "close"),
                    ("Ctrl+X", Color::Rgb(90, 98, 107), "close"),
                ],
            ])
        });
        frame.render_widget(footer, sections[3]);
    }

    fn selected_version(&self) -> Option<&DashboardHistoryVersion> {
        self.versions.get(self.selected_index)
    }

    fn default_restore_message(&self) -> String {
        let version = self
            .selected_version()
            .map(|item| item.version)
            .unwrap_or_default();
        let base = format!("Restore {} to version {}", self.dashboard_title, version);
        let Some(item) = self.selected_version() else {
            return base;
        };
        if item.message.trim().is_empty() {
            base
        } else {
            format!("{base} ({})", item.message.trim())
        }
    }

    pub(crate) fn set_busy_message(&mut self, value: impl Into<String>) {
        self.busy_message = Some(value.into());
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
