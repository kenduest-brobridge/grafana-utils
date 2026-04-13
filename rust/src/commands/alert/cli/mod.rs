//! Clap schema for alerting CLI commands.
//! Defines args/enums/normalization helpers used by alert dispatcher and handlers.

#[path = "args.rs"]
mod alert_cli_args;
#[path = "auth.rs"]
mod alert_cli_auth;
#[path = "commands.rs"]
mod alert_cli_commands;
#[path = "normalize.rs"]
mod alert_cli_normalize;
#[path = "help_texts.rs"]
mod alert_help_texts;

pub use self::alert_cli_args::{
    cli_args_from_common, parse_cli_from, root_command, AlertAddContactPointArgs, AlertAddRuleArgs,
    AlertApplyArgs, AlertCliArgs, AlertCloneRuleArgs, AlertCommonArgs, AlertDeleteArgs,
    AlertDiffArgs, AlertExportArgs, AlertImportArgs, AlertInitArgs, AlertLegacyArgs, AlertListArgs,
    AlertNamespaceArgs, AlertNewResourceArgs, AlertPlanArgs, AlertPreviewRouteArgs,
    AlertSetRouteArgs,
};
pub use self::alert_cli_auth::{build_auth_context, AlertAuthContext};
pub use self::alert_cli_commands::{
    AlertAuthoringCommandKind, AlertCommandKind, AlertCommandOutputFormat, AlertGroupCommand,
    AlertListKind, AlertResourceKind,
};
pub use self::alert_cli_normalize::{
    normalize_alert_group_command, normalize_alert_namespace_args,
};
