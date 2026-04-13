//! Long-form profile CLI help text constants.

pub(crate) const PROFILE_HELP_TEXT: &str = r#"Examples:

  grafana-util config profile list
  grafana-util config profile current
  grafana-util config profile show --profile prod --output-format yaml
  grafana-util config profile validate --profile prod
  grafana-util config profile validate --profile prod --live --output-format json
  grafana-util config profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file
  grafana-util config profile example --mode basic
  grafana-util config profile example --mode full
  grafana-util config profile init --overwrite"#;

pub(crate) const PROFILE_LIST_AFTER_HELP: &str = r#"Prints one discovered profile name per line from the resolved config path.

Examples:

  grafana-util config profile list
  grafana-util config profile list --color never"#;
pub(crate) const PROFILE_SHOW_AFTER_HELP: &str = r#"Use --profile NAME to show a specific profile instead of the default-selection rules.

Examples:

  grafana-util config profile show --profile prod --output-format yaml
  grafana-util config profile show --profile prod --show-secrets --output-format json"#;
pub(crate) const PROFILE_CURRENT_AFTER_HELP: &str = r#"Use this to confirm which repo-local profile would be selected before running status live, status overview, or any Grafana command that accepts --profile.

Examples:

  grafana-util config profile current
  grafana-util config profile current --output-format json"#;
pub(crate) const PROFILE_VALIDATE_AFTER_HELP: &str = r#"Static validation checks profile selection, auth shape, env-backed credentials, and secret-store resolution. Add --live to also call Grafana /api/health with the selected profile.

Examples:

  grafana-util config profile validate --profile prod
  grafana-util config profile validate --profile prod --live --output-format json"#;
pub(crate) const PROFILE_ADD_AFTER_HELP: &str = r#"Creates or updates one profile entry without requiring manual YAML editing.

Examples:

  grafana-util config profile add prod --url https://grafana.example.com --token-env GRAFANA_API_TOKEN --set-default
  grafana-util config profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file"#;
pub(crate) const PROFILE_INIT_AFTER_HELP: &str = r#"Creates grafana-util.yaml from the built-in profile template and refuses to overwrite it unless --overwrite is set.

Examples:

  grafana-util config profile init
  grafana-util config profile init --overwrite"#;
pub(crate) const PROFILE_EXAMPLE_AFTER_HELP: &str = r#"Use this when you want a full reference config instead of the minimal init template.

Examples:

  grafana-util config profile example --mode basic
  grafana-util config profile example --mode full --output-format yaml"#;
