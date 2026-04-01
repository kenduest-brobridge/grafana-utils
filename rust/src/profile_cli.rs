//! Profile namespace CLI for repo-local grafana-util configuration.
//!
//! Owns the first usable `grafana-util profile` surface for listing, showing,
//! and initializing `grafana-util.yaml`.
use clap::{Args, CommandFactory, Parser, Subcommand};
use serde::Serialize;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, validation, Result};
use crate::dashboard::SimpleOutputFormat;
use crate::profile_config::{
    default_profile_config_path, load_profile_config_file, render_profile_init_template,
    resolve_profile_config_path, select_profile, ConnectionProfile, ProfileConfigFile,
    SelectedProfile,
};
use crate::tabular_output::{render_summary_csv, render_summary_table, render_yaml};

const PROFILE_HELP_TEXT: &str = "Examples:\n\n  grafana-util profile list\n  grafana-util profile show --profile prod --output-format yaml\n  grafana-util profile init --overwrite";

#[derive(Debug, Clone, Parser)]
#[command(
    name = "grafana-util profile",
    about = "List, inspect, and initialize repo-local grafana-util profiles.",
    after_help = PROFILE_HELP_TEXT,
    styles = crate::help_styles::CLI_HELP_STYLES
)]
pub struct ProfileCliArgs {
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
        about = "Initialize grafana-util.yaml in the current working directory.",
        after_help = "Creates grafana-util.yaml from the built-in profile template and refuses to overwrite it unless --overwrite is set."
    )]
    Init(ProfileInitArgs),
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
        value_enum,
        default_value_t = SimpleOutputFormat::Text,
        help = "Render the selected profile as text, table, csv, json, or yaml."
    )]
    pub output_format: SimpleOutputFormat,
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

#[derive(Debug, Clone, Serialize)]
struct ProfileShowDocument {
    name: String,
    source_path: PathBuf,
    profile: ConnectionProfile,
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

fn load_profile_config_at_resolved_path() -> Result<(PathBuf, ProfileConfigFile)> {
    let path = resolve_profile_config_path();
    if !path.exists() {
        return Err(validation(format!(
            "Profile config file {} does not exist. Run `grafana-util profile init` to create one.",
            path.display()
        )));
    }
    Ok((path.clone(), load_profile_config_file(&path)?))
}

fn select_profile_or_error(
    config: &ProfileConfigFile,
    requested_profile: Option<&str>,
    source_path: &Path,
) -> Result<SelectedProfile> {
    select_profile(config, requested_profile, source_path)?.ok_or_else(|| {
        validation(format!(
            "No profile could be selected from {}. Add default_profile or pass --profile NAME.",
            source_path.display()
        ))
    })
}

fn render_profile_text(selected: &SelectedProfile) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "name: {}", selected.name);
    let _ = writeln!(output, "source_path: {}", selected.source_path.display());
    append_profile_fields_text(&mut output, &selected.profile);
    output.trim_end().to_string()
}

fn render_profile_summary_rows(selected: &SelectedProfile) -> Vec<(&'static str, String)> {
    let profile = &selected.profile;
    let mut rows = vec![
        ("name", selected.name.clone()),
        ("source_path", selected.source_path.display().to_string()),
    ];
    if let Some(value) = profile.url.as_deref() {
        rows.push(("url", value.to_string()));
    }
    if let Some(value) = profile.token.as_deref() {
        rows.push(("token", value.to_string()));
    }
    if let Some(value) = profile.token_env.as_deref() {
        rows.push(("token_env", value.to_string()));
    }
    if let Some(value) = profile.username.as_deref() {
        rows.push(("username", value.to_string()));
    }
    if let Some(value) = profile.username_env.as_deref() {
        rows.push(("username_env", value.to_string()));
    }
    if let Some(value) = profile.password.as_deref() {
        rows.push(("password", value.to_string()));
    }
    if let Some(value) = profile.password_env.as_deref() {
        rows.push(("password_env", value.to_string()));
    }
    if let Some(value) = profile.org_id {
        rows.push(("org_id", value.to_string()));
    }
    if let Some(value) = profile.timeout {
        rows.push(("timeout", value.to_string()));
    }
    if let Some(value) = profile.verify_ssl {
        rows.push(("verify_ssl", value.to_string()));
    }
    if let Some(value) = profile.insecure {
        rows.push(("insecure", value.to_string()));
    }
    if let Some(value) = profile.ca_cert.as_ref() {
        rows.push(("ca_cert", value.display().to_string()));
    }
    rows
}

fn append_profile_fields_text(output: &mut String, profile: &ConnectionProfile) {
    if let Some(value) = profile.url.as_deref() {
        let _ = writeln!(output, "url: {value}");
    }
    if let Some(value) = profile.token.as_deref() {
        let _ = writeln!(output, "token: {value}");
    }
    if let Some(value) = profile.token_env.as_deref() {
        let _ = writeln!(output, "token_env: {value}");
    }
    if let Some(value) = profile.username.as_deref() {
        let _ = writeln!(output, "username: {value}");
    }
    if let Some(value) = profile.username_env.as_deref() {
        let _ = writeln!(output, "username_env: {value}");
    }
    if let Some(value) = profile.password.as_deref() {
        let _ = writeln!(output, "password: {value}");
    }
    if let Some(value) = profile.password_env.as_deref() {
        let _ = writeln!(output, "password_env: {value}");
    }
    if let Some(value) = profile.org_id {
        let _ = writeln!(output, "org_id: {value}");
    }
    if let Some(value) = profile.timeout {
        let _ = writeln!(output, "timeout: {value}");
    }
    if let Some(value) = profile.verify_ssl {
        let _ = writeln!(output, "verify_ssl: {value}");
    }
    if let Some(value) = profile.insecure {
        let _ = writeln!(output, "insecure: {value}");
    }
    if let Some(value) = profile.ca_cert.as_deref() {
        let _ = writeln!(output, "ca_cert: {}", value.display());
    }
}

fn render_profile_yaml(selected: &SelectedProfile) -> Result<String> {
    Ok(format!(
        "{}\n",
        render_yaml(&ProfileShowDocument {
            name: selected.name.clone(),
            source_path: selected.source_path.clone(),
            profile: selected.profile.clone(),
        })?
    ))
}

fn render_profile_table(selected: &SelectedProfile) -> Vec<String> {
    render_summary_table(&render_profile_summary_rows(selected))
}

fn render_profile_csv(selected: &SelectedProfile) -> Vec<String> {
    render_summary_csv(&render_profile_summary_rows(selected))
}

fn run_profile_list() -> Result<()> {
    let (path, config) = load_profile_config_at_resolved_path()?;
    for name in config.profiles.keys() {
        println!("{name}");
    }
    if config.profiles.is_empty() {
        println!("No profiles found in {}.", path.display());
    }
    Ok(())
}

fn run_profile_show(args: ProfileShowArgs) -> Result<()> {
    let (path, config) = load_profile_config_at_resolved_path()?;
    let selected = select_profile_or_error(&config, args.profile.as_deref(), &path)?;
    match args.output_format {
        SimpleOutputFormat::Text => {
            println!("{}", render_profile_text(&selected));
            Ok(())
        }
        SimpleOutputFormat::Table => {
            for line in render_profile_table(&selected) {
                println!("{line}");
            }
            Ok(())
        }
        SimpleOutputFormat::Csv => {
            for line in render_profile_csv(&selected) {
                println!("{line}");
            }
            Ok(())
        }
        SimpleOutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&ProfileShowDocument {
                    name: selected.name.clone(),
                    source_path: selected.source_path.clone(),
                    profile: selected.profile.clone(),
                })?
            );
            Ok(())
        }
        SimpleOutputFormat::Yaml => {
            println!("{}", render_profile_yaml(&selected)?);
            Ok(())
        }
    }
}

fn run_profile_init(args: ProfileInitArgs) -> Result<()> {
    let path = std::env::current_dir()
        .map_err(|error| message(format!("Failed to resolve current directory: {error}")))?
        .join(default_profile_config_path());
    if path.exists() && !args.overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing file: {}. Use --overwrite.",
            path.display()
        )));
    }
    fs::write(&path, render_profile_init_template()).map_err(|error| {
        message(format!(
            "Failed to write grafana-util profile config {}: {error}",
            path.display()
        ))
    })?;
    println!("Wrote {}.", path.display());
    Ok(())
}

pub fn run_profile_cli(args: ProfileCliArgs) -> Result<()> {
    match args.command {
        ProfileCommand::List(_) => run_profile_list(),
        ProfileCommand::Show(show_args) => run_profile_show(show_args),
        ProfileCommand::Init(init_args) => run_profile_init(init_args),
    }
}
