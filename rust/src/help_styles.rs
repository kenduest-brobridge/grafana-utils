//! Shared clap help styling for the Rust CLI surfaces.
use clap::builder::styling::{AnsiColor, Styles};

pub const CLI_HELP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Blue.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default().bold())
    .context(AnsiColor::Yellow.on_default());
