//! Clap schema for dashboard CLI commands.
//! Hosts dashboard command enums/args and parser helpers consumed by the dashboard runtime module.

#[path = "cli_defs_command.rs"]
mod cli_defs_command;
#[path = "cli_defs_inspect.rs"]
mod cli_defs_inspect;
#[path = "cli_defs_shared.rs"]
mod cli_defs_shared;
#[path = "cli_help_texts.rs"]
mod cli_help_texts;
#[path = "dashboard_runtime.rs"]
mod dashboard_runtime;

pub use cli_defs_command::*;
pub use cli_defs_inspect::*;
pub use cli_defs_shared::*;
pub(crate) use cli_help_texts::*;
pub(crate) use dashboard_runtime::materialize_dashboard_common_auth;
pub(crate) use dashboard_runtime::{build_api_client, build_http_client_for_org_from_api};
pub use dashboard_runtime::{
    build_auth_context, build_http_client, build_http_client_for_org, normalize_dashboard_cli_args,
    parse_cli_from, DashboardAuthContext,
};
