//! Shared project-status helpers used by live status workflows.
//!
//! Responsibilities:
//! - Build authenticated HTTP clients and auth header sets for status checks.
//! - Resolve per-org connection settings and default behavior for live runs.

use crate::common::{resolve_auth_headers, Result as CommonResult};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};
use crate::profile_config::{
    load_selected_profile, resolve_connection_settings, ConnectionMergeInput,
    ResolvedConnectionSettings,
};
use crate::project_status_command::ProjectStatusLiveArgs;

fn resolve_live_project_status_settings(
    args: &ProjectStatusLiveArgs,
    org_id: Option<i64>,
) -> CommonResult<ResolvedConnectionSettings> {
    let selected_profile = load_selected_profile(args.profile.as_deref())?;
    resolve_connection_settings(
        ConnectionMergeInput {
            url: &args.url,
            url_default: "http://localhost:3000",
            api_token: args.api_token.as_deref(),
            username: args.username.as_deref(),
            password: args.password.as_deref(),
            org_id,
            timeout: args.timeout,
            timeout_default: 30,
            verify_ssl: args.verify_ssl,
            insecure: args.insecure,
            ca_cert: args.ca_cert.as_deref(),
        },
        selected_profile.as_ref(),
    )
}

fn resolve_live_project_status_headers_from_resolved(
    args: &ProjectStatusLiveArgs,
    resolved: &ResolvedConnectionSettings,
) -> CommonResult<Vec<(String, String)>> {
    let token = if args.prompt_token && args.api_token.is_none() {
        None
    } else {
        resolved.api_token.as_deref()
    };
    let username = if args.prompt_password {
        args.username.as_deref().or(resolved.username.as_deref())
    } else {
        resolved.username.as_deref()
    };
    let password = if args.prompt_password && args.password.is_none() {
        None
    } else {
        resolved.password.as_deref()
    };
    let mut headers = resolve_auth_headers(
        token,
        username,
        password,
        args.prompt_password,
        args.prompt_token,
    )?;
    if let Some(org_id) = resolved.org_id {
        headers.push(("X-Grafana-Org-Id".to_string(), org_id.to_string()));
    }
    Ok(headers)
}

#[cfg(test)]
pub(crate) fn resolve_live_project_status_headers(
    args: &ProjectStatusLiveArgs,
    org_id: Option<i64>,
) -> CommonResult<Vec<(String, String)>> {
    let resolved = resolve_live_project_status_settings(args, org_id)?;
    resolve_live_project_status_headers_from_resolved(args, &resolved)
}

pub(crate) fn build_live_project_status_client(
    args: &ProjectStatusLiveArgs,
) -> CommonResult<JsonHttpClient> {
    build_live_project_status_client_for_org(args, if args.all_orgs { None } else { args.org_id })
}

pub(crate) fn build_live_project_status_client_for_org(
    args: &ProjectStatusLiveArgs,
    org_id: Option<i64>,
) -> CommonResult<JsonHttpClient> {
    let resolved = resolve_live_project_status_settings(args, org_id)?;
    let headers = resolve_live_project_status_headers_from_resolved(args, &resolved)?;
    JsonHttpClient::new_with_ca_cert(
        JsonHttpClientConfig {
            base_url: resolved.url,
            headers,
            timeout_secs: resolved.timeout,
            verify_ssl: resolved.verify_ssl,
        },
        resolved.ca_cert.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::resolve_live_project_status_headers;
    use crate::project_status_command::{ProjectStatusLiveArgs, ProjectStatusOutputFormat};

    #[test]
    fn resolve_live_project_status_headers_adds_org_scope_when_requested() {
        let args = ProjectStatusLiveArgs {
            profile: None,
            url: "http://localhost:3000".to_string(),
            api_token: Some("token-123".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
            insecure: false,
            ca_cert: None,
            all_orgs: false,
            org_id: Some(7),
            sync_summary_file: None,
            bundle_preflight_file: None,
            promotion_summary_file: None,
            mapping_file: None,
            availability_file: None,
            output_format: ProjectStatusOutputFormat::Text,
        };

        let headers = resolve_live_project_status_headers(&args, args.org_id).unwrap();

        assert!(headers
            .iter()
            .any(|(name, value)| { name == "X-Grafana-Org-Id" && value == "7" }));
    }
}
