use rpassword::prompt_password;
use std::fs;
use std::path::Path;

use crate::common::{env_value, message, set_json_color_choice, validation, Result};
use crate::profile_config::{
    default_profile_config_path, load_profile_config_file, render_profile_init_template,
    resolve_profile_config_path, save_profile_config_file, select_profile, ConnectionProfile,
    ProfileConfigFile, SelectedProfile,
};
use crate::profile_secret_store::{
    ensure_owner_only_permissions, normalize_secret_ref_path, resolve_secret_file_path,
    resolve_secret_key_path, write_secret_to_encrypted_file, write_secret_to_os_store,
    EncryptedSecretKeySource, OsSecretStore, StoredSecretRef, SystemOsSecretStore,
};

use super::profile_cli_defs::{
    ProfileAddArgs, ProfileCliArgs, ProfileCommand, ProfileExampleArgs, ProfileInitArgs,
    ProfileSecretStorageMode, ProfileShowArgs,
};
use super::profile_cli_render::{
    build_display_profile, render_profile_csv, render_profile_example, render_profile_json,
    render_profile_table, render_profile_text, render_profile_yaml,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileAddAction {
    Added,
    Updated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProfileAddOutcome {
    pub(crate) action: ProfileAddAction,
    pub(crate) default_set: bool,
    pub(crate) local_key_warning: bool,
}

fn load_profile_config_at_resolved_path() -> Result<(std::path::PathBuf, ProfileConfigFile)> {
    let path = resolve_profile_config_path();
    if !path.exists() {
        return Err(validation(format!(
            "Profile config file {} does not exist. Run `grafana-util profile init` to create one.",
            path.display()
        )));
    }
    Ok((path.clone(), load_profile_config_file(&path)?))
}

fn select_profile_or_error(
    config: &ProfileConfigFile,
    requested_profile: Option<&str>,
    source_path: &Path,
) -> Result<SelectedProfile> {
    select_profile(config, requested_profile, source_path)?.ok_or_else(|| {
        validation(format!(
            "No profile could be selected from {}. Add default_profile or pass --profile NAME.",
            source_path.display()
        ))
    })
}

fn prompt_optional_secret(prompt: &str, enabled: bool) -> Result<Option<String>> {
    if !enabled {
        return Ok(None);
    }
    let value = prompt_password(prompt)
        .map_err(|error| message(format!("Failed to read secret input: {error}")))?;
    if value.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn resolve_secret_passphrase_from_args(
    prompt_enabled: bool,
    env_name: Option<&str>,
) -> Result<Option<String>> {
    if prompt_enabled {
        return prompt_optional_secret("Encrypted secret file passphrase: ", true);
    }
    let Some(name) = env_name else {
        return Ok(None);
    };
    env_value(name).map(Some).ok_or_else(|| {
        validation(format!(
            "Encrypted secret passphrase env var `{name}` is not set."
        ))
    })
}

fn resolve_add_secret_value(
    literal_value: Option<&str>,
    prompt_label: &str,
    prompt_enabled: bool,
) -> Result<Option<String>> {
    if let Some(value) = literal_value.filter(|value| !value.trim().is_empty()) {
        return Ok(Some(value.to_string()));
    }
    prompt_optional_secret(prompt_label, prompt_enabled)
}

fn build_profile_store_key(profile_name: &str, field_name: &str) -> String {
    format!("grafana-util/profile/{profile_name}/{field_name}")
}

fn validate_profile_add_args(args: &ProfileAddArgs) -> Result<()> {
    let has_token_source = args.token.is_some() || args.token_env.is_some() || args.prompt_token;
    let has_basic_password_source =
        args.basic_password.is_some() || args.password_env.is_some() || args.prompt_password;
    let has_basic_auth_identity = args.basic_user.is_some() || has_basic_password_source;

    if args.name.trim().is_empty() {
        return Err(validation("Profile name cannot be empty."));
    }
    if args.url.trim().is_empty() {
        return Err(validation("Profile URL cannot be empty."));
    }
    if !has_token_source && !has_basic_auth_identity {
        return Err(validation(
            "Profile credentials require either token auth or Basic auth.",
        ));
    }
    if has_token_source && has_basic_auth_identity {
        return Err(validation(
            "Choose exactly one authentication style: token or Basic auth.",
        ));
    }
    if args.token.is_some() && (args.token_env.is_some() || args.prompt_token) {
        return Err(validation("Choose exactly one token source."));
    }
    if args.token_env.is_some() && args.prompt_token {
        return Err(validation("Choose exactly one token source."));
    }
    if has_basic_password_source && args.basic_user.is_none() {
        return Err(validation(
            "--basic-user is required when using Basic auth.",
        ));
    }
    if args.basic_user.is_some() && !has_basic_password_source {
        return Err(validation(
            "--basic-user requires --basic-password, --password-env, or --prompt-password.",
        ));
    }
    if args.basic_password.is_some() && (args.password_env.is_some() || args.prompt_password) {
        return Err(validation("Choose exactly one Basic-auth password source."));
    }
    if args.password_env.is_some() && args.prompt_password {
        return Err(validation("Choose exactly one Basic-auth password source."));
    }
    if args.prompt_password && args.basic_user.is_none() {
        return Err(validation("--prompt-password requires --basic-user."));
    }
    if args.verify_ssl && args.insecure {
        return Err(validation(
            "Choose either --verify-ssl or --insecure, not both.",
        ));
    }
    if args.insecure && args.ca_cert.is_some() {
        return Err(validation(
            "Choose either --insecure or --ca-cert, not both.",
        ));
    }
    if matches!(
        args.store_secret,
        ProfileSecretStorageMode::Os | ProfileSecretStorageMode::EncryptedFile
    ) && (args.token_env.is_some() || args.password_env.is_some())
    {
        return Err(validation(
            "--token-env and --password-env cannot be combined with OS or encrypted-file secret storage.",
        ));
    }
    if !matches!(args.store_secret, ProfileSecretStorageMode::EncryptedFile)
        && (args.secret_file.is_some()
            || args.prompt_secret_passphrase
            || args.secret_passphrase_env.is_some())
    {
        return Err(validation(
            "--secret-file and encrypted-file passphrase flags require --store-secret encrypted-file.",
        ));
    }
    if args.prompt_secret_passphrase && args.secret_passphrase_env.is_some() {
        return Err(validation(
            "Choose either --prompt-secret-passphrase or --secret-passphrase-env, not both.",
        ));
    }
    Ok(())
}

fn build_profile_secret_ref(
    provider: &str,
    key: String,
    config_path: &Path,
    effective_secret_file_path: Option<&Path>,
    passphrase_env: Option<&str>,
) -> StoredSecretRef {
    StoredSecretRef {
        provider: provider.to_string(),
        key,
        path: effective_secret_file_path.map(|path| normalize_secret_ref_path(config_path, path)),
        passphrase_env: passphrase_env.map(str::to_string),
    }
}

fn upsert_profile_config(
    config: &mut ProfileConfigFile,
    name: &str,
    profile: ConnectionProfile,
    replace_existing: bool,
    set_default: bool,
) -> Result<bool> {
    let existed = config.profiles.contains_key(name);
    if existed && !replace_existing {
        return Err(validation(format!(
            "Profile `{name}` already exists. Use --replace-existing to overwrite it."
        )));
    }
    config.profiles.insert(name.to_string(), profile);
    if set_default || config.default_profile.is_none() || (config.profiles.len() == 1 && !existed) {
        config.default_profile = Some(name.to_string());
    }
    Ok(existed)
}

pub(crate) fn apply_profile_add_with_store<S: OsSecretStore>(
    args: &ProfileAddArgs,
    config_path: &Path,
    os_store: &S,
) -> Result<ProfileAddOutcome> {
    validate_profile_add_args(args)?;
    let mut config = if config_path.exists() {
        load_profile_config_file(config_path)?
    } else {
        ProfileConfigFile::default()
    };

    let token_value = resolve_add_secret_value(
        args.token.as_deref(),
        "Grafana API token: ",
        args.prompt_token,
    )?;
    let password_value = resolve_add_secret_value(
        args.basic_password.as_deref(),
        "Grafana password: ",
        args.prompt_password,
    )?;
    let secret_passphrase = resolve_secret_passphrase_from_args(
        args.prompt_secret_passphrase,
        args.secret_passphrase_env.as_deref(),
    )?;
    let local_key_warning = args.store_secret == ProfileSecretStorageMode::EncryptedFile
        && secret_passphrase.is_none()
        && (token_value.is_some() || password_value.is_some());

    let mut profile = ConnectionProfile {
        url: Some(args.url.clone()),
        token: None,
        token_env: args.token_env.clone(),
        token_store: None,
        username: args.basic_user.clone(),
        username_env: None,
        password: None,
        password_env: args.password_env.clone(),
        password_store: None,
        org_id: args.org_id,
        timeout: args.timeout,
        verify_ssl: if args.verify_ssl { Some(true) } else { None },
        insecure: if args.insecure { Some(true) } else { None },
        ca_cert: args.ca_cert.clone(),
    };

    let effective_secret_file_path = if args.store_secret == ProfileSecretStorageMode::EncryptedFile
    {
        Some(resolve_secret_file_path(
            config_path,
            args.secret_file.as_deref(),
        ))
    } else {
        None
    };

    if let Some(value) = token_value.as_deref() {
        match args.store_secret {
            ProfileSecretStorageMode::File => profile.token = Some(value.to_string()),
            ProfileSecretStorageMode::Os => {
                let key = build_profile_store_key(&args.name, "token");
                write_secret_to_os_store(os_store, &key, value)?;
                profile.token_store =
                    Some(build_profile_secret_ref("os", key, config_path, None, None));
            }
            ProfileSecretStorageMode::EncryptedFile => {
                let secret_file_path = effective_secret_file_path
                    .as_ref()
                    .expect("encrypted-file path should exist");
                let key = build_profile_store_key(&args.name, "token");
                let key_source = if let Some(passphrase) = secret_passphrase.as_deref() {
                    EncryptedSecretKeySource::Passphrase(passphrase.to_string())
                } else {
                    EncryptedSecretKeySource::LocalKeyFile(resolve_secret_key_path(
                        secret_file_path,
                    ))
                };
                write_secret_to_encrypted_file(secret_file_path, &key_source, &key, value)?;
                profile.token_store = Some(build_profile_secret_ref(
                    "encrypted-file",
                    key,
                    config_path,
                    Some(secret_file_path),
                    args.secret_passphrase_env.as_deref(),
                ));
            }
        }
    }

    if let Some(value) = password_value.as_deref() {
        match args.store_secret {
            ProfileSecretStorageMode::File => profile.password = Some(value.to_string()),
            ProfileSecretStorageMode::Os => {
                let key = build_profile_store_key(&args.name, "password");
                write_secret_to_os_store(os_store, &key, value)?;
                profile.password_store =
                    Some(build_profile_secret_ref("os", key, config_path, None, None));
            }
            ProfileSecretStorageMode::EncryptedFile => {
                let secret_file_path = effective_secret_file_path
                    .as_ref()
                    .expect("encrypted-file path should exist");
                let key = build_profile_store_key(&args.name, "password");
                let key_source = if let Some(passphrase) = secret_passphrase.as_deref() {
                    EncryptedSecretKeySource::Passphrase(passphrase.to_string())
                } else {
                    EncryptedSecretKeySource::LocalKeyFile(resolve_secret_key_path(
                        secret_file_path,
                    ))
                };
                write_secret_to_encrypted_file(secret_file_path, &key_source, &key, value)?;
                profile.password_store = Some(build_profile_secret_ref(
                    "encrypted-file",
                    key,
                    config_path,
                    Some(secret_file_path),
                    args.secret_passphrase_env.as_deref(),
                ));
            }
        }
    }

    let existed = upsert_profile_config(
        &mut config,
        &args.name,
        profile,
        args.replace_existing,
        args.set_default,
    )?;
    save_profile_config_file(config_path, &config)?;
    Ok(ProfileAddOutcome {
        action: if existed {
            ProfileAddAction::Updated
        } else {
            ProfileAddAction::Added
        },
        default_set: config.default_profile.as_deref() == Some(args.name.as_str()),
        local_key_warning,
    })
}

fn run_profile_list() -> Result<()> {
    let (path, config) = load_profile_config_at_resolved_path()?;
    for name in config.profiles.keys() {
        println!("{name}");
    }
    if config.profiles.is_empty() {
        println!("No profiles found in {}.", path.display());
    }
    Ok(())
}

fn run_profile_show(args: ProfileShowArgs) -> Result<()> {
    let (path, config) = load_profile_config_at_resolved_path()?;
    let selected = select_profile_or_error(&config, args.profile.as_deref(), &path)?;
    let explicit_passphrase = resolve_secret_passphrase_from_args(
        args.prompt_secret_passphrase,
        args.secret_passphrase_env.as_deref(),
    )?;
    let display_profile =
        build_display_profile(&selected, args.show_secrets, explicit_passphrase.as_deref())?;
    match args.output_format {
        crate::dashboard::SimpleOutputFormat::Text => {
            println!(
                "{}",
                render_profile_text(&selected.name, &selected.source_path, &display_profile)
            );
        }
        crate::dashboard::SimpleOutputFormat::Table => {
            for line in
                render_profile_table(&selected.name, &selected.source_path, &display_profile)
            {
                println!("{line}");
            }
        }
        crate::dashboard::SimpleOutputFormat::Csv => {
            for line in render_profile_csv(&selected.name, &selected.source_path, &display_profile)
            {
                println!("{line}");
            }
        }
        crate::dashboard::SimpleOutputFormat::Json => {
            println!(
                "{}",
                render_profile_json(&selected.name, &selected.source_path, &display_profile)?
            );
        }
        crate::dashboard::SimpleOutputFormat::Yaml => {
            println!(
                "{}",
                render_profile_yaml(&selected.name, &selected.source_path, &display_profile)?
            );
        }
    }
    Ok(())
}

fn run_profile_add(args: ProfileAddArgs) -> Result<()> {
    let config_path = resolve_profile_config_path();
    let outcome = apply_profile_add_with_store(&args, &config_path, &SystemOsSecretStore)?;
    if outcome.action == ProfileAddAction::Updated {
        println!(
            "Updated profile `{}` in {}.",
            args.name,
            config_path.display()
        );
    } else {
        println!(
            "Added profile `{}` to {}.",
            args.name,
            config_path.display()
        );
    }
    if outcome.default_set {
        println!("Set default_profile -> {}.", args.name);
    }
    if outcome.local_key_warning {
        println!(
            "Stored secrets in encrypted-file mode without a passphrase. This protects against casual disclosure, not local account compromise."
        );
    }
    println!(
        "Next: grafana-util profile show --profile {} --output-format yaml",
        args.name
    );
    Ok(())
}

fn run_profile_example(args: ProfileExampleArgs) -> Result<()> {
    println!("{}", render_profile_example(args.mode));
    Ok(())
}

fn run_profile_init(args: ProfileInitArgs) -> Result<()> {
    let path = std::env::current_dir()
        .map_err(|error| message(format!("Failed to resolve current directory: {error}")))?
        .join(default_profile_config_path());
    if path.exists() && !args.overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing file: {}. Use --overwrite.",
            path.display()
        )));
    }
    fs::write(&path, render_profile_init_template()).map_err(|error| {
        message(format!(
            "Failed to write grafana-util profile config {}: {error}",
            path.display()
        ))
    })?;
    ensure_owner_only_permissions(&path)?;
    println!("Wrote {}.", path.display());
    println!("Next: grafana-util profile show --profile dev --output-format yaml");
    Ok(())
}

pub fn run_profile_cli(args: ProfileCliArgs) -> Result<()> {
    set_json_color_choice(args.color);
    match args.command {
        ProfileCommand::List(_) => run_profile_list(),
        ProfileCommand::Show(show_args) => run_profile_show(show_args),
        ProfileCommand::Add(add_args) => run_profile_add(add_args.as_ref().clone()),
        ProfileCommand::Init(init_args) => run_profile_init(init_args),
        ProfileCommand::Example(example_args) => run_profile_example(example_args),
    }
}
