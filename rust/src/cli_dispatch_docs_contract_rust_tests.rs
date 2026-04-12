use crate::cli::{dispatch_with_handlers, parse_cli_from, CliArgs};
use std::cell::RefCell;

#[test]
fn docs_describe_dashboard_and_legacy_compatibility_surfaces() {
    let en_index = include_str!("../../docs/commands/en/index.md");
    assert!(en_index.contains("Start Here"));
    assert!(en_index.contains("Common Tasks"));
    assert!(en_index.contains("dashboard convert raw-to-prompt"));
    assert!(!en_index.contains("advanced dashboard"));
    assert!(!en_index.contains("migrate dashboard"));
    assert!(en_index.contains("dashboard"));
    assert!(en_index.contains("status"));
    assert!(en_index.contains("export"));
    assert!(en_index.contains("config profile"));

    let zh_index = include_str!("../../docs/commands/zh-TW/index.md");
    assert!(zh_index.contains("先從這裡開始"));
    assert!(zh_index.contains("先選一條操作路徑"));
    assert!(zh_index.contains("dashboard summary"));
    assert!(!zh_index.contains("advanced dashboard"));
    assert!(!zh_index.contains("migrate dashboard"));
    assert!(zh_index.contains("status"));
    assert!(zh_index.contains("export"));
    assert!(zh_index.contains("config profile"));
}

#[test]
fn dispatch_routes_status_live_to_project_status_handler() {
    let routed = RefCell::new(Vec::<String>::new());
    let args: CliArgs = parse_cli_from(["grafana-util", "status", "live", "--all-orgs"]);

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| Ok(()),
        |_datasource_args| Ok(()),
        |_sync_args| Ok(()),
        |_alert_args| Ok(()),
        |_access_args| Ok(()),
        |_profile_args| Ok(()),
        |_snapshot_args| Ok(()),
        |_resource_args| Ok(()),
        |_overview_args| Ok(()),
        |_status_args| {
            routed.borrow_mut().push("status".to_string());
            Ok(())
        },
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["status".to_string()]);
}

#[test]
fn dispatch_routes_export_dashboard_to_dashboard_handler() {
    let routed = RefCell::new(Vec::<String>::new());
    let args: CliArgs = parse_cli_from([
        "grafana-util",
        "export",
        "dashboard",
        "--output-dir",
        "./dashboards",
    ]);

    let result = dispatch_with_handlers(
        args,
        |_dashboard_args| {
            routed.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        |_datasource_args| Ok(()),
        |_sync_args| Ok(()),
        |_alert_args| Ok(()),
        |_access_args| Ok(()),
        |_profile_args| Ok(()),
        |_snapshot_args| Ok(()),
        |_resource_args| Ok(()),
        |_overview_args| Ok(()),
        |_status_args| Ok(()),
    );

    assert!(result.is_ok());
    assert_eq!(*routed.borrow(), vec!["dashboard".to_string()]);
}
