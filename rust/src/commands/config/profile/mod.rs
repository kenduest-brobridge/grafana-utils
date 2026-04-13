//! Profile namespace facade for repo-local grafana-util configuration.
//!
//! Keeps the public `crate::profile_cli` surface stable while the
//! implementation lives in smaller internal modules.

#[path = "cli_defs.rs"]
mod profile_cli_defs;
#[path = "render.rs"]
mod profile_cli_render;
#[path = "runtime.rs"]
mod profile_cli_runtime;

pub use profile_cli_defs::{
    parse_cli_from, root_command, ProfileAddArgs, ProfileCliArgs, ProfileCommand,
    ProfileCurrentArgs, ProfileExampleArgs, ProfileExampleMode, ProfileInitArgs, ProfileListArgs,
    ProfileSecretStorageMode, ProfileShowArgs, ProfileValidateArgs,
};
pub use profile_cli_runtime::run_profile_cli;

#[cfg(test)]
#[path = "tests.rs"]
mod profile_cli_rust_tests;
