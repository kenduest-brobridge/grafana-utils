#![cfg(any(feature = "tui", test))]
#![cfg_attr(test, allow(dead_code))]

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use super::{ProjectStatusPane, ProjectStatusTuiState};

pub(crate) fn render_project_status_frame(
    frame: &mut ratatui::Frame,
    state: &mut ProjectStatusTuiState,
) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(frame.area());
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(42),
            Constraint::Percentage(28),
        ])
        .split(outer[2]);

    let header_lines = vec![
        summary_line(&[
            summary_cell("Scope", state.document().scope.clone(), Color::White),
            summary_cell(
                "Domains",
                state.document().overall.domain_count.to_string(),
                Color::White,
            ),
            summary_cell(
                "Present",
                state.document().overall.present_count.to_string(),
                Color::White,
            ),
            summary_cell(
                "Blocked",
                state.document().overall.blocked_count.to_string(),
                Color::LightRed,
            ),
            summary_cell(
                "Warnings",
                state.document().overall.warning_count.to_string(),
                Color::Yellow,
            ),
        ]),
        summary_line(&[
            summary_cell(
                "Overall",
                state.document().overall.status.clone(),
                status_color(state.document().overall.status.as_str()),
            ),
            summary_cell(
                "Freshness",
                state.document().overall.freshness.status.clone(),
                Color::White,
            ),
            summary_cell(
                "Domain",
                state
                    .current_domain()
                    .map(|domain| domain.id.as_str())
                    .unwrap_or("No domain"),
                Color::White,
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Focus ",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            key_chip(focus_label(state.focus()), Color::Blue),
            Span::raw("  "),
            Span::styled(
                "Path ",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            plain("Home -> Domains -> Details -> Actions"),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(header_lines)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Project Status Workbench")
                    .border_style(Style::default().fg(Color::LightBlue)),
            ),
        outer[0],
    );

    let home_lines = state
        .home_lines()
        .into_iter()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::White))))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(home_lines)
            .wrap(Wrap { trim: false })
            .block(pane_block(
                "Project Home",
                state.focus() == ProjectStatusPane::Home,
                status_color(state.document().overall.status.as_str()),
            )),
        outer[1],
    );

    let domain_items = state
        .document()
        .domains
        .iter()
        .enumerate()
        .map(|(index, domain)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:>2}. ", index + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    domain.id.clone(),
                    Style::default()
                        .fg(status_color(domain.status.as_str()))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(
                        "  {}  blockers={} warnings={}",
                        domain.status, domain.blocker_count, domain.warning_count
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect::<Vec<_>>();
    let domain_title = state
        .current_domain()
        .map(|domain| {
            format!(
                "Domains {}/{}  current={}",
                state
                    .current_domain_index()
                    .map(|index| index + 1)
                    .unwrap_or(0),
                state.document().domains.len(),
                domain.id
            )
        })
        .unwrap_or_else(|| "Domains".to_string());
    frame.render_stateful_widget(
        List::new(domain_items)
            .block(pane_block(
                &domain_title,
                state.focus() == ProjectStatusPane::Domains,
                status_color(
                    state
                        .current_domain()
                        .map(|domain| domain.status.as_str())
                        .unwrap_or("unknown"),
                ),
            ))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            ),
        body[0],
        state.domain_state_mut(),
    );

    let detail_lines = state
        .current_domain_lines()
        .into_iter()
        .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::White))))
        .collect::<Vec<_>>();
    let detail_title = state
        .current_domain()
        .map(|domain| {
            format!(
                "Domain Detail {}/{}  {}",
                state
                    .current_domain_index()
                    .map(|index| index + 1)
                    .unwrap_or(0),
                state.document().domains.len(),
                domain.id
            )
        })
        .unwrap_or_else(|| "Domain Detail".to_string());
    frame.render_widget(
        Paragraph::new(detail_lines)
            .wrap(Wrap { trim: false })
            .scroll((state.detail_scroll(), 0))
            .block(pane_block(
                &detail_title,
                state.focus() == ProjectStatusPane::Details,
                Color::Cyan,
            )),
        body[1],
    );

    let action_items = state
        .document()
        .next_actions
        .iter()
        .enumerate()
        .map(|(index, action)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:>2}. ", index + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    action.domain.clone(),
                    Style::default()
                        .fg(action_color(action.reason_code.as_str()))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {}  {}", action.reason_code, action.action),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect::<Vec<_>>();
    let action_title = state
        .current_action()
        .map(|action| {
            format!(
                "Actions {}/{}  recommended={}",
                state
                    .current_action_index()
                    .map(|index| index + 1)
                    .unwrap_or(0),
                state.document().next_actions.len(),
                action.domain
            )
        })
        .unwrap_or_else(|| "Actions".to_string());
    frame.render_stateful_widget(
        List::new(action_items)
            .block(pane_block(
                &action_title,
                state.focus() == ProjectStatusPane::Actions,
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
        body[2],
        state.action_state_mut(),
    );

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(state.status_line()),
            aligned_control_line(&[
                ("Tab", Color::Blue, "next pane"),
                ("Shift+Tab", Color::Blue, "previous pane"),
                ("h", Color::Magenta, "home"),
                ("Enter", Color::Magenta, "open handoff"),
            ]),
            aligned_control_line(&[
                ("Up/Down", Color::Blue, "move"),
                ("PgUp/PgDn", Color::Blue, "scroll detail"),
                ("q", Color::Gray, "exit"),
                ("Esc", Color::Gray, "exit"),
            ]),
        ])
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Status & Controls")
                .border_style(Style::default().fg(Color::LightBlue)),
        ),
        outer[3],
    );
}

fn pane_block(title: &str, focused: bool, accent: Color) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(if focused {
            format!("{title} [Focused]")
        } else {
            title.to_string()
        })
        .border_style(Style::default().fg(if focused { accent } else { Color::Gray }))
        .title_style(
            Style::default()
                .fg(if focused { Color::Black } else { Color::White })
                .bg(if focused { accent } else { Color::Reset })
                .add_modifier(Modifier::BOLD),
        )
}

fn status_color(status: &str) -> Color {
    match status {
        "blocked" => Color::LightRed,
        "partial" => Color::Yellow,
        "ready" => Color::Green,
        _ => Color::Gray,
    }
}

fn action_color(reason_code: &str) -> Color {
    match reason_code {
        "blocked-by-blockers" => Color::LightRed,
        "blocked-by-warnings" => Color::Yellow,
        "ready" => Color::Green,
        _ => Color::Cyan,
    }
}

fn key_chip(label: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {label} "),
        Style::default()
            .fg(Color::White)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    )
}

fn plain(value: impl Into<String>) -> Span<'static> {
    Span::styled(value.into(), Style::default().fg(Color::White))
}

struct SummaryCell {
    label: String,
    value: String,
    color: Color,
}

fn summary_cell(label: impl Into<String>, value: impl Into<String>, color: Color) -> SummaryCell {
    SummaryCell {
        label: label.into(),
        value: value.into(),
        color,
    }
}

fn summary_line(items: &[SummaryCell]) -> Line<'static> {
    let cell_width = items
        .iter()
        .map(|item| item.label.chars().count() + item.value.chars().count() + 2)
        .max()
        .unwrap_or(0);
    let mut spans = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        let used_width = item.label.chars().count() + item.value.chars().count() + 2;
        let trailing_padding = cell_width.saturating_sub(used_width);
        spans.push(Span::styled(
            format!("{} ", item.label),
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            item.value.clone(),
            Style::default().fg(item.color).add_modifier(Modifier::BOLD),
        ));
        if trailing_padding > 0 {
            spans.push(Span::raw(" ".repeat(trailing_padding)));
        }
    }
    Line::from(spans)
}

fn aligned_control_line(items: &[(&str, Color, &str)]) -> Line<'static> {
    let key_width = items
        .iter()
        .map(|(key, _, _)| key.chars().count())
        .max()
        .unwrap_or(0);
    let body_width = items
        .iter()
        .map(|(_, _, text)| text.chars().count())
        .max()
        .unwrap_or(0);
    let cell_width = key_width + body_width + 3;
    let mut spans = Vec::new();
    for (index, (key, color, text)) in items.iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw("  "));
        }
        let padded_key = format!("{key:<key_width$}");
        let text_span = format!(" {:<body_width$}", text);
        let used_width = key_width + body_width + 3;
        let trailing_padding = cell_width.saturating_sub(used_width);
        spans.push(key_chip(&padded_key, *color));
        spans.push(plain(text_span));
        if trailing_padding > 0 {
            spans.push(Span::raw(" ".repeat(trailing_padding)));
        }
    }
    Line::from(spans)
}

fn focus_label(focus: ProjectStatusPane) -> &'static str {
    match focus {
        ProjectStatusPane::Home => "Home",
        ProjectStatusPane::Domains => "Domains",
        ProjectStatusPane::Details => "Details",
        ProjectStatusPane::Actions => "Actions",
    }
}
