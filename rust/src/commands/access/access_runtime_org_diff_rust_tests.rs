use super::*;

#[test]
fn org_diff_with_request_reports_same_state() {
    let temp = tempdir().unwrap();
    let diff_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&diff_dir).unwrap();
    let bundle = json!({
        "kind": "grafana-utils-access-org-export-index",
        "version": 1,
        "records": [
            {
                "name": "Main Org",
                "users": [
                    {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Viewer"}
                ]
            }
        ]
    });
    fs::write(
        diff_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgDiffArgs {
        common: make_basic_common_no_org_id(),
        diff_dir: diff_dir.clone(),
    };
    let result = diff_orgs_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/orgs") => Ok(Some(json!([{"id": 1, "name": "Main Org"}]))),
            (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                {"userId": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn org_diff_with_request_reports_user_role_drift() {
    let temp = tempdir().unwrap();
    let diff_dir = temp.path().join("access-orgs");
    fs::create_dir_all(&diff_dir).unwrap();
    let bundle = json!({
        "kind": "grafana-utils-access-org-export-index",
        "version": 1,
        "records": [
            {
                "name": "Main Org",
                "users": [
                    {"login": "alice", "email": "alice@example.com", "name": "Alice", "orgRole": "Editor"}
                ]
            }
        ]
    });
    fs::write(
        diff_dir.join("orgs.json"),
        serde_json::to_string_pretty(&bundle).unwrap(),
    )
    .unwrap();
    let args = OrgDiffArgs {
        common: make_basic_common_no_org_id(),
        diff_dir: diff_dir.clone(),
    };
    let result = diff_orgs_with_request(
        |method, path, _params, _payload| match (method, path) {
            (Method::GET, "/api/orgs") => Ok(Some(json!([{"id": 1, "name": "Main Org"}]))),
            (Method::GET, "/api/orgs/1/users") => Ok(Some(json!([
                {"userId": 7, "login": "alice", "email": "alice@example.com", "name": "Alice", "role": "Viewer"}
            ]))),
            _ => panic!("unexpected path {path}"),
        },
        &args,
    )
    .unwrap();
    assert_eq!(result, 1);
}
