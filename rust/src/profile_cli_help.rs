//! Long-form profile CLI help text constants.

pub(crate) const PROFILE_HELP_TEXT: &str = r#"Examples:

  grafana-util profile list
  grafana-util profile current
  grafana-util profile show --profile prod --output-format yaml
  grafana-util profile validate --profile prod
  grafana-util profile validate --profile prod --live --output-format json
  grafana-util profile add prod --url https://grafana.example.com --basic-user admin --prompt-password --store-secret encrypted-file
  grafana-util profile example --mode basic
  grafana-util profile example --mode full
  grafana-util profile init --overwrite"#;

pub(crate) const PROFILE_LIST_AFTER_HELP: &str =
    "Prints one discovered profile name per line from the resolved config path.";
pub(crate) const PROFILE_SHOW_AFTER_HELP: &str =
    "Use --profile NAME to show a specific profile instead of the default-selection rules.";
pub(crate) const PROFILE_CURRENT_AFTER_HELP: &str =
    "Use this to confirm which repo-local profile would be selected before running status live, overview live, or any Grafana command that accepts --profile.";
pub(crate) const PROFILE_VALIDATE_AFTER_HELP: &str =
    "Static validation checks profile selection, auth shape, env-backed credentials, and secret-store resolution. Add --live to also call Grafana /api/health with the selected profile.";
pub(crate) const PROFILE_ADD_AFTER_HELP: &str =
    "Creates or updates one profile entry without requiring manual YAML editing.";
pub(crate) const PROFILE_INIT_AFTER_HELP: &str =
    "Creates grafana-util.yaml from the built-in profile template and refuses to overwrite it unless --overwrite is set.";
pub(crate) const PROFILE_EXAMPLE_AFTER_HELP: &str =
    "Use this when you want a full reference config instead of the minimal init template.";
