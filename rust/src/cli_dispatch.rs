//! Crate-private CLI dispatch spine.
//!
//! This module owns the decision-complete route from parsed unified CLI args
//! to final domain runner invocations.

use crate::access::{
    AccessCliArgs, AccessCommand, OrgCommand, ServiceAccountCommand, TeamCommand, UserCommand,
};
use crate::alert::{normalize_alert_group_command, AlertCliArgs, AlertGroupCommand};
use crate::cli::{
    CliArgs, ConfigCommand, DashboardConvertCommand, DashboardRootCommand, ExportAccessCommand,
    ExportCommand, StatusCommand, UnifiedCommand,
};
use crate::common::{set_json_color_choice, CliColorChoice, Result};
use crate::dashboard::{run_raw_to_prompt, DashboardCliArgs, DashboardCommand, RawToPromptArgs};
use crate::overview::OverviewCliArgs;
use crate::profile_cli::ProfileCliArgs;
use crate::project_status_command::{ProjectStatusCliArgs, ProjectStatusSubcommand};
use crate::resource::ResourceCliArgs;
use crate::snapshot::SnapshotCommand;
use crate::sync::SyncGroupCommand;

#[derive(Debug)]
pub(crate) enum DomainInvocation {
    Version {
        json: bool,
        color: CliColorChoice,
    },
    Dashboard(DashboardCliArgs),
    Datasource {
        default_color: CliColorChoice,
        color: Option<CliColorChoice>,
        command: crate::datasource::DatasourceGroupCommand,
    },
    Alert {
        default_color: CliColorChoice,
        color: Option<CliColorChoice>,
        command: AlertCliArgs,
    },
    Access(AccessCliArgs),
    Workspace(SyncGroupCommand),
    Profile(ProfileCliArgs),
    Snapshot(SnapshotCommand),
    Overview(OverviewCliArgs),
    ProjectStatus(ProjectStatusCliArgs),
    Resource(ResourceCliArgs),
    RawToPrompt(RawToPromptArgs),
}

fn wrap_dashboard_root(command: DashboardRootCommand) -> DashboardCommand {
    match command {
        DashboardRootCommand::Browse(inner) => DashboardCommand::Browse(inner),
        DashboardRootCommand::List(inner) => DashboardCommand::List(inner),
        DashboardRootCommand::Variables(inner) => DashboardCommand::InspectVars(inner),
        DashboardRootCommand::Get(inner) => DashboardCommand::Get(inner),
        DashboardRootCommand::History(inner) => DashboardCommand::History(inner),
        DashboardRootCommand::Clone(inner) => DashboardCommand::CloneLive(inner),
        DashboardRootCommand::EditLive(inner) => DashboardCommand::EditLive(inner),
        DashboardRootCommand::Delete(inner) => DashboardCommand::Delete(inner),
        DashboardRootCommand::Export(inner) => DashboardCommand::Export(inner),
        DashboardRootCommand::Import(inner) => DashboardCommand::Import(inner),
        DashboardRootCommand::Diff(inner) => DashboardCommand::Diff(inner),
        DashboardRootCommand::Convert { .. } => {
            unreachable!("convert is handled before dashboard dispatch")
        }
        DashboardRootCommand::Review(inner) => DashboardCommand::Review(inner),
        DashboardRootCommand::Patch(inner) => DashboardCommand::PatchFile(inner),
        DashboardRootCommand::Serve(inner) => DashboardCommand::Serve(inner),
        DashboardRootCommand::Publish(inner) => DashboardCommand::Publish(inner),
        DashboardRootCommand::Summary(inner) => DashboardCommand::Analyze(inner),
        DashboardRootCommand::Dependencies(inner) => DashboardCommand::Topology(inner),
        DashboardRootCommand::Impact(inner) => DashboardCommand::Impact(inner),
        DashboardRootCommand::Policy(inner) => DashboardCommand::GovernanceGate(inner),
        DashboardRootCommand::Screenshot(inner) => DashboardCommand::Screenshot(inner),
    }
}

fn wrap_export_access(command: ExportAccessCommand) -> AccessCliArgs {
    let command = match command {
        ExportAccessCommand::User(inner) => AccessCommand::User {
            command: UserCommand::Export(inner),
        },
        ExportAccessCommand::Org(inner) => AccessCommand::Org {
            command: OrgCommand::Export(inner),
        },
        ExportAccessCommand::Team(inner) => AccessCommand::Team {
            command: TeamCommand::Export(inner),
        },
        ExportAccessCommand::ServiceAccount(inner) => AccessCommand::ServiceAccount {
            command: ServiceAccountCommand::Export(inner),
        },
    };
    AccessCliArgs { command }
}

fn wrap_project_status(
    color: CliColorChoice,
    command: StatusCommand,
) -> Option<ProjectStatusCliArgs> {
    match command {
        StatusCommand::Live(inner) => Some(ProjectStatusCliArgs {
            color,
            command: ProjectStatusSubcommand::Live(inner),
        }),
        StatusCommand::Staged(inner) => Some(ProjectStatusCliArgs {
            color,
            command: ProjectStatusSubcommand::Staged(inner),
        }),
        _ => None,
    }
}

fn route_cli_args(args: CliArgs) -> DomainInvocation {
    let CliArgs { color, command } = args;
    match command {
        UnifiedCommand::Version(args) => DomainInvocation::Version {
            json: args.json,
            color,
        },
        UnifiedCommand::Status { command } => match command {
            StatusCommand::Overview { staged, command } => {
                DomainInvocation::Overview(OverviewCliArgs {
                    color,
                    staged,
                    command,
                })
            }
            StatusCommand::Snapshot { command } => DomainInvocation::Snapshot(command),
            StatusCommand::Resource { command } => {
                DomainInvocation::Resource(ResourceCliArgs { color, command })
            }
            other => DomainInvocation::ProjectStatus(wrap_project_status(color, other).expect(
                "status routing should cover all non-overview, non-snapshot, non-resource paths",
            )),
        },
        UnifiedCommand::Export { command } => match command {
            ExportCommand::Dashboard(inner) => DomainInvocation::Dashboard(DashboardCliArgs {
                color,
                command: DashboardCommand::Export(inner),
            }),
            ExportCommand::Alert(inner) => DomainInvocation::Alert {
                default_color: color,
                color: None,
                command: normalize_alert_group_command(AlertGroupCommand::Export(inner)),
            },
            ExportCommand::Datasource(inner) => DomainInvocation::Datasource {
                default_color: color,
                color: None,
                command: crate::datasource::normalize_datasource_group_command(
                    crate::datasource::DatasourceGroupCommand::Export(inner),
                ),
            },
            ExportCommand::Access { command } => {
                DomainInvocation::Access(wrap_export_access(command))
            }
        },
        UnifiedCommand::Dashboard { command } => match command {
            DashboardRootCommand::Convert {
                command: DashboardConvertCommand::RawToPrompt(inner),
            } => DomainInvocation::RawToPrompt(inner),
            other => DomainInvocation::Dashboard(DashboardCliArgs {
                color,
                command: wrap_dashboard_root(other),
            }),
        },
        UnifiedCommand::Datasource {
            color: namespace_color,
            command,
        } => DomainInvocation::Datasource {
            default_color: color,
            color: namespace_color,
            command: crate::datasource::normalize_datasource_group_command(command),
        },
        UnifiedCommand::Alert {
            color: namespace_color,
            command,
        } => DomainInvocation::Alert {
            default_color: color,
            color: namespace_color,
            command: normalize_alert_group_command(command),
        },
        UnifiedCommand::Access(inner) => DomainInvocation::Access(inner),
        UnifiedCommand::Workspace { command } => DomainInvocation::Workspace(command),
        UnifiedCommand::Config { command } => match command {
            ConfigCommand::Profile(inner) => DomainInvocation::Profile(inner),
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_with_handlers<FD, FS, FY, FA, FX, FP, FR, FO, FQ, FP2>(
    args: CliArgs,
    mut run_dashboard: FD,
    mut run_datasource: FS,
    mut run_sync: FY,
    mut run_alert: FA,
    mut run_access: FX,
    mut run_profile: FP,
    mut run_snapshot: FR,
    mut run_resource: FO,
    mut run_overview: FQ,
    mut run_project_status: FP2,
) -> Result<()>
where
    FD: FnMut(DashboardCliArgs) -> Result<()>,
    FS: FnMut(crate::datasource::DatasourceGroupCommand) -> Result<()>,
    FY: FnMut(SyncGroupCommand) -> Result<()>,
    FA: FnMut(AlertCliArgs) -> Result<()>,
    FX: FnMut(AccessCliArgs) -> Result<()>,
    FP: FnMut(ProfileCliArgs) -> Result<()>,
    FR: FnMut(SnapshotCommand) -> Result<()>,
    FO: FnMut(ResourceCliArgs) -> Result<()>,
    FQ: FnMut(OverviewCliArgs) -> Result<()>,
    FP2: FnMut(ProjectStatusCliArgs) -> Result<()>,
{
    match route_cli_args(args) {
        DomainInvocation::Version { json, color } => {
            if json {
                let payload = serde_json::json!({
                    "schemaVersion": crate::common::TOOL_VERSION_SCHEMA_VERSION,
                    "name": "grafana-util",
                    "version": crate::common::tool_version(),
                    "commit": crate::common::tool_git_commit(),
                    "buildTime": crate::common::tool_build_time(),
                });
                print!(
                    "{}",
                    crate::common::render_json_value_with_choice(&payload, color, false)?
                );
            } else {
                print!("{}", crate::cli_help::render_unified_version_text());
            }
            Ok(())
        }
        DomainInvocation::Dashboard(args) => run_dashboard(args),
        DomainInvocation::Datasource {
            default_color,
            color,
            command,
        } => {
            set_json_color_choice(color.unwrap_or(default_color));
            run_datasource(command)
        }
        DomainInvocation::Alert {
            default_color,
            color,
            command,
        } => {
            set_json_color_choice(color.unwrap_or(default_color));
            run_alert(command)
        }
        DomainInvocation::Access(args) => run_access(args),
        DomainInvocation::Workspace(command) => run_sync(command),
        DomainInvocation::Profile(args) => run_profile(args),
        DomainInvocation::Snapshot(command) => run_snapshot(command),
        DomainInvocation::Overview(args) => run_overview(args),
        DomainInvocation::ProjectStatus(args) => run_project_status(args),
        DomainInvocation::Resource(args) => run_resource(args),
        DomainInvocation::RawToPrompt(args) => {
            set_json_color_choice(args.color);
            run_raw_to_prompt(&args)
        }
    }
}
