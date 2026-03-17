//! Shared primitives for CLI execution.
//! Contains canonical error/result types, auth/header resolution rules, and
//! reusable JSON/FS helpers that are consumed by all Rust command domains.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use regex::Regex;
use rpassword::prompt_password;
use serde_json::{Map, Value};
use std::env;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// Enum definition for GrafanaCliError.
#[derive(Debug, Error)]
pub enum GrafanaCliError {
    #[error("{0}")]
    Message(String),
    #[error("HTTP error {status_code} for {url}: {body}")]
    ApiResponse {
        status_code: u16,
        url: String,
        body: String,
    },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),
}

/// Type alias for Result.
pub type Result<T> = std::result::Result<T, GrafanaCliError>;

/// message.
pub fn message(text: impl Into<String>) -> GrafanaCliError {
    GrafanaCliError::Message(text.into())
}

/// api response.
pub fn api_response(
    status_code: u16,
    url: impl Into<String>,
    body: impl Into<String>,
) -> GrafanaCliError {
    GrafanaCliError::ApiResponse {
        status_code,
        url: url.into(),
        body: body.into(),
    }
}

impl GrafanaCliError {
    /// status code.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            GrafanaCliError::ApiResponse { status_code, .. } => Some(*status_code),
            _ => None,
        }
    }
}

/// env value.
pub fn env_value(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

/// Purpose: implementation note.
///
/// Args: see function signature.
/// Returns: see implementation.
pub fn resolve_auth_headers(
    api_token: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
    prompt_for_password: bool,
    prompt_for_token: bool,
) -> Result<Vec<(String, String)>> {
    resolve_auth_headers_with_prompt(
        api_token,
        username,
        password,
        prompt_for_password,
        prompt_for_token,
        || prompt_password("Grafana Basic auth password: ").map_err(GrafanaCliError::from),
        || prompt_password("Grafana API token: ").map_err(GrafanaCliError::from),
    )
}

fn resolve_auth_headers_with_prompt<F, G>(
    api_token: Option<&str>,
    username: Option<&str>,
    password: Option<&str>,
    prompt_for_password: bool,
    prompt_for_token: bool,
    prompt_password_reader: F,
    prompt_token_reader: G,
) -> Result<Vec<(String, String)>>
where
    F: FnOnce() -> Result<String>,
    G: FnOnce() -> Result<String>,
{
    let cli_token = api_token
        .map(str::to_owned)
        .filter(|value| !value.is_empty());
    let cli_username = username
        .map(str::to_owned)
        .filter(|value| !value.is_empty());
    let mut cli_password = password
        .map(str::to_owned)
        .filter(|value| !value.is_empty());

    if cli_token.is_some() && prompt_for_token {
        return Err(message(
            "Choose either --token / --api-token or --prompt-token, not both.",
        ));
    }
    if (cli_token.is_some() || prompt_for_token)
        && (cli_username.is_some() || cli_password.is_some() || prompt_for_password)
    {
        return Err(message(
            "Choose either token auth (--token / --api-token) or Basic auth \
(--basic-user with --basic-password / --prompt-password), not both.",
        ));
    }
    if prompt_for_password && cli_password.is_some() {
        return Err(message(
            "Choose either --basic-password or --prompt-password, not both.",
        ));
    }
    if cli_username.is_some() && cli_password.is_none() && !prompt_for_password {
        return Err(message(
            "Basic auth requires both --basic-user and \
--basic-password or --prompt-password.",
        ));
    }
    if cli_password.is_some() && cli_username.is_none() {
        return Err(message(
            "Basic auth requires both --basic-user and \
--basic-password or --prompt-password.",
        ));
    }
    if prompt_for_password && cli_username.is_none() {
        return Err(message("--prompt-password requires --basic-user."));
    }

    if prompt_for_token {
        let token = prompt_token_reader()?;
        return Ok(vec![(
            "Authorization".to_string(),
            format!("Bearer {token}"),
        )]);
    }

    let token = cli_token.or_else(|| env_value("GRAFANA_API_TOKEN"));
    if let Some(token) = token {
        return Ok(vec![(
            "Authorization".to_string(),
            format!("Bearer {token}"),
        )]);
    }

    if prompt_for_password && cli_username.is_some() {
        cli_password = Some(prompt_password_reader()?);
    }

    let username = cli_username.or_else(|| env_value("GRAFANA_USERNAME"));
    let password = cli_password.or_else(|| env_value("GRAFANA_PASSWORD"));
    if let (Some(username), Some(password)) = (username.as_ref(), password.as_ref()) {
        let encoded = STANDARD.encode(format!("{username}:{password}"));
        return Ok(vec![(
            "Authorization".to_string(),
            format!("Basic {encoded}"),
        )]);
    }
    if username.is_some() || password.is_some() {
        return Err(message(
            "Basic auth requires both --basic-user and \
--basic-password or --prompt-password.",
        ));
    }

    Err(message(
        "Authentication required. Set --token / --api-token / GRAFANA_API_TOKEN \
or --prompt-token / --basic-user and --basic-password / --prompt-password / \
GRAFANA_USERNAME and GRAFANA_PASSWORD.",
    ))
}

/// sanitize path component.
pub fn sanitize_path_component(value: &str) -> String {
    let invalid = Regex::new(r"[^\w.\- ]+").expect("invalid hard-coded regex");
    let spaces = Regex::new(r"\s+").expect("invalid hard-coded regex");
    let duplicate_underscores = Regex::new(r"_+").expect("invalid hard-coded regex");

    let normalized = invalid.replace_all(value.trim(), "_");
    let normalized = spaces.replace_all(normalized.as_ref(), "_");
    let normalized = duplicate_underscores.replace_all(normalized.as_ref(), "_");
    let normalized = normalized.trim_matches(|character| character == '.' || character == '_');
    if normalized.is_empty() {
        "untitled".to_string()
    } else {
        normalized.to_string()
    }
}

/// value as object.
pub fn value_as_object<'a>(
    value: &'a Value,
    error_message: &str,
) -> Result<&'a Map<String, Value>> {
    match value.as_object() {
        Some(object) => Ok(object),
        None => Err(message(error_message)),
    }
}

/// object field.
pub fn object_field<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Option<&'a Map<String, Value>> {
    object.get(key).and_then(Value::as_object)
}

/// string field.
pub fn string_field(object: &Map<String, Value>, key: &str, default: &str) -> String {
// Call graph (hierarchy): this function is used in related modules.
// Upstream callers: 無
// Downstream callees: 無

    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(default)
        .to_string()
}

/// load json object file.
pub fn load_json_object_file(path: &Path, object_label: &str) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&raw)?;
    if !value.is_object() {
        return Err(message(format!(
            "{object_label} file must contain a JSON object: {}",
            path.display()
        )));
    }
    Ok(value)
}

/// write json file.
pub fn write_json_file(path: &Path, payload: &Value, overwrite: bool) -> Result<()> {
    if path.exists() && !overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing file: {}. Use --overwrite.",
            path.display()
        )));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(payload)?),
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "common_rust_tests.rs"]
mod common_rust_tests;
