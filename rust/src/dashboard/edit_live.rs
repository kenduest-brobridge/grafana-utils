//! External-editor live dashboard edit flow with a safe local-draft default.

use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::common::{
    json_color_choice, json_color_enabled, message, string_field, value_as_object, Result,
};
use crate::http::JsonHttpClient;

use super::authoring::{
    build_live_dashboard_authoring_document,
    review_dashboard_file as build_dashboard_review_from_path, DashboardAuthoringReviewResult,
};
use super::{
    extract_dashboard_object, fetch_dashboard, import_dashboard_request,
    publish_dashboard_with_client, EditLiveArgs, PublishArgs, DEFAULT_IMPORT_MESSAGE,
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

fn write_temp_payload(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
    Ok(())
}

// Resolve VISUAL/EDITOR and execute it against a single live-edit payload path.
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

const ANSI_RESET: &str = "\x1b[0m";
const ANSI_HEADER: &str = "\x1b[1;36m";
const ANSI_SUCCESS: &str = "\x1b[1;32m";
const ANSI_WARNING: &str = "\x1b[1;33m";
const ANSI_ERROR: &str = "\x1b[1;31m";
const ANSI_DIM: &str = "\x1b[2;90m";
const ANSI_LABEL: &str = "\x1b[1;37m";
const ANSI_VALUE: &str = "\x1b[0;37m";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditStatusTone {
    Success,
    Warning,
    Error,
}

fn style_enabled() -> bool {
    json_color_enabled(json_color_choice(), std::io::stdout().is_terminal())
}

fn paint(text: &str, ansi: &str, enabled: bool) -> String {
    if enabled {
        format!("{ansi}{text}{ANSI_RESET}")
    } else {
        text.to_string()
    }
}

fn render_section_heading(title: &str, enabled: bool) -> String {
    format!(
        "{} {}",
        paint("==", ANSI_HEADER, enabled),
        paint(title, ANSI_HEADER, enabled)
    )
}

fn render_status_line(tone: EditStatusTone, text: &str, enabled: bool) -> String {
    let (label, ansi) = match tone {
        EditStatusTone::Success => ("OK", ANSI_SUCCESS),
        EditStatusTone::Warning => ("INFO", ANSI_WARNING),
        EditStatusTone::Error => ("ERROR", ANSI_ERROR),
    };
    format!("{} {}", paint(label, ansi, enabled), text)
}

fn render_key_value(label: &str, value: &str, enabled: bool) -> String {
    format!(
        "{} {}",
        paint(&format!("{label}:"), ANSI_LABEL, enabled),
        paint(value, ANSI_VALUE, enabled)
    )
}

fn summarize_changes(original: &Value, edited: &Value) -> Result<Vec<(String, String)>> {
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
        (
            "Dashboard UID".to_string(),
            format!(
                "{} -> {}",
                string_field(original_dashboard, "uid", ""),
                string_field(edited_dashboard, "uid", "")
            ),
        ),
        (
            "Title".to_string(),
            format!(
                "{} -> {}",
                string_field(original_dashboard, "title", ""),
                string_field(edited_dashboard, "title", "")
            ),
        ),
        (
            "Folder UID".to_string(),
            format!(
                "{} -> {}",
                string_field(original_object, "folderUid", "-"),
                string_field(edited_object, "folderUid", "-"),
            ),
        ),
        (
            "Tags".to_string(),
            format!("{original_tags} -> {edited_tags}"),
        ),
    ])
}

fn render_change_summary(
    source_uid: &str,
    original: &Value,
    edited: &Value,
    enabled: bool,
) -> Result<Vec<String>> {
    let mut lines = vec![
        render_section_heading("Edit Summary", enabled),
        render_status_line(
            EditStatusTone::Success,
            &format!("Captured edited dashboard draft for {source_uid}."),
            enabled,
        ),
    ];
    for (label, value) in summarize_changes(original, edited)? {
        lines.push(render_key_value(&label, &value, enabled));
    }
    Ok(lines)
}

fn render_review_summary(review: &DashboardAuthoringReviewResult, enabled: bool) -> Vec<String> {
    let tags = if review.tags.is_empty() {
        "-".to_string()
    } else {
        review.tags.join(", ")
    };
    let mut lines = vec![render_section_heading("Review", enabled)];
    lines.push(render_key_value("File", &review.input_file, enabled));
    lines.push(render_key_value("Kind", &review.document_kind, enabled));
    lines.push(render_key_value("Title", &review.title, enabled));
    lines.push(render_key_value("UID", &review.uid, enabled));
    lines.push(render_key_value(
        "Folder UID",
        review.folder_uid.as_deref().unwrap_or("-"),
        enabled,
    ));
    lines.push(render_key_value("Tags", &tags, enabled));
    lines.push(render_key_value(
        "dashboard.id",
        if review.dashboard_id_is_null {
            "null"
        } else {
            "non-null"
        },
        enabled,
    ));
    lines.push(render_key_value(
        "meta.message",
        if review.meta_message_present {
            "present"
        } else {
            "absent"
        },
        enabled,
    ));
    if review.blocking_issues.is_empty() {
        lines.push(render_status_line(
            EditStatusTone::Success,
            "Blocking issues: none",
            enabled,
        ));
    } else {
        lines.push(render_status_line(
            EditStatusTone::Error,
            &format!("Blocking issues: {}", review.blocking_issues.len()),
            enabled,
        ));
        for issue in &review.blocking_issues {
            lines.push(format!("  - {issue}"));
        }
    }
    lines.push(render_key_value(
        "Next action",
        &review.suggested_next_action,
        enabled,
    ));
    lines
}

fn render_final_status(
    source_uid: &str,
    output_path: Option<&Path>,
    apply_live: bool,
    dry_run_only: bool,
    enabled: bool,
) -> Vec<String> {
    let mut lines = vec![render_section_heading("Result", enabled)];
    if apply_live {
        lines.push(render_status_line(
            EditStatusTone::Success,
            &format!("Applied edited dashboard {source_uid} back to Grafana."),
            enabled,
        ));
        lines.push(paint(
            "A new Grafana revision should now exist in live history.",
            ANSI_DIM,
            enabled,
        ));
        return lines;
    }

    if dry_run_only {
        lines.push(render_status_line(
            EditStatusTone::Success,
            &format!("Prepared edited dashboard preview for {source_uid}."),
            enabled,
        ));
        lines.push(paint(
            "No local draft file was written. The next output block is the live publish dry-run preview.",
            ANSI_DIM,
            enabled,
        ));
        return lines;
    }

    let path_text = output_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "-".to_string());
    lines.push(render_status_line(
        EditStatusTone::Success,
        &format!("Wrote edited dashboard draft for {source_uid}."),
        enabled,
    ));
    lines.push(render_key_value("Output", &path_text, enabled));
    lines.push(paint(
        "Nothing was written back to Grafana. Use publish or edit-live --apply-live when ready.",
        ANSI_DIM,
        enabled,
    ));
    lines
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

fn preview_temp_publish_path(uid: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    env::temp_dir().join(format!(
        "grafana-util-dashboard-edit-preview-{uid}-{timestamp}.json"
    ))
}

fn build_publish_args(input: PathBuf, source: &EditLiveArgs, dry_run: bool) -> PublishArgs {
    PublishArgs {
        common: source.common.clone(),
        input,
        replace_existing: false,
        folder_uid: None,
        message: source.message.clone(),
        dry_run,
        watch: false,
        table: false,
        json: false,
    }
}

// Launch interactive live dashboard edit flow:
// fetch live payload, open editable draft, validate/apply review, then optionally publish.
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
        let enabled = style_enabled();
        println!("{}", render_section_heading("Result", enabled));
        println!(
            "{}",
            render_status_line(
                EditStatusTone::Warning,
                &format!("No dashboard changes detected for {}.", args.dashboard_uid),
                enabled
            )
        );
        println!(
            "{}",
            paint(
                "Nothing was written to disk or back to Grafana.",
                ANSI_DIM,
                enabled
            )
        );
        return Ok(());
    };

    let enabled = style_enabled();

    for line in render_change_summary(&args.dashboard_uid, &wrapped, &edited, enabled)? {
        println!("{line}");
    }
    println!();

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
    for line in render_review_summary(&review, enabled) {
        println!("{line}");
    }
    println!();

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
        for line in render_final_status(&args.dashboard_uid, None, true, false, enabled) {
            println!("{line}");
        }
        return Ok(());
    }

    if let Some(output) = args.output.clone() {
        if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        fs::write(&output, serde_json::to_string_pretty(&edited)? + "\n")?;
        for line in render_final_status(&args.dashboard_uid, Some(&output), false, false, enabled) {
            println!("{line}");
        }
        if args.publish_dry_run {
            println!();
            println!("{}", render_section_heading("Publish Dry Run", enabled));
            println!(
                "{}",
                paint(
                    "Replaying the saved draft through the live publish dry-run path.",
                    ANSI_DIM,
                    enabled
                )
            );
            let publish_args = build_publish_args(output, args, true);
            publish_dashboard_with_client(client, &publish_args)?;
        }
        return Ok(());
    }

    for line in render_final_status(&args.dashboard_uid, None, false, true, enabled) {
        println!("{line}");
    }
    println!();
    println!("{}", render_section_heading("Publish Dry Run", enabled));
    println!(
        "{}",
        paint(
            "Replaying the edited dashboard through the live publish dry-run path.",
            ANSI_DIM,
            enabled
        )
    );
    let preview_path = preview_temp_publish_path(&args.dashboard_uid);
    let preview_result = (|| -> Result<()> {
        write_temp_payload(&preview_path, &edited)?;
        let publish_args = build_publish_args(preview_path.clone(), args, true);
        publish_dashboard_with_client(client, &publish_args)
    })();
    let _ = fs::remove_file(&preview_path);
    preview_result
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
        assert!(lines
            .iter()
            .any(|(label, value)| label == "Title" && value == "CPU Main -> CPU Main Updated"));
        assert!(lines
            .iter()
            .any(|(label, value)| label == "Dashboard UID" && value == "cpu-main -> cpu-main"));
        assert!(lines
            .iter()
            .any(|(label, value)| label == "Folder UID" && value == "infra -> platform"));
        assert!(lines
            .iter()
            .any(|(label, value)| label == "Tags" && value == "ops -> ops, sre"));
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

    #[test]
    fn render_review_summary_highlights_next_action_and_clean_review() {
        let lines = render_review_summary(&review_fixture(), false);
        assert_eq!(lines.first().unwrap(), "== Review");
        assert!(lines.iter().any(|line| line == "OK Blocking issues: none"));
        assert!(lines
            .iter()
            .any(|line| line == "Next action: publish --dry-run"));
    }

    #[test]
    fn render_final_status_for_local_draft_mentions_output_and_no_live_write() {
        let lines = render_final_status(
            "cpu-main",
            Some(Path::new("./drafts/cpu-main.edited.json")),
            false,
            false,
            false,
        );
        assert_eq!(lines.first().unwrap(), "== Result");
        assert!(lines
            .iter()
            .any(|line| line == "OK Wrote edited dashboard draft for cpu-main."));
        assert!(lines
            .iter()
            .any(|line| { line == "Output: ./drafts/cpu-main.edited.json" }));
        assert!(lines
            .iter()
            .any(|line| { line.contains("Nothing was written back to Grafana") }));
    }

    #[test]
    fn render_final_status_for_dry_run_preview_mentions_no_file_written() {
        let lines = render_final_status("cpu-main", None, false, true, false);
        assert_eq!(lines.first().unwrap(), "== Result");
        assert!(lines
            .iter()
            .any(|line| line == "OK Prepared edited dashboard preview for cpu-main."));
        assert!(lines
            .iter()
            .any(|line| line.contains("No local draft file was written")));
    }

    #[test]
    fn build_publish_args_reuses_connection_and_sets_dry_run_mode() {
        let args = EditLiveArgs {
            common: super::super::CommonCliArgs {
                color: crate::common::CliColorChoice::Auto,
                profile: Some("prod".to_string()),
                url: "https://grafana.example.com".to_string(),
                api_token: Some("token".to_string()),
                username: None,
                password: None,
                prompt_password: false,
                prompt_token: false,
                timeout: 30,
                verify_ssl: true,
            },
            dashboard_uid: "cpu-main".to_string(),
            output: None,
            apply_live: false,
            publish_dry_run: false,
            message: "Preview CPU edit".to_string(),
            yes: false,
        };

        let publish_args =
            build_publish_args(PathBuf::from("./cpu-main.preview.json"), &args, true);
        assert_eq!(publish_args.input, PathBuf::from("./cpu-main.preview.json"));
        assert!(publish_args.dry_run);
        assert_eq!(publish_args.message, "Preview CPU edit");
        assert_eq!(publish_args.common.url, "https://grafana.example.com");
        assert_eq!(publish_args.common.profile.as_deref(), Some("prod"));
    }
}
