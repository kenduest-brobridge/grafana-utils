use crate::common::{resolve_auth_headers, Result as CommonResult};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};
use crate::project_status_command::ProjectStatusLiveArgs;

pub(crate) fn resolve_live_project_status_headers(
    args: &ProjectStatusLiveArgs,
    org_id: Option<i64>,
) -> CommonResult<Vec<(String, String)>> {
    let mut headers = resolve_auth_headers(
        args.api_token.as_deref(),
        args.username.as_deref(),
        args.password.as_deref(),
        args.prompt_password,
        args.prompt_token,
    )?;
    if let Some(org_id) = org_id {
        headers.push(("X-Grafana-Org-Id".to_string(), org_id.to_string()));
    }
    Ok(headers)
}

pub(crate) fn build_live_project_status_client(
    args: &ProjectStatusLiveArgs,
) -> CommonResult<JsonHttpClient> {
    build_live_project_status_client_for_org(args, args.org_id)
}

pub(crate) fn build_live_project_status_client_for_org(
    args: &ProjectStatusLiveArgs,
    org_id: Option<i64>,
) -> CommonResult<JsonHttpClient> {
    let headers = resolve_live_project_status_headers(args, org_id)?;
    JsonHttpClient::new_with_ca_cert(
        JsonHttpClientConfig {
            base_url: args.url.clone(),
            headers,
            timeout_secs: args.timeout,
            verify_ssl: args.verify_ssl && !args.insecure,
        },
        args.ca_cert.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::resolve_live_project_status_headers;
    use crate::project_status_command::{ProjectStatusLiveArgs, ProjectStatusOutputFormat};

    #[test]
    fn resolve_live_project_status_headers_adds_org_scope_when_requested() {
        let args = ProjectStatusLiveArgs {
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
            output: ProjectStatusOutputFormat::Text,
        };

        let headers = resolve_live_project_status_headers(&args, args.org_id).unwrap();

        assert!(headers
            .iter()
            .any(|(name, value)| { name == "X-Grafana-Org-Id" && value == "7" }));
    }
}
