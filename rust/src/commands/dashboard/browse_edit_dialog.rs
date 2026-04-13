#![cfg(feature = "tui")]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use super::browse_support::{DashboardBrowseDocument, DashboardBrowseNodeKind};
use super::delete_support::normalize_folder_path;
use super::edit::{DashboardEditDraft, DashboardEditUpdate};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditField {
    Title,
    Folder,
    Tags,
}

impl EditField {
    fn next(self) -> Self {
        match self {
            EditField::Title => EditField::Folder,
            EditField::Folder => EditField::Tags,
            EditField::Tags => EditField::Title,
        }
    }

    fn previous(self) -> Self {
        match self {
            EditField::Title => EditField::Tags,
            EditField::Folder => EditField::Title,
            EditField::Tags => EditField::Folder,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EditDialogState {
    draft: DashboardEditDraft,
    title: String,
    folder: String,
    tags: String,
    active_field: EditField,
    folder_options: Vec<String>,
    folder_picker_open: bool,
    folder_picker_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EditDialogAction {
    Continue,
    Cancelled,
    Save {
        draft: DashboardEditDraft,
        update: DashboardEditUpdate,
    },
}

impl EditDialogState {
    pub(crate) fn from_draft(
        draft: DashboardEditDraft,
        document: &DashboardBrowseDocument,
    ) -> Self {
        let current_folder = normalize_folder_path(&draft.folder_path);
        let mut folder_options = document
            .nodes
            .iter()
            .filter(|node| node.kind == DashboardBrowseNodeKind::Folder)
            .map(|node| normalize_folder_path(&node.path))
            .filter(|path| !path.is_empty())
            .collect::<Vec<_>>();
        folder_options.sort();
        folder_options.dedup();
        let folder_picker_index = folder_options
            .iter()
            .position(|path| *path == current_folder)
            .unwrap_or(0);
        Self {
            title: draft.title.clone(),
            folder: draft.folder_path.clone(),
            tags: draft.tags.join(", "),
            draft,
            active_field: EditField::Title,
            folder_options,
            folder_picker_open: false,
            folder_picker_index,
        }
    }

    pub(crate) fn handle_key(&mut self, key: &KeyEvent) -> EditDialogAction {
        if self.folder_picker_open {
            return self.handle_folder_picker_key(key);
        }
        match key.code {
            KeyCode::Esc => EditDialogAction::Cancelled,
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                EditDialogAction::Cancelled
            }
            KeyCode::Enter if self.active_field == EditField::Folder => {
                if !self.folder_options.is_empty() {
                    self.folder_picker_open = true;
                }
                EditDialogAction::Continue
            }
            KeyCode::Tab | KeyCode::Down => {
                self.active_field = self.active_field.next();
                EditDialogAction::Continue
            }
            KeyCode::BackTab | KeyCode::Up => {
                self.active_field = self.active_field.previous();
                EditDialogAction::Continue
            }
            KeyCode::Backspace => {
                self.active_value_mut().pop();
                EditDialogAction::Continue
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                EditDialogAction::Save {
                    draft: self.draft.clone(),
                    update: self.build_update(),
                }
            }
            KeyCode::Char(character)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.active_value_mut().push(character);
                EditDialogAction::Continue
            }
            _ => EditDialogAction::Continue,
        }
    }

    pub(crate) fn focus_title_rename(&mut self) {
        self.active_field = EditField::Title;
        self.folder_picker_open = false;
    }

    pub(crate) fn focus_folder_move(&mut self) {
        self.active_field = EditField::Folder;
        if !self.folder_options.is_empty() {
            self.folder_picker_open = true;
        }
    }

    pub(crate) fn render(&self, frame: &mut ratatui::Frame) {
        let area = centered_rect(72, 22, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(18, 24, 33)))
                .border_style(
                    Style::default()
                        .fg(Color::LightCyan)
                        .bg(Color::Rgb(18, 24, 33))
                        .add_modifier(Modifier::BOLD),
                ),
            area,
        );
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Length(4),
                Constraint::Length(2),
            ])
            .margin(1)
            .split(area);
        let header = Paragraph::new(vec![
            Line::from(Span::styled(
                format!("Edit Dashboard {}", self.draft.uid),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(24, 78, 140))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Ctrl+S save  Ctrl+X close  Tab next",
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(24, 78, 140))
                    .add_modifier(Modifier::BOLD),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(24, 78, 140)))
                .border_style(
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(24, 78, 140))
                        .add_modifier(Modifier::BOLD),
                ),
        );
        frame.render_widget(header, rows[0]);
        render_edit_field(
            frame,
            rows[1],
            if self.active_field == EditField::Title {
                "Title  [Ctrl+S Save]"
            } else {
                "Title"
            },
            &self.title,
            self.active_field == EditField::Title,
        );
        render_edit_field(
            frame,
            rows[2],
            if self.active_field == EditField::Folder {
                "Folder Path  [Enter Select | Ctrl+S Save]"
            } else {
                "Folder Path"
            },
            &self.folder,
            self.active_field == EditField::Folder,
        );
        render_edit_field(
            frame,
            rows[3],
            if self.active_field == EditField::Tags {
                "Tags Csv  [Ctrl+S Save]"
            } else {
                "Tags Csv"
            },
            &self.tags,
            self.active_field == EditField::Tags,
        );
        frame.set_cursor_position(edit_cursor_position(self, rows[1], rows[2], rows[3]));
        let help = Paragraph::new(
            "Ctrl+S save  Ctrl+X close  Esc cancel  Tab next field  Shift+Tab previous".to_string(),
        )
        .style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Hotkeys")
                .style(Style::default().bg(Color::Rgb(43, 38, 18)))
                .border_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Rgb(43, 38, 18))
                        .add_modifier(Modifier::BOLD),
                )
                .title_style(
                    Style::default()
                        .fg(Color::LightYellow)
                        .bg(Color::Rgb(43, 38, 18))
                        .add_modifier(Modifier::BOLD),
                ),
        );
        frame.render_widget(help, rows[4]);
        let preview = Paragraph::new(vec![
            Line::from(format!("Current title: {}", self.draft.title)),
            Line::from(format!("Current folder: {}", self.draft.folder_path)),
            Line::from(format!(
                "Current tags: {}",
                if self.draft.tags.is_empty() {
                    "-".to_string()
                } else {
                    self.draft.tags.join(", ")
                }
            )),
            Line::from("Saving applies only changed values."),
        ])
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White).bg(Color::Rgb(24, 31, 41)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Preview")
                .style(Style::default().bg(Color::Rgb(24, 31, 41)))
                .border_style(
                    Style::default()
                        .fg(Color::LightBlue)
                        .bg(Color::Rgb(24, 31, 41)),
                )
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(Color::Rgb(24, 31, 41))
                        .add_modifier(Modifier::BOLD),
                ),
        );
        frame.render_widget(preview, rows[5]);
        let footer = Paragraph::new(
            "Ctrl+S Save   Ctrl+X Close   Esc Cancel   Tab Next   Shift+Tab Previous",
        )
        .style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(24, 78, 140))
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(24, 78, 140)))
                .border_style(
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::Rgb(24, 78, 140))
                        .add_modifier(Modifier::BOLD),
                ),
        );
        frame.render_widget(footer, rows[6]);
        if self.folder_picker_open {
            self.render_folder_picker(frame, area);
        }
    }

    fn active_value_mut(&mut self) -> &mut String {
        match self.active_field {
            EditField::Title => &mut self.title,
            EditField::Folder => &mut self.folder,
            EditField::Tags => &mut self.tags,
        }
    }

    fn build_update(&self) -> DashboardEditUpdate {
        let mut update = DashboardEditUpdate::default();
        let title = self.title.trim();
        if !title.is_empty() && title != self.draft.title {
            update.title = Some(title.to_string());
        }

        let folder = normalize_folder_path(self.folder.trim());
        if !folder.is_empty() && folder != normalize_folder_path(&self.draft.folder_path) {
            update.folder_path = Some(folder);
        }

        let tags = self.tags.trim();
        let parsed_tags = if tags.is_empty() {
            Vec::new()
        } else {
            tags.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        if parsed_tags != self.draft.tags {
            update.tags = Some(parsed_tags);
        }
        update
    }

    fn handle_folder_picker_key(&mut self, key: &KeyEvent) -> EditDialogAction {
        match key.code {
            KeyCode::Char('x') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                EditDialogAction::Cancelled
            }
            KeyCode::Esc => {
                self.folder_picker_open = false;
                EditDialogAction::Continue
            }
            KeyCode::Up => {
                self.folder_picker_index = self.folder_picker_index.saturating_sub(1);
                EditDialogAction::Continue
            }
            KeyCode::Down => {
                if self.folder_picker_index + 1 < self.folder_options.len() {
                    self.folder_picker_index += 1;
                }
                EditDialogAction::Continue
            }
            KeyCode::Enter => {
                if let Some(path) = self.folder_options.get(self.folder_picker_index) {
                    self.folder = path.clone();
                }
                self.folder_picker_open = false;
                EditDialogAction::Continue
            }
            _ => EditDialogAction::Continue,
        }
    }

    fn render_folder_picker(&self, frame: &mut ratatui::Frame, area: Rect) {
        let picker_area = centered_rect(58, 10, area);
        frame.render_widget(Clear, picker_area);
        let mut list_state = ListState::default();
        list_state.select((!self.folder_options.is_empty()).then_some(self.folder_picker_index));
        let items = self
            .folder_options
            .iter()
            .map(|path| ListItem::new(Line::from(path.clone())))
            .collect::<Vec<_>>();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Select Folder  [Enter Apply | Esc Close]")
                    .style(Style::default().bg(Color::Rgb(17, 28, 40)))
                    .border_style(
                        Style::default()
                            .fg(Color::LightBlue)
                            .bg(Color::Rgb(17, 28, 40))
                            .add_modifier(Modifier::BOLD),
                    )
                    .title_style(
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Rgb(17, 28, 40))
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .highlight_symbol(">> ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            );
        frame.render_stateful_widget(list, picker_area, &mut list_state);
    }
}

fn render_edit_field(
    frame: &mut ratatui::Frame,
    area: Rect,
    label: &str,
    value: &str,
    active: bool,
) {
    let (field_bg, border_fg, text_fg, title_fg) = if active {
        (
            Color::Rgb(32, 59, 86),
            Color::LightCyan,
            Color::White,
            Color::LightYellow,
        )
    } else {
        (
            Color::Rgb(28, 34, 44),
            Color::Gray,
            Color::White,
            Color::Cyan,
        )
    };
    let mut block = Block::default()
        .borders(Borders::ALL)
        .title(label)
        .style(Style::default().bg(field_bg))
        .border_style(Style::default().fg(border_fg).bg(field_bg))
        .title_style(
            Style::default()
                .fg(title_fg)
                .bg(field_bg)
                .add_modifier(Modifier::BOLD),
        );
    if active {
        block = block.border_style(
            Style::default()
                .fg(border_fg)
                .bg(field_bg)
                .add_modifier(Modifier::BOLD),
        );
    }
    frame.render_widget(
        Paragraph::new(if value.is_empty() {
            " ".to_string()
        } else {
            value.to_string()
        })
        .style(Style::default().fg(text_fg).bg(field_bg))
        .block(block),
        area,
    );
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

fn edit_cursor_position(
    edit_state: &EditDialogState,
    title_row: Rect,
    folder_row: Rect,
    tags_row: Rect,
) -> Position {
    let (row, value) = match edit_state.active_field {
        EditField::Title => (title_row, edit_state.title.as_str()),
        EditField::Folder => (folder_row, edit_state.folder.as_str()),
        EditField::Tags => (tags_row, edit_state.tags.as_str()),
    };
    let max_x = row.right().saturating_sub(2);
    let cursor_x = row.x.saturating_add(1 + value.chars().count() as u16);
    Position::new(cursor_x.min(max_x), row.y.saturating_add(1))
}
