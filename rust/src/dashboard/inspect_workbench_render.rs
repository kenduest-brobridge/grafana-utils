#![cfg(feature = "tui")]
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use super::inspect_workbench_state::{InspectPane, InspectWorkbenchState};

#[path = "inspect_workbench_render_helpers.rs"]
mod inspect_workbench_render_helpers;
#[path = "inspect_workbench_render_modal.rs"]
mod inspect_workbench_render_modal;

use self::inspect_workbench_render_helpers::{
    compact_count_label, control_line, group_color, item_color, item_row_text, pane_block,
    slice_visible,
};
use self::inspect_workbench_render_modal::{
    render_detail_panel, render_full_detail_viewer, render_search_prompt,
};

pub(crate) fn render_frame(frame: &mut ratatui::Frame, state: &mut InspectWorkbenchState) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(1),
            Constraint::Length(4),
        ])
        .split(frame.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(22),
            Constraint::Percentage(35),
            Constraint::Percentage(43),
        ])
        .split(outer[1]);

    frame.render_widget(
        Paragraph::new(
            state
                .document
                .summary_lines
                .iter()
                .cloned()
                .map(Line::from)
                .collect::<Vec<_>>(),
        )
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(state.document.title.clone()),
        ),
        outer[0],
    );

    let group_items = state
        .document
        .groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            let view_index = state.group_view_indexes.get(index).copied().unwrap_or(0);
            let count = group
                .views
                .get(view_index)
                .map(|view| view.items.len())
                .unwrap_or(0);
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", compact_count_label(count)),
                    Style::default()
                        .fg(Color::White)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    group.label.clone(),
                    Style::default()
                        .fg(group_color(&group.kind))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  {}", group.subtitle)),
            ]))
        })
        .collect::<Vec<_>>();
    frame.render_stateful_widget(
        List::new(group_items)
            .block(pane_block("Modes", state.focus == InspectPane::Groups))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
        panes[0],
        &mut state.group_state,
    );

    let items_title = state
        .current_group()
        .map(|group| {
            format!(
                "Items {}/{}  {} / {}",
                state.item_state.selected().map(|i| i + 1).unwrap_or(0),
                state.current_items().len(),
                group.label,
                state.current_view_label()
            )
        })
        .unwrap_or_else(|| "Items".to_string());
    let item_row_texts = state
        .current_items()
        .iter()
        .enumerate()
        .map(|(index, item)| item_row_text(index, item))
        .collect::<Vec<_>>();
    let items_inner_width = panes[1].width.saturating_sub(4) as usize;
    let max_item_width = item_row_texts
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(0);
    state.clamp_item_horizontal_offset(max_item_width.saturating_sub(items_inner_width));
    let item_rows = state
        .current_items()
        .iter()
        .enumerate()
        .zip(item_row_texts.iter())
        .map(|((_, item), row_text)| {
            let visible = slice_visible(row_text, state.item_horizontal_offset, items_inner_width);
            ListItem::new(Line::from(vec![Span::styled(
                visible,
                Style::default().fg(item_color(&item.kind)),
            )]))
        })
        .collect::<Vec<_>>();
    frame.render_stateful_widget(
        List::new(item_rows)
            .block(pane_block(&items_title, state.focus == InspectPane::Items))
            .highlight_symbol("▌ ")
            .repeat_highlight_symbol(true)
            .highlight_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
        panes[1],
        &mut state.item_state,
    );

    render_detail_panel(frame, panes[2], state);

    frame.render_widget(
        Paragraph::new(vec![
            control_line(&[
                ("Tab", Color::Blue, "next pane"),
                ("Shift+Tab", Color::Blue, "prev pane"),
                ("g", Color::Magenta, "mode"),
                ("v", Color::Magenta, "mode view"),
                ("/ ?", Color::Yellow, "search"),
                ("n", Color::Yellow, "next"),
            ]),
            control_line(&[
                ("Up/Down", Color::Blue, "move"),
                ("Left/Right", Color::Blue, "items pan"),
                ("Home/End", Color::Blue, "bounds"),
                ("PgUp/PgDn", Color::Blue, "jump"),
                ("Enter", Color::Blue, "open viewer"),
                ("q", Color::Gray, "exit"),
                ("Esc", Color::Gray, "exit"),
            ]),
            Line::from(Span::styled(
                state.status.clone(),
                Style::default().fg(Color::Gray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title("Controls")),
        outer[2],
    );

    if let Some(search) = state.pending_search.as_ref() {
        render_search_prompt(frame, search);
    }
    if state.full_detail_open {
        render_full_detail_viewer(frame, state);
    }
}
