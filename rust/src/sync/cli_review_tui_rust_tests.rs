//! Sync CLI review TUI regression test suite.
//! Keeps diff preview and line-ordering contracts stable for review mode.
use super::review_tui;
use serde_json::json;

#[test]
fn build_review_operation_diff_model_formats_changed_fields_side_by_side() {
    let operation = json!({
        "kind": "dashboard",
        "identity": "cpu-main",
        "action": "would-update",
        "changedFields": ["title", "refresh"],
        "live": {
            "title": "CPU Old",
            "refresh": "30s"
        },
        "desired": {
            "title": "CPU New",
            "refresh": "5s"
        }
    });

    let model = review_tui::build_review_operation_diff_model(&operation).unwrap();

    assert_eq!(model.title, "dashboard cpu-main");
    assert_eq!(model.action, "would-update");
    assert_eq!(model.live_lines.len(), 2);
    assert_eq!(model.desired_lines.len(), 2);
    assert!(model.live_lines.iter().all(|row| row.changed));
    assert!(model.desired_lines.iter().all(|row| row.changed));
    assert_eq!(model.live_lines[0].marker, '-');
    assert_eq!(model.desired_lines[0].marker, '+');
    assert!(model.live_lines[0].content.starts_with("  1 | "));
    assert!(model.desired_lines[1].content.starts_with("  2 | "));
    assert!(model.live_lines[0].content.contains("title"));
    assert!(model.desired_lines[1].content.contains("refresh"));
    assert!(model.live_lines[0].highlight_range.is_some());
    assert!(model.desired_lines[0].highlight_range.is_some());
}

#[test]
fn review_operation_preview_uses_readable_action_labels() {
    let plan = json!({
        "kind": "grafana-utils-sync-plan",
        "operations": [
            {
                "kind": "datasource",
                "identity": "prom-main",
                "action": "would-update",
                "changedFields": ["url"]
            }
        ]
    });

    let items = review_tui::collect_reviewable_operations(&plan).unwrap();
    let preview = review_tui::operation_preview(&items[0]);
    let title = review_tui::selection_title_with_position(Some(&items[0]), None, None);
    let positioned_title =
        review_tui::selection_title_with_position(Some(&items[0]), Some(0), Some(3));
    let detail_lines = review_tui::operation_detail_line_count(&items[0]);
    let changed_count = review_tui::operation_changed_count(&items[0]);

    assert_eq!(preview[0], "Action: UPDATE");
    assert_eq!(title, "Selection [UPDATE] prom-main");
    assert_eq!(positioned_title, "Selection 1/3 [UPDATE] prom-main");
    assert_eq!(detail_lines, 1);
    assert_eq!(changed_count, 0);
    assert_eq!(
        review_tui::diff_pane_title("Live", "would-update", "datasource prom-main", 0, 3),
        "Live 1/3 [would-update] datasource prom-main"
    );
    #[cfg(feature = "tui")]
    {
        let controls = review_tui::build_diff_controls_lines(&review_tui::DiffControlsState {
            selected: 0,
            total: 3,
            diff_focus: review_tui::DiffPaneFocus::Live,
            live_wrap_lines: false,
            desired_wrap_lines: true,
            live_diff_cursor: 2,
            live_horizontal_offset: 12,
            desired_diff_cursor: 1,
            desired_horizontal_offset: 0,
        });
        assert_eq!(controls.len(), 3);
        assert!(controls[0].to_string().contains("Item 1/3"));
        assert!(controls[0].to_string().contains("Focus"));
        assert!(controls[0].to_string().contains("Live wrap OFF"));
        assert!(controls[0].to_string().contains("Desired wrap ON"));
        assert!(controls[1].to_string().contains("Tab"));
        assert!(controls[1].to_string().contains("switch pane"));
        assert!(controls[1].to_string().contains("PgUp/PgDn"));
        assert!(controls[2].to_string().contains("Home/End"));
        assert!(controls[2].to_string().contains("confirm staged"));
        assert!(controls[2].to_string().contains("Esc/q"));
    }
    #[cfg(feature = "tui")]
    {
        let header_lines =
            review_tui::build_review_header_lines(3, 2, true, review_tui::DiffPaneFocus::Desired);
        assert_eq!(header_lines.len(), 3);
        assert!(header_lines[0]
            .to_string()
            .contains("Reviewable staged operations=3"));
        assert!(header_lines[1].to_string().contains("Mode=diff"));
        assert!(header_lines[1].to_string().contains("active-pane=desired"));
        assert_eq!(
            review_tui::review_status(true),
            "Diff mode active. Tab switches pane, Esc returns to the checklist, c confirms the staged selection."
        );
        assert_eq!(
            review_tui::review_status(false),
            "Checklist mode active. Space keeps or drops staged operations, Enter opens the diff view, c confirms the staged selection."
        );
        assert!(header_lines[2]
            .to_string()
            .contains("Keep the staged plan primary."));
    }
}

#[test]
fn review_diff_scroll_max_uses_longer_side() {
    let model = review_tui::ReviewDiffModel {
        title: "dashboard cpu-main".to_string(),
        action: "would-update".to_string(),
        live_lines: vec![review_tui::ReviewDiffLine {
            changed: true,
            marker: '-',
            content: "  1 | title: \"old\"".to_string(),
            highlight_range: Some((13, 16)),
        }],
        desired_lines: vec![
            review_tui::ReviewDiffLine {
                changed: true,
                marker: '+',
                content: "  1 | title: \"new\"".to_string(),
                highlight_range: Some((13, 16)),
            },
            review_tui::ReviewDiffLine {
                changed: true,
                marker: '+',
                content: "  2 | refresh: \"5s\"".to_string(),
                highlight_range: Some((15, 19)),
            },
            review_tui::ReviewDiffLine {
                changed: true,
                marker: '+',
                content: "  3 | tags: [\"prod\"]".to_string(),
                highlight_range: Some((11, 19)),
            },
        ],
    };

    assert_eq!(
        review_tui::diff_scroll_max(&model, review_tui::DiffPaneFocus::Live),
        0
    );
    assert_eq!(
        review_tui::diff_scroll_max(&model, review_tui::DiffPaneFocus::Desired),
        2
    );
}

#[test]
fn wrap_text_chunks_splits_long_diff_lines_for_pane_width() {
    let wrapped = review_tui::wrap_text_chunks(
        "  1 | datasourceUid: \"smoke-prom-extra-with-very-long-name\"",
        18,
    );

    assert!(wrapped.len() > 1);
    assert_eq!(wrapped[0], "  1 | datasourceUi");
}

#[test]
fn clip_text_window_slices_nowrap_diff_content() {
    let clipped = review_tui::clip_text_window(
        "  1 | datasourceUid: \"smoke-prom-extra-with-very-long-name\"",
        6,
        16,
    );

    assert_eq!(clipped, "datasourceUid: \"");
}

#[test]
fn review_diff_model_orders_changed_fields_before_unchanged_fields() {
    let operation = json!({
        "kind": "datasource",
        "identity": "prom-main",
        "action": "would-update",
        "live": {
            "name": "Prometheus Main",
            "type": "prometheus",
            "url": "http://prometheus:9090"
        },
        "desired": {
            "name": "Prometheus Main",
            "type": "prometheus",
            "url": "http://prometheus-v2:9090"
        }
    });

    let model = review_tui::build_review_operation_diff_model(&operation).unwrap();

    assert_eq!(model.live_lines[0].marker, '-');
    assert!(model.live_lines[0].content.contains("url"));
    assert_eq!(model.live_lines[1].marker, '=');
}
