use serde::Serialize;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::common::{env_value, render_json_value, validation, Result};
use crate::profile_config::{ConnectionProfile, SelectedProfile};
use crate::profile_secret_store::{
    read_secret_from_encrypted_file, read_secret_from_os_store, resolve_secret_file_path,
    resolve_secret_key_path, EncryptedSecretKeySource, StoredSecretRef, SystemOsSecretStore,
};
use crate::tabular_output::{render_summary_csv, render_summary_table, render_yaml};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProfileShowDocument {
    pub(crate) name: String,
    pub(crate) source_path: PathBuf,
    pub(crate) profile: ConnectionProfile,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProfileCurrentDocument {
    pub(crate) config_path: PathBuf,
    pub(crate) config_exists: bool,
    pub(crate) selected_profile: Option<String>,
    pub(crate) auth_mode: String,
    pub(crate) secret_mode: String,
    pub(crate) profile: Option<ConnectionProfile>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProfileValidateCheck {
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProfileValidateDocument {
    pub(crate) config_path: PathBuf,
    pub(crate) profile: String,
    pub(crate) valid: bool,
    pub(crate) live_checked: bool,
    pub(crate) auth_mode: String,
    pub(crate) secret_mode: String,
    pub(crate) checks: Vec<ProfileValidateCheck>,
}

fn mask_secret_value(value: &str, show_secrets: bool) -> String {
    if show_secrets {
        value.to_string()
    } else {
        "********".to_string()
    }
}

fn resolve_profile_store_value(
    selected: &SelectedProfile,
    store_ref: &StoredSecretRef,
    explicit_passphrase: Option<&str>,
) -> Result<String> {
    match store_ref.provider.as_str() {
        "os" => read_secret_from_os_store(&SystemOsSecretStore, &store_ref.key),
        "encrypted-file" => {
            let secret_file_path =
                resolve_secret_file_path(&selected.source_path, store_ref.path.as_deref());
            let key_source = if let Some(passphrase) = explicit_passphrase {
                EncryptedSecretKeySource::Passphrase(passphrase.to_string())
            } else if let Some(env_name) = store_ref.passphrase_env.as_deref() {
                let passphrase = env_value(env_name).ok_or_else(|| {
                    validation(format!(
                        "Encrypted secret passphrase env var `{env_name}` is not set."
                    ))
                })?;
                EncryptedSecretKeySource::Passphrase(passphrase)
            } else {
                EncryptedSecretKeySource::LocalKeyFile(resolve_secret_key_path(&secret_file_path))
            };
            read_secret_from_encrypted_file(&secret_file_path, &key_source, &store_ref.key)
        }
        other => Err(validation(format!(
            "Unsupported profile secret provider `{other}`."
        ))),
    }
}

pub(crate) fn build_display_profile(
    selected: &SelectedProfile,
    show_secrets: bool,
    explicit_passphrase: Option<&str>,
) -> Result<ConnectionProfile> {
    let mut profile = selected.profile.clone();
    if let Some(value) = profile.token.clone() {
        profile.token = Some(mask_secret_value(&value, show_secrets));
    } else if let Some(store_ref) = profile.token_store.as_ref() {
        profile.token = Some(if show_secrets {
            resolve_profile_store_value(selected, store_ref, explicit_passphrase)?
        } else {
            "********".to_string()
        });
    }
    if let Some(value) = profile.password.clone() {
        profile.password = Some(mask_secret_value(&value, show_secrets));
    } else if let Some(store_ref) = profile.password_store.as_ref() {
        profile.password = Some(if show_secrets {
            resolve_profile_store_value(selected, store_ref, explicit_passphrase)?
        } else {
            "********".to_string()
        });
    }
    Ok(profile)
}

pub(crate) fn detect_profile_auth_mode(profile: &ConnectionProfile) -> &'static str {
    if profile.token.is_some() || profile.token_env.is_some() || profile.token_store.is_some() {
        "token"
    } else if profile.username.is_some()
        || profile.username_env.is_some()
        || profile.password.is_some()
        || profile.password_env.is_some()
        || profile.password_store.is_some()
    {
        "basic"
    } else {
        "none"
    }
}

pub(crate) fn detect_profile_secret_mode(profile: &ConnectionProfile) -> &'static str {
    let normalize_secret_mode = |provider: &str| match provider {
        "file" => "file",
        "os" => "os",
        "encrypted-file" => "encrypted-file",
        _ => "unknown",
    };
    let token_mode = profile
        .token_store
        .as_ref()
        .map(|store| normalize_secret_mode(store.provider.as_str()))
        .or_else(|| profile.token.as_ref().map(|_| "file"))
        .or_else(|| profile.token_env.as_ref().map(|_| "env"));
    let password_mode = profile
        .password_store
        .as_ref()
        .map(|store| normalize_secret_mode(store.provider.as_str()))
        .or_else(|| profile.password.as_ref().map(|_| "file"))
        .or_else(|| profile.password_env.as_ref().map(|_| "env"));
    match (token_mode, password_mode) {
        (None, None) => "none",
        (Some(left), None) | (None, Some(left)) => left,
        (Some(left), Some(right)) if left == right => left,
        _ => "mixed",
    }
}

pub(crate) fn render_profile_current_text(document: &ProfileCurrentDocument) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "config_path: {}", document.config_path.display());
    let _ = writeln!(output, "config_exists: {}", document.config_exists);
    let _ = writeln!(
        output,
        "selected_profile: {}",
        document.selected_profile.as_deref().unwrap_or("none")
    );
    let _ = writeln!(output, "auth_mode: {}", document.auth_mode);
    let _ = writeln!(output, "secret_mode: {}", document.secret_mode);
    if let Some(profile) = document.profile.as_ref() {
        append_profile_fields_text(&mut output, profile);
    }
    output.trim_end().to_string()
}

pub(crate) fn render_profile_current_table(document: &ProfileCurrentDocument) -> Vec<String> {
    let mut rows = vec![
        ("config_path", document.config_path.display().to_string()),
        ("config_exists", document.config_exists.to_string()),
        (
            "selected_profile",
            document
                .selected_profile
                .as_deref()
                .unwrap_or("none")
                .to_string(),
        ),
        ("auth_mode", document.auth_mode.clone()),
        ("secret_mode", document.secret_mode.clone()),
    ];
    if let Some(profile) = document.profile.as_ref() {
        rows.extend(render_profile_summary_rows(
            document.selected_profile.as_deref().unwrap_or("none"),
            &document.config_path,
            profile,
        ));
    }
    render_summary_table(&rows)
}

pub(crate) fn render_profile_current_csv(document: &ProfileCurrentDocument) -> Vec<String> {
    let mut rows = vec![
        ("config_path", document.config_path.display().to_string()),
        ("config_exists", document.config_exists.to_string()),
        (
            "selected_profile",
            document
                .selected_profile
                .as_deref()
                .unwrap_or("none")
                .to_string(),
        ),
        ("auth_mode", document.auth_mode.clone()),
        ("secret_mode", document.secret_mode.clone()),
    ];
    if let Some(profile) = document.profile.as_ref() {
        rows.extend(render_profile_summary_rows(
            document.selected_profile.as_deref().unwrap_or("none"),
            &document.config_path,
            profile,
        ));
    }
    render_summary_csv(&rows)
}

pub(crate) fn render_profile_current_json(document: &ProfileCurrentDocument) -> Result<String> {
    render_json_value(document)
}

pub(crate) fn render_profile_current_yaml(document: &ProfileCurrentDocument) -> Result<String> {
    Ok(format!("{}\n", render_yaml(document)?))
}

pub(crate) fn render_profile_validate_text(document: &ProfileValidateDocument) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "config_path: {}", document.config_path.display());
    let _ = writeln!(output, "profile: {}", document.profile);
    let _ = writeln!(output, "valid: {}", document.valid);
    let _ = writeln!(output, "live_checked: {}", document.live_checked);
    let _ = writeln!(output, "auth_mode: {}", document.auth_mode);
    let _ = writeln!(output, "secret_mode: {}", document.secret_mode);
    let _ = writeln!(output, "checks:");
    for check in &document.checks {
        let _ = writeln!(
            output,
            "  - {} [{}] {}",
            check.name, check.status, check.message
        );
    }
    output.trim_end().to_string()
}

pub(crate) fn render_profile_validate_table(document: &ProfileValidateDocument) -> Vec<String> {
    let mut rows = vec![
        ("config_path", document.config_path.display().to_string()),
        ("profile", document.profile.clone()),
        ("valid", document.valid.to_string()),
        ("live_checked", document.live_checked.to_string()),
        ("auth_mode", document.auth_mode.clone()),
        ("secret_mode", document.secret_mode.clone()),
    ];
    for check in &document.checks {
        let key = format!("check.{}.{}", check.name, check.status);
        rows.push((Box::leak(key.into_boxed_str()), check.message.clone()));
    }
    render_summary_table(&rows)
}

pub(crate) fn render_profile_validate_csv(document: &ProfileValidateDocument) -> Vec<String> {
    let mut rows = vec![
        ("config_path", document.config_path.display().to_string()),
        ("profile", document.profile.clone()),
        ("valid", document.valid.to_string()),
        ("live_checked", document.live_checked.to_string()),
        ("auth_mode", document.auth_mode.clone()),
        ("secret_mode", document.secret_mode.clone()),
    ];
    for check in &document.checks {
        let key = format!("check.{}.{}", check.name, check.status);
        rows.push((Box::leak(key.into_boxed_str()), check.message.clone()));
    }
    render_summary_csv(&rows)
}

pub(crate) fn render_profile_validate_json(document: &ProfileValidateDocument) -> Result<String> {
    render_json_value(document)
}

pub(crate) fn render_profile_validate_yaml(document: &ProfileValidateDocument) -> Result<String> {
    Ok(format!("{}\n", render_yaml(document)?))
}

pub(crate) fn render_profile_text(
    name: &str,
    source_path: &Path,
    profile: &ConnectionProfile,
) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "name: {name}");
    let _ = writeln!(output, "source_path: {}", source_path.display());
    append_profile_fields_text(&mut output, profile);
    output.trim_end().to_string()
}

fn append_store_summary_rows(
    rows: &mut Vec<(&'static str, String)>,
    field_name: &'static str,
    store_ref: &StoredSecretRef,
) {
    let provider_label = format!("{field_name}_store.provider");
    let key_label = format!("{field_name}_store.key");
    rows.push((
        Box::leak(provider_label.into_boxed_str()),
        store_ref.provider.clone(),
    ));
    rows.push((Box::leak(key_label.into_boxed_str()), store_ref.key.clone()));
    if let Some(path) = store_ref.path.as_ref() {
        let path_label = format!("{field_name}_store.path");
        rows.push((
            Box::leak(path_label.into_boxed_str()),
            path.display().to_string(),
        ));
    }
    if let Some(env_name) = store_ref.passphrase_env.as_ref() {
        let env_label = format!("{field_name}_store.passphrase_env");
        rows.push((Box::leak(env_label.into_boxed_str()), env_name.clone()));
    }
}

fn render_profile_summary_rows(
    name: &str,
    source_path: &Path,
    profile: &ConnectionProfile,
) -> Vec<(&'static str, String)> {
    let mut rows = vec![
        ("name", name.to_string()),
        ("source_path", source_path.display().to_string()),
    ];
    if let Some(value) = profile.url.as_deref() {
        rows.push(("url", value.to_string()));
    }
    if let Some(value) = profile.token.as_deref() {
        rows.push(("token", value.to_string()));
    }
    if let Some(store_ref) = profile.token_store.as_ref() {
        append_store_summary_rows(&mut rows, "token", store_ref);
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
    if let Some(store_ref) = profile.password_store.as_ref() {
        append_store_summary_rows(&mut rows, "password", store_ref);
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
    if let Some(store_ref) = profile.token_store.as_ref() {
        let _ = writeln!(output, "token_store.provider: {}", store_ref.provider);
        let _ = writeln!(output, "token_store.key: {}", store_ref.key);
        if let Some(path) = store_ref.path.as_ref() {
            let _ = writeln!(output, "token_store.path: {}", path.display());
        }
        if let Some(env_name) = store_ref.passphrase_env.as_ref() {
            let _ = writeln!(output, "token_store.passphrase_env: {env_name}");
        }
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
    if let Some(store_ref) = profile.password_store.as_ref() {
        let _ = writeln!(output, "password_store.provider: {}", store_ref.provider);
        let _ = writeln!(output, "password_store.key: {}", store_ref.key);
        if let Some(path) = store_ref.path.as_ref() {
            let _ = writeln!(output, "password_store.path: {}", path.display());
        }
        if let Some(env_name) = store_ref.passphrase_env.as_ref() {
            let _ = writeln!(output, "password_store.passphrase_env: {env_name}");
        }
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

pub(crate) fn render_profile_yaml(
    name: &str,
    source_path: &Path,
    profile: &ConnectionProfile,
) -> Result<String> {
    Ok(format!(
        "{}\n",
        render_yaml(&ProfileShowDocument {
            name: name.to_string(),
            source_path: source_path.to_path_buf(),
            profile: profile.clone(),
        })?
    ))
}

pub(crate) fn render_profile_table(
    name: &str,
    source_path: &Path,
    profile: &ConnectionProfile,
) -> Vec<String> {
    render_summary_table(&render_profile_summary_rows(name, source_path, profile))
}

pub(crate) fn render_profile_csv(
    name: &str,
    source_path: &Path,
    profile: &ConnectionProfile,
) -> Vec<String> {
    render_summary_csv(&render_profile_summary_rows(name, source_path, profile))
}

pub(crate) fn render_profile_json(
    name: &str,
    source_path: &Path,
    profile: &ConnectionProfile,
) -> Result<String> {
    render_json_value(&ProfileShowDocument {
        name: name.to_string(),
        source_path: source_path.to_path_buf(),
        profile: profile.clone(),
    })
}

pub(crate) fn render_profile_example(mode: super::ProfileExampleMode) -> String {
    match mode {
        super::ProfileExampleMode::Basic => r#"# Minimal repo-local profile config.
# Start here when you want one useful profile without the longer annotated template.
default_profile: dev
profiles:
  dev:
    url: http://127.0.0.1:3000
    token_env: GRAFANA_API_TOKEN
"#
        .to_string(),
        super::ProfileExampleMode::Full => r#"# Full repo-local profile example.
# Use this as a reference when editing grafana-util.yaml by hand.
# Secrets can stay in YAML, move to the OS secret store, or move to encrypted-file mode.
default_profile: dev
profiles:
  # Local development profile.
  dev:
    url: http://127.0.0.1:3000
    token_env: GRAFANA_API_TOKEN
    timeout: 30
    verify_ssl: false

  # Plaintext example.
  # Convenient, but the password is stored directly in grafana-util.yaml.
  prod_plaintext:
    url: https://grafana.example.com
    username: admin
    password: change-me
    verify_ssl: true

  # OS secret store example.
  # macOS uses Keychain. Linux uses Secret Service.
  prod_os_store:
    url: https://grafana.example.com
    username: admin
    password_store:
      provider: os
      key: grafana-util/profile/prod_os_store/password

  # Encrypted secret file with a passphrase.
  # The passphrase itself should come from --prompt-secret-passphrase or an env var.
  prod_encrypted:
    url: https://grafana.example.com
    username: admin
    password_store:
      provider: encrypted-file
      key: grafana-util/profile/prod_encrypted/password
      path: .grafana-util.secrets.yaml
      passphrase_env: GRAFANA_UTIL_SECRET_PASSPHRASE

  # Encrypted secret file without a passphrase.
  # This protects against casual disclosure, not local account compromise.
  stage_encrypted_local_key:
    url: https://grafana-stage.example.com
    token_store:
      provider: encrypted-file
      key: grafana-util/profile/stage_encrypted_local_key/token
      path: .grafana-util.secrets.yaml
"#
        .to_string(),
    }
}
