#![cfg(feature = "tui")]
#![cfg_attr(test, allow(dead_code))]

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState};

use crate::tui_shell;

use super::super::overview_kind::parse_overview_artifact_kind;
use super::OverviewWorkbenchState;

fn section_color(kind: &str) -> Color {
    parse_overview_artifact_kind(kind)
        .map(|artifact_kind| artifact_kind.section_color())
        .unwrap_or(Color::Gray)
}

fn item_color(kind: &str) -> Color {
    match kind {
        "dashboard" => Color::Yellow,
        "datasource" => Color::Cyan,
        "alert" | "alert-rule" => Color::Red,
        "user" | "team" | "org" | "service-account" => Color::Green,
        "warning" | "violation" => Color::LightRed,
        "drift" => Color::LightBlue,
        "policy" => Color::Magenta,
        _ => Color::Gray,
    }
}

fn build_header_lines(state: &OverviewWorkbenchState) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![
            tui_shell::label("Artifacts "),
            tui_shell::accent(
                state.document.summary.artifact_count.to_string(),
                Color::White,
            ),
            Span::raw("  "),
            tui_shell::label("Sections "),
            tui_shell::accent(state.document.sections.len().to_string(), Color::White),
            Span::raw("  "),
            tui_shell::label("Focus "),
            tui_shell::key_chip(state.status_focus_label(), Color::Blue),
        ]),
        Line::from(vec![
            tui_shell::label("Section "),
            tui_shell::accent(
                state
                    .current_section()
                    .map(|section| section.label.clone())
                    .unwrap_or_else(|| "none".to_string()),
                state
                    .current_section()
                    .map(|section| section_color(&section.kind))
                    .unwrap_or(Color::Gray),
            ),
            Span::raw("  "),
            tui_shell::label("View "),
            tui_shell::accent(state.current_view_label(), Color::White),
            Span::raw("  "),
            tui_shell::label("Item "),
            tui_shell::accent(
                state
                    .selected_item()
                    .map(|item| item.title.clone())
                    .unwrap_or_else(|| "none".to_string()),
                Color::White,
            ),
        ]),
    ];
    lines.extend(
        state
            .project_home_lines()
            .into_iter()
            .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::White)))),
    );
    lines
}

fn pane_block(title: &str, focused: bool, accent: Color) -> Block<'static> {
    tui_shell::pane_block(title, focused, accent, Color::Black)
}

pub(super) fn render_overview_frame(
    frame: &mut ratatui::Frame,
    state: &mut OverviewWorkbenchState,
) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(frame.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(18),
            Constraint::Percentage(22),
            Constraint::Percentage(27),
            Constraint::Percentage(33),
        ])
        .split(outer[1]);

    let header_lines = build_header_lines(state);
    frame.render_widget(tui_shell::build_header("Overview", header_lines), outer[0]);

    let section_items = state
        .document
        .sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:>2}. ", index + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    section.label.clone(),
                    Style::default()
                        .fg(section_color(&section.kind))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {} views  {}", section.views.len(), section.subtitle),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect::<Vec<_>>();
    frame.render_stateful_widget(
        List::new(section_items)
            .block(pane_block(
                "Sections",
                state.focus == super::OverviewPane::Sections,
                Color::Yellow,
            ))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        panes[0],
        &mut state.section_state,
    );

    let view_items = state
        .current_section()
        .map(|section| {
            section
                .views
                .iter()
                .enumerate()
                .map(|(index, view)| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{:>2}. ", index + 1),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            view.label.clone(),
                            Style::default()
                                .fg(Color::LightCyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  {} items", view.items.len()),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let view_title = state
        .current_section()
        .map(|section| {
            format!(
                "Views {}/{}",
                state
                    .view_state
                    .selected()
                    .map(|index| index + 1)
                    .unwrap_or(0),
                section.views.len()
            )
        })
        .unwrap_or_else(|| "Views".to_string());
    frame.render_stateful_widget(
        List::new(view_items)
            .block(pane_block(
                &view_title,
                state.focus == super::OverviewPane::Views,
                Color::LightCyan,
            ))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
        panes[1],
        &mut state.view_state,
    );

    let item_items = state
        .current_items()
        .iter()
        .enumerate()
        .map(|(index, item)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:>2}. ", index + 1),
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
            ]))
        })
        .collect::<Vec<_>>();
    let item_title = state
        .current_section()
        .map(|section| {
            format!(
                "Items {}/{}  {} / {}",
                state
                    .item_state
                    .selected()
                    .map(|index| index + 1)
                    .unwrap_or(0),
                state.current_items().len(),
                section.label,
                state.current_view_label()
            )
        })
        .unwrap_or_else(|| "Items".to_string());
    frame.render_stateful_widget(
        List::new(item_items)
            .block(pane_block(
                &item_title,
                state.focus == super::OverviewPane::Items,
                Color::Cyan,
            ))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        panes[2],
        &mut state.item_state,
    );

    let detail_lines = state.current_detail_lines();
    let detail_title = state
        .selected_item()
        .map(|item| {
            format!(
                "Details {}/{} [{}]  line {}/{}",
                state
                    .item_state
                    .selected()
                    .map(|index| index + 1)
                    .unwrap_or(0),
                state.current_items().len(),
                item.kind,
                (state.detail_scroll as usize + 1).min(detail_lines.len().max(1)),
                detail_lines.len().max(1)
            )
        })
        .unwrap_or_else(|| "Details".to_string());
    let detail_items = detail_lines
        .iter()
        .map(|line| {
            ListItem::new(Line::from(Span::styled(
                line.clone(),
                Style::default().fg(Color::White),
            )))
        })
        .collect::<Vec<_>>();
    let mut detail_state = ListState::default();
    detail_state.select(Some(
        (state.detail_scroll as usize).min(detail_lines.len().saturating_sub(1)),
    ));
    frame.render_stateful_widget(
        List::new(detail_items)
            .block(pane_block(
                &detail_title,
                state.focus == super::OverviewPane::Details,
                Color::LightBlue,
            ))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
        panes[3],
        &mut detail_state,
    );

    frame.render_widget(
        tui_shell::build_footer_controls(vec![
            tui_shell::control_line(&[
                ("Tab", Color::Blue, "next pane"),
                ("Shift+Tab", Color::Blue, "previous pane"),
                ("h", Color::Blue, "home"),
                ("Enter", Color::Blue, "open handoff"),
            ]),
            tui_shell::control_line(&[
                ("Up/Down", Color::Blue, "move"),
                ("Home/End", Color::Blue, "jump"),
                ("PgUp/PgDn", Color::Blue, "scroll detail"),
                ("q/Esc", Color::Gray, "exit"),
            ]),
        ]),
        outer[2],
    );
}
