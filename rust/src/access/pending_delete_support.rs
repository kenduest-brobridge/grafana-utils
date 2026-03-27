use clap::{Args, Subcommand};
use serde_json::{Map, Value};

use crate::common::{message, Result};

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

/// Render a JSON object as pretty-printed output.
pub(crate) fn render_single_object_json(object: &Map<String, Value>) -> Result<String> {
    serde_json::to_string_pretty(&Value::Object(object.clone())).map_err(Into::into)
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
}
