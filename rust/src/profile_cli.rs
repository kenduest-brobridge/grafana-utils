//! Profile namespace facade for repo-local grafana-util configuration.
//!
//! Keeps the public `crate::profile_cli` surface stable while the
//! implementation lives in smaller internal modules.

#[path = "profile_cli_defs.rs"]
mod profile_cli_defs;
#[path = "profile_cli_render.rs"]
mod profile_cli_render;
#[path = "profile_cli_runtime.rs"]
mod profile_cli_runtime;

pub use profile_cli_defs::{
    parse_cli_from, root_command, ProfileAddArgs, ProfileCliArgs, ProfileCommand,
    ProfileExampleArgs, ProfileExampleMode, ProfileInitArgs, ProfileListArgs,
    ProfileSecretStorageMode, ProfileShowArgs,
};
pub use profile_cli_runtime::run_profile_cli;

#[cfg(test)]
#[path = "profile_cli_rust_tests.rs"]
mod profile_cli_rust_tests;
