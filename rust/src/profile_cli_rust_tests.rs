use crate::common::{validation, Result};
use crate::profile_cli::profile_cli_render::{build_display_profile, render_profile_example};
use crate::profile_cli::profile_cli_runtime::{apply_profile_add_with_store, ProfileAddAction};
use crate::profile_cli::{ProfileAddArgs, ProfileExampleMode, ProfileSecretStorageMode};
use crate::profile_config::{
    load_profile_config_file, resolve_connection_settings, ConnectionMergeInput, ConnectionProfile,
    SelectedProfile,
};
use crate::profile_secret_store::OsSecretStore;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[derive(Default)]
struct MemoryOsSecretStore {
    values: RefCell<BTreeMap<String, String>>,
}

impl OsSecretStore for MemoryOsSecretStore {
    fn set_secret(&self, key: &str, value: &str) -> Result<()> {
        self.values
            .borrow_mut()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get_secret(&self, key: &str) -> Result<String> {
        self.values
            .borrow()
            .get(key)
            .cloned()
            .ok_or_else(|| validation(format!("missing key {key}")))
    }
}

#[test]
fn full_profile_example_mentions_all_secret_modes() {
    let rendered = render_profile_example(ProfileExampleMode::Full);
    assert!(rendered.contains("provider: os"));
    assert!(rendered.contains("provider: encrypted-file"));
    assert!(rendered.contains("passphrase_env: GRAFANA_UTIL_SECRET_PASSPHRASE"));
    assert!(rendered.contains("casual disclosure"));
}

#[test]
fn profile_add_creates_plaintext_profile_config() {
    let temp = tempdir().unwrap();
    let config_path = temp.path().join("grafana-util.yaml");
    let store = MemoryOsSecretStore::default();
    let result = apply_profile_add_with_store(
        &ProfileAddArgs {
            name: "dev".to_string(),
            url: "http://127.0.0.1:3000".to_string(),
            token: Some("secret-token".to_string()),
            token_env: None,
            prompt_token: false,
            basic_user: None,
            basic_password: None,
            password_env: None,
            prompt_password: false,
            org_id: None,
            timeout: Some(30),
            verify_ssl: false,
            insecure: true,
            ca_cert: None,
            set_default: true,
            replace_existing: false,
            store_secret: ProfileSecretStorageMode::File,
            secret_file: None,
            prompt_secret_passphrase: false,
            secret_passphrase_env: None,
        },
        &config_path,
        &store,
    );

    assert!(result.is_ok());
    let config = load_profile_config_file(&config_path).unwrap();
    assert_eq!(config.default_profile.as_deref(), Some("dev"));
    let profile = config.profiles.get("dev").unwrap();
    assert_eq!(profile.token.as_deref(), Some("secret-token"));
    assert_eq!(profile.timeout, Some(30));
    assert_eq!(profile.insecure, Some(true));
}

#[test]
fn profile_add_creates_encrypted_file_store_ref_and_runtime_can_resolve_it() {
    let temp = tempdir().unwrap();
    let config_path = temp.path().join("nested").join("grafana-util.yaml");
    let store = MemoryOsSecretStore::default();
    let result = apply_profile_add_with_store(
        &ProfileAddArgs {
            name: "prod".to_string(),
            url: "https://grafana.example.com".to_string(),
            token: None,
            token_env: None,
            prompt_token: false,
            basic_user: Some("admin".to_string()),
            basic_password: Some("s3cr3t".to_string()),
            password_env: None,
            prompt_password: false,
            org_id: None,
            timeout: None,
            verify_ssl: true,
            insecure: false,
            ca_cert: None,
            set_default: true,
            replace_existing: false,
            store_secret: ProfileSecretStorageMode::EncryptedFile,
            secret_file: None,
            prompt_secret_passphrase: false,
            secret_passphrase_env: None,
        },
        &config_path,
        &store,
    );

    assert!(result.is_ok());
    let config = load_profile_config_file(&config_path).unwrap();
    let profile = config.profiles.get("prod").unwrap();
    assert!(profile.password.is_none());
    let store_ref = profile.password_store.as_ref().unwrap();
    assert_eq!(store_ref.provider, "encrypted-file");
    assert_eq!(
        store_ref.path.as_deref(),
        Some(Path::new(".grafana-util.secrets.yaml"))
    );
    assert!(config_path
        .parent()
        .unwrap()
        .join(".grafana-util.secrets.yaml")
        .exists());
    assert!(config_path
        .parent()
        .unwrap()
        .join(".grafana-util.secrets.key")
        .exists());

    let selected = SelectedProfile {
        name: "prod".to_string(),
        source_path: config_path.clone(),
        profile: profile.clone(),
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
        Some(&selected),
    )
    .unwrap();
    assert_eq!(resolved.username.as_deref(), Some("admin"));
    assert_eq!(resolved.password.as_deref(), Some("s3cr3t"));
    assert!(resolved.verify_ssl);
}

#[test]
fn build_display_profile_masks_and_reveals_plaintext_secrets() {
    let selected = SelectedProfile {
        name: "prod".to_string(),
        source_path: PathBuf::from("grafana-util.yaml"),
        profile: ConnectionProfile {
            url: Some("https://grafana.example.com".to_string()),
            token: Some("abc123".to_string()),
            password: Some("secret".to_string()),
            ..ConnectionProfile::default()
        },
    };

    let masked = build_display_profile(&selected, false, None).unwrap();
    assert_eq!(masked.token.as_deref(), Some("********"));
    assert_eq!(masked.password.as_deref(), Some("********"));

    let revealed = build_display_profile(&selected, true, None).unwrap();
    assert_eq!(revealed.token.as_deref(), Some("abc123"));
    assert_eq!(revealed.password.as_deref(), Some("secret"));
}

#[test]
fn apply_profile_add_replaces_existing_profile_when_requested() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("grafana-util.yaml");
    let store = MemoryOsSecretStore::default();
    let first = ProfileAddArgs {
        name: "prod".to_string(),
        url: "https://grafana.example.com".to_string(),
        token: None,
        token_env: Some("GRAFANA_TOKEN".to_string()),
        prompt_token: false,
        basic_user: None,
        basic_password: None,
        password_env: None,
        prompt_password: false,
        org_id: None,
        timeout: None,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
        set_default: false,
        replace_existing: false,
        store_secret: ProfileSecretStorageMode::File,
        secret_file: None,
        prompt_secret_passphrase: false,
        secret_passphrase_env: None,
    };
    apply_profile_add_with_store(&first, &config_path, &store).unwrap();

    let second = ProfileAddArgs {
        name: "prod".to_string(),
        url: "https://grafana.example.com".to_string(),
        token: None,
        token_env: None,
        prompt_token: false,
        basic_user: Some("admin".to_string()),
        basic_password: Some("secret".to_string()),
        password_env: None,
        prompt_password: false,
        org_id: None,
        timeout: None,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
        set_default: true,
        replace_existing: true,
        store_secret: ProfileSecretStorageMode::File,
        secret_file: None,
        prompt_secret_passphrase: false,
        secret_passphrase_env: None,
    };

    let outcome = apply_profile_add_with_store(&second, &config_path, &store).unwrap();
    let config = load_profile_config_file(&config_path).unwrap();
    let profile = &config.profiles["prod"];

    assert_eq!(outcome.action, ProfileAddAction::Updated);
    assert!(outcome.default_set);
    assert_eq!(profile.username.as_deref(), Some("admin"));
    assert_eq!(profile.password.as_deref(), Some("secret"));
    assert!(profile.token_env.is_none());
}

#[test]
fn apply_profile_add_writes_os_store_refs() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("grafana-util.yaml");
    let store = MemoryOsSecretStore::default();
    let args = ProfileAddArgs {
        name: "prod".to_string(),
        url: "https://grafana.example.com".to_string(),
        token: None,
        token_env: None,
        prompt_token: false,
        basic_user: Some("admin".to_string()),
        basic_password: Some("secret".to_string()),
        password_env: None,
        prompt_password: false,
        org_id: None,
        timeout: None,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
        set_default: false,
        replace_existing: false,
        store_secret: ProfileSecretStorageMode::Os,
        secret_file: None,
        prompt_secret_passphrase: false,
        secret_passphrase_env: None,
    };

    apply_profile_add_with_store(&args, &config_path, &store).unwrap();
    let config = load_profile_config_file(&config_path).unwrap();
    let profile = &config.profiles["prod"];

    assert!(profile.password.is_none());
    assert_eq!(
        profile
            .password_store
            .as_ref()
            .map(|item| item.provider.as_str()),
        Some("os")
    );
    assert_eq!(
        store
            .get_secret("grafana-util/profile/prod/password")
            .unwrap(),
        "secret"
    );
}

#[test]
fn apply_profile_add_writes_encrypted_file_refs_relative_to_config_dir() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("envs/dev/grafana-util.yaml");
    let store = MemoryOsSecretStore::default();
    let args = ProfileAddArgs {
        name: "stage".to_string(),
        url: "https://grafana.example.com".to_string(),
        token: Some("secret-token".to_string()),
        token_env: None,
        prompt_token: false,
        basic_user: None,
        basic_password: None,
        password_env: None,
        prompt_password: false,
        org_id: None,
        timeout: None,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
        set_default: false,
        replace_existing: false,
        store_secret: ProfileSecretStorageMode::EncryptedFile,
        secret_file: None,
        prompt_secret_passphrase: false,
        secret_passphrase_env: None,
    };

    let outcome = apply_profile_add_with_store(&args, &config_path, &store).unwrap();
    let config = load_profile_config_file(&config_path).unwrap();
    let profile = &config.profiles["stage"];

    assert!(outcome.local_key_warning);
    assert!(outcome.gitignore_updated);
    assert!(dir
        .path()
        .join("envs/dev/.grafana-util.secrets.yaml")
        .exists());
    assert!(dir
        .path()
        .join("envs/dev/.grafana-util.secrets.key")
        .exists());
    assert_eq!(
        profile
            .token_store
            .as_ref()
            .and_then(|item| item.path.clone()),
        Some(PathBuf::from(".grafana-util.secrets.yaml"))
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("envs/dev/.gitignore")).unwrap(),
        ".grafana-util.secrets.key\n.grafana-util.secrets.yaml\n"
    );
}

#[test]
fn apply_profile_add_appends_missing_secret_entries_to_existing_gitignore() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("grafana-util.yaml");
    std::fs::write(
        dir.path().join(".gitignore"),
        "target/\n.grafana-util.secrets.yaml\n",
    )
    .unwrap();
    let store = MemoryOsSecretStore::default();
    let args = ProfileAddArgs {
        name: "stage".to_string(),
        url: "https://grafana.example.com".to_string(),
        token: Some("secret-token".to_string()),
        token_env: None,
        prompt_token: false,
        basic_user: None,
        basic_password: None,
        password_env: None,
        prompt_password: false,
        org_id: None,
        timeout: None,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
        set_default: false,
        replace_existing: false,
        store_secret: ProfileSecretStorageMode::EncryptedFile,
        secret_file: None,
        prompt_secret_passphrase: false,
        secret_passphrase_env: None,
    };

    let outcome = apply_profile_add_with_store(&args, &config_path, &store).unwrap();

    assert!(outcome.gitignore_updated);
    assert_eq!(
        std::fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
        "target/\n.grafana-util.secrets.yaml\n.grafana-util.secrets.key\n"
    );
}
