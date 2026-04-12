//! CLI definitions for Access command surface and option compatibility behavior.

use super::*;
use crate::access::{
    build_auth_context,
    render::{normalize_user_row, user_table_headers},
    ACCESS_EXPORT_KIND_ORGS, ACCESS_EXPORT_KIND_SERVICE_ACCOUNTS, ACCESS_EXPORT_KIND_TEAMS,
    ACCESS_EXPORT_KIND_USERS, ACCESS_ORG_EXPORT_FILENAME, ACCESS_SERVICE_ACCOUNT_EXPORT_FILENAME,
    ACCESS_TEAM_EXPORT_FILENAME, ACCESS_USER_EXPORT_FILENAME,
};
use serde_json::{Map, Value};

fn render_access_subcommand_help(path: &[&str]) -> String {
    let mut command = AccessCliRoot::command();
    let mut current = &mut command;
    for segment in path {
        current = current
            .find_subcommand_mut(segment)
            .unwrap_or_else(|| panic!("missing access subcommand help for {segment}"));
    }
    let mut output = Vec::new();
    current.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn render_access_root_help() -> String {
    let mut command = AccessCliRoot::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    String::from_utf8(output).unwrap()
}

fn make_token_common() -> CommonCliArgs {
    CommonCliArgs {
        profile: None,
        url: "http://127.0.0.1:3000".to_string(),
        api_token: Some("token".to_string()),
        username: None,
        password: None,
        prompt_password: false,
        prompt_token: false,
        org_id: None,
        timeout: 30,
        verify_ssl: false,
        insecure: false,
        ca_cert: None,
    }
}

fn load_access_bundle_contract_cases() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!(
        "../../../fixtures/access_bundle_contract_cases.json"
    ))
    .unwrap()
    .get("cases")
    .and_then(Value::as_array)
    .cloned()
    .unwrap_or_default()
}

mod access_cli_help_examples_rust_tests;
mod access_cli_org_team_rust_tests;
mod access_cli_render_normalize_rust_tests;
mod access_cli_user_service_account_rust_tests;
