use std::path::PathBuf;

use crate::common::{resolve_auth_headers, Result};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};
use crate::profile_config::{
    load_selected_profile, resolve_connection_settings, ConnectionMergeInput,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct AuthInputs<'a> {
    pub api_token: Option<&'a str>,
    pub username: Option<&'a str>,
    pub password: Option<&'a str>,
    pub prompt_password: bool,
    pub prompt_token: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GrafanaConnection {
    pub base_url: String,
    pub headers: Vec<(String, String)>,
    pub timeout_secs: u64,
    pub verify_ssl: bool,
    pub ca_cert: Option<PathBuf>,
    pub auth_mode: String,
}

impl GrafanaConnection {
    pub(crate) fn new(
        base_url: String,
        headers: Vec<(String, String)>,
        timeout_secs: u64,
        verify_ssl: bool,
        ca_cert: Option<PathBuf>,
        auth_mode: String,
    ) -> Self {
        Self {
            base_url,
            headers,
            timeout_secs,
            verify_ssl,
            ca_cert,
            auth_mode,
        }
    }

    pub(crate) fn resolve(
        profile_name: Option<&str>,
        merge_input: ConnectionMergeInput<'_>,
        auth_inputs: AuthInputs<'_>,
        include_resolved_org_header: bool,
    ) -> Result<Self> {
        let selected_profile = load_selected_profile(profile_name)?;
        let resolved = resolve_connection_settings(merge_input, selected_profile.as_ref())?;

        let token = if auth_inputs.prompt_token && auth_inputs.api_token.is_none() {
            None
        } else {
            resolved.api_token.as_deref()
        };
        let username = if auth_inputs.prompt_password {
            auth_inputs.username.or(resolved.username.as_deref())
        } else {
            resolved.username.as_deref()
        };
        let password = if auth_inputs.prompt_password && auth_inputs.password.is_none() {
            None
        } else {
            resolved.password.as_deref()
        };

        let mut headers = resolve_auth_headers(
            token,
            username,
            password,
            auth_inputs.prompt_password,
            auth_inputs.prompt_token,
        )?;
        if include_resolved_org_header {
            if let Some(org_id) = resolved.org_id {
                upsert_org_header(&mut headers, org_id);
            }
        }

        Ok(Self::new(
            resolved.url,
            headers.clone(),
            resolved.timeout,
            resolved.verify_ssl,
            resolved.ca_cert,
            auth_mode_from_headers(&headers),
        ))
    }

    pub(crate) fn with_org_id(&self, org_id: i64) -> Self {
        let mut scoped = self.clone();
        upsert_org_header(&mut scoped.headers, org_id);
        scoped
    }

    pub(crate) fn build_http_client(&self) -> Result<JsonHttpClient> {
        JsonHttpClient::new_with_ca_cert(
            JsonHttpClientConfig {
                base_url: self.base_url.clone(),
                headers: self.headers.clone(),
                timeout_secs: self.timeout_secs,
                verify_ssl: self.verify_ssl,
            },
            self.ca_cert.as_deref(),
        )
    }
}

pub(crate) fn auth_mode_from_headers(headers: &[(String, String)]) -> String {
    headers
        .iter()
        .find(|(name, _)| name == "Authorization")
        .map(|(_, value)| {
            if value.starts_with("Basic ") {
                "basic".to_string()
            } else {
                "token".to_string()
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn upsert_org_header(headers: &mut Vec<(String, String)>, org_id: i64) {
    headers.retain(|(name, _)| !name.eq_ignore_ascii_case("X-Grafana-Org-Id"));
    headers.push(("X-Grafana-Org-Id".to_string(), org_id.to_string()));
}
