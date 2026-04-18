//! Snapshot export wrapper tests.

use std::cell::RefCell;
use std::rc::Rc;

use super::tests_fixtures::sample_common_args;
use crate::access::{AccessCommand, OrgCommand, ServiceAccountCommand, TeamCommand, UserCommand};
use crate::datasource::DatasourceGroupCommand;
use crate::snapshot::{
    run_snapshot_export_selected_with_handlers, run_snapshot_export_with_handlers,
    SnapshotExportArgs, SnapshotExportLane, SnapshotExportSelection,
};
use tempfile::tempdir;

#[test]
fn snapshot_export_wrapper_calls_dashboard_then_datasource_runners() {
    let temp = tempdir().unwrap();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let dashboard_args = Rc::new(RefCell::new(None));
    let datasource_args = Rc::new(RefCell::new(None));

    let export_args = SnapshotExportArgs {
        common: sample_common_args(),
        output_dir: temp.path().join("snapshot"),
        overwrite: true,
        prompt: false,
    };

    let dashboard_calls = Rc::clone(&calls);
    let dashboard_seen = Rc::clone(&dashboard_args);
    let datasource_calls = Rc::clone(&calls);
    let datasource_seen = Rc::clone(&datasource_args);
    let access_calls = Rc::clone(&calls);

    run_snapshot_export_with_handlers(
        export_args,
        move |args| {
            dashboard_calls.borrow_mut().push("dashboard".to_string());
            match args.command {
                crate::dashboard::DashboardCommand::Export(inner) => {
                    *dashboard_seen.borrow_mut() = Some(inner);
                    Ok(())
                }
                other => panic!("unexpected dashboard command: {:?}", other),
            }
        },
        move |command| {
            datasource_calls.borrow_mut().push("datasource".to_string());
            match command {
                DatasourceGroupCommand::Export(inner) => {
                    *datasource_seen.borrow_mut() = Some(inner);
                    Ok(())
                }
                other => panic!("unexpected datasource command: {:?}", other),
            }
        },
        move |cli| {
            match cli.command {
                AccessCommand::User {
                    command: UserCommand::Export(_),
                } => access_calls.borrow_mut().push("access-user".to_string()),
                AccessCommand::Team {
                    command: TeamCommand::Export(_),
                } => access_calls.borrow_mut().push("access-team".to_string()),
                AccessCommand::Org {
                    command: OrgCommand::Export(_),
                } => access_calls.borrow_mut().push("access-org".to_string()),
                AccessCommand::ServiceAccount {
                    command: ServiceAccountCommand::Export(_),
                } => access_calls
                    .borrow_mut()
                    .push("access-service-account".to_string()),
                other => panic!("unexpected access command: {:?}", other),
            }
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(
        *calls.borrow(),
        vec![
            "dashboard".to_string(),
            "datasource".to_string(),
            "access-user".to_string(),
            "access-team".to_string(),
            "access-org".to_string(),
            "access-service-account".to_string()
        ]
    );

    let dashboard_args = dashboard_args.borrow().clone().expect("dashboard args");
    let datasource_args = datasource_args.borrow().clone().expect("datasource args");
    assert!(dashboard_args.all_orgs);
    assert_eq!(
        dashboard_args.output_dir,
        temp.path().join("snapshot").join("dashboards")
    );
    assert!(datasource_args.all_orgs);
    assert_eq!(
        datasource_args.output_dir,
        temp.path().join("snapshot").join("datasources")
    );
    assert!(dashboard_args.overwrite);
    assert!(datasource_args.overwrite);
}

#[test]
fn snapshot_export_selected_with_handlers_runs_only_selected_lanes() {
    let temp = tempdir().unwrap();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let selection = SnapshotExportSelection {
        lanes: vec![
            SnapshotExportLane::Datasources,
            SnapshotExportLane::AccessTeams,
            SnapshotExportLane::AccessServiceAccounts,
        ],
    };
    let export_args = SnapshotExportArgs {
        common: sample_common_args(),
        output_dir: temp.path().join("snapshot"),
        overwrite: false,
        prompt: false,
    };

    let dashboard_calls = Rc::clone(&calls);
    let datasource_calls = Rc::clone(&calls);
    let access_calls = Rc::clone(&calls);

    run_snapshot_export_selected_with_handlers(
        export_args,
        &selection,
        move |_args| {
            dashboard_calls.borrow_mut().push("dashboard".to_string());
            Ok(())
        },
        move |command| match command {
            DatasourceGroupCommand::Export(_) => {
                datasource_calls.borrow_mut().push("datasource".to_string());
                Ok(())
            }
            other => panic!("unexpected datasource command: {:?}", other),
        },
        move |cli| {
            match cli.command {
                AccessCommand::Team {
                    command: TeamCommand::Export(_),
                } => access_calls.borrow_mut().push("access-team".to_string()),
                AccessCommand::ServiceAccount {
                    command: ServiceAccountCommand::Export(_),
                } => access_calls
                    .borrow_mut()
                    .push("access-service-account".to_string()),
                other => panic!("unexpected access command: {:?}", other),
            }
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(
        *calls.borrow(),
        vec![
            "datasource".to_string(),
            "access-team".to_string(),
            "access-service-account".to_string()
        ]
    );
}
