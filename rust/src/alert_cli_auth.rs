use crate::common::Result;
use crate::grafana_api::{AuthInputs, GrafanaConnection};
use crate::profile_config::ConnectionMergeInput;

use super::super::DEFAULT_TIMEOUT;
use super::alert_cli_args::AlertCliArgs;

/// Struct definition for AlertAuthContext.
#[derive(Debug, Clone)]
pub struct AlertAuthContext {
    pub url: String,
    pub timeout: u64,
    pub verify_ssl: bool,
    pub headers: Vec<(String, String)>,
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn build_auth_context(args: &AlertCliArgs) -> Result<AlertAuthContext> {
    let connection = GrafanaConnection::resolve(
        args.profile.as_deref(),
        ConnectionMergeInput {
            url: &args.url,
            url_default: "",
            api_token: args.api_token.as_deref(),
            username: args.username.as_deref(),
            password: args.password.as_deref(),
            org_id: args.org_id,
            timeout: args.timeout,
            timeout_default: DEFAULT_TIMEOUT,
            verify_ssl: args.verify_ssl,
            insecure: false,
            ca_cert: None,
        },
        AuthInputs {
            api_token: args.api_token.as_deref(),
            username: args.username.as_deref(),
            password: args.password.as_deref(),
            prompt_password: args.prompt_password,
            prompt_token: args.prompt_token,
        },
        false,
    )?;
    Ok(AlertAuthContext {
        url: connection.base_url,
        timeout: connection.timeout_secs,
        verify_ssl: connection.verify_ssl,
        headers: connection.headers,
    })
}
