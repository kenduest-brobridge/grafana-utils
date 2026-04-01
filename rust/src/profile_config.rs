//! Profile configuration loading, merge, and persistence contracts.
//!
//! Responsibilities:
//! - Resolve active profile selection from CLI/profile/env inputs.
//! - Merge profile data with inline arguments and defaults.
//! - Read/write profile configuration files used by Rust/CLI entrypoints.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{env_value, validation, Result};

pub const DEFAULT_PROFILE_CONFIG_FILENAME: &str = "grafana-util.yaml";
pub const PROFILE_CONFIG_ENV_VAR: &str = "GRAFANA_UTIL_CONFIG";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileConfigFile {
    #[serde(default)]
    pub default_profile: Option<String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, ConnectionProfile>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionProfile {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub token_env: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub username_env: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub password_env: Option<String>,
    #[serde(default)]
    pub org_id: Option<i64>,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub verify_ssl: Option<bool>,
    #[serde(default)]
    pub insecure: Option<bool>,
    #[serde(default)]
    pub ca_cert: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedProfile {
    pub name: String,
    pub source_path: PathBuf,
    pub profile: ConnectionProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConnectionSettings {
    pub url: String,
    pub api_token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub org_id: Option<i64>,
    pub timeout: u64,
    pub verify_ssl: bool,
    pub ca_cert: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy)]
pub struct ConnectionMergeInput<'a> {
    pub url: &'a str,
    pub url_default: &'a str,
    pub api_token: Option<&'a str>,
    pub username: Option<&'a str>,
    pub password: Option<&'a str>,
    pub org_id: Option<i64>,
    pub timeout: u64,
    pub timeout_default: u64,
    pub verify_ssl: bool,
    pub insecure: bool,
    pub ca_cert: Option<&'a Path>,
}

pub fn default_profile_config_path() -> PathBuf {
    PathBuf::from(DEFAULT_PROFILE_CONFIG_FILENAME)
}

pub fn resolve_profile_config_path() -> PathBuf {
    env_value(PROFILE_CONFIG_ENV_VAR)
        .map(PathBuf::from)
        .unwrap_or_else(default_profile_config_path)
}

pub fn load_profile_config_file(path: &Path) -> Result<ProfileConfigFile> {
    let raw = fs::read_to_string(path)?;
    serde_yaml::from_str::<ProfileConfigFile>(&raw).map_err(|error| {
        validation(format!(
            "Failed to parse grafana-util profile config {}: {error}",
            path.display()
        ))
    })
}

pub fn load_selected_profile(profile_name: Option<&str>) -> Result<Option<SelectedProfile>> {
    let path = resolve_profile_config_path();
    if !path.exists() {
        if let Some(name) = profile_name {
            return Err(validation(format!(
                "Requested profile `{name}` but config file {} does not exist.",
                path.display()
            )));
        }
        return Ok(None);
    }
    let config = load_profile_config_file(&path)?;
    select_profile(&config, profile_name, &path)
}

pub fn select_profile(
    config: &ProfileConfigFile,
    requested_profile: Option<&str>,
    source_path: &Path,
) -> Result<Option<SelectedProfile>> {
    let chosen_name = if let Some(name) = requested_profile {
        Some(name.to_string())
    } else if let Some(default_name) = config.default_profile.as_ref() {
        Some(default_name.clone())
    } else if config.profiles.len() == 1 {
        config.profiles.keys().next().cloned()
    } else {
        None
    };
    let Some(name) = chosen_name else {
        return Ok(None);
    };
    let Some(profile) = config.profiles.get(&name) else {
        return Err(validation(format!(
            "Profile `{name}` was not found in {}.",
            source_path.display()
        )));
    };
    Ok(Some(SelectedProfile {
        name,
        source_path: source_path.to_path_buf(),
        profile: profile.clone(),
    }))
}

pub fn render_profile_init_template() -> String {
    r#"default_profile: dev
profiles:
  dev:
    url: http://127.0.0.1:3000
    token_env: GRAFANA_API_TOKEN
    timeout: 30
    verify_ssl: false

  prod:
    url: https://grafana.example.com
    basic_user: admin
    password_env: GRAFANA_PROD_PASSWORD
    verify_ssl: true
"#
    .replace("basic_user", "username")
}

pub fn resolve_connection_settings(
    input: ConnectionMergeInput<'_>,
    selected_profile: Option<&SelectedProfile>,
) -> Result<ResolvedConnectionSettings> {
    let profile = selected_profile.map(|selected| &selected.profile);
    let url = if input.url != input.url_default {
        input.url.to_string()
    } else {
        profile
            .and_then(|item| item.url.clone())
            .unwrap_or_else(|| input.url.to_string())
    };
    let api_token = resolve_credential_value(
        input.api_token,
        profile.and_then(|item| item.token.as_deref()),
        profile.and_then(|item| item.token_env.as_deref()),
        selected_profile,
        "token",
    )?;
    let username = resolve_credential_value(
        input.username,
        profile.and_then(|item| item.username.as_deref()),
        profile.and_then(|item| item.username_env.as_deref()),
        selected_profile,
        "username",
    )?;
    let password = resolve_credential_value(
        input.password,
        profile.and_then(|item| item.password.as_deref()),
        profile.and_then(|item| item.password_env.as_deref()),
        selected_profile,
        "password",
    )?;
    let timeout = if input.timeout != input.timeout_default {
        input.timeout
    } else {
        profile
            .and_then(|item| item.timeout)
            .unwrap_or(input.timeout_default)
    };
    let org_id = input
        .org_id
        .or_else(|| profile.and_then(|item| item.org_id));
    let ca_cert = input
        .ca_cert
        .map(Path::to_path_buf)
        .or_else(|| profile.and_then(|item| item.ca_cert.clone()));
    let verify_ssl = resolve_verify_ssl(input, selected_profile, profile, ca_cert.is_some())?;

    Ok(ResolvedConnectionSettings {
        url,
        api_token,
        username,
        password,
        org_id,
        timeout,
        verify_ssl,
        ca_cert,
    })
}

fn resolve_verify_ssl(
    input: ConnectionMergeInput<'_>,
    selected_profile: Option<&SelectedProfile>,
    profile: Option<&ConnectionProfile>,
    ca_cert_present: bool,
) -> Result<bool> {
    if input.insecure && input.verify_ssl {
        return Err(validation(
            "Choose either --insecure or --verify-ssl, not both.",
        ));
    }
    if let Some(profile) = profile {
        if profile.insecure == Some(true) && profile.verify_ssl == Some(true) {
            let profile_name = selected_profile
                .map(|item| item.name.as_str())
                .unwrap_or("default");
            let source_path = selected_profile
                .map(|item| item.source_path.display().to_string())
                .unwrap_or_else(|| DEFAULT_PROFILE_CONFIG_FILENAME.to_string());
            return Err(validation(format!(
                "Profile `{}` in {} cannot set both verify_ssl: true and insecure: true.",
                profile_name, source_path
            )));
        }
    }
    if input.insecure {
        return Ok(false);
    }
    if input.verify_ssl || input.ca_cert.is_some() {
        return Ok(true);
    }
    if let Some(profile) = profile {
        if profile.insecure == Some(true) {
            return Ok(false);
        }
        if profile.verify_ssl == Some(true) || profile.ca_cert.is_some() {
            return Ok(true);
        }
        if let Some(value) = profile.verify_ssl {
            return Ok(value);
        }
    }
    Ok(ca_cert_present)
}

fn resolve_credential_value(
    cli_value: Option<&str>,
    profile_literal: Option<&str>,
    profile_env: Option<&str>,
    selected_profile: Option<&SelectedProfile>,
    field_name: &str,
) -> Result<Option<String>> {
    if let Some(value) = cli_value.filter(|value| !value.is_empty()) {
        return Ok(Some(value.to_string()));
    }
    if let Some(value) = profile_literal.filter(|value| !value.is_empty()) {
        return Ok(Some(value.to_string()));
    }
    let Some(env_name) = profile_env.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let value = env_value(env_name).ok_or_else(|| {
        let profile_name = selected_profile
            .map(|profile| profile.name.as_str())
            .unwrap_or("default");
        validation(format!(
            "Profile `{profile_name}` expected env var `{env_name}` for {field_name}, but it is not set."
        ))
    })?;
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::{
        default_profile_config_path, load_profile_config_file, render_profile_init_template,
        resolve_connection_settings, select_profile, ConnectionMergeInput, ConnectionProfile,
        ProfileConfigFile,
    };
    use std::collections::BTreeMap;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[test]
    fn default_profile_config_path_uses_repo_local_filename() {
        assert_eq!(
            default_profile_config_path().to_string_lossy(),
            "grafana-util.yaml"
        );
    }

    #[test]
    fn select_profile_prefers_requested_name_then_default_then_single_profile() {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "dev".to_string(),
            ConnectionProfile {
                url: Some("http://dev".to_string()),
                ..ConnectionProfile::default()
            },
        );
        profiles.insert(
            "prod".to_string(),
            ConnectionProfile {
                url: Some("http://prod".to_string()),
                ..ConnectionProfile::default()
            },
        );
        let config = ProfileConfigFile {
            default_profile: Some("prod".to_string()),
            profiles,
        };

        let selected = select_profile(&config, Some("dev"), Path::new("./grafana-util.yaml"))
            .unwrap()
            .unwrap();
        assert_eq!(selected.name, "dev");

        let selected = select_profile(&config, None, Path::new("./grafana-util.yaml"))
            .unwrap()
            .unwrap();
        assert_eq!(selected.name, "prod");
    }

    #[test]
    fn resolve_connection_settings_prefers_cli_and_falls_back_to_profile() {
        let selected_profile = super::SelectedProfile {
            name: "prod".to_string(),
            source_path: PathBuf::from("./grafana-util.yaml"),
            profile: ConnectionProfile {
                url: Some("https://grafana.example.com".to_string()),
                token: Some("profile-token".to_string()),
                org_id: Some(9),
                timeout: Some(45),
                verify_ssl: Some(true),
                ..ConnectionProfile::default()
            },
        };
        let resolved = resolve_connection_settings(
            ConnectionMergeInput {
                url: "http://127.0.0.1:3000",
                url_default: "http://127.0.0.1:3000",
                api_token: None,
                username: None,
                password: None,
                org_id: None,
                timeout: 30,
                timeout_default: 30,
                verify_ssl: false,
                insecure: false,
                ca_cert: None,
            },
            Some(&selected_profile),
        )
        .unwrap();

        assert_eq!(resolved.url, "https://grafana.example.com");
        assert_eq!(resolved.api_token.as_deref(), Some("profile-token"));
        assert_eq!(resolved.org_id, Some(9));
        assert_eq!(resolved.timeout, 45);
        assert!(resolved.verify_ssl);
    }

    #[test]
    fn resolve_connection_settings_supports_profile_env_credentials() {
        let selected_profile = super::SelectedProfile {
            name: "prod".to_string(),
            source_path: PathBuf::from("./grafana-util.yaml"),
            profile: ConnectionProfile {
                token_env: Some("TEST_GRAFANA_PROFILE_TOKEN".to_string()),
                ..ConnectionProfile::default()
            },
        };
        env::set_var("TEST_GRAFANA_PROFILE_TOKEN", "token-from-env");
        let resolved = resolve_connection_settings(
            ConnectionMergeInput {
                url: "http://127.0.0.1:3000",
                url_default: "http://127.0.0.1:3000",
                api_token: None,
                username: None,
                password: None,
                org_id: None,
                timeout: 30,
                timeout_default: 30,
                verify_ssl: false,
                insecure: false,
                ca_cert: None,
            },
            Some(&selected_profile),
        )
        .unwrap();
        env::remove_var("TEST_GRAFANA_PROFILE_TOKEN");

        assert_eq!(resolved.api_token.as_deref(), Some("token-from-env"));
    }

    #[test]
    fn load_profile_config_file_reads_yaml_document() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("grafana-util.yaml");
        fs::write(
            &config_path,
            r#"default_profile: dev
profiles:
  dev:
    url: http://localhost:3000
    token_env: TEST_PROFILE_TOKEN
"#,
        )
        .unwrap();

        let config = load_profile_config_file(&config_path).unwrap();

        assert_eq!(config.default_profile.as_deref(), Some("dev"));
        assert_eq!(
            config.profiles["dev"].url.as_deref(),
            Some("http://localhost:3000")
        );
    }

    #[test]
    fn render_profile_init_template_contains_default_profiles() {
        let rendered = render_profile_init_template();

        assert!(rendered.contains("default_profile: dev"));
        assert!(rendered.contains("profiles:"));
        assert!(rendered.contains("token_env: GRAFANA_API_TOKEN"));
        assert!(rendered.contains("username: admin"));
    }
}
