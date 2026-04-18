use super::*;
use serde_json::json;

#[test]
fn dashboard_delete_validate_args_requires_yes_without_dry_run() {
    let args = DeleteArgs {
        common: CommonCliArgs {
            color: crate::common::CliColorChoice::Auto,
            profile: None,
            url: "https://grafana.example.com".to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        },
        page_size: 500,
        org_id: None,
        uid: Some("cpu-main".to_string()),
        path: None,
        delete_folders: false,
        yes: false,
        prompt: false,
        dry_run: false,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
    };

    let error = validate_delete_args(&args).unwrap_err();
    assert!(error.to_string().contains("requires --yes"));
}

#[test]
fn dashboard_delete_build_plan_matches_path_subtree() {
    let args = DeleteArgs {
        common: CommonCliArgs {
            color: crate::common::CliColorChoice::Auto,
            profile: None,
            url: "https://grafana.example.com".to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        },
        page_size: 500,
        org_id: None,
        uid: None,
        path: Some("Platform / Infra".to_string()),
        delete_folders: true,
        yes: true,
        prompt: false,
        dry_run: false,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
    };

    let plan = build_delete_plan_with_request(
        |method, path, params, _payload| match (method.clone(), path) {
            (Method::GET, "/api/search") => {
                let page = params
                    .iter()
                    .find(|(key, _)| key == "page")
                    .map(|(_, value)| value.as_str())
                    .unwrap_or("1");
                if page == "1" {
                    Ok(Some(json!([
                        {"uid":"cpu-main","title":"CPU","folderUid":"infra","folderTitle":"Infra"},
                        {"uid":"mem-main","title":"Memory","folderUid":"child","folderTitle":"Child"},
                        {"uid":"ops-main","title":"Ops","folderUid":"ops","folderTitle":"Ops"}
                    ])))
                } else {
                    Ok(Some(json!([])))
                }
            }
            (Method::GET, "/api/folders/infra") => Ok(Some(json!({
                "uid":"infra",
                "title":"Infra",
                "parents":[{"uid":"platform","title":"Platform"}]
            }))),
            (Method::GET, "/api/folders/child") => Ok(Some(json!({
                "uid":"child",
                "title":"Child",
                "parents":[{"uid":"platform","title":"Platform"},{"uid":"infra","title":"Infra"}]
            }))),
            (Method::GET, "/api/folders/ops") => Ok(Some(json!({
                "uid":"ops",
                "title":"Ops"
            }))),
            (Method::GET, "/api/folders/platform") => Ok(Some(json!({
                "uid":"platform",
                "title":"Platform"
            }))),
            _ => Err(message(format!("unexpected request {method} {path}"))),
        },
        &args,
    )
    .unwrap();

    assert_eq!(plan.dashboards.len(), 2);
    assert_eq!(plan.folders.len(), 2);
    assert_eq!(plan.dashboards[0].uid, "cpu-main");
    assert_eq!(plan.dashboards[1].uid, "mem-main");
    assert_eq!(plan.folders[0].uid, "child");
    assert_eq!(plan.folders[1].uid, "infra");
}

#[test]
fn dashboard_delete_with_request_deletes_dashboards_then_folders() {
    let args = DeleteArgs {
        common: CommonCliArgs {
            color: crate::common::CliColorChoice::Auto,
            profile: None,
            url: "https://grafana.example.com".to_string(),
            api_token: Some("token".to_string()),
            username: None,
            password: None,
            prompt_password: false,
            prompt_token: false,
            timeout: 30,
            verify_ssl: false,
        },
        page_size: 500,
        org_id: None,
        uid: None,
        path: Some("Platform / Infra".to_string()),
        delete_folders: true,
        yes: true,
        prompt: false,
        dry_run: false,
        table: false,
        json: false,
        output_format: None,
        no_header: false,
    };
    let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = calls.clone();

    let count = delete_dashboards_with_request(
        move |method, path, params, _payload| {
            recorded
                .lock()
                .unwrap()
                .push((method.clone(), path.to_string(), params.to_vec()));
            match (method.clone(), path) {
                (Method::GET, "/api/search") => {
                    let page = params
                        .iter()
                        .find(|(key, _)| key == "page")
                        .map(|(_, value)| value.as_str())
                        .unwrap_or("1");
                    if page == "1" {
                        Ok(Some(json!([
                            {"uid":"cpu-main","title":"CPU","folderUid":"infra","folderTitle":"Infra"},
                            {"uid":"mem-main","title":"Memory","folderUid":"child","folderTitle":"Child"}
                        ])))
                    } else {
                        Ok(Some(json!([])))
                    }
                }
                (Method::GET, "/api/folders/infra") => Ok(Some(json!({
                    "uid":"infra",
                    "title":"Infra",
                    "parents":[{"uid":"platform","title":"Platform"}]
                }))),
                (Method::GET, "/api/folders/child") => Ok(Some(json!({
                    "uid":"child",
                    "title":"Child",
                    "parents":[{"uid":"platform","title":"Platform"},{"uid":"infra","title":"Infra"}]
                }))),
                (Method::GET, "/api/folders/platform") => Ok(Some(json!({
                    "uid":"platform",
                    "title":"Platform"
                }))),
                (Method::DELETE, "/api/dashboards/uid/cpu-main") => {
                    Ok(Some(json!({"status":"success"})))
                }
                (Method::DELETE, "/api/dashboards/uid/mem-main") => {
                    Ok(Some(json!({"status":"success"})))
                }
                (Method::DELETE, "/api/folders/child") => Ok(Some(json!({"status":"success"}))),
                (Method::DELETE, "/api/folders/infra") => Ok(Some(json!({"status":"success"}))),
                _ => Err(message(format!("unexpected request {method} {path}"))),
            }
        },
        &args,
    )
    .unwrap();

    assert_eq!(count, 4);
    let calls = calls.lock().unwrap();
    let delete_paths: Vec<String> = calls
        .iter()
        .filter(|(method, _, _)| *method == Method::DELETE)
        .map(|(_, path, _)| path.clone())
        .collect();
    assert_eq!(
        delete_paths,
        vec![
            "/api/dashboards/uid/cpu-main".to_string(),
            "/api/dashboards/uid/mem-main".to_string(),
            "/api/folders/child".to_string(),
            "/api/folders/infra".to_string(),
        ]
    );
}
