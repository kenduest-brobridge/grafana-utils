//! Interactive sync review TUI.
//! Allows operators to keep or drop actionable sync operations before the plan is marked reviewed.
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;
use serde_json::Value;
use std::collections::BTreeSet;
use std::io::{self, Stdout};
use std::time::Duration;

use crate::common::{message, Result};

use super::{build_sync_alert_assessment_document, build_sync_plan_summary_document};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReviewableOperation {
    key: String,
    label: String,
    operation: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewDiffModel {
    pub title: String,
    pub action: String,
    pub live_lines: Vec<(bool, String)>,
    pub desired_lines: Vec<(bool, String)>,
}

fn operation_key(operation: &serde_json::Map<String, Value>) -> String {
    format!(
        "{}::{}",
        operation
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        operation
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    )
}

fn operation_label(operation: &serde_json::Map<String, Value>) -> String {
    format!(
        "[{}] {} {}",
        operation
            .get("action")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        operation
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        operation
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    )
}

fn collect_reviewable_operations(plan: &Value) -> Result<Vec<ReviewableOperation>> {
    let operations = plan
        .get("operations")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Sync plan document is missing operations."))?;
    Ok(operations
        .iter()
        .filter_map(Value::as_object)
        .filter(|operation| {
            matches!(
                operation.get("action").and_then(Value::as_str),
                Some("would-create" | "would-update" | "would-delete")
            )
        })
        .map(|operation| ReviewableOperation {
            key: operation_key(operation),
            label: operation_label(operation),
            operation: Value::Object(operation.clone()),
        })
        .collect())
}

pub(crate) fn filter_review_plan_operations(
    plan: &Value,
    selected_keys: &BTreeSet<String>,
) -> Result<Value> {
    let plan_object = plan
        .as_object()
        .ok_or_else(|| message("Sync plan document must be a JSON object."))?;
    let operations = plan_object
        .get("operations")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Sync plan document is missing operations."))?;
    let filtered_operations = operations
        .iter()
        .filter(|item| {
            let Some(object) = item.as_object() else {
                return false;
            };
            match object.get("action").and_then(Value::as_str) {
                Some("would-create" | "would-update" | "would-delete") => {
                    selected_keys.contains(&operation_key(object))
                }
                _ => true,
            }
        })
        .cloned()
        .collect::<Vec<_>>();

    let mut filtered = plan_object.clone();
    filtered.insert(
        "summary".to_string(),
        build_sync_plan_summary_document(&filtered_operations),
    );
    filtered.insert(
        "alertAssessment".to_string(),
        build_sync_alert_assessment_document(&filtered_operations),
    );
    filtered.insert("operations".to_string(), Value::Array(filtered_operations));
    Ok(Value::Object(filtered))
}

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

fn pretty_inline_json(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "null".to_string(),
        Some(Value::String(text)) => format!("{text:?}"),
        Some(other) => serde_json::to_string(other).unwrap_or_else(|_| "null".to_string()),
    }
}

pub(crate) fn build_review_operation_diff_model(operation: &Value) -> Result<ReviewDiffModel> {
    let object = operation
        .as_object()
        .ok_or_else(|| message("Sync review operation must be a JSON object."))?;
    let action = object
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let title = format!(
        "{} {}",
        object
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        object
            .get("identity")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    );
    let desired = object.get("desired").and_then(Value::as_object);
    let live = object.get("live").and_then(Value::as_object);
    let changed_fields = object
        .get("changedFields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| item.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    let mut fields = if changed_fields.is_empty() {
        let mut combined = BTreeSet::new();
        if let Some(object) = live {
            combined.extend(object.keys().cloned());
        }
        if let Some(object) = desired {
            combined.extend(object.keys().cloned());
        }
        combined.into_iter().collect::<Vec<_>>()
    } else {
        changed_fields
    };
    if fields.is_empty() {
        fields.push("<no managed field changes>".to_string());
    }
    let mut live_lines = Vec::new();
    let mut desired_lines = Vec::new();
    for field in fields {
        if field == "<no managed field changes>" {
            live_lines.push((false, field.clone()));
            desired_lines.push((false, field));
            continue;
        }
        let live_value = live.and_then(|object| object.get(&field));
        let desired_value = desired.and_then(|object| object.get(&field));
        let changed = live_value != desired_value;
        live_lines.push((
            changed,
            format!("{field}: {}", pretty_inline_json(live_value)),
        ));
        desired_lines.push((
            changed,
            format!("{field}: {}", pretty_inline_json(desired_value)),
        ));
    }
    Ok(ReviewDiffModel {
        title,
        action,
        live_lines,
        desired_lines,
    })
}

fn render_diff_lines(lines: &[(bool, String)], color: Color) -> Text<'static> {
    Text::from(
        lines
            .iter()
            .map(|(changed, line)| {
                if *changed {
                    Line::from(Span::styled(
                        line.clone(),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(line.clone())
                }
            })
            .collect::<Vec<_>>(),
    )
}

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

    loop {
        session.terminal.draw(|frame| {
            let outer = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(3)])
                .split(frame.area());
            if diff_mode {
                let selected = state.selected().unwrap_or(0);
                let model = items
                    .get(selected)
                    .and_then(|item| build_review_operation_diff_model(&item.operation).ok());
                let panes = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(outer[0]);
                if let Some(model) = model {
                    let live = Paragraph::new(render_diff_lines(&model.live_lines, Color::Red))
                        .block(
                            Block::default()
                                .title(format!("Live [{}] {}", model.action, model.title))
                                .borders(Borders::ALL),
                        );
                    let desired = Paragraph::new(render_diff_lines(
                        &model.desired_lines,
                        Color::Green,
                    ))
                    .block(
                        Block::default()
                            .title(format!("Desired [{}] {}", model.action, model.title))
                            .borders(Borders::ALL),
                    );
                    frame.render_widget(live, panes[0]);
                    frame.render_widget(desired, panes[1]);
                }
                let help = Paragraph::new("Esc/q back  Up/Down change selection  c confirm  Space toggle")
                    .block(Block::default().borders(Borders::ALL).title("Diff Controls"));
                frame.render_widget(help, outer[1]);
            } else {
                let list_items = items
                    .iter()
                    .map(|item| {
                        let checked = if selected_keys.contains(&item.key) {
                            "[x]"
                        } else {
                            "[ ]"
                        };
                        ListItem::new(format!("{checked} {}", item.label))
                    })
                    .collect::<Vec<_>>();
                let list = List::new(list_items)
                    .block(Block::default().title("Sync Review").borders(Borders::ALL))
                    .highlight_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    );
                frame.render_stateful_widget(list, outer[0], &mut state);
                let help = Paragraph::new(
                    "Up/Down move  Space toggle  a select-all  n select-none  Enter diff  c confirm  q cancel",
                )
                .block(Block::default().borders(Borders::ALL).title("Controls"));
                frame.render_widget(help, outer[1]);
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
                KeyCode::Up => {
                    let next = selected.saturating_sub(1);
                    state.select(Some(next));
                }
                KeyCode::Down => {
                    let next = (selected + 1).min(items.len().saturating_sub(1));
                    state.select(Some(next));
                }
                KeyCode::Char(' ') => {
                    if let Some(item) = items.get(selected) {
                        if !selected_keys.insert(item.key.clone()) {
                            selected_keys.remove(&item.key);
                        }
                    }
                }
                KeyCode::Char('a') => {
                    if !diff_mode {
                        selected_keys = items.iter().map(|item| item.key.clone()).collect();
                    }
                }
                KeyCode::Char('n') => {
                    if !diff_mode {
                        selected_keys.clear();
                    }
                }
                KeyCode::Enter => {
                    if !diff_mode {
                        diff_mode = true;
                    }
                }
                KeyCode::Char('c') => return filter_review_plan_operations(plan, &selected_keys),
                KeyCode::Char('q') | KeyCode::Esc => {
                    if diff_mode {
                        diff_mode = false;
                        continue;
                    }
                    return Err(message("Interactive sync review cancelled."));
                }
                _ => {}
            }
        }
    }
}
