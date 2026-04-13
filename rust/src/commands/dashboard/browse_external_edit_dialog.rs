#![cfg(feature = "tui")]
use crate::tui_shell;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct ExternalEditDialogState {
    pub(crate) uid: String,
    pub(crate) title: String,
    pub(crate) updated_payload: Value,
    pub(crate) summary_lines: Vec<String>,
    pub(crate) preview_lines: Option<Vec<String>>,
    pub(crate) save_path: PathBuf,
    pub(crate) save_path_input: String,
    pub(crate) editing_save_path: bool,
    pub(crate) busy_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExternalEditDialogAction {
    Continue,
    Close,
    PreviewDryRun,
    SaveDraft(PathBuf),
    ApplyLive,
}

#[derive(Clone, Debug)]
pub(crate) struct ExternalEditErrorState {
    pub(crate) uid: String,
    pub(crate) title: String,
    pub(crate) error_message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExternalEditErrorAction {
    Continue,
    Retry,
    Close,
}

impl ExternalEditDialogState {
    pub(crate) fn new(
        uid: String,
        title: String,
        updated_payload: Value,
        summary_lines: Vec<String>,
    ) -> Self {
        let save_path = PathBuf::from(format!("{uid}.edited.json"));
        Self {
            uid,
            title,
            updated_payload,
            summary_lines,
            preview_lines: None,
            save_path_input: save_path.display().to_string(),
            editing_save_path: false,
            save_path,
            busy_message: None,
        }
    }

    pub(crate) fn handle_key(&mut self, key: &KeyEvent) -> ExternalEditDialogAction {
        if self.busy_message.is_some() {
            return ExternalEditDialogAction::Continue;
        }
        if self.editing_save_path {
            return match key.code {
                KeyCode::Esc => {
                    self.editing_save_path = false;
                    self.save_path_input = self.save_path.display().to_string();
                    ExternalEditDialogAction::Continue
                }
                KeyCode::Enter => {
                    let input = self.save_path_input.trim();
                    let path = if input.is_empty() {
                        self.save_path.clone()
                    } else {
                        PathBuf::from(input)
                    };
                    self.save_path = path.clone();
                    self.save_path_input = self.save_path.display().to_string();
                    self.editing_save_path = false;
                    ExternalEditDialogAction::SaveDraft(path)
                }
                KeyCode::Backspace => {
                    self.save_path_input.pop();
                    ExternalEditDialogAction::Continue
                }
                KeyCode::Char(character)
                    if !key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                        && !key.modifiers.contains(crossterm::event::KeyModifiers::ALT) =>
                {
                    self.save_path_input.push(character);
                    ExternalEditDialogAction::Continue
                }
                _ => ExternalEditDialogAction::Continue,
            };
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => ExternalEditDialogAction::Close,
            KeyCode::Char('p') => ExternalEditDialogAction::PreviewDryRun,
            KeyCode::Char('w') => {
                self.editing_save_path = true;
                self.save_path_input = self.save_path.display().to_string();
                ExternalEditDialogAction::Continue
            }
            KeyCode::Char('a') | KeyCode::Enter => ExternalEditDialogAction::ApplyLive,
            _ => ExternalEditDialogAction::Continue,
        }
    }

    pub(crate) fn render(&self, frame: &mut ratatui::Frame) {
        let backdrop = frame.area();
        let area = tui_shell::centered_rect(frame.area(), 80, 70);
        frame.render_widget(Clear, backdrop);
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Rgb(10, 12, 16))),
            backdrop,
        );
        frame.render_widget(Clear, area);
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Min(8),
            ])
            .split(area);
        let header = Paragraph::new(vec![
            Line::from(Span::styled(
                format!("Raw JSON Edit {}", self.uid),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(73, 64, 12))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                self.title.clone(),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(73, 64, 12))
                    .add_modifier(Modifier::BOLD),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(73, 64, 12)))
                .border_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Rgb(73, 64, 12))
                        .add_modifier(Modifier::BOLD),
                ),
        );
        frame.render_widget(header, rows[0]);

        if self.busy_message.is_some() {
            let controls = Paragraph::new(tui_shell::control_grid(&[
                vec![(
                    "Working",
                    Color::Rgb(24, 78, 140),
                    "running selected action",
                )],
                vec![("Esc/q", Color::Rgb(90, 98, 107), "wait for completion")],
            ]))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Actions")
                    .style(Style::default().bg(Color::Rgb(24, 28, 34)))
                    .border_style(Style::default().fg(Color::LightYellow)),
            );
            frame.render_widget(controls, rows[1]);
        } else if self.editing_save_path {
            render_write_draft_panel(frame, rows[1], &self.save_path_input);
        } else {
            let mut control_lines = tui_shell::control_grid(&[
                vec![
                    ("a", Color::Rgb(24, 106, 59), "apply live"),
                    ("w", Color::Rgb(164, 116, 19), "write draft"),
                    ("q", Color::Rgb(150, 38, 46), "discard"),
                ],
                vec![
                    ("Enter", Color::Rgb(24, 106, 59), "apply live"),
                    ("p", Color::Rgb(24, 78, 140), "refresh preview"),
                ],
            ]);
            control_lines.push(Line::from(""));
            control_lines.push(Line::from(vec![
                Span::styled("Draft path ", Style::default().fg(Color::Gray)),
                Span::styled(
                    self.save_path.display().to_string(),
                    Style::default().fg(Color::White),
                ),
            ]));
            let controls = Paragraph::new(control_lines)
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Actions")
                        .style(Style::default().bg(Color::Rgb(24, 28, 34)))
                        .border_style(Style::default().fg(Color::LightYellow)),
                );
            frame.render_widget(controls, rows[1]);
        }

        let mut body_lines = vec![Line::from(Span::styled(
            "Edit Summary",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ))];
        if let Some(message) = &self.busy_message {
            body_lines.push(Line::from(""));
            body_lines.push(Line::from(Span::styled(
                message.clone(),
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            )));
            body_lines.push(Line::from(""));
        }
        for line in &self.summary_lines {
            body_lines.push(Line::from(line.clone()));
        }
        if let Some(preview_lines) = &self.preview_lines {
            body_lines.push(Line::from(""));
            body_lines.push(Line::from(Span::styled(
                "Live Preview",
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            )));
            for line in preview_lines {
                body_lines.push(Line::from(line.clone()));
            }
        }
        let body = Paragraph::new(body_lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Review")
                .style(Style::default().bg(Color::Rgb(20, 24, 31)))
                .border_style(Style::default().fg(Color::LightYellow)),
        );
        frame.render_widget(body, rows[2]);
    }

    pub(crate) fn set_busy_message(&mut self, value: impl Into<String>) {
        self.busy_message = Some(value.into());
    }

    pub(crate) fn clear_busy_message(&mut self) {
        self.busy_message = None;
    }
}

impl ExternalEditErrorState {
    pub(crate) fn new(uid: String, title: String, error_message: String) -> Self {
        Self {
            uid,
            title,
            error_message,
        }
    }

    pub(crate) fn handle_key(&self, key: &KeyEvent) -> ExternalEditErrorAction {
        match key.code {
            KeyCode::Char('r') | KeyCode::Enter => ExternalEditErrorAction::Retry,
            KeyCode::Esc | KeyCode::Char('q') => ExternalEditErrorAction::Close,
            _ => ExternalEditErrorAction::Continue,
        }
    }

    pub(crate) fn render(&self, frame: &mut ratatui::Frame) {
        let backdrop = frame.area();
        let area = tui_shell::centered_rect(frame.area(), 76, 46);
        frame.render_widget(Clear, backdrop);
        frame.render_widget(
            Block::default().style(Style::default().bg(Color::Rgb(10, 10, 14))),
            backdrop,
        );
        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(24, 18, 22)))
                .border_style(
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                ),
            area,
        );
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Min(7),
                Constraint::Length(3),
            ])
            .margin(1)
            .split(area);
        let header = Paragraph::new(vec![
            Line::from(Span::styled(
                "Raw JSON Edit Error",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(116, 32, 48))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "The edited file could not be loaded. Fix it and retry, or abort this raw edit.",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(116, 32, 48))
                    .add_modifier(Modifier::BOLD),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(116, 32, 48)))
                .border_style(
                    Style::default()
                        .fg(Color::LightRed)
                        .bg(Color::Rgb(116, 32, 48))
                        .add_modifier(Modifier::BOLD),
                ),
        );
        frame.render_widget(header, rows[0]);

        let facts = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    "Dashboard ",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(self.title.clone(), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(
                    "UID ",
                    Style::default()
                        .fg(Color::LightRed)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(self.uid.clone(), Style::default().fg(Color::White)),
            ]),
        ])
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Context")
                .style(Style::default().bg(Color::Rgb(28, 22, 27)))
                .border_style(Style::default().fg(Color::LightRed)),
        );
        frame.render_widget(facts, rows[1]);

        let details = Paragraph::new(vec![
            Line::from(Span::styled(
                "Parser Message",
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(self.error_message.clone()),
        ])
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Details")
                .style(Style::default().bg(Color::Rgb(20, 16, 21)))
                .border_style(Style::default().fg(Color::LightRed)),
        );
        frame.render_widget(details, rows[2]);

        let footer = Paragraph::new(tui_shell::control_grid(&[
            vec![
                ("r", Color::Rgb(24, 106, 59), "reopen editor"),
                ("Enter", Color::Rgb(24, 106, 59), "reopen editor"),
            ],
            vec![
                ("Esc", Color::Rgb(90, 98, 107), "abort raw edit"),
                ("q", Color::Rgb(90, 98, 107), "abort raw edit"),
            ],
        ]))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Actions")
                .style(Style::default().bg(Color::Rgb(28, 22, 27)))
                .border_style(Style::default().fg(Color::LightRed)),
        );
        frame.render_widget(footer, rows[3]);
    }
}

fn render_write_draft_panel(frame: &mut ratatui::Frame, area: ratatui::layout::Rect, input: &str) {
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title("Write Draft")
            .style(Style::default().bg(Color::Rgb(24, 28, 34)))
            .border_style(
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
        area,
    );
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .margin(1)
        .split(area);
    let helper = Paragraph::new(vec![
        Line::from(Span::styled(
            "Choose where to write the draft file.",
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("Enter writes the draft. Esc returns to review."),
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(helper, rows[0]);
    let input_box = Paragraph::new(input.to_string()).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Filename")
            .style(Style::default().bg(Color::Rgb(15, 18, 24)))
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(input_box, rows[1]);
    frame.set_cursor_position(Position::new(
        rows[1].x.saturating_add(1 + input.chars().count() as u16),
        rows[1].y.saturating_add(1),
    ));
    let hotkeys = Paragraph::new(tui_shell::control_grid(&[
        vec![
            ("Enter", Color::Rgb(24, 106, 59), "write draft"),
            ("Esc", Color::Rgb(90, 98, 107), "back to review"),
        ],
        vec![
            ("Backspace", Color::Rgb(90, 98, 107), "delete char"),
            ("Type", Color::Rgb(24, 78, 140), "edit filename"),
        ],
    ]))
    .wrap(Wrap { trim: false });
    frame.render_widget(hotkeys, rows[2]);
}
