use crate::common::DiffOutputFormat;

use super::alert_cli_args::{
    cli_args_from_common, cli_args_from_defaults, empty_legacy_args, AlertCliArgs,
    AlertNamespaceArgs,
};
use super::alert_cli_commands::{
    AlertAuthoringCommandKind, AlertCommandKind, AlertCommandOutputFormat, AlertGroupCommand,
    AlertListKind, AlertListOutputFormat,
};

/// Lift nested alert command variants into one canonical argument struct and
/// apply single-output-mode migration for list commands.
pub fn normalize_alert_namespace_args(args: AlertNamespaceArgs) -> AlertCliArgs {
    fn apply_output_format(args: &mut AlertCliArgs, output_format: Option<AlertListOutputFormat>) {
        match output_format {
            Some(AlertListOutputFormat::Text) => args.text = true,
            Some(AlertListOutputFormat::Table) => args.table = true,
            Some(AlertListOutputFormat::Csv) => args.csv = true,
            Some(AlertListOutputFormat::Json) => args.json = true,
            Some(AlertListOutputFormat::Yaml) => args.yaml = true,
            None => {}
        }
    }

    match args.command {
        Some(AlertGroupCommand::Export(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Export);
            args.output_dir = inner.output_dir;
            args.flat = inner.flat;
            args.overwrite = inner.overwrite;
            args
        }
        Some(AlertGroupCommand::Import(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Import);
            args.input_dir = Some(inner.input_dir);
            args.replace_existing = inner.replace_existing;
            args.dry_run = inner.dry_run;
            args.json = inner.json;
            args.dashboard_uid_map = inner.dashboard_uid_map;
            args.panel_id_map = inner.panel_id_map;
            args
        }
        Some(AlertGroupCommand::Diff(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Diff);
            args.diff_dir = Some(inner.diff_dir);
            args.diff_output = Some(if inner.json {
                DiffOutputFormat::Json
            } else {
                inner.output_format
            });
            args.json = inner.json || matches!(inner.output_format, DiffOutputFormat::Json);
            args.dashboard_uid_map = inner.dashboard_uid_map;
            args.panel_id_map = inner.panel_id_map;
            args
        }
        Some(AlertGroupCommand::Plan(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Plan);
            args.desired_dir = Some(inner.desired_dir);
            args.prune = inner.prune;
            args.dashboard_uid_map = inner.dashboard_uid_map;
            args.panel_id_map = inner.panel_id_map;
            args.command_output = Some(inner.output_format);
            args.json = matches!(inner.output_format, AlertCommandOutputFormat::Json);
            args
        }
        Some(AlertGroupCommand::Apply(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Apply);
            args.plan_file = Some(inner.plan_file);
            args.approve = inner.approve;
            args.command_output = Some(inner.output_format);
            args.json = matches!(inner.output_format, AlertCommandOutputFormat::Json);
            args
        }
        Some(AlertGroupCommand::Delete(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::Delete);
            args.resource_kind = Some(inner.kind);
            args.resource_identity = Some(inner.identity);
            args.allow_policy_reset = inner.allow_policy_reset;
            args.command_output = Some(inner.output_format);
            args.json = matches!(inner.output_format, AlertCommandOutputFormat::Json);
            args
        }
        Some(AlertGroupCommand::Init(inner)) => {
            let mut args = cli_args_from_defaults();
            args.command_kind = Some(AlertCommandKind::Init);
            args.desired_dir = Some(inner.desired_dir);
            args
        }
        Some(AlertGroupCommand::AddRule(inner)) => {
            let mut args = cli_args_from_defaults();
            args.authoring_command_kind = Some(AlertAuthoringCommandKind::AddRule);
            args.desired_dir = Some(inner.base.desired_dir);
            args.scaffold_name = Some(inner.name);
            args.folder = Some(inner.folder);
            args.rule_group = Some(inner.rule_group);
            args.receiver = inner.receiver;
            args.no_route = inner.no_route;
            args.labels = inner.labels;
            args.annotations = inner.annotations;
            args.severity = inner.severity;
            args.for_duration = inner.for_duration;
            args.expr = inner.expr;
            args.threshold = inner.threshold;
            args.above = inner.above;
            args.below = inner.below;
            args.dry_run = inner.dry_run;
            args
        }
        Some(AlertGroupCommand::CloneRule(inner)) => {
            let mut args = cli_args_from_defaults();
            args.authoring_command_kind = Some(AlertAuthoringCommandKind::CloneRule);
            args.desired_dir = Some(inner.base.desired_dir);
            args.source_name = Some(inner.source);
            args.scaffold_name = Some(inner.name);
            args.folder = inner.folder;
            args.rule_group = inner.rule_group;
            args.receiver = inner.receiver;
            args.no_route = inner.no_route;
            args.dry_run = inner.dry_run;
            args
        }
        Some(AlertGroupCommand::AddContactPoint(inner)) => {
            let mut args = cli_args_from_defaults();
            args.authoring_command_kind = Some(AlertAuthoringCommandKind::AddContactPoint);
            args.desired_dir = Some(inner.base.desired_dir);
            args.scaffold_name = Some(inner.name);
            args.dry_run = inner.dry_run;
            args
        }
        Some(AlertGroupCommand::SetRoute(inner)) => {
            let mut args = cli_args_from_defaults();
            args.authoring_command_kind = Some(AlertAuthoringCommandKind::SetRoute);
            args.desired_dir = Some(inner.base.desired_dir);
            args.receiver = Some(inner.receiver);
            args.labels = inner.labels;
            args.severity = inner.severity;
            args.dry_run = inner.dry_run;
            args
        }
        Some(AlertGroupCommand::PreviewRoute(inner)) => {
            let mut args = cli_args_from_defaults();
            args.authoring_command_kind = Some(AlertAuthoringCommandKind::PreviewRoute);
            args.desired_dir = Some(inner.base.desired_dir);
            args.labels = inner.labels;
            args.severity = inner.severity;
            args
        }
        Some(AlertGroupCommand::NewRule(inner)) => {
            let mut args = cli_args_from_defaults();
            args.command_kind = Some(AlertCommandKind::NewRule);
            args.desired_dir = Some(inner.desired_dir);
            args.scaffold_name = Some(inner.name);
            args
        }
        Some(AlertGroupCommand::NewContactPoint(inner)) => {
            let mut args = cli_args_from_defaults();
            args.command_kind = Some(AlertCommandKind::NewContactPoint);
            args.desired_dir = Some(inner.desired_dir);
            args.scaffold_name = Some(inner.name);
            args
        }
        Some(AlertGroupCommand::NewTemplate(inner)) => {
            let mut args = cli_args_from_defaults();
            args.command_kind = Some(AlertCommandKind::NewTemplate);
            args.desired_dir = Some(inner.desired_dir);
            args.scaffold_name = Some(inner.name);
            args
        }
        Some(AlertGroupCommand::ListRules(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::ListRules);
            args.list_kind = Some(AlertListKind::Rules);
            args.org_id = inner.org_id;
            args.all_orgs = inner.all_orgs;
            args.text = inner.text;
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            args.yaml = inner.yaml;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        Some(AlertGroupCommand::ListContactPoints(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::ListContactPoints);
            args.list_kind = Some(AlertListKind::ContactPoints);
            args.org_id = inner.org_id;
            args.all_orgs = inner.all_orgs;
            args.text = inner.text;
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            args.yaml = inner.yaml;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        Some(AlertGroupCommand::ListMuteTimings(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::ListMuteTimings);
            args.list_kind = Some(AlertListKind::MuteTimings);
            args.org_id = inner.org_id;
            args.all_orgs = inner.all_orgs;
            args.text = inner.text;
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            args.yaml = inner.yaml;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        Some(AlertGroupCommand::ListTemplates(inner)) => {
            let mut args = cli_args_from_common(inner.common);
            args.command_kind = Some(AlertCommandKind::ListTemplates);
            args.list_kind = Some(AlertListKind::Templates);
            args.org_id = inner.org_id;
            args.all_orgs = inner.all_orgs;
            args.text = inner.text;
            args.table = inner.table;
            args.csv = inner.csv;
            args.json = inner.json;
            args.yaml = inner.yaml;
            apply_output_format(&mut args, inner.output_format);
            args.no_header = inner.no_header;
            args
        }
        None => {
            let legacy = args.legacy;
            AlertCliArgs {
                command_kind: None,
                authoring_command_kind: None,
                profile: legacy.common.profile,
                url: legacy.common.url,
                api_token: legacy.common.api_token,
                username: legacy.common.username,
                password: legacy.common.password,
                prompt_password: legacy.common.prompt_password,
                prompt_token: legacy.common.prompt_token,
                output_dir: legacy.output_dir,
                input_dir: legacy.input_dir,
                diff_dir: legacy.diff_dir,
                timeout: legacy.common.timeout,
                flat: legacy.flat,
                overwrite: legacy.overwrite,
                replace_existing: legacy.replace_existing,
                dry_run: legacy.dry_run,
                dashboard_uid_map: legacy.dashboard_uid_map,
                panel_id_map: legacy.panel_id_map,
                verify_ssl: legacy.common.verify_ssl,
                org_id: None,
                all_orgs: false,
                list_kind: None,
                text: false,
                table: false,
                csv: false,
                json: false,
                yaml: false,
                no_header: false,
                diff_output: None,
                desired_dir: None,
                prune: false,
                plan_file: None,
                approve: false,
                allow_policy_reset: false,
                resource_kind: None,
                resource_identity: None,
                command_output: None,
                scaffold_name: None,
                source_name: None,
                folder: None,
                rule_group: None,
                receiver: None,
                no_route: false,
                labels: Vec::new(),
                annotations: Vec::new(),
                severity: None,
                for_duration: None,
                expr: None,
                threshold: None,
                above: false,
                below: false,
            }
        }
    }
}

/// Small adapter for callers that already have a concrete group command and need
/// the full normalized AlertCliArgs form.
pub fn normalize_alert_group_command(command: AlertGroupCommand) -> AlertCliArgs {
    normalize_alert_namespace_args(AlertNamespaceArgs {
        command: Some(command),
        legacy: empty_legacy_args(),
    })
}
