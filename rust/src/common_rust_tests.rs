//! Common utility unit tests.
//! Verifies path sanitization and authentication-header resolution logic for all Rust
//! domains.
use super::{resolve_auth_headers, resolve_auth_headers_with_prompt, sanitize_path_component};

#[test]
fn sanitize_path_component_normalizes_symbols_and_spaces() {
    assert_eq!(sanitize_path_component(" Ops / CPU % "), "Ops_CPU");
    assert_eq!(sanitize_path_component("..."), "untitled");
}

#[test]
fn resolve_auth_headers_prefers_bearer_token() {
    let headers = resolve_auth_headers(Some("abc123"), None, None, false, false).unwrap();
    assert_eq!(
        headers[0],
        ("Authorization".to_string(), "Bearer abc123".to_string())
    );
}

#[test]
fn resolve_auth_headers_rejects_mixed_token_and_basic_auth() {
    let error =
        resolve_auth_headers(Some("abc123"), Some("user"), Some("pass"), false, false).unwrap_err();
    assert!(error.to_string().contains("Choose either token auth"));
}

#[test]
fn resolve_auth_headers_rejects_partial_basic_auth() {
    let error = resolve_auth_headers(None, Some("user"), None, false, false).unwrap_err();
    assert!(error.to_string().contains(
        "Basic auth requires both --basic-user and --basic-password or --prompt-password."
    ));
}

#[test]
fn resolve_auth_headers_supports_prompt_password() {
    let headers = resolve_auth_headers_with_prompt(
        None,
        Some("user"),
        None,
        true,
        false,
        || Ok("secret".to_string()),
        || Ok("ignored".to_string()),
    )
    .unwrap();
    assert_eq!(
        headers[0],
        (
            "Authorization".to_string(),
            "Basic dXNlcjpzZWNyZXQ=".to_string()
        )
    );
}

#[test]
fn resolve_auth_headers_rejects_prompt_without_username() {
    let error = resolve_auth_headers_with_prompt(
        None,
        None,
        None,
        true,
        false,
        || Ok("secret".to_string()),
        || Ok("ignored".to_string()),
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("--prompt-password requires --basic-user."));
}

#[test]
fn resolve_auth_headers_rejects_prompt_with_explicit_password() {
    let error = resolve_auth_headers_with_prompt(
        None,
        Some("user"),
        Some("pass"),
        true,
        false,
        || Ok("secret".to_string()),
        || Ok("ignored".to_string()),
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("Choose either --basic-password or --prompt-password, not both."));
}

#[test]
fn resolve_auth_headers_supports_prompt_token() {
    let headers = resolve_auth_headers_with_prompt(
        None,
        None,
        None,
        false,
        true,
        || Ok("ignored".to_string()),
        || Ok("prompt-token".to_string()),
    )
    .unwrap();
    assert_eq!(
        headers[0],
        (
            "Authorization".to_string(),
            "Bearer prompt-token".to_string()
        )
    );
}

#[test]
fn resolve_auth_headers_rejects_prompt_token_with_explicit_token() {
    let error = resolve_auth_headers_with_prompt(
        Some("abc123"),
        None,
        None,
        false,
        true,
        || Ok("ignored".to_string()),
        || Ok("prompt-token".to_string()),
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("Choose either --token / --api-token or --prompt-token, not both."));
}
