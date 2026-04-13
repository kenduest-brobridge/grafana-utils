//! Shared Access helpers for internal state transitions and reusable orchestration logic.

use clap::{Args, Subcommand};
use dialoguer::{
    console::{style, Style},
    theme::ColorfulTheme,
    Confirm, MultiSelect, Select,
};
use std::io::{self, IsTerminal};

#[cfg(test)]
use crate::common::render_json_value;
use crate::common::{message, Result};
#[cfg(test)]
use serde_json::{Map, Value};

use super::super::{CommonCliArgs, TeamAddArgs, TeamListArgs, TeamModifyArgs};

/// CLI arguments for team delete.
#[derive(Debug, Clone, Args)]
pub struct TeamDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long, conflicts_with = "name")]
    pub team_id: Option<String>,
    #[arg(long, conflicts_with = "team_id")]
    pub name: Option<String>,
    #[arg(long, default_value_t = false)]
    pub prompt: bool,
    #[arg(long, default_value_t = false)]
    pub yes: bool,
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

/// CLI arguments for service-account delete.
#[derive(Debug, Clone, Args)]
pub struct ServiceAccountDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long = "service-account-id", conflicts_with = "name")]
    pub service_account_id: Option<String>,
    #[arg(long, conflicts_with = "service_account_id")]
    pub name: Option<String>,
    #[arg(long, default_value_t = false)]
    pub prompt: bool,
    #[arg(long, default_value_t = false)]
    pub yes: bool,
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

/// CLI arguments for service-account token delete.
#[derive(Debug, Clone, Args)]
pub struct ServiceAccountTokenDeleteArgs {
    #[command(flatten)]
    pub common: CommonCliArgs,
    #[arg(long = "service-account-id", conflicts_with = "name")]
    pub service_account_id: Option<String>,
    #[arg(long, conflicts_with = "service_account_id")]
    pub name: Option<String>,
    #[arg(long = "token-id", conflicts_with = "token_name")]
    pub token_id: Option<String>,
    #[arg(long = "token-name", conflicts_with = "token_id")]
    pub token_name: Option<String>,
    #[arg(long, default_value_t = false)]
    pub prompt: bool,
    #[arg(long, default_value_t = false)]
    pub yes: bool,
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

/// Parser grouping for the team command surface.
#[derive(Debug, Clone, Subcommand)]
pub enum GroupCommandStage {
    List(TeamListArgs),
    Add(TeamAddArgs),
    Modify(TeamModifyArgs),
    Delete(TeamDeleteArgs),
}

/// Ensure a destructive command only proceeds with explicit confirmation.
pub(crate) fn validate_confirmation(yes: bool, noun: &str) -> Result<()> {
    if yes {
        Ok(())
    } else {
        Err(message(format!("{noun} delete requires --yes.")))
    }
}

pub(crate) fn validate_delete_prompt(prompt: bool, json: bool, noun: &str) -> Result<()> {
    if prompt && json {
        return Err(message(format!(
            "{noun} delete --prompt cannot be combined with --json."
        )));
    }
    if prompt && (!io::stdin().is_terminal() || !io::stdout().is_terminal()) {
        return Err(message(format!("{noun} delete --prompt requires a TTY.")));
    }
    Ok(())
}

pub(crate) fn prompt_select_index(prompt: &str, labels: &[String]) -> Result<Option<usize>> {
    if labels.is_empty() {
        return Ok(None);
    }
    print_prompt_step(prompt, "Use ↑/↓ to move, Enter to choose, Esc to cancel.");
    Select::with_theme(&delete_prompt_theme())
        .with_prompt("Select")
        .items(labels)
        .default(0)
        .interact_opt()
        .map_err(|error| message(format!("Delete prompt failed: {error}")))
}

pub(crate) fn prompt_select_indexes(prompt: &str, labels: &[String]) -> Result<Option<Vec<usize>>> {
    if labels.is_empty() {
        return Ok(None);
    }
    print_prompt_step(
        prompt,
        "Use ↑/↓ to move, Space to toggle, Enter to continue, Esc to cancel.",
    );
    let selections = MultiSelect::with_theme(&delete_prompt_theme())
        .with_prompt("Select")
        .items(labels)
        .interact_opt()
        .map_err(|error| message(format!("Delete prompt failed: {error}")))?;
    let Some(indexes) = selections else {
        return Ok(None);
    };
    if indexes.is_empty() {
        return Err(message("Select at least one item before deleting."));
    }
    Ok(Some(indexes))
}

pub(crate) fn print_delete_confirmation_summary(title: &str, labels: &[String]) {
    println!();
    println!("{title}");
    for label in labels {
        println!("  - {label}");
    }
    println!();
}

pub(crate) fn prompt_confirm_delete(prompt: &str) -> Result<bool> {
    print_prompt_step(prompt, "Review the list above before confirming.");
    Confirm::with_theme(&delete_prompt_theme())
        .with_prompt("Confirm")
        .default(false)
        .interact_opt()
        .map(|choice| choice.unwrap_or(false))
        .map_err(|error| message(format!("Delete confirmation prompt failed: {error}")))
}

fn print_prompt_step(title: &str, hint: &str) {
    println!();
    println!("{title}");
    println!("  {hint}");
    println!();
}

pub(crate) fn truncate_prompt_text(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() <= width {
        return text.to_string();
    }
    if width <= 3 {
        return ".".repeat(width);
    }
    format!(
        "{}...",
        chars[..width.saturating_sub(3)].iter().collect::<String>()
    )
}

pub(crate) fn format_prompt_column(text: &str, width: usize) -> String {
    format!(
        "{:<width$}",
        truncate_prompt_text(text, width),
        width = width
    )
}

pub(crate) fn format_prompt_row(columns: &[(&str, usize)], trailer: &str) -> String {
    let body = columns
        .iter()
        .map(|(value, width)| format_prompt_column(value, *width))
        .collect::<Vec<_>>()
        .join("  ");
    if trailer.is_empty() {
        body
    } else {
        format!("{body}  {trailer}")
    }
}

fn delete_prompt_theme() -> ColorfulTheme {
    ColorfulTheme {
        prompt_style: Style::new().for_stderr().bold().yellow(),
        active_item_style: Style::new().for_stderr().cyan().bold(),
        inactive_item_style: Style::new().for_stderr(),
        active_item_prefix: style(">".to_string()).for_stderr().cyan().bold(),
        inactive_item_prefix: style(" ".to_string()).for_stderr(),
        checked_item_prefix: style("[x]".to_string()).for_stderr().green().bold(),
        unchecked_item_prefix: style("[ ]".to_string()).for_stderr().white().dim(),
        ..ColorfulTheme::default()
    }
}

/// Render a JSON object as pretty-printed output.
#[cfg(test)]
pub(crate) fn render_single_object_json(object: &Map<String, Value>) -> Result<String> {
    render_json_value(&Value::Object(object.clone()))
}

/// Validate one and only one identity selector was provided.
pub(crate) fn validate_exactly_one_identity(
    id_present: bool,
    name_present: bool,
    noun: &str,
    id_flag: &str,
) -> Result<()> {
    match (id_present, name_present) {
        (true, false) | (false, true) => Ok(()),
        (false, false) => Err(message(format!(
            "{noun} delete requires one of {id_flag} or --name."
        ))),
        (true, true) => Err(message(format!(
            "{noun} delete accepts either {id_flag} or --name, not both."
        ))),
    }
}

/// Validate service-account token delete identity and token selection constraints.
pub(crate) fn validate_token_identity(args: &ServiceAccountTokenDeleteArgs) -> Result<()> {
    validate_exactly_one_identity(
        args.service_account_id.is_some(),
        args.name.is_some(),
        "Service-account token",
        "--service-account-id",
    )?;
    match (args.token_id.is_some(), args.token_name.is_some()) {
        (true, false) | (false, true) => Ok(()),
        (false, false) => Err(message(
            "Service-account token delete requires one of --token-id or --token-name.",
        )),
        (true, true) => Err(message(
            "Service-account token delete accepts either --token-id or --token-name, not both.",
        )),
    }
}

#[cfg(test)]
mod pending_delete_support_tests {
    use super::*;
    use crate::access::cli_defs::{DEFAULT_TIMEOUT, DEFAULT_URL};

    fn common_args() -> CommonCliArgs {
        CommonCliArgs {
            profile: None,
            url: DEFAULT_URL.to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            org_id: None,
            timeout: DEFAULT_TIMEOUT,
            verify_ssl: false,
            insecure: false,
            ca_cert: None,
        }
    }

    #[test]
    fn validate_confirmation_requires_yes() {
        let error = validate_confirmation(false, "Team").unwrap_err();
        assert!(error.to_string().contains("Team delete requires --yes."));
    }

    #[test]
    fn validate_delete_prompt_rejects_json_combo() {
        let error = validate_delete_prompt(true, true, "Team").unwrap_err();
        assert!(error
            .to_string()
            .contains("Team delete --prompt cannot be combined with --json."));
    }

    #[test]
    fn validate_exactly_one_identity_rejects_missing_and_both() {
        assert!(
            validate_exactly_one_identity(false, false, "Team", "--team-id")
                .unwrap_err()
                .to_string()
                .contains("requires one of --team-id or --name")
        );
        assert!(
            validate_exactly_one_identity(true, true, "Team", "--team-id")
                .unwrap_err()
                .to_string()
                .contains("accepts either --team-id or --name, not both")
        );
    }

    #[test]
    fn validate_token_identity_requires_selector() {
        let error = validate_token_identity(&ServiceAccountTokenDeleteArgs {
            common: common_args(),
            service_account_id: Some("4".to_string()),
            name: None,
            token_id: None,
            token_name: None,
            prompt: false,
            yes: true,
            json: false,
        })
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("Service-account token delete requires one of --token-id or --token-name."));
    }

    #[test]
    fn render_single_object_json_returns_object_payload() {
        let payload = Map::from_iter(vec![
            (
                "serviceAccountId".to_string(),
                Value::String("4".to_string()),
            ),
            ("message".to_string(), Value::String("deleted".to_string())),
        ]);
        let rendered = render_single_object_json(&payload).unwrap();
        assert!(rendered.trim_start().starts_with('{'));
        assert!(!rendered.trim_start().starts_with('['));
        assert!(rendered.contains("\"serviceAccountId\": \"4\""));
    }

    #[test]
    fn truncate_prompt_text_adds_ascii_ellipsis() {
        assert_eq!(
            truncate_prompt_text("browse-editor@example.com", 12),
            "browse-ed..."
        );
        assert_eq!(truncate_prompt_text("short", 12), "short");
    }

    #[test]
    fn format_prompt_row_aligns_columns_and_trailer() {
        let row = format_prompt_row(&[("browse-editor", 8), ("editor@example.com", 10)], "id=3");
        assert!(row.contains("brows..."));
        assert!(row.contains("editor@..."));
        assert!(row.ends_with("id=3"));
    }
}
