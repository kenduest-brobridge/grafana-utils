//! Sync CLI apply/review execution regression test facade.
//! Keeps the review/trace-lineage and apply/preflight clusters in dedicated submodules.
use crate::dashboard::CommonCliArgs;

fn sync_common_args() -> CommonCliArgs {
    CommonCliArgs {
        color: crate::common::CliColorChoice::Auto,
        profile: None,
        url: "http://127.0.0.1:3000".to_string(),
        api_token: Some("test-token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        timeout: 30,
        verify_ssl: false,
    }
}

#[cfg(test)]
#[path = "cli_apply_review_exec_review_rust_tests.rs"]
mod cli_apply_review_exec_review_rust_tests;

#[cfg(test)]
#[path = "cli_apply_review_exec_apply_rust_tests.rs"]
mod cli_apply_review_exec_apply_rust_tests;
