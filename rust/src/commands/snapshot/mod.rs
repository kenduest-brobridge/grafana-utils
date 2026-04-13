//! Snapshot wrappers for dashboard and datasource exports plus local review.
//!
//! This module stays thin: it derives the per-domain paths/args for a snapshot
//! export root, then builds a snapshot-native inventory review document from
//! the exported dashboard and datasource metadata.

#[path = "cli_defs.rs"]
mod snapshot_cli_defs;
#[path = "review/mod.rs"]
mod snapshot_review;
#[path = "review/counts.rs"]
mod snapshot_review_counts;
#[path = "review/document.rs"]
mod snapshot_review_document;
#[path = "review/lanes.rs"]
mod snapshot_review_lanes;
#[path = "support.rs"]
mod snapshot_support;

use std::path::PathBuf;

pub(crate) use crate::dashboard::ROOT_INDEX_KIND;

pub(crate) use self::snapshot_cli_defs::{
    prompt_snapshot_export_selection, SnapshotExportLane, SnapshotExportSelection,
};
pub use self::snapshot_cli_defs::{
    SnapshotCliArgs, SnapshotCommand, SnapshotExportArgs, SnapshotReviewArgs,
};
pub use self::snapshot_review::render_snapshot_review_text;
#[cfg(test)]
pub(crate) use self::snapshot_review::{
    build_snapshot_review_browser_items, build_snapshot_review_summary_lines,
};
pub use self::snapshot_review_document::build_snapshot_review_document;
#[allow(unused_imports)]
#[cfg(any(feature = "tui", test))]
pub use self::snapshot_support::root_command;
pub(crate) use self::snapshot_support::run_snapshot_cli;
#[cfg(test)]
pub(crate) use self::snapshot_support::{
    build_snapshot_overview_args, build_snapshot_paths, build_snapshot_root_metadata,
    materialize_snapshot_common_auth_with_prompt, run_snapshot_export_selected_with_handlers,
    run_snapshot_export_with_handlers, run_snapshot_review_document_with_handler,
};

pub const SNAPSHOT_DASHBOARD_DIR: &str = "dashboards";
pub const SNAPSHOT_DATASOURCE_DIR: &str = "datasources";
pub const SNAPSHOT_ACCESS_DIR: &str = "access";
pub const SNAPSHOT_ACCESS_USERS_DIR: &str = "users";
pub const SNAPSHOT_ACCESS_TEAMS_DIR: &str = "teams";
pub const SNAPSHOT_ACCESS_ORGS_DIR: &str = "orgs";
pub const SNAPSHOT_ACCESS_SERVICE_ACCOUNTS_DIR: &str = "service-accounts";
pub const SNAPSHOT_DATASOURCE_EXPORT_FILENAME: &str = "datasources.json";
pub const SNAPSHOT_DATASOURCE_EXPORT_METADATA_FILENAME: &str = "export-metadata.json";
pub const SNAPSHOT_DATASOURCE_ROOT_INDEX_KIND: &str = "grafana-utils-datasource-export-index";
pub const SNAPSHOT_DATASOURCE_TOOL_SCHEMA_VERSION: i64 = 1;
pub const SNAPSHOT_METADATA_FILENAME: &str = "snapshot-metadata.json";
pub(crate) const SNAPSHOT_REVIEW_KIND: &str = "grafana-utils-snapshot-review";
pub(crate) const SNAPSHOT_REVIEW_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotPaths {
    pub dashboards: PathBuf,
    pub datasources: PathBuf,
    pub access: PathBuf,
    pub access_users: PathBuf,
    pub access_teams: PathBuf,
    pub access_orgs: PathBuf,
    pub access_service_accounts: PathBuf,
    pub metadata: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::materialize_snapshot_common_auth_with_prompt;
    use crate::dashboard::CommonCliArgs;

    fn sample_common_args() -> CommonCliArgs {
        CommonCliArgs {
            color: crate::common::CliColorChoice::Auto,
            profile: Some("prod".to_string()),
            url: "http://grafana.example.com".to_string(),
            api_token: None,
            username: Some("admin".to_string()),
            password: None,
            prompt_password: true,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        }
    }

    #[test]
    fn materialize_snapshot_common_auth_prompts_password_once_and_clears_prompt_flags() {
        let common = sample_common_args();
        let mut password_prompts = 0;
        let mut token_prompts = 0;

        let resolved = materialize_snapshot_common_auth_with_prompt(
            common,
            || {
                password_prompts += 1;
                Ok("secret".to_string())
            },
            || {
                token_prompts += 1;
                Ok("token".to_string())
            },
        )
        .expect("resolved auth");

        assert_eq!(resolved.password.as_deref(), Some("secret"));
        assert!(!resolved.prompt_password);
        assert!(!resolved.prompt_token);
        assert_eq!(password_prompts, 1);
        assert_eq!(token_prompts, 0);
    }
}
