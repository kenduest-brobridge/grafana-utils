//! Interactive browse workflows and terminal-driven state flow for Access entities.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use serde_json::{Map, Value};

use super::team_browse_state::{SearchDirection, SearchPromptState};
use crate::access::render::map_get_text;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EditDialogState {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) add_member: String,
    pub(super) remove_member: String,
    pub(super) add_admin: String,
    pub(super) remove_admin: String,
    pub(super) active_field: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EditDialogAction {
    None,
    Save,
    Cancel,
}

impl EditDialogState {
    pub(super) fn new(row: &Map<String, Value>) -> Self {
        Self {
            id: map_get_text(row, "id"),
            name: map_get_text(row, "name"),
            add_member: String::new(),
            remove_member: String::new(),
            add_admin: String::new(),
            remove_admin: String::new(),
            active_field: 0,
        }
    }

    pub(super) fn handle_key(&mut self, key: &KeyEvent) -> EditDialogAction {
        match key.code {
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
            KeyCode::Backspace => {
                self.active_value_mut().pop();
                EditDialogAction::None
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.active_value_mut().push(ch);
                EditDialogAction::None
            }
            _ => EditDialogAction::None,
        }
    }

    fn active_value_mut(&mut self) -> &mut String {
        match self.active_field {
            0 => &mut self.add_member,
            1 => &mut self.remove_member,
            2 => &mut self.add_admin,
            _ => &mut self.remove_admin,
        }
    }

    pub(super) fn render(&self, frame: &mut ratatui::Frame) {
        let area = centered_rect(frame.area(), 74, 19);
        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(18, 24, 33)))
                .border_style(Style::default().fg(Color::LightCyan)),
            area,
        );
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .margin(1)
            .split(area);
        let header = Paragraph::new(vec![
            Line::from(Span::styled(
                format!("Edit Team {} ({})", self.name, self.id),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(24, 78, 140))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Ctrl+S save  Ctrl+X close  Esc cancel  CSV values",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(24, 78, 140)),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(24, 78, 140))),
        );
        frame.render_widget(header, rows[0]);
        render_field(
            frame,
            rows[1],
            "Add Member CSV",
            &self.add_member,
            self.active_field == 0,
        );
        render_field(
            frame,
            rows[2],
            "Remove Member CSV",
            &self.remove_member,
            self.active_field == 1,
        );
        render_field(
            frame,
            rows[3],
            "Add Admin CSV",
            &self.add_admin,
            self.active_field == 2,
        );
        render_field(
            frame,
            rows[4],
            "Remove Admin CSV",
            &self.remove_admin,
            self.active_field == 3,
        );
        frame.set_cursor_position(edit_cursor(self, rows[1], rows[2], rows[3], rows[4]));
        frame.render_widget(
            Paragraph::new(
                "Values accept user id, exact login, or exact email. Separate entries with commas."
                    .to_string(),
            )
            .block(Block::default().borders(Borders::TOP).title("Hints")),
            rows[5],
        );
    }
}

pub(super) fn delete_lines(row: Option<&Map<String, Value>>) -> Vec<Line<'static>> {
    let Some(row) = row else {
        return vec![Line::from("No team selected.")];
    };
    vec![
        Line::from(format!(
            "Delete team {}",
            blank_dash(&map_get_text(row, "name"))
        )),
        Line::from(format!("ID: {}", blank_dash(&map_get_text(row, "id")))),
        Line::from(format!(
            "Email: {}",
            blank_dash(&map_get_text(row, "email"))
        )),
        Line::from(""),
        Line::from("Press y to confirm delete."),
        Line::from("Press n, Esc, or q to cancel."),
    ]
}

pub(super) fn render_search_prompt(frame: &mut ratatui::Frame, search: &SearchPromptState) {
    let area = centered_rect(frame.area(), 60, 5);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(search.query.clone()).block(Block::default().borders(Borders::ALL).title(
            match search.direction {
                SearchDirection::Forward => "Search /",
                SearchDirection::Backward => "Search ?",
            },
        )),
        area,
    );
    let max_offset = area.width.saturating_sub(3) as usize;
    let offset = search.query.chars().count().min(max_offset) as u16;
    frame.set_cursor_position(Position::new(area.x + 1 + offset, area.y + 1));
}

fn render_field(frame: &mut ratatui::Frame, area: Rect, label: &str, value: &str, active: bool) {
    frame.render_widget(
        Paragraph::new(value.to_string())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(if active {
                        format!("{label}  [Ctrl+S Save]")
                    } else {
                        label.to_string()
                    })
                    .border_style(if active {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .style(if active {
                Style::default().fg(Color::White).bg(Color::Rgb(24, 38, 60))
            } else {
                Style::default().fg(Color::White).bg(Color::Rgb(20, 24, 30))
            }),
        area,
    );
}

fn edit_cursor(edit: &EditDialogState, a: Rect, b: Rect, c: Rect, d: Rect) -> Position {
    let (area, value) = match edit.active_field {
        0 => (a, &edit.add_member),
        1 => (b, &edit.remove_member),
        2 => (c, &edit.add_admin),
        _ => (d, &edit.remove_admin),
    };
    Position::new(
        area.x.saturating_add(value.chars().count() as u16 + 1),
        area.y + 1,
    )
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

fn blank_dash(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "-"
    } else {
        trimmed
    }
}
