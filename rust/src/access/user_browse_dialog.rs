//! Interactive browse workflows and terminal-driven state flow for Access entities.

use crate::tui_shell;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use serde_json::{Map, Value};

use crate::access::render::map_get_text;

use super::user_browse_state::{SearchDirection, SearchPromptState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct EditDialogState {
    pub(super) id: String,
    pub(super) login: String,
    pub(super) email: String,
    pub(super) name: String,
    pub(super) org_role: String,
    pub(super) grafana_admin: String,
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
            login: map_get_text(row, "login"),
            email: map_get_text(row, "email"),
            name: map_get_text(row, "name"),
            org_role: map_get_text(row, "orgRole"),
            grafana_admin: map_get_text(row, "grafanaAdmin"),
            active_field: 0,
        }
    }

    pub(super) fn active_value_mut(&mut self) -> &mut String {
        match self.active_field {
            0 => &mut self.login,
            1 => &mut self.email,
            2 => &mut self.name,
            3 => &mut self.org_role,
            _ => &mut self.grafana_admin,
        }
    }

    pub(super) fn render(&self, frame: &mut ratatui::Frame) {
        let area = tui_shell::render_dialog_shell(
            frame,
            format!("Edit User {}", self.id),
            72,
            21,
            Color::LightCyan,
        );
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
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
                "Edit selected user".to_string(),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(24, 78, 140))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Ctrl+S save  Ctrl+X close  Esc cancel  Tab next",
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
        render_field(frame, rows[1], "Login", &self.login, self.active_field == 0);
        render_field(frame, rows[2], "Email", &self.email, self.active_field == 1);
        render_field(frame, rows[3], "Name", &self.name, self.active_field == 2);
        render_field(
            frame,
            rows[4],
            "Org Role",
            &self.org_role,
            self.active_field == 3,
        );
        render_field(
            frame,
            rows[5],
            "Grafana Admin",
            &self.grafana_admin,
            self.active_field == 4,
        );
        frame.set_cursor_position(edit_cursor(
            self, rows[1], rows[2], rows[3], rows[4], rows[5],
        ));
        let footer = Paragraph::new(
            "Hint: Grafana Admin accepts true/false. Empty role keeps the current role."
                .to_string(),
        )
        .block(Block::default().borders(Borders::TOP).title("Hints"));
        frame.render_widget(footer, rows[6]);
    }
}

pub(super) fn delete_lines(row: Option<&Map<String, Value>>) -> Vec<Line<'static>> {
    let Some(row) = row else {
        return vec![Line::from("No user selected.")];
    };
    vec![
        Line::from(format!(
            "Delete user {}",
            blank_dash(&map_get_text(row, "login"))
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
    let title = match search.direction {
        SearchDirection::Forward => "Search /",
        SearchDirection::Backward => "Search ?",
    };
    let area = tui_shell::render_dialog_shell(frame, title, 60, 5, Color::Yellow);
    frame.render_widget(
        Paragraph::new(search.query.clone())
            .style(Style::default().fg(Color::White).bg(Color::Rgb(16, 22, 30))),
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

fn edit_cursor(edit: &EditDialogState, a: Rect, b: Rect, c: Rect, d: Rect, e: Rect) -> Position {
    let (area, value) = match edit.active_field {
        0 => (a, edit.login.as_str()),
        1 => (b, edit.email.as_str()),
        2 => (c, edit.name.as_str()),
        3 => (d, edit.org_role.as_str()),
        _ => (e, edit.grafana_admin.as_str()),
    };
    Position::new(
        area.x
            + 1
            + value
                .chars()
                .count()
                .min(area.width.saturating_sub(3) as usize) as u16,
        area.y + 1,
    )
}

fn blank_dash(value: &str) -> &str {
    if value.is_empty() {
        "-"
    } else {
        value
    }
}
