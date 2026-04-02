//! Shared foundation for all Rust CLI domains.
//!
//! Responsibilities:
//! - Provide one canonical `Result` and `GrafanaCliError` API shared by all modules.
//! - Centralize auth/header derivation, interactive credential prompting, and input parsing.
//! - Own generic JSON helpers, FS helpers, and output serializers that keep command behavior uniform.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use clap::{ColorChoice, ValueEnum};
use regex::Regex;
use rpassword::prompt_password;
use serde::Serialize;
use serde_json::{Map, Value};
use std::cell::Cell;
use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::Path;
use thiserror::Error;

thread_local! {
    static JSON_COLOR_CHOICE: Cell<CliColorChoice> = const { Cell::new(CliColorChoice::Auto) };
}

const ANSI_RESET: &str = "\x1b[0m";
const ANSI_JSON_KEY: &str = "\x1b[1;36m";
const ANSI_JSON_STRING: &str = "\x1b[32m";
const ANSI_JSON_NUMBER: &str = "\x1b[33m";
const ANSI_JSON_BOOL: &str = "\x1b[35m";
const ANSI_JSON_NULL: &str = "\x1b[2;90m";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JsonContext {
    ObjectExpectKey,
    ObjectExpectValue,
    Array,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum CliColorChoice {
    Auto,
    Always,
    #[value(alias = "none", alias = "off")]
    Never,
}

impl From<CliColorChoice> for ColorChoice {
    fn from(value: CliColorChoice) -> Self {
        match value {
            CliColorChoice::Auto => ColorChoice::Auto,
            CliColorChoice::Always => ColorChoice::Always,
            CliColorChoice::Never => ColorChoice::Never,
        }
    }
}

/// Set the thread-local JSON color policy used by shared render helpers.
pub fn set_json_color_choice(choice: CliColorChoice) {
    JSON_COLOR_CHOICE.with(|cell| cell.set(choice));
}

/// Read the current thread-local JSON color policy.
pub fn json_color_choice() -> CliColorChoice {
    JSON_COLOR_CHOICE.with(Cell::get)
}

/// Return whether JSON output should be colorized for the given policy and terminal state.
pub fn json_color_enabled(choice: CliColorChoice, stdout_is_terminal: bool) -> bool {
    match choice {
        CliColorChoice::Always => true,
        CliColorChoice::Never => false,
        CliColorChoice::Auto => stdout_is_terminal,
    }
}

/// Render pretty JSON with the active thread-local color policy.
pub fn render_json_value<T>(payload: &T) -> Result<String>
where
    T: Serialize + ?Sized,
{
    render_json_value_with_choice(
        payload,
        json_color_choice(),
        std::io::stdout().is_terminal(),
    )
}

/// Render pretty JSON with an explicit color policy.
pub fn render_json_value_with_choice<T>(
    payload: &T,
    choice: CliColorChoice,
    stdout_is_terminal: bool,
) -> Result<String>
where
    T: Serialize + ?Sized,
{
    let rendered = serde_json::to_string_pretty(payload)?;
    if json_color_enabled(choice, stdout_is_terminal) {
        Ok(format!("{}\n", colorize_json_pretty(&rendered)))
    } else {
        Ok(format!("{rendered}\n"))
    }
}

fn colorize_json_pretty(rendered: &str) -> String {
    let mut colored = String::with_capacity(rendered.len() + 32);
    let mut chars = rendered.chars().peekable();
    let mut contexts: Vec<JsonContext> = Vec::new();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                let mut token = String::from("\"");
                let mut escaped = false;
                for next in chars.by_ref() {
                    token.push(next);
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if next == '\\' {
                        escaped = true;
                    } else if next == '"' {
                        break;
                    }
                }
                let is_key = matches!(contexts.last(), Some(JsonContext::ObjectExpectKey));
                push_ansi_colored(
                    &mut colored,
                    if is_key {
                        ANSI_JSON_KEY
                    } else {
                        ANSI_JSON_STRING
                    },
                    &token,
                );
            }
            '{' => {
                colored.push(ch);
                contexts.push(JsonContext::ObjectExpectKey);
            }
            '}' => {
                colored.push(ch);
                contexts.pop();
            }
            '[' => {
                colored.push(ch);
                contexts.push(JsonContext::Array);
            }
            ']' => {
                colored.push(ch);
                contexts.pop();
            }
            ':' => {
                colored.push(ch);
                if let Some(context) = contexts.last_mut() {
                    if matches!(context, JsonContext::ObjectExpectKey) {
                        *context = JsonContext::ObjectExpectValue;
                    }
                }
            }
            ',' => {
                colored.push(ch);
                if let Some(context) = contexts.last_mut() {
                    if matches!(context, JsonContext::ObjectExpectValue) {
                        *context = JsonContext::ObjectExpectKey;
                    }
                }
            }
            ch if ch.is_whitespace() => colored.push(ch),
            ch => {
                let mut token = String::from(ch);
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace()
                        || matches!(next, '{' | '}' | '[' | ']' | ':' | ',' | '"')
                    {
                        break;
                    }
                    token.push(chars.next().expect("peeked character should exist"));
                }
                let color = match token.as_str() {
                    "true" | "false" => ANSI_JSON_BOOL,
                    "null" => ANSI_JSON_NULL,
                    _ if token
                        .chars()
                        .next()
                        .is_some_and(|first| first == '-' || first.is_ascii_digit()) =>
                    {
                        ANSI_JSON_NUMBER
                    }
                    _ => "",
                };
                if color.is_empty() {
                    colored.push_str(&token);
                } else {
                    push_ansi_colored(&mut colored, color, &token);
                }
            }
        }
    }

    colored
}

fn push_ansi_colored(output: &mut String, color: &str, token: &str) {
    output.push_str(color);
    output.push_str(token);
    output.push_str(ANSI_RESET);
}

/// Canonical error type shared by all Rust CLI domains.
#[derive(Debug, Error)]
pub enum GrafanaCliError {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Tui(String),
    #[error("{0}")]
    Editor(String),
    #[error("Invalid URL for {context}: {details}")]
    Url { context: String, details: String },
    #[error("Invalid header name: {name}")]
    HeaderName { name: String },
    #[error("Invalid header value for {name}: {details}")]
    HeaderValue { name: String, details: String },
    #[error("Failed to parse {target}: {details}")]
    Parse { target: String, details: String },
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

/// Repository-wide result alias using [`GrafanaCliError`].
pub type Result<T> = std::result::Result<T, GrafanaCliError>;

/// Canonical grafana-util version embedded in emitted JSON documents.
pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Return the current grafana-util version for staged/export/status metadata.
pub fn tool_version() -> &'static str {
    TOOL_VERSION
}

/// Build a plain user-facing CLI error message.
pub fn message(text: impl Into<String>) -> GrafanaCliError {
    GrafanaCliError::Message(text.into())
}

/// Build a structured local validation failure.
pub fn validation(text: impl Into<String>) -> GrafanaCliError {
    GrafanaCliError::Validation(text.into())
}

/// Build a structured terminal/TUI failure.
pub fn tui(text: impl Into<String>) -> GrafanaCliError {
    GrafanaCliError::Tui(text.into())
}

/// Build a structured external-editor failure.
pub fn editor(text: impl Into<String>) -> GrafanaCliError {
    GrafanaCliError::Editor(text.into())
}

/// Build a structured HTTP/API error with status code and response body context.
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

/// Build a structured URL parsing/validation failure.
pub fn invalid_url(context: impl Into<String>, source: impl std::fmt::Display) -> GrafanaCliError {
    GrafanaCliError::Url {
        context: context.into(),
        details: source.to_string(),
    }
}

/// Build a structured invalid-header-name failure.
pub fn invalid_header_name(name: impl Into<String>) -> GrafanaCliError {
    GrafanaCliError::HeaderName { name: name.into() }
}

/// Build a structured invalid-header-value failure.
pub fn invalid_header_value(
    name: impl Into<String>,
    source: impl std::fmt::Display,
) -> GrafanaCliError {
    GrafanaCliError::HeaderValue {
        name: name.into(),
        details: source.to_string(),
    }
}

/// Build a structured parsing failure for local text/value decoding.
pub fn parse_error(target: impl Into<String>, details: impl Into<String>) -> GrafanaCliError {
    GrafanaCliError::Parse {
        target: target.into(),
        details: details.into(),
    }
}

impl GrafanaCliError {
    /// Return the HTTP status code for API errors and `None` for local failures.
    pub fn status_code(&self) -> Option<u16> {
        match self {
            GrafanaCliError::ApiResponse { status_code, .. } => Some(*status_code),
            _ => None,
        }
    }

    /// Return a stable category label for shared error handling/reporting.
    pub fn kind(&self) -> &'static str {
        match self {
            GrafanaCliError::Message(_) => "message",
            GrafanaCliError::Validation(_) => "validation",
            GrafanaCliError::Tui(_) => "tui",
            GrafanaCliError::Editor(_) => "editor",
            GrafanaCliError::Url { .. } => "url",
            GrafanaCliError::HeaderName { .. } => "header-name",
            GrafanaCliError::HeaderValue { .. } => "header-value",
            GrafanaCliError::Parse { .. } => "parse",
            GrafanaCliError::ApiResponse { .. } => "api-response",
            GrafanaCliError::Io(_) => "io",
            GrafanaCliError::Json(_) => "json",
            GrafanaCliError::Http(_) => "http",
        }
    }
}

/// Read an environment variable and treat blank values as unset.
pub fn env_value(name: &str) -> Option<String> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

/// Resolve Grafana authentication headers from CLI args, prompts, and environment.
///
/// Resolution order is intentional:
/// - explicit token or prompted token
/// - explicit/basic credentials
/// - environment fallbacks
///
/// The function rejects mixed auth modes so downstream HTTP code never has to
/// guess which credential source should win.
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
        return Err(validation(
            "Choose either --token / --api-token or --prompt-token, not both.",
        ));
    }
    if (cli_token.is_some() || prompt_for_token)
        && (cli_username.is_some() || cli_password.is_some() || prompt_for_password)
    {
        return Err(validation(
            "Choose either token auth (--token / --api-token) or Basic auth \
(--basic-user with --basic-password / --prompt-password), not both.",
        ));
    }
    if prompt_for_password && cli_password.is_some() {
        return Err(validation(
            "Choose either --basic-password or --prompt-password, not both.",
        ));
    }
    if cli_username.is_some() && cli_password.is_none() && !prompt_for_password {
        return Err(validation(
            "Basic auth requires both --basic-user and \
--basic-password or --prompt-password.",
        ));
    }
    if cli_password.is_some() && cli_username.is_none() {
        return Err(validation(
            "Basic auth requires both --basic-user and \
--basic-password or --prompt-password.",
        ));
    }
    if prompt_for_password && cli_username.is_none() {
        return Err(validation("--prompt-password requires --basic-user."));
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
        return Err(validation(
            "Basic auth requires both --basic-user and \
--basic-password or --prompt-password.",
        ));
    }

    Err(validation(
        "Authentication required. Set --token / --api-token / GRAFANA_API_TOKEN \
or --prompt-token / --basic-user and --basic-password / --prompt-password / \
GRAFANA_USERNAME and GRAFANA_PASSWORD.",
    ))
}

/// Normalize user-provided strings into filesystem-safe path components.
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

/// Require a JSON value to be an object and return a borrowed map view.
pub fn value_as_object<'a>(
    value: &'a Value,
    error_message: &str,
) -> Result<&'a Map<String, Value>> {
    match value.as_object() {
        Some(object) => Ok(object),
        None => Err(message(error_message)),
    }
}

/// Read one nested object field if present.
pub fn object_field<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Option<&'a Map<String, Value>> {
    object.get(key).and_then(Value::as_object)
}

/// Read a non-empty string field or fall back to the provided default.
pub fn string_field(object: &Map<String, Value>, key: &str, default: &str) -> String {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(default)
        .to_string()
}

/// Load a JSON file and require the top-level value to be an object.
pub fn load_json_object_file(path: &Path, object_label: &str) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&raw)?;
    if !value.is_object() {
        return Err(validation(format!(
            "{object_label} file must contain a JSON object: {}",
            path.display()
        )));
    }
    Ok(value)
}

/// Write JSON to disk with an explicit overwrite gate.
pub fn write_json_file(path: &Path, payload: &Value, overwrite: bool) -> Result<()> {
    if path.exists() && !overwrite {
        return Err(validation(format!(
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
