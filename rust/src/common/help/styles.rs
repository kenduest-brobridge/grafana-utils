//! Shared clap help styling for the Rust CLI surfaces.
use clap::builder::styling::{AnsiColor, Styles};

pub(crate) struct HelpPalette {
    pub(crate) section: &'static str,
    pub(crate) command: &'static str,
    pub(crate) argument: &'static str,
    pub(crate) support: &'static str,
    pub(crate) reset: &'static str,
}

pub(crate) const HELP_PALETTE: HelpPalette = HelpPalette {
    section: "\x1b[1;36m",
    command: "\x1b[1;97m",
    argument: "\x1b[1;97m",
    support: "\x1b[90m",
    reset: "\x1b[0m",
};

/// Constant for cli help styles.
pub const CLI_HELP_STYLES: Styles = Styles::styled()
    .header(AnsiColor::Cyan.on_default().bold())
    .usage(AnsiColor::Cyan.on_default().bold())
    .literal(AnsiColor::BrightWhite.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default().bold())
    .context(AnsiColor::BrightGreen.on_default().bold());

pub(crate) fn paint_with(color: &str, text: &str) -> String {
    format!("{color}{text}{}", HELP_PALETTE.reset)
}

pub(crate) fn paint_section(text: &str) -> String {
    paint_with(HELP_PALETTE.section, text)
}

pub(crate) fn paint_command(text: &str) -> String {
    paint_with(HELP_PALETTE.command, text)
}

pub(crate) fn paint_argument(text: &str) -> String {
    paint_with(HELP_PALETTE.argument, text)
}

pub(crate) fn paint_support(text: &str) -> String {
    paint_with(HELP_PALETTE.support, text)
}
