//! External-editor live dashboard edit flow with a safe local-draft default.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::common::{message, string_field, value_as_object, Result};
use crate::http::JsonHttpClient;

use super::authoring::{
    build_live_dashboard_authoring_document,
    review_dashboard_file as build_dashboard_review_from_path, DashboardAuthoringReviewResult,
};
use super::{
    extract_dashboard_object, fetch_dashboard, import_dashboard_request,
    render_dashboard_review_text, EditLiveArgs, DEFAULT_IMPORT_MESSAGE,
};

fn temp_edit_path(uid: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    env::temp_dir().join(format!(
        "grafana-util-dashboard-edit-{uid}-{timestamp}.json"
    ))
}

fn default_output_path(uid: &str) -> PathBuf {
    PathBuf::from(format!("{uid}.edited.json"))
}

fn write_temp_payload(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
    Ok(())
}

fn run_editor_command(path: &Path) -> Result<()> {
    let editor = env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var("EDITOR")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "vi".to_string());
    let mut parts = editor.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| message("Could not resolve an external editor command."))?;
    let mut command = Command::new(program);
    for part in parts {
        command.arg(part);
    }
    let status = command.arg(path).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(message(format!(
            "External editor exited with status {status}."
        )))
    }
}

fn edit_payload_in_external_editor(uid: &str, value: &Value) -> Result<Option<Value>> {
    let temp_path = temp_edit_path(uid);
    write_temp_payload(&temp_path, value)?;
    let result = (|| -> Result<Option<Value>> {
        run_editor_command(&temp_path)?;
        let edited = fs::read_to_string(&temp_path)?;
        let parsed: Value = serde_json::from_str(&edited).map_err(|error| {
            message(format!(
                "Edited dashboard JSON is invalid in {}: {error}",
                temp_path.display()
            ))
        })?;
        if parsed == *value {
            Ok(None)
        } else {
            Ok(Some(parsed))
        }
    })();
    let _ = fs::remove_file(&temp_path);
    result
}

fn edited_dashboard_review(
    source_uid: &str,
    temp_path: &Path,
) -> Result<DashboardAuthoringReviewResult> {
    let mut review = build_dashboard_review_from_path(temp_path)?;
    review.input_file = format!("edited draft for {source_uid}");
    Ok(review)
}

fn join_tags(value: &Value) -> String {
    value
        .as_array()
        .map(|tags| {
            tags.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string())
}

fn summarize_changes(original: &Value, edited: &Value) -> Result<Vec<String>> {
    let original_object = value_as_object(
        original,
        "Original dashboard edit payload must be an object.",
    )?;
    let edited_object = value_as_object(edited, "Edited dashboard payload must be an object.")?;
    let original_dashboard = extract_dashboard_object(original_object)?;
    let edited_dashboard = extract_dashboard_object(edited_object)?;
    let original_tags = join_tags(original_dashboard.get("tags").unwrap_or(&Value::Null));
    let edited_tags = join_tags(edited_dashboard.get("tags").unwrap_or(&Value::Null));
    Ok(vec![
        format!(
            "Dashboard edit review uid={} title={} -> {}",
            string_field(edited_dashboard, "uid", ""),
            string_field(original_dashboard, "title", ""),
            string_field(edited_dashboard, "title", "")
        ),
        format!(
            "Dashboard UID: {} -> {}",
            string_field(original_dashboard, "uid", ""),
            string_field(edited_dashboard, "uid", "")
        ),
        format!(
            "Folder UID: {} -> {}",
            string_field(original_object, "folderUid", "-"),
            string_field(edited_object, "folderUid", "-"),
        ),
        format!("Tags: {} -> {}", original_tags, edited_tags),
    ])
}

fn validate_live_apply_review(
    review: &DashboardAuthoringReviewResult,
    source_uid: &str,
) -> Result<()> {
    if !review.blocking_issues.is_empty() {
        return Err(message(format!(
            "Cannot apply live dashboard {} because review still has blocking issues: {}",
            source_uid,
            review.blocking_issues.join(" | ")
        )));
    }
    if !review.dashboard_id_is_null {
        return Err(message(format!(
            "Cannot apply live dashboard {} because dashboard.id must stay null in the edited draft.",
            source_uid
        )));
    }
    if review.uid != source_uid {
        return Err(message(format!(
            "Cannot apply live dashboard {} because the edited draft changed dashboard.uid to {}.",
            source_uid, review.uid
        )));
    }
    Ok(())
}

pub(crate) fn run_dashboard_edit_live(
    client: Option<&JsonHttpClient>,
    args: &EditLiveArgs,
) -> Result<()> {
    let client =
        client.ok_or_else(|| message("Dashboard edit-live requires a live Grafana client."))?;
    if args.apply_live && !args.yes {
        return Err(message(
            "--apply-live requires --yes because it writes the edited dashboard back to Grafana.",
        ));
    }

    let live_payload = fetch_dashboard(client, &args.dashboard_uid)?;
    let wrapped = build_live_dashboard_authoring_document(&live_payload, None, None, None)?;
    let Some(edited) = edit_payload_in_external_editor(&args.dashboard_uid, &wrapped)? else {
        println!(
            "No dashboard changes detected for {}. Nothing written.",
            args.dashboard_uid
        );
        return Ok(());
    };

    for line in summarize_changes(&wrapped, &edited)? {
        println!("{line}");
    }

    let review_temp_dir = std::env::temp_dir().join(format!(
        "grafana-dashboard-edit-live-review-{}-{}",
        process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| message(format!("Failed to build review temp path: {error}")))?
            .as_nanos()
    ));
    fs::create_dir_all(&review_temp_dir)?;
    let review_path = review_temp_dir.join("dashboard.json");
    let review = (|| -> Result<DashboardAuthoringReviewResult> {
        write_temp_payload(&review_path, &edited)?;
        let mut review = edited_dashboard_review(&args.dashboard_uid, &review_path)?;
        review.input_file = format!("edited draft for {}", args.dashboard_uid);
        Ok(review)
    })();
    let _ = fs::remove_file(&review_path);
    let _ = fs::remove_dir_all(&review_temp_dir);
    let review = review?;
    for line in render_dashboard_review_text(&review) {
        println!("{line}");
    }

    if args.apply_live {
        validate_live_apply_review(&review, &args.dashboard_uid)?;
        let payload = value_as_object(&edited, "Edited dashboard payload must be an object.")?;
        let mut import_payload = payload.clone();
        import_payload.insert("overwrite".to_string(), Value::Bool(true));
        import_payload.insert(
            "message".to_string(),
            Value::String(if args.message.is_empty() {
                DEFAULT_IMPORT_MESSAGE.to_string()
            } else {
                args.message.clone()
            }),
        );
        let _ = import_dashboard_request(client, &Value::Object(import_payload))?;
        println!(
            "Applied edited dashboard {} back to Grafana.",
            args.dashboard_uid
        );
        return Ok(());
    }

    let output = args
        .output
        .clone()
        .unwrap_or_else(|| default_output_path(&args.dashboard_uid));
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_string_pretty(&edited)? + "\n")?;
    println!(
        "Wrote edited dashboard draft for {} to {}.",
        args.dashboard_uid,
        output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn review_fixture() -> DashboardAuthoringReviewResult {
        DashboardAuthoringReviewResult {
            input_file: "edited draft for cpu-main".to_string(),
            document_kind: "wrapped".to_string(),
            title: "CPU Main".to_string(),
            uid: "cpu-main".to_string(),
            folder_uid: Some("infra".to_string()),
            tags: vec!["ops".to_string(), "sre".to_string()],
            dashboard_id_is_null: true,
            meta_message_present: true,
            blocking_issues: Vec::new(),
            suggested_next_action: "publish --dry-run".to_string(),
        }
    }

    #[test]
    fn summarize_changes_reports_uid_title_folder_and_tags() {
        let original = json!({
            "dashboard": {
                "id": null,
                "uid": "cpu-main",
                "title": "CPU Main",
                "tags": ["ops"]
            },
            "folderUid": "infra"
        });
        let edited = json!({
            "dashboard": {
                "id": null,
                "uid": "cpu-main",
                "title": "CPU Main Updated",
                "tags": ["ops", "sre"]
            },
            "folderUid": "platform"
        });

        let lines = summarize_changes(&original, &edited).unwrap();
        assert!(lines.iter().any(|line| line
            .contains("Dashboard edit review uid=cpu-main title=CPU Main -> CPU Main Updated")));
        assert!(lines
            .iter()
            .any(|line| line.contains("Dashboard UID: cpu-main -> cpu-main")));
        assert!(lines
            .iter()
            .any(|line| line.contains("Folder UID: infra -> platform")));
        assert!(lines
            .iter()
            .any(|line| line.contains("Tags: ops -> ops, sre")));
    }

    #[test]
    fn validate_live_apply_review_blocks_on_validation_and_uid_drift() {
        let review = review_fixture();
        validate_live_apply_review(&review, "cpu-main").unwrap();

        let mut blocked = review.clone();
        blocked.blocking_issues = vec!["missing datasource".to_string()];
        assert!(validate_live_apply_review(&blocked, "cpu-main").is_err());

        let mut non_null_id = review.clone();
        non_null_id.dashboard_id_is_null = false;
        assert!(validate_live_apply_review(&non_null_id, "cpu-main").is_err());

        let mut uid_drift = review;
        uid_drift.uid = "cpu-main-clone".to_string();
        assert!(validate_live_apply_review(&uid_drift, "cpu-main").is_err());
    }
}
