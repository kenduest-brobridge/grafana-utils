use super::*;

#[test]
fn run_access_cli_with_request_routes_org_list_local_input_dir() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("access-orgs");
    write_local_access_bundle(
        &input_dir,
        "orgs.json",
        r#"{
            "kind":"grafana-utils-access-org-export-index",
            "version":1,
            "records":[
                {"name":"Main Org","users":[{"login":"alice","email":"alice@example.com","orgRole":"Editor"}]}
            ]
        }"#,
    );

    let args = parse_cli_from([
        "grafana-util access",
        "org",
        "list",
        "--input-dir",
        input_dir.to_str().unwrap(),
        "--output-format",
        "yaml",
    ]);
    let mut request_called = false;
    let result = run_access_cli_with_request(
        |_method, _path, _params, _payload| {
            request_called = true;
            panic!("local org list should not hit the request layer");
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(!request_called);
}

#[test]
fn run_access_cli_with_request_routes_team_list_local_input_dir() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("access-teams");
    write_local_access_bundle(
        &input_dir,
        "teams.json",
        r#"{
            "kind":"grafana-utils-access-team-export-index",
            "version":1,
            "records":[
                {"name":"Ops","email":"ops@example.com","members":["alice"],"admins":["bob"]}
            ]
        }"#,
    );

    let args = parse_cli_from([
        "grafana-util access",
        "team",
        "list",
        "--input-dir",
        input_dir.to_str().unwrap(),
        "--output-format",
        "table",
    ]);
    let mut request_called = false;
    let result = run_access_cli_with_request(
        |_method, _path, _params, _payload| {
            request_called = true;
            panic!("local team list should not hit the request layer");
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(!request_called);
}

#[test]
fn run_access_cli_with_request_routes_service_account_list_local_input_dir() {
    let temp = tempdir().unwrap();
    let input_dir = temp.path().join("access-service-accounts");
    write_local_access_bundle(
        &input_dir,
        "service-accounts.json",
        r#"{
            "kind":"grafana-utils-access-service-account-export-index",
            "version":1,
            "records":[
                {"name":"deploy-bot","role":"Editor","disabled":false,"tokens":1}
            ]
        }"#,
    );

    let args = parse_cli_from([
        "grafana-util access",
        "service-account",
        "list",
        "--input-dir",
        input_dir.to_str().unwrap(),
        "--output-format",
        "csv",
    ]);
    let mut request_called = false;
    let result = run_access_cli_with_request(
        |_method, _path, _params, _payload| {
            request_called = true;
            panic!("local service-account list should not hit the request layer");
        },
        &args,
    );

    assert!(result.is_ok());
    assert!(!request_called);
}
