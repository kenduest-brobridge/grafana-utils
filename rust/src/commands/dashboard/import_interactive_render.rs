#![cfg(feature = "tui")]

use std::time::Duration;

use crossterm::event::{self, Event};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, Paragraph, Wrap};
use reqwest::Method;
use serde_json::Value;

use crate::common::Result;
use crate::grafana_api::DashboardResourceClient;
use crate::tui_shell;

use super::browse_terminal::TerminalSession;
use super::import_interactive::{
    InteractiveImportAction, InteractiveImportGrouping, InteractiveImportItem,
    InteractiveImportReviewState, InteractiveImportState,
};
use super::import_interactive_context::build_context_lines;
use super::import_lookup::ImportLookupCache;
use super::import_render::describe_dashboard_import_mode;

pub(crate) fn run_import_selector<F>(
    request_json: &mut F,
    lookup_cache: &mut ImportLookupCache,
    args: &super::ImportArgs,
    import_dir_label: String,
    items: Vec<InteractiveImportItem>,
) -> Result<Option<Vec<std::path::PathBuf>>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let mut session = TerminalSession::enter()?;
    session.terminal.hide_cursor()?;
    let mut state = InteractiveImportState::new(
        items,
        describe_dashboard_import_mode(args.replace_existing, args.update_existing_only)
            .to_string(),
        args.dry_run,
    );
    loop {
        session.terminal.draw(|frame| {
            let size = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(10),
                    Constraint::Length(3),
                ])
                .split(size);
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
                .split(chunks[1]);
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(56), Constraint::Percentage(44)])
                .split(body[1]);
            let summary = state.review_summary_counts();

            let header = tui_shell::build_header(
                "Interactive Dashboard Import",
                vec![
                    Line::from(format!(
                        "Import dir={}   Mode={}   ReviewMode={}   Grouping={}   Selected={}/{}",
                        import_dir_label,
                        state.import_mode,
                        if state.dry_run { "dry-run" } else { "import" },
                        state.grouping.label(),
                        state.selected_paths.len(),
                        state.items.len()
                    )),
                    Line::from(format!(
                        "Review={} pending={} create={} update={} skip-missing={} skip-folder={} blocked={} selected={}",
                        summary.reviewed,
                        summary.pending,
                        summary.create,
                        summary.update,
                        summary.skip_missing,
                        summary.skip_folder,
                        summary.blocked,
                        summary.selected
                    )),
                    Line::from(state.focused_group_summary().unwrap_or_else(|| {
                        "Flat grouping keeps one continuous dashboard list.".to_string()
                    })),
                ],
            );
            frame.render_widget(header, chunks[0]);

            let ordered = state.ordered_indices();
            let list_items: Vec<ListItem> = {
                let grouping = state.grouping;
                let selected_paths = &state.selected_paths;
                let items = &state.items;
                ordered
                    .iter()
                    .enumerate()
                    .map(|(visible_index, item_index)| {
                        let item = &items[*item_index];
                        let marker = if selected_paths.contains(&item.path) {
                            "[x]"
                        } else {
                            "[ ]"
                        };
                        let folder = if item.folder_path.is_empty() {
                            "General"
                        } else {
                            item.folder_path.as_str()
                        };
                        let mut lines = Vec::new();
                        if grouping != InteractiveImportGrouping::Flat {
                            let current_group = match grouping {
                                InteractiveImportGrouping::Folder => {
                                    if item.folder_path.is_empty() {
                                        "General".to_string()
                                    } else {
                                        item.folder_path.clone()
                                    }
                                }
                                InteractiveImportGrouping::Action => state.action_group_title(item),
                                InteractiveImportGrouping::Flat => String::new(),
                            };
                            let previous_group = visible_index.checked_sub(1).map(|previous| {
                                let previous_item = &items[ordered[previous]];
                                match grouping {
                                    InteractiveImportGrouping::Folder => {
                                        if previous_item.folder_path.is_empty() {
                                            "General".to_string()
                                        } else {
                                            previous_item.folder_path.clone()
                                        }
                                    }
                                    InteractiveImportGrouping::Action => {
                                        state.action_group_title(previous_item)
                                    }
                                    InteractiveImportGrouping::Flat => String::new(),
                                }
                            });
                            if previous_group.as_deref() != Some(current_group.as_str()) {
                                lines.push(Line::from(Span::styled(
                                    format!(" {} ", current_group),
                                    Style::default()
                                        .fg(Color::Black)
                                        .bg(Color::Rgb(132, 146, 166))
                                        .add_modifier(Modifier::BOLD),
                                )));
                            }
                        }
                        lines.push(Line::from(vec![
                            Span::styled(review_badge(item), review_badge_style(item)),
                            Span::raw(" "),
                            Span::styled(marker, Style::default().fg(Color::Green)),
                            Span::raw(" "),
                            Span::styled(
                                item.title.as_str(),
                                Style::default().add_modifier(Modifier::BOLD),
                            ),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("uid ", Style::default().fg(Color::DarkGray)),
                            Span::raw(item.uid.as_str()),
                            Span::raw("  "),
                            Span::styled("folder ", Style::default().fg(Color::DarkGray)),
                            Span::raw(folder),
                        ]));
                        ListItem::new(lines)
                    })
                    .collect()
            };
            let list = List::new(list_items)
                .block(tui_shell::pane_block("Dashboards", true, Color::Cyan, Color::Reset))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightBlue)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");
            frame.render_stateful_widget(list, body[0], &mut state.list_state);

            let detail = Paragraph::new(build_review_lines(&state))
                .block(tui_shell::pane_block("Review", false, Color::Yellow, Color::Reset))
                .wrap(Wrap { trim: false });
            frame.render_widget(detail, right[0]);

            let context_title = format!(
                "Context [{} | scope={} | diff={}]",
                state.context_view.label(),
                state.summary_scope.label(),
                state.diff_depth.label()
            );
            let context = Paragraph::new(build_context_lines(&state))
                .block(tui_shell::pane_block(
                    &context_title,
                    false,
                    Color::LightBlue,
                    Color::Reset,
                ))
                .wrap(Wrap { trim: false });
            frame.render_widget(context, right[1]);

            let footer = tui_shell::build_footer(
                vec![tui_shell::control_line(&[
                    ("Up/Down", Color::Rgb(24, 78, 140), "move"),
                    ("Space", Color::Rgb(24, 106, 59), "toggle"),
                    ("a", Color::Rgb(24, 106, 59), "all/none"),
                    ("g", Color::Rgb(164, 116, 19), "grouping"),
                    ("v", Color::Rgb(71, 55, 152), "context view"),
                    ("s", Color::Rgb(71, 55, 152), "scope"),
                    ("d", Color::Rgb(71, 55, 152), "diff depth"),
                    (
                        "Enter",
                        Color::Rgb(24, 106, 59),
                        if state.dry_run {
                            "dry-run selected"
                        } else {
                            "import selected"
                        },
                    ),
                    ("q", Color::Rgb(90, 98, 107), "cancel"),
                ])],
                state.status.as_str(),
            );
            frame.render_widget(Clear, chunks[2]);
            frame.render_widget(footer, chunks[2]);
        })?;

        if state.focus_needs_review() {
            state.resolve_focused_review_with_request(request_json, lookup_cache, args);
            continue;
        }
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        match state.handle_key(key) {
            InteractiveImportAction::Continue => {}
            InteractiveImportAction::Confirm(files) => return Ok(Some(files)),
            InteractiveImportAction::Cancel => return Ok(None),
        }
    }
}

pub(crate) fn run_import_selector_with_client(
    client: &DashboardResourceClient<'_>,
    lookup_cache: &mut ImportLookupCache,
    args: &super::ImportArgs,
    import_dir_label: String,
    items: Vec<InteractiveImportItem>,
) -> Result<Option<Vec<std::path::PathBuf>>> {
    let mut session = TerminalSession::enter()?;
    session.terminal.hide_cursor()?;
    let mut state = InteractiveImportState::new(
        items,
        describe_dashboard_import_mode(args.replace_existing, args.update_existing_only)
            .to_string(),
        args.dry_run,
    );
    loop {
        session.terminal.draw(|frame| {
            let size = frame.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(10),
                    Constraint::Length(3),
                ])
                .split(size);
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
                .split(chunks[1]);
            let right = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(56), Constraint::Percentage(44)])
                .split(body[1]);
            let summary = state.review_summary_counts();

            let header = tui_shell::build_header(
                "Interactive Dashboard Import",
                vec![
                    Line::from(format!(
                        "Import dir={}   Mode={}   ReviewMode={}   Grouping={}   Selected={}/{}",
                        import_dir_label,
                        state.import_mode,
                        if state.dry_run { "dry-run" } else { "import" },
                        state.grouping.label(),
                        state.selected_paths.len(),
                        state.items.len()
                    )),
                    Line::from(format!(
                        "Review={} pending={} create={} update={} skip-missing={} skip-folder={} blocked={} selected={}",
                        summary.reviewed,
                        summary.pending,
                        summary.create,
                        summary.update,
                        summary.skip_missing,
                        summary.skip_folder,
                        summary.blocked,
                        summary.selected
                    )),
                    Line::from(state.focused_group_summary().unwrap_or_else(|| {
                        "Flat grouping keeps one continuous dashboard list.".to_string()
                    })),
                ],
            );
            frame.render_widget(header, chunks[0]);

            let ordered = state.ordered_indices();
            let list_items: Vec<ListItem> = {
                let grouping = state.grouping;
                let selected_paths = &state.selected_paths;
                let items = &state.items;
                ordered
                    .iter()
                    .enumerate()
                    .map(|(visible_index, item_index)| {
                        let item = &items[*item_index];
                        let marker = if selected_paths.contains(&item.path) {
                            "[x]"
                        } else {
                            "[ ]"
                        };
                        let folder = if item.folder_path.is_empty() {
                            "General"
                        } else {
                            item.folder_path.as_str()
                        };
                        let mut lines = Vec::new();
                        if grouping != InteractiveImportGrouping::Flat {
                            let current_group = match grouping {
                                InteractiveImportGrouping::Folder => {
                                    if item.folder_path.is_empty() {
                                        "General".to_string()
                                    } else {
                                        item.folder_path.clone()
                                    }
                                }
                                InteractiveImportGrouping::Action => state.action_group_title(item),
                                InteractiveImportGrouping::Flat => String::new(),
                            };
                            let previous_group = visible_index.checked_sub(1).map(|previous| {
                                let previous_item = &items[ordered[previous]];
                                match grouping {
                                    InteractiveImportGrouping::Folder => {
                                        if previous_item.folder_path.is_empty() {
                                            "General".to_string()
                                        } else {
                                            previous_item.folder_path.clone()
                                        }
                                    }
                                    InteractiveImportGrouping::Action => {
                                        state.action_group_title(previous_item)
                                    }
                                    InteractiveImportGrouping::Flat => String::new(),
                                }
                            });
                            if previous_group.as_deref() != Some(current_group.as_str()) {
                                lines.push(Line::from(Span::styled(
                                    format!(" {} ", current_group),
                                    Style::default()
                                        .fg(Color::Black)
                                        .bg(Color::Rgb(132, 146, 166))
                                        .add_modifier(Modifier::BOLD),
                                )));
                            }
                        }
                        lines.push(Line::from(vec![
                            Span::styled(review_badge(item), review_badge_style(item)),
                            Span::raw(" "),
                            Span::styled(marker, Style::default().fg(Color::Green)),
                            Span::raw(" "),
                            Span::styled(
                                item.title.as_str(),
                                Style::default().add_modifier(Modifier::BOLD),
                            ),
                        ]));
                        lines.push(Line::from(vec![
                            Span::styled("uid ", Style::default().fg(Color::DarkGray)),
                            Span::raw(item.uid.as_str()),
                            Span::raw("  "),
                            Span::styled("folder ", Style::default().fg(Color::DarkGray)),
                            Span::raw(folder),
                        ]));
                        ListItem::new(lines)
                    })
                    .collect()
            };
            let list = List::new(list_items)
                .block(tui_shell::pane_block("Dashboards", true, Color::Cyan, Color::Reset))
                .highlight_style(
                    Style::default()
                        .bg(Color::LightBlue)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");
            frame.render_stateful_widget(list, body[0], &mut state.list_state);

            let detail = Paragraph::new(build_review_lines(&state))
                .block(tui_shell::pane_block("Review", false, Color::Yellow, Color::Reset))
                .wrap(Wrap { trim: false });
            frame.render_widget(detail, right[0]);

            let context_title = format!(
                "Context [{} | scope={} | diff={}]",
                state.context_view.label(),
                state.summary_scope.label(),
                state.diff_depth.label()
            );
            let context = Paragraph::new(build_context_lines(&state))
                .block(tui_shell::pane_block(
                    &context_title,
                    false,
                    Color::LightBlue,
                    Color::Reset,
                ))
                .wrap(Wrap { trim: false });
            frame.render_widget(context, right[1]);

            let footer = tui_shell::build_footer(
                vec![tui_shell::control_line(&[
                    ("Up/Down", Color::Rgb(24, 78, 140), "move"),
                    ("Space", Color::Rgb(24, 106, 59), "toggle"),
                    ("a", Color::Rgb(24, 106, 59), "all/none"),
                    ("g", Color::Rgb(164, 116, 19), "grouping"),
                    ("v", Color::Rgb(71, 55, 152), "context view"),
                    ("s", Color::Rgb(71, 55, 152), "scope"),
                    ("d", Color::Rgb(71, 55, 152), "diff depth"),
                    (
                        "Enter",
                        Color::Rgb(24, 106, 59),
                        if state.dry_run {
                            "dry-run selected"
                        } else {
                            "import selected"
                        },
                    ),
                    ("q", Color::Rgb(90, 98, 107), "cancel"),
                ])],
                state.status.as_str(),
            );
            frame.render_widget(Clear, chunks[2]);
            frame.render_widget(footer, chunks[2]);
        })?;

        if state.focus_needs_review() {
            state.resolve_focused_review_with_client(client, lookup_cache, args);
            continue;
        }
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        match state.handle_key(key) {
            InteractiveImportAction::Continue => {}
            InteractiveImportAction::Confirm(files) => return Ok(Some(files)),
            InteractiveImportAction::Cancel => return Ok(None),
        }
    }
}

fn build_review_lines<'a>(state: &'a InteractiveImportState) -> Vec<Line<'a>> {
    if let Some(item) = state.selected_item() {
        let mut lines = vec![
            Line::from(Span::styled(
                item.title.as_str(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("UID: ", Style::default().fg(Color::Yellow)),
                Span::raw(item.uid.as_str()),
            ]),
            Line::from(vec![
                Span::styled("File: ", Style::default().fg(Color::Yellow)),
                Span::raw(item.file_label.as_str()),
            ]),
            Line::from(vec![
                Span::styled("Source Folder: ", Style::default().fg(Color::Yellow)),
                Span::raw(if item.folder_path.is_empty() {
                    "General"
                } else {
                    item.folder_path.as_str()
                }),
            ]),
            Line::from(vec![
                Span::styled("Import Mode: ", Style::default().fg(Color::Yellow)),
                Span::raw(state.import_mode.as_str()),
            ]),
            Line::from(""),
        ];
        match &item.review {
            InteractiveImportReviewState::Pending => {
                lines.push(Line::from(Span::styled(
                    "Review pending. Move focus here to resolve create/update/skip behavior.",
                    Style::default().fg(Color::Gray),
                )));
            }
            InteractiveImportReviewState::Failed(error) => {
                lines.push(Line::from(Span::styled(
                    "BLOCKED REVIEW",
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
                lines.push(Line::from(error.as_str()));
            }
            InteractiveImportReviewState::Resolved(review) => {
                lines.push(Line::from(vec![
                    Span::styled("Review: ", Style::default().fg(Color::Yellow)),
                    Span::raw(format!("{} {}", review.destination, review.action_label)),
                ]));
                lines.push(Line::from(vec![
                    Span::styled("Target Folder: ", Style::default().fg(Color::Yellow)),
                    Span::raw(review.folder_path.as_str()),
                ]));
                if !review.destination_folder_path.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("Existing Folder: ", Style::default().fg(Color::Yellow)),
                        Span::raw(review.destination_folder_path.as_str()),
                    ]));
                }
                if !review.reason.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("Reason: ", Style::default().fg(Color::Yellow)),
                        Span::raw(review.reason.as_str()),
                    ]));
                }
                lines.push(Line::from(vec![
                    Span::styled("Live Diff: ", Style::default().fg(Color::Yellow)),
                    Span::raw(review.diff_status.as_str()),
                ]));
                for diff_line in &review.diff_summary_lines {
                    lines.push(Line::from(diff_line.as_str()));
                }
            }
        }
        lines
    } else {
        vec![Line::from("No dashboard selected.")]
    }
}

fn review_badge(item: &InteractiveImportItem) -> &'static str {
    match &item.review {
        InteractiveImportReviewState::Pending => "PENDING",
        InteractiveImportReviewState::Failed(_) => "BLOCKED",
        InteractiveImportReviewState::Resolved(review) => match review.action_label.as_str() {
            "create" => "CREATE",
            "update" => "UPDATE",
            "skip-missing" => "SKIP-MISSING",
            "skip-folder-mismatch" => "SKIP-FOLDER",
            "blocked-existing" => "BLOCKED",
            _ => "REVIEWED",
        },
    }
}

fn review_badge_style(item: &InteractiveImportItem) -> Style {
    match &item.review {
        InteractiveImportReviewState::Pending => Style::default().fg(Color::Black).bg(Color::Gray),
        InteractiveImportReviewState::Failed(_) => {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        }
        InteractiveImportReviewState::Resolved(review) => match review.action_label.as_str() {
            "create" => Style::default().fg(Color::Black).bg(Color::Green),
            "update" => Style::default().fg(Color::Black).bg(Color::Yellow),
            "skip-missing" | "skip-folder-mismatch" => {
                Style::default().fg(Color::Black).bg(Color::LightBlue)
            }
            "blocked-existing" => Style::default().fg(Color::White).bg(Color::Red),
            _ => Style::default().fg(Color::White).bg(Color::DarkGray),
        },
    }
}
