//! Secret backends for repo-local profile credentials.
//!
//! Supported modes:
//! - OS-backed secret storage via macOS Keychain or Linux Secret Service.
//! - Repo-local encrypted secret files with either a passphrase or a local key file.

use aes_gcm_siv::aead::Aead;
use aes_gcm_siv::{Aes256GcmSiv, KeyInit, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use pbkdf2::pbkdf2_hmac_array;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, validation, Result};

const PROFILE_SECRET_SERVICE: &str = "grafana-util";
const ENCRYPTED_SECRET_FILE_VERSION: u8 = 1;
const ENCRYPTED_SECRET_MODE_PASSPHRASE: &str = "passphrase";
const ENCRYPTED_SECRET_MODE_LOCAL_KEY: &str = "local-key";
const PBKDF2_ROUNDS: u32 = 600_000;
const ENCRYPTED_SECRET_FILENAME: &str = ".grafana-util.secrets.yaml";
const ENCRYPTED_SECRET_KEY_FILENAME: &str = ".grafana-util.secrets.key";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StoredSecretRef {
    pub provider: String,
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passphrase_env: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncryptedSecretKeySource {
    Passphrase(String),
    LocalKeyFile(PathBuf),
}

pub trait OsSecretStore {
    fn set_secret(&self, key: &str, value: &str) -> Result<()>;
    fn get_secret(&self, key: &str) -> Result<String>;
}

#[derive(Debug, Default)]
pub struct SystemOsSecretStore;

impl OsSecretStore for SystemOsSecretStore {
    fn set_secret(&self, key: &str, value: &str) -> Result<()> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let entry = keyring::Entry::new(PROFILE_SECRET_SERVICE, key).map_err(|error| {
                message(format!(
                    "Failed to prepare OS secret entry for `{key}`: {error}"
                ))
            })?;
            entry.set_password(value).map_err(|error| {
                message(format!(
                    "Failed to store secret `{key}` in the OS secret store: {error}"
                ))
            })
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = (key, value);
            Err(validation(
                "OS secret storage is only supported on macOS and Linux.",
            ))
        }
    }

    fn get_secret(&self, key: &str) -> Result<String> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let entry = keyring::Entry::new(PROFILE_SECRET_SERVICE, key).map_err(|error| {
                message(format!(
                    "Failed to prepare OS secret entry for `{key}`: {error}"
                ))
            })?;
            entry.get_password().map_err(|error| {
                message(format!(
                    "Failed to read secret `{key}` from the OS secret store: {error}"
                ))
            })
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let _ = key;
            Err(validation(
                "OS secret storage is only supported on macOS and Linux.",
            ))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedSecretFile {
    version: u8,
    mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    salt: Option<String>,
    #[serde(default)]
    secrets: BTreeMap<String, EncryptedSecretEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedSecretEntry {
    nonce: String,
    ciphertext: String,
}

fn new_encrypted_secret_file(mode: &str, salt: Option<[u8; 16]>) -> EncryptedSecretFile {
    EncryptedSecretFile {
        version: ENCRYPTED_SECRET_FILE_VERSION,
        mode: mode.to_string(),
        salt: salt.map(|bytes| STANDARD.encode(bytes)),
        secrets: BTreeMap::new(),
    }
}

pub fn default_secret_file_path_for_config(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(ENCRYPTED_SECRET_FILENAME)
}

pub fn default_secret_key_path_for_secret_file(secret_file_path: &Path) -> PathBuf {
    secret_file_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(ENCRYPTED_SECRET_KEY_FILENAME)
}

pub fn normalize_secret_ref_path(config_path: &Path, effective_path: &Path) -> PathBuf {
    let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    if let Ok(relative) = effective_path.strip_prefix(base_dir) {
        relative.to_path_buf()
    } else {
        effective_path.to_path_buf()
    }
}

pub fn resolve_secret_file_path(config_path: &Path, explicit_path: Option<&Path>) -> PathBuf {
    let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    match explicit_path {
        Some(path) if path.is_absolute() => path.to_path_buf(),
        Some(path) => base_dir.join(path),
        None => default_secret_file_path_for_config(config_path),
    }
}

pub fn resolve_secret_key_path(secret_file_path: &Path) -> PathBuf {
    default_secret_key_path_for_secret_file(secret_file_path)
}

pub fn ensure_owner_only_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, permissions).map_err(|error| {
            message(format!(
                "Failed to set owner-only permissions on {}: {error}",
                path.display()
            ))
        })?;
    }
    Ok(())
}

pub fn write_secret_to_os_store<S: OsSecretStore>(store: &S, key: &str, value: &str) -> Result<()> {
    store.set_secret(key, value)
}

pub fn read_secret_from_os_store<S: OsSecretStore>(store: &S, key: &str) -> Result<String> {
    store.get_secret(key)
}

pub fn write_secret_to_encrypted_file(
    secret_file_path: &Path,
    key_source: &EncryptedSecretKeySource,
    key_name: &str,
    secret_value: &str,
) -> Result<()> {
    let mut document = if secret_file_path.exists() {
        load_encrypted_secret_file(secret_file_path)?
    } else {
        match key_source {
            EncryptedSecretKeySource::Passphrase(_) => {
                let mut salt = [0u8; 16];
                rand::thread_rng().fill_bytes(&mut salt);
                new_encrypted_secret_file(ENCRYPTED_SECRET_MODE_PASSPHRASE, Some(salt))
            }
            EncryptedSecretKeySource::LocalKeyFile(_) => {
                new_encrypted_secret_file(ENCRYPTED_SECRET_MODE_LOCAL_KEY, None)
            }
        }
    };
    match (document.mode.as_str(), key_source) {
        (ENCRYPTED_SECRET_MODE_PASSPHRASE, EncryptedSecretKeySource::Passphrase(_)) => {}
        (ENCRYPTED_SECRET_MODE_LOCAL_KEY, EncryptedSecretKeySource::LocalKeyFile(_)) => {}
        _ => {
            return Err(validation(format!(
                "Encrypted secret file {} uses mode `{}` which does not match the requested key source.",
                secret_file_path.display(),
                document.mode
            )));
        }
    }
    let key_bytes = derive_encrypted_secret_key(secret_file_path, &document, key_source)?;
    let cipher = Aes256GcmSiv::new_from_slice(&key_bytes)
        .map_err(|error| message(format!("Failed to initialize secret cipher: {error}")))?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, secret_value.as_bytes())
        .map_err(|error| message(format!("Failed to encrypt secret `{key_name}`: {error}")))?;
    document.secrets.insert(
        key_name.to_string(),
        EncryptedSecretEntry {
            nonce: STANDARD.encode(nonce_bytes),
            ciphertext: STANDARD.encode(ciphertext),
        },
    );
    save_encrypted_secret_file(secret_file_path, &document)?;
    Ok(())
}

pub fn read_secret_from_encrypted_file(
    secret_file_path: &Path,
    key_source: &EncryptedSecretKeySource,
    key_name: &str,
) -> Result<String> {
    let document = load_encrypted_secret_file(secret_file_path)?;
    match (document.mode.as_str(), key_source) {
        (ENCRYPTED_SECRET_MODE_PASSPHRASE, EncryptedSecretKeySource::Passphrase(_)) => {}
        (ENCRYPTED_SECRET_MODE_LOCAL_KEY, EncryptedSecretKeySource::LocalKeyFile(_)) => {}
        (ENCRYPTED_SECRET_MODE_PASSPHRASE, EncryptedSecretKeySource::LocalKeyFile(_)) => {
            return Err(validation(format!(
                "Encrypted secret file {} requires a passphrase.",
                secret_file_path.display()
            )));
        }
        (ENCRYPTED_SECRET_MODE_LOCAL_KEY, EncryptedSecretKeySource::Passphrase(_)) => {
            return Err(validation(format!(
                "Encrypted secret file {} uses a local key file, not a passphrase.",
                secret_file_path.display()
            )));
        }
        _ => {
            return Err(validation(format!(
                "Encrypted secret file {} has an unknown mode `{}`.",
                secret_file_path.display(),
                document.mode
            )));
        }
    }
    let entry = document.secrets.get(key_name).ok_or_else(|| {
        validation(format!(
            "Encrypted secret file {} does not contain key `{key_name}`.",
            secret_file_path.display()
        ))
    })?;
    let key_bytes = derive_encrypted_secret_key(secret_file_path, &document, key_source)?;
    let cipher = Aes256GcmSiv::new_from_slice(&key_bytes)
        .map_err(|error| message(format!("Failed to initialize secret cipher: {error}")))?;
    let nonce_bytes = STANDARD
        .decode(&entry.nonce)
        .map_err(|error| message(format!("Failed to decode encrypted secret nonce: {error}")))?;
    let ciphertext = STANDARD.decode(&entry.ciphertext).map_err(|error| {
        message(format!(
            "Failed to decode encrypted secret ciphertext: {error}"
        ))
    })?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref()).map_err(|_| {
        validation(format!(
            "Failed to decrypt secret `{key_name}` from {}. Check the passphrase or key file.",
            secret_file_path.display()
        ))
    })?;
    String::from_utf8(plaintext).map_err(|error| {
        message(format!(
            "Decrypted secret `{key_name}` is not valid UTF-8: {error}"
        ))
    })
}

fn derive_encrypted_secret_key(
    secret_file_path: &Path,
    document: &EncryptedSecretFile,
    key_source: &EncryptedSecretKeySource,
) -> Result<[u8; 32]> {
    match key_source {
        EncryptedSecretKeySource::Passphrase(passphrase) => {
            let salt = document.salt.as_deref().ok_or_else(|| {
                validation(format!(
                    "Encrypted secret file {} is missing the passphrase salt.",
                    secret_file_path.display()
                ))
            })?;
            let salt_bytes = STANDARD.decode(salt).map_err(|error| {
                message(format!("Failed to decode encrypted secret salt: {error}"))
            })?;
            Ok(pbkdf2_hmac_array::<Sha256, 32>(
                passphrase.as_bytes(),
                &salt_bytes,
                PBKDF2_ROUNDS,
            ))
        }
        EncryptedSecretKeySource::LocalKeyFile(path) => read_or_create_local_key(path),
    }
}

fn read_or_create_local_key(path: &Path) -> Result<[u8; 32]> {
    if path.exists() {
        let raw = fs::read_to_string(path).map_err(|error| {
            message(format!(
                "Failed to read local secret key file {}: {error}",
                path.display()
            ))
        })?;
        let trimmed = raw.trim();
        let bytes = STANDARD.decode(trimmed).map_err(|error| {
            message(format!(
                "Failed to decode local secret key file {}: {error}",
                path.display()
            ))
        })?;
        let array: [u8; 32] = bytes.try_into().map_err(|_| {
            validation(format!(
                "Local secret key file {} did not contain a 32-byte key.",
                path.display()
            ))
        })?;
        return Ok(array);
    }
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            message(format!(
                "Failed to create local secret key directory {}: {error}",
                parent.display()
            ))
        })?;
    }
    fs::write(path, format!("{}\n", STANDARD.encode(key))).map_err(|error| {
        message(format!(
            "Failed to write local secret key file {}: {error}",
            path.display()
        ))
    })?;
    ensure_owner_only_permissions(path)?;
    Ok(key)
}

fn load_encrypted_secret_file(path: &Path) -> Result<EncryptedSecretFile> {
    let raw = fs::read_to_string(path).map_err(|error| {
        message(format!(
            "Failed to read encrypted secret file {}: {error}",
            path.display()
        ))
    })?;
    serde_yaml::from_str(&raw).map_err(|error| {
        validation(format!(
            "Failed to parse encrypted secret file {}: {error}",
            path.display()
        ))
    })
}

fn save_encrypted_secret_file(path: &Path, document: &EncryptedSecretFile) -> Result<()> {
    let rendered = serde_yaml::to_string(document)
        .map_err(|error| message(format!("Failed to render encrypted secret file: {error}")))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            message(format!(
                "Failed to create encrypted secret directory {}: {error}",
                parent.display()
            ))
        })?;
    }
    fs::write(path, rendered).map_err(|error| {
        message(format!(
            "Failed to write encrypted secret file {}: {error}",
            path.display()
        ))
    })?;
    ensure_owner_only_permissions(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    #[cfg(target_os = "macos")]
    use std::process::Command;
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
    fn memory_os_store_round_trips() {
        let store = MemoryOsSecretStore::default();
        write_secret_to_os_store(&store, "grafana-util/profile/dev/token", "secret").unwrap();
        let value = read_secret_from_os_store(&store, "grafana-util/profile/dev/token").unwrap();
        assert_eq!(value, "secret");
    }

    #[test]
    fn encrypted_secret_file_round_trips_with_passphrase() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".grafana-util.secrets.yaml");
        write_secret_to_encrypted_file(
            &path,
            &EncryptedSecretKeySource::Passphrase("hunter2".to_string()),
            "grafana-util/profile/prod/password",
            "s3cr3t",
        )
        .unwrap();
        let value = read_secret_from_encrypted_file(
            &path,
            &EncryptedSecretKeySource::Passphrase("hunter2".to_string()),
            "grafana-util/profile/prod/password",
        )
        .unwrap();
        assert_eq!(value, "s3cr3t");
    }

    #[test]
    fn encrypted_secret_file_round_trips_with_local_key_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".grafana-util.secrets.yaml");
        let key_path = dir.path().join(".grafana-util.secrets.key");
        write_secret_to_encrypted_file(
            &path,
            &EncryptedSecretKeySource::LocalKeyFile(key_path.clone()),
            "grafana-util/profile/dev/token",
            "token-value",
        )
        .unwrap();
        let value = read_secret_from_encrypted_file(
            &path,
            &EncryptedSecretKeySource::LocalKeyFile(key_path),
            "grafana-util/profile/dev/token",
        )
        .unwrap();
        assert_eq!(value, "token-value");
    }

    #[test]
    fn normalize_secret_ref_path_prefers_relative_to_config_dir() {
        let config_path = Path::new("/tmp/work/grafana-util.yaml");
        let secret_path = Path::new("/tmp/work/.grafana-util.secrets.yaml");
        assert_eq!(
            normalize_secret_ref_path(config_path, secret_path),
            PathBuf::from(".grafana-util.secrets.yaml")
        );
    }

    #[cfg(target_os = "macos")]
    fn delete_keychain_test_entry(key: &str) {
        let _ = Command::new("security")
            .args([
                "delete-generic-password",
                "-s",
                PROFILE_SECRET_SERVICE,
                "-a",
                key,
            ])
            .output();
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "Touches the logged-in macOS Keychain and is meant for manual compatibility smoke checks."]
    fn system_os_store_reads_legacy_security_cli_entries_and_writes_cli_visible_entries() {
        let key = format!(
            "grafana-util/profile/test/macos-compat-{}",
            std::process::id()
        );
        let legacy_value = "legacy-secret-value";
        let new_value = "keyring-secret-value";
        delete_keychain_test_entry(&key);

        let add_output = Command::new("security")
            .args([
                "add-generic-password",
                "-U",
                "-A",
                "-s",
                PROFILE_SECRET_SERVICE,
                "-a",
                &key,
                "-w",
                legacy_value,
            ])
            .output()
            .expect("run security add-generic-password");
        assert!(
            add_output.status.success(),
            "security add-generic-password failed: {}",
            String::from_utf8_lossy(&add_output.stderr).trim()
        );

        let store = SystemOsSecretStore;
        let read_back = read_secret_from_os_store(&store, &key).expect("read legacy keychain item");
        assert_eq!(read_back, legacy_value);

        write_secret_to_os_store(&store, &key, new_value)
            .expect("write keyring-backed keychain item");

        let find_output = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                PROFILE_SECRET_SERVICE,
                "-a",
                &key,
                "-w",
            ])
            .output()
            .expect("run security find-generic-password");
        assert!(
            find_output.status.success(),
            "security find-generic-password failed: {}",
            String::from_utf8_lossy(&find_output.stderr).trim()
        );
        let cli_value = String::from_utf8(find_output.stdout).expect("decode keychain value");
        assert_eq!(cli_value.trim_end(), new_value);

        delete_keychain_test_entry(&key);
    }
}
