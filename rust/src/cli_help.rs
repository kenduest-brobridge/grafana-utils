//! Unified CLI help examples and rendering helpers.
//!
//! The CLI help subsystem is split by responsibility:
//! grouped entrypoint specs, grouped rendering, schema-help routing,
//! and contextual clap rendering.

mod contextual;
mod flat;
mod grouped;
pub(crate) mod grouped_specs;
mod routing;
mod schema;

pub(crate) use contextual::canonicalize_inferred_subcommands;
pub use flat::render_unified_help_flat_text;
pub use routing::{
    maybe_render_unified_help_from_os_args, render_unified_help_full_text,
    render_unified_help_text, render_unified_version_text,
};
pub(crate) use routing::{
    UNIFIED_ACCESS_HELP_TEXT, UNIFIED_ALERT_HELP_TEXT, UNIFIED_DATASOURCE_HELP_TEXT,
    UNIFIED_SYNC_HELP_TEXT,
};
