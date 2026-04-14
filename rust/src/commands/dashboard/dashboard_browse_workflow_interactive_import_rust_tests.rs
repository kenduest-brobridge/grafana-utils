//! Interactive import workflow regression tests for dashboard browse flows.

use super::*;

#[test]
fn interactive_import_loads_dashboard_titles_and_folder_paths() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "rate(cpu[5m])",
    );

    let args = make_import_args(raw_dir);
    let items = load_interactive_import_items(&args).unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].uid, "cpu-main");
    assert_eq!(items[0].title, "CPU Main");
    assert_eq!(items[0].folder_path, "Infra");
}

#[test]
fn interactive_import_state_toggles_and_confirms_selected_files() {
    let items = vec![
        crate::dashboard::import_interactive::InteractiveImportItem {
            path: PathBuf::from("a.json"),
            uid: "a".to_string(),
            title: "CPU".to_string(),
            folder_path: "Infra".to_string(),
            file_label: "a.json".to_string(),
            review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
        },
        crate::dashboard::import_interactive::InteractiveImportItem {
            path: PathBuf::from("b.json"),
            uid: "b".to_string(),
            title: "Memory".to_string(),
            folder_path: "Infra".to_string(),
            file_label: "b.json".to_string(),
            review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
        },
    ];
    let mut state = InteractiveImportState::new(items, "create-only".to_string(), false);

    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(state.selected_files(), vec![PathBuf::from("a.json")]);
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.selected_files(),
        vec![PathBuf::from("a.json"), PathBuf::from("b.json")]
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        InteractiveImportAction::Confirm(vec![PathBuf::from("a.json"), PathBuf::from("b.json")])
    );
}

#[test]
fn interactive_import_grouping_cycles_folder_action_flat() {
    let items = vec![
        crate::dashboard::import_interactive::InteractiveImportItem {
            path: PathBuf::from("a.json"),
            uid: "a".to_string(),
            title: "CPU".to_string(),
            folder_path: "Infra".to_string(),
            file_label: "a.json".to_string(),
            review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
        },
    ];
    let mut state = InteractiveImportState::new(items, "create-only".to_string(), false);

    assert_eq!(
        state.grouping,
        crate::dashboard::import_interactive::InteractiveImportGrouping::Folder
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.grouping,
        crate::dashboard::import_interactive::InteractiveImportGrouping::Action
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.grouping,
        crate::dashboard::import_interactive::InteractiveImportGrouping::Flat
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.grouping,
        crate::dashboard::import_interactive::InteractiveImportGrouping::Folder
    );
}

#[test]
fn interactive_import_context_view_scope_and_diff_depth_cycle() {
    let items = vec![
        crate::dashboard::import_interactive::InteractiveImportItem {
            path: PathBuf::from("a.json"),
            uid: "a".to_string(),
            title: "CPU".to_string(),
            folder_path: "Infra".to_string(),
            file_label: "a.json".to_string(),
            review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
        },
    ];
    let mut state = InteractiveImportState::new(items, "create-only".to_string(), false);

    assert_eq!(
        state.context_view,
        crate::dashboard::import_interactive::InteractiveImportContextView::Summary
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.context_view,
        crate::dashboard::import_interactive::InteractiveImportContextView::Destination
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.summary_scope,
        crate::dashboard::import_interactive::InteractiveImportSummaryScope::Selected
    );
    assert_eq!(
        state.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)),
        InteractiveImportAction::Continue
    );
    assert_eq!(
        state.diff_depth,
        crate::dashboard::import_interactive::InteractiveImportDiffDepth::Structural
    );
}

#[test]
fn interactive_import_summary_counts_track_pending_selected_and_reviewed_actions() {
    let mut state = InteractiveImportState::new(
        vec![
            crate::dashboard::import_interactive::InteractiveImportItem {
                path: PathBuf::from("a.json"),
                uid: "a".to_string(),
                title: "CPU".to_string(),
                folder_path: "Infra".to_string(),
                file_label: "a.json".to_string(),
                review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
            },
            crate::dashboard::import_interactive::InteractiveImportItem {
                path: PathBuf::from("b.json"),
                uid: "b".to_string(),
                title: "Memory".to_string(),
                folder_path: "Infra".to_string(),
                file_label: "b.json".to_string(),
                review:
                    crate::dashboard::import_interactive::InteractiveImportReviewState::Resolved(
                        Box::new(
                            crate::dashboard::import_interactive::InteractiveImportReview {
                                action: "would-update".to_string(),
                                destination: "exists".to_string(),
                                action_label: "update".to_string(),
                                folder_path: "Infra".to_string(),
                                source_folder_path: "Infra".to_string(),
                                destination_folder_path: "Infra".to_string(),
                                reason: "".to_string(),
                                diff_status: "changed".to_string(),
                                diff_summary_lines: vec!["Title: old -> new".to_string()],
                                diff_structural_lines: vec!["Panels: 1 -> 2".to_string()],
                                diff_raw_lines: vec!["REMOTE".to_string(), "LOCAL".to_string()],
                            },
                        ),
                    ),
            },
        ],
        "create-only".to_string(),
        false,
    );
    state.selected_paths.insert(PathBuf::from("b.json"));

    let counts = state.review_summary_counts();

    assert_eq!(counts.total, 2);
    assert_eq!(counts.selected, 1);
    assert_eq!(counts.pending, 1);
    assert_eq!(counts.reviewed, 1);
    assert_eq!(counts.update, 1);
}

#[test]
fn interactive_import_dry_run_state_uses_dry_run_status_and_enter_copy() {
    let state = InteractiveImportState::new(
        vec![
            crate::dashboard::import_interactive::InteractiveImportItem {
                path: PathBuf::from("a.json"),
                uid: "a".to_string(),
                title: "CPU".to_string(),
                folder_path: "Infra".to_string(),
                file_label: "a.json".to_string(),
                review: crate::dashboard::import_interactive::InteractiveImportReviewState::Pending,
            },
        ],
        "create-only".to_string(),
        true,
    );

    assert!(state.dry_run);
    assert!(state.status.contains("dry-run"));
}

#[test]
fn interactive_import_resolves_focused_review_to_update_existing() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "rate(cpu[5m])",
    );
    let args = make_import_args(raw_dir.clone());
    let items = load_interactive_import_items(&args).unwrap();
    let mut state = InteractiveImportState::new(items, "create-only".to_string(), false);
    let mut cache = crate::dashboard::import_lookup::ImportLookupCache::default();

    state.resolve_focused_review_with_request(
        &mut |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/search") => Ok(Some(json!([
                {"uid":"cpu-main","title":"CPU Main","folderUid":"infra"}
            ]))),
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "uid":"cpu-main",
                    "title":"CPU Main",
                    "tags":[],
                    "panels":[{"id":1}]
                },
                "meta": {"folderUid":"infra"}
            }))),
            (Method::GET, "/api/folders/infra") => Ok(Some(json!({
                "uid":"infra",
                "title":"Infra"
            }))),
            _ => Err(message(format!("unexpected request {method} {path}"))),
        },
        &mut cache,
        &args,
    );

    let item = state.selected_item().unwrap();
    match &item.review {
        crate::dashboard::import_interactive::InteractiveImportReviewState::Resolved(review) => {
            assert_eq!(review.action, "would-fail-existing");
            assert_eq!(review.destination, "exists");
            assert_eq!(review.action_label, "blocked-existing");
            assert_eq!(review.folder_path, "Infra");
            assert_eq!(review.diff_status, "matches live");
            assert!(review.diff_summary_lines[0].contains("already matches"));
        }
        other => panic!("expected resolved review, got {other:?}"),
    }
}

#[test]
fn interactive_import_resolves_skip_missing_for_update_existing_only() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "rate(cpu[5m])",
    );
    let mut args = make_import_args(raw_dir.clone());
    args.update_existing_only = true;
    let items = load_interactive_import_items(&args).unwrap();
    let mut state = InteractiveImportState::new(items, "update-or-skip-missing".to_string(), false);
    let mut cache = crate::dashboard::import_lookup::ImportLookupCache::default();

    state.resolve_focused_review_with_request(
        &mut |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/search") => Ok(Some(json!([]))),
            _ => Err(message(format!("unexpected request {method} {path}"))),
        },
        &mut cache,
        &args,
    );

    let item = state.selected_item().unwrap();
    match &item.review {
        crate::dashboard::import_interactive::InteractiveImportReviewState::Resolved(review) => {
            assert_eq!(review.action, "would-skip-missing");
            assert_eq!(review.action_label, "skip-missing");
            assert_eq!(review.destination, "missing");
            assert_eq!(review.diff_status, "new dashboard");
        }
        other => panic!("expected resolved review, got {other:?}"),
    }
}

#[test]
fn interactive_import_review_surfaces_changed_live_summary() {
    let temp = tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    write_basic_raw_export(
        &raw_dir,
        "1",
        "Main Org.",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "rate(cpu[5m])",
    );
    let args = make_import_args(raw_dir.clone());
    let items = load_interactive_import_items(&args).unwrap();
    let mut state = InteractiveImportState::new(items, "create-only".to_string(), false);
    let mut cache = crate::dashboard::import_lookup::ImportLookupCache::default();

    state.resolve_focused_review_with_request(
        &mut |method, path, _params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/search") => Ok(Some(json!([
                {"uid":"cpu-main","title":"CPU Overview","folderUid":"ops"}
            ]))),
            (Method::GET, "/api/dashboards/uid/cpu-main") => Ok(Some(json!({
                "dashboard": {
                    "uid":"cpu-main",
                    "title":"CPU Overview",
                    "tags":["gold","ops"],
                    "panels":[{"id":1},{"id":2}]
                },
                "meta": {"folderUid":"ops"}
            }))),
            (Method::GET, "/api/folders/infra") => Ok(Some(json!({
                "uid":"infra",
                "title":"Infra"
            }))),
            _ => Err(message(format!("unexpected request {method} {path}"))),
        },
        &mut cache,
        &args,
    );

    let item = state.selected_item().unwrap();
    match &item.review {
        crate::dashboard::import_interactive::InteractiveImportReviewState::Resolved(review) => {
            assert_eq!(review.diff_status, "changed");
            assert!(review
                .diff_summary_lines
                .iter()
                .any(|line| line.contains("Title:")));
            assert!(review
                .diff_summary_lines
                .iter()
                .any(|line| line.contains("Folder UID:")));
            assert!(review
                .diff_summary_lines
                .iter()
                .any(|line| line.contains("Tags:")));
            assert!(review
                .diff_summary_lines
                .iter()
                .any(|line| line.contains("Panels:")));
        }
        other => panic!("expected resolved review, got {other:?}"),
    }
}

#[test]
fn interactive_import_with_use_export_org_falls_through_to_tty_validation() {
    let temp = tempdir().unwrap();
    let export_root = temp.path().join("exports");
    write_combined_export_root_metadata(&export_root, &[("1", "Main Org", "org_1_Main_Org")]);
    let raw_root = export_root.join("org_1_Main_Org/raw");
    write_basic_raw_export(
        &raw_root,
        "1",
        "Main Org",
        "cpu-main",
        "CPU Main",
        "prom-main",
        "prometheus",
        "timeseries",
        "infra",
        "Infra",
        "expr",
        "up",
    );
    let mut args = make_import_args(export_root);
    args.use_export_org = true;
    args.interactive = true;
    let mut cache = crate::dashboard::import_lookup::ImportLookupCache::default();
    let resolved_import = crate::dashboard::import::resolve_import_source(&args).unwrap();
    let dashboard_files =
        crate::dashboard::import::dashboard_files_for_import(resolved_import.dashboard_dir())
            .unwrap();

    let error = crate::dashboard::import_interactive::select_import_dashboard_files(
        &mut |_method, _path, _params, _payload| Ok(None),
        &mut cache,
        &args,
        &resolved_import,
        dashboard_files.as_slice(),
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("Dashboard import interactive mode requires a TTY."));
}
