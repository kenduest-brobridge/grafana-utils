use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::common::CliColorChoice;
use crate::dashboard::SimpleOutputFormat;

const PROFILE_HELP_TEXT: &str = "Examples:\n\n  grafana-util profile list\n  grafana-util profile current\n  grafana-util profile show --profile prod --output-format yaml\n  grafana-util profile validate --profile prod\n  grafana-util profile validate --profile prod --live --output-format json\n  grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file\n  grafana-util profile example --mode basic\n  grafana-util profile example --mode full\n  grafana-util profile init --overwrite";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProfileSecretStorageMode {
    File,
    Os,
    EncryptedFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProfileExampleMode {
    Basic,
    Full,
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util profile",
    about = "List, inspect, add, render examples for, and initialize repo-local grafana-util profiles.",
    after_help = PROFILE_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
pub struct ProfileCliArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = CliColorChoice::Auto,
        help = "Colorize JSON output. Use auto, always, or never."
    )]
    pub color: CliColorChoice,
    #[command(subcommand)]
    pub command: ProfileCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ProfileCommand {
    #[command(
        about = "List profile names from the resolved grafana-util config file.",
        after_help = "Prints one discovered profile name per line from the resolved config path."
    )]
    List(ProfileListArgs),
    #[command(
        about = "Show the selected profile as YAML or text.",
        after_help = "Use --profile NAME to show a specific profile instead of the default-selection rules."
    )]
    Show(ProfileShowArgs),
    #[command(
        about = "Show the currently selected profile and resolved config path.",
        after_help = "Use this to confirm which repo-local profile would be selected before running status live, overview live, or any Grafana command that accepts --profile."
    )]
    Current(ProfileCurrentArgs),
    #[command(
        about = "Validate the selected profile and optionally check live Grafana reachability.",
        after_help = "Static validation checks profile selection, auth shape, env-backed credentials, and secret-store resolution. Add --live to also call Grafana /api/health with the selected profile."
    )]
    Validate(ProfileValidateArgs),
    #[command(
        about = "Add one named profile to grafana-util.yaml.",
        after_help = "Creates or updates one profile entry without requiring manual YAML editing."
    )]
    Add(Box<ProfileAddArgs>),
    #[command(
        about = "Initialize grafana-util.yaml in the current working directory.",
        after_help = "Creates grafana-util.yaml from the built-in profile template and refuses to overwrite it unless --overwrite is set."
    )]
    Init(ProfileInitArgs),
    #[command(
        about = "Render a complete annotated profile config example.",
        after_help = "Use this when you want a full reference config instead of the minimal init template."
    )]
    Example(ProfileExampleArgs),
}

#[derive(Debug, Clone, Args, Default)]
pub struct ProfileListArgs {}

#[derive(Debug, Clone, Args)]
pub struct ProfileShowArgs {
    #[arg(
        long,
        help = "Show a specific profile by name instead of using the default-selection rules."
    )]
    pub profile: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Reveal secret values instead of masking them."
    )]
    pub show_secrets: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the encrypted-file passphrase without echoing it."
    )]
    pub prompt_secret_passphrase: bool,
    #[arg(
        long,
        help = "Environment variable name that contains the encrypted-file passphrase."
    )]
    pub secret_passphrase_env: Option<String>,
    #[arg(
        long,
        value_enum,
        default_value_t = SimpleOutputFormat::Text,
        help = "Render the selected profile as text, table, csv, json, or yaml."
    )]
    pub output_format: SimpleOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct ProfileCurrentArgs {
    #[arg(
        long,
        help = "Show a specific profile by name instead of using the default-selection rules."
    )]
    pub profile: Option<String>,
    #[arg(
        long,
        value_enum,
        default_value_t = SimpleOutputFormat::Text,
        help = "Render the selected profile summary as text, table, csv, json, or yaml."
    )]
    pub output_format: SimpleOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct ProfileValidateArgs {
    #[arg(
        long,
        help = "Validate a specific profile by name instead of using the default-selection rules."
    )]
    pub profile: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Also call Grafana /api/health with the selected profile after static validation succeeds."
    )]
    pub live: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = SimpleOutputFormat::Text,
        help = "Render the validation result as text, table, csv, json, or yaml."
    )]
    pub output_format: SimpleOutputFormat,
}

#[derive(Debug, Clone, Args)]
pub struct ProfileAddArgs {
    #[arg(help = "Name of the profile to add or replace.")]
    pub name: String,
    #[arg(long, help = "Grafana base URL for this profile.")]
    pub url: String,
    #[arg(long, help = "API token to store in the selected secret mode.")]
    pub token: Option<String>,
    #[arg(long, help = "Environment variable name that contains the API token.")]
    pub token_env: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the API token without echoing it."
    )]
    pub prompt_token: bool,
    #[arg(long, help = "Basic-auth username for this profile.")]
    pub basic_user: Option<String>,
    #[arg(
        long,
        help = "Basic-auth password to store in the selected secret mode."
    )]
    pub basic_password: Option<String>,
    #[arg(
        long,
        help = "Environment variable name that contains the Basic-auth password."
    )]
    pub password_env: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Basic-auth password without echoing it."
    )]
    pub prompt_password: bool,
    #[arg(long, help = "Optional Grafana org ID to store in the profile.")]
    pub org_id: Option<i64>,
    #[arg(long, help = "Optional timeout in seconds to store in the profile.")]
    pub timeout: Option<u64>,
    #[arg(
        long,
        default_value_t = false,
        help = "Store verify_ssl: true in the profile."
    )]
    pub verify_ssl: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Store insecure: true in the profile."
    )]
    pub insecure: bool,
    #[arg(long, help = "Store a custom CA certificate path in the profile.")]
    pub ca_cert: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Make the added profile the default selection."
    )]
    pub set_default: bool,
    #[arg(
        long,
        default_value_t = false,
        help = "Replace an existing profile with the same name."
    )]
    pub replace_existing: bool,
    #[arg(
        long,
        value_enum,
        default_value_t = ProfileSecretStorageMode::File,
        help = "Choose how secret values are stored: file, os, or encrypted-file."
    )]
    pub store_secret: ProfileSecretStorageMode,
    #[arg(
        long,
        help = "Path to the encrypted secret file used by encrypted-file mode."
    )]
    pub secret_file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the encrypted-file passphrase without echoing it."
    )]
    pub prompt_secret_passphrase: bool,
    #[arg(
        long,
        help = "Environment variable name that contains the encrypted-file passphrase."
    )]
    pub secret_passphrase_env: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct ProfileExampleArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = ProfileExampleMode::Full,
        help = "Render either a compact example or the full annotated template."
    )]
    pub mode: ProfileExampleMode,
}

#[derive(Debug, Clone, Args)]
pub struct ProfileInitArgs {
    #[arg(
        long,
        default_value_t = false,
        help = "Allow overwriting an existing grafana-util.yaml file."
    )]
    pub overwrite: bool,
}

pub fn root_command() -> clap::Command {
    ProfileCliArgs::command()
}

pub fn parse_cli_from<I, T>(iter: I) -> ProfileCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    ProfileCliArgs::parse_from(iter)
}
