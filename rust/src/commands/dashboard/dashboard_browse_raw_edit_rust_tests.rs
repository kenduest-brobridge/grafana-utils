use super::*;
use serde_json::{json, Value};

mod raw_edit_workflow {
    use super::*;

    #[test]
    fn dashboard_raw_edit_validation_rejects_overwrite_in_user_payload() {
        let error = validate_external_dashboard_edit_value(&json!({
            "dashboard": {
                "uid": "cpu-main",
                "title": "CPU Main"
            },
            "overwrite": true
        }))
        .unwrap_err();

        assert!(error.to_string().contains("must not include overwrite"));
    }

    #[test]
    fn dashboard_raw_edit_review_summarizes_title_tags_and_folder_uid_changes() {
        let draft = ExternalDashboardEditDraft {
            uid: "cpu-main".to_string(),
            title: "CPU Main".to_string(),
            payload: json!({
                "dashboard": {
                    "uid": "cpu-main",
                    "title": "CPU Main",
                    "tags": ["prod"]
                },
                "folderUid": "infra"
            }),
        };

        let review = review_external_dashboard_edit(
            &draft,
            &json!({
                "dashboard": {
                    "uid": "cpu-main",
                    "title": "CPU Overview",
                    "tags": ["gold", "ops"]
                },
                "folderUid": "ops"
            }),
        )
        .unwrap()
        .unwrap();

        assert!(review.summary_lines[0].contains("uid=cpu-main"));
        assert!(review.summary_lines[1].contains("CPU Main -> CPU Overview"));
        assert!(review.summary_lines[3].contains("infra -> ops"));
        assert!(review.summary_lines[4].contains("prod -> gold, ops"));
    }

    #[test]
    fn dashboard_raw_edit_apply_posts_payload_with_overwrite_and_message() {
        let payloads = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
        let recorded = payloads.clone();

        apply_external_dashboard_edit_with_request(
            move |method, path, _params, payload| match (method, path) {
                (Method::POST, "/api/dashboards/db") => {
                    recorded
                        .lock()
                        .unwrap()
                        .push(payload.cloned().unwrap_or(Value::Null));
                    Ok(Some(json!({"status":"success"})))
                }
                _ => Err(message("unexpected request")),
            },
            &json!({
                "dashboard": {
                    "uid": "cpu-main",
                    "title": "CPU Overview",
                    "tags": ["gold", "ops"]
                },
                "folderUid": "ops"
            }),
        )
        .unwrap();

        let payloads = payloads.lock().unwrap();
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0]["dashboard"]["title"], "CPU Overview");
        assert_eq!(payloads[0]["folderUid"], "ops");
        assert_eq!(payloads[0]["overwrite"], true);
        assert_eq!(
            payloads[0]["message"],
            "Edited by grafana-utils dashboard browse"
        );
    }
}
