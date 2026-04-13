#![cfg(feature = "tui")]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::common::Result;

use super::datasource_browse_support::DatasourceBrowseItem;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EditDialogState {
    pub(crate) uid: String,
    pub(crate) original_name: String,
    pub(crate) original_url: String,
    pub(crate) original_access: String,
    pub(crate) original_is_default: bool,
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) access: String,
    pub(crate) is_default: bool,
    pub(crate) active_field: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditDialogAction {
    None,
    Save,
    Cancel,
}

impl EditDialogState {
    pub(crate) fn new(item: &DatasourceBrowseItem) -> Self {
        Self {
            uid: item.uid.clone(),
            original_name: item.name.clone(),
            original_url: item.url.clone(),
            original_access: item.access.clone(),
            original_is_default: item.is_default,
            name: item.name.clone(),
            url: item.url.clone(),
            access: item.access.clone(),
            is_default: item.is_default,
            active_field: 0,
        }
    }

    pub(crate) fn handle_key(&mut self, key: &KeyEvent) -> Result<EditDialogAction> {
        Ok(match key.code {
            KeyCode::Esc => EditDialogAction::Cancel,
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                EditDialogAction::Cancel
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                EditDialogAction::Save
            }
            KeyCode::BackTab => {
                self.active_field = self.active_field.saturating_sub(1);
                EditDialogAction::None
            }
            KeyCode::Tab => {
                self.active_field = (self.active_field + 1).min(3);
                EditDialogAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ') if self.active_field == 3 => {
                self.is_default = !self.is_default;
                EditDialogAction::None
            }
            KeyCode::Backspace => {
                match self.active_field {
                    0 => {
                        self.name.pop();
                    }
                    1 => {
                        self.url.pop();
                    }
                    2 => {
                        self.access.pop();
                    }
                    _ => {}
                }
                EditDialogAction::None
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                match self.active_field {
                    0 => self.name.push(ch),
                    1 => self.url.push(ch),
                    2 => self.access.push(ch),
                    3 => {
                        let lowered = ch.to_ascii_lowercase();
                        if lowered == 't' || lowered == 'y' || lowered == '1' {
                            self.is_default = true;
                        } else if lowered == 'f' || lowered == 'n' || lowered == '0' {
                            self.is_default = false;
                        }
                    }
                    _ => {}
                }
                EditDialogAction::None
            }
            _ => EditDialogAction::None,
        })
    }

    pub(crate) fn render(&self, frame: &mut ratatui::Frame) {
        let area = centered_rect(frame.area(), 72, 18);
        frame.render_widget(Clear, area);

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(area);

        let container = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Rgb(15, 20, 28)))
            .border_style(Style::default().fg(Color::LightBlue));
        frame.render_widget(container, area);

        let header = Paragraph::new(vec![
            Line::from(Span::styled(
                format!(" Edit Datasource {} ", self.uid),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Ctrl+S Save   Esc Cancel   Ctrl+X Close   Tab Next",
                Style::default().fg(Color::White),
            )),
        ])
        .alignment(Alignment::Left)
        .block(Block::default().style(Style::default().bg(Color::Rgb(15, 20, 28))));
        frame.render_widget(header, sections[0]);

        render_field(
            frame,
            sections[1],
            "Name",
            &self.name,
            self.active_field == 0,
        );
        render_field(frame, sections[2], "URL", &self.url, self.active_field == 1);
        render_field(
            frame,
            sections[3],
            "Access",
            &self.access,
            self.active_field == 2,
        );
        render_field(
            frame,
            sections[4],
            "Default",
            if self.is_default { "true" } else { "false" },
            self.active_field == 3,
        );

        let preview = Paragraph::new(vec![
            Line::from(format!("Original name: {}", self.original_name)),
            Line::from(format!("Original url: {}", self.original_url)),
            Line::from(format!("Original access: {}", self.original_access)),
            Line::from(format!(
                "Original default: {}",
                if self.original_is_default {
                    "true"
                } else {
                    "false"
                }
            )),
        ])
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::TOP)
                .title("Preview")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        frame.render_widget(preview, sections[5]);
    }
}

fn render_field(frame: &mut ratatui::Frame, area: Rect, label: &str, value: &str, active: bool) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        })
        .title(if active {
            format!("{label}  [Ctrl+S Save]")
        } else {
            label.to_string()
        });
    let paragraph = Paragraph::new(value.to_string())
        .block(block)
        .style(if active {
            Style::default().fg(Color::White).bg(Color::Rgb(24, 38, 60))
        } else {
            Style::default().fg(Color::White).bg(Color::Rgb(20, 24, 30))
        });
    frame.render_widget(paragraph, area);
}

fn centered_rect(area: Rect, width_percent: u16, height: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(height.min(area.height.saturating_sub(2))),
            Constraint::Min(1),
        ])
        .split(area);
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - width_percent) / 2),
            Constraint::Percentage(width_percent),
            Constraint::Percentage((100 - width_percent) / 2),
        ])
        .split(vertical[1]);
    horizontal[1]
}
