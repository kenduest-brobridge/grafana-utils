#![cfg(feature = "tui")]
use serde_json::Value;

use crate::common::{message, string_field, Result};
use reqwest::Method;

use super::browse_support::{DashboardBrowseNode, DashboardBrowseNodeKind};
use super::history::list_dashboard_history_versions_with_request;
use super::live::fetch_dashboard_with_request;
use super::DEFAULT_DASHBOARD_TITLE;

pub(crate) fn fetch_dashboard_view_lines_with_request<F>(
    mut request_json: F,
    node: &DashboardBrowseNode,
) -> Result<Vec<String>>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    if node.kind != DashboardBrowseNodeKind::Dashboard {
        return Ok(node.details.clone());
    }
    let Some(uid) = node.uid.as_deref() else {
        return Err(message("Dashboard browse requires a dashboard UID."));
    };
    let dashboard = fetch_dashboard_with_request(&mut request_json, uid)?;
    let dashboard_object = dashboard
        .get("dashboard")
        .and_then(Value::as_object)
        .ok_or_else(|| message("Grafana returned a dashboard payload without dashboard data."))?;
    let meta = dashboard
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mut lines = vec![
        "Live details:".to_string(),
        format!("Org: {}", node.org_name),
        format!("Org ID: {}", node.org_id),
        format!(
            "Title: {}",
            string_field(dashboard_object, "title", DEFAULT_DASHBOARD_TITLE)
        ),
        format!("UID: {}", string_field(dashboard_object, "uid", uid)),
        format!(
            "Version: {}",
            dashboard_object
                .get("version")
                .map(Value::to_string)
                .unwrap_or_else(|| "-".to_string())
        ),
        format!("Folder path: {}", node.path),
        format!(
            "Folder UID: {}",
            string_field(&meta, "folderUid", node.uid.as_deref().unwrap_or("-"))
        ),
        format!(
            "Slug: {}",
            string_field(&meta, "slug", "")
                .split('?')
                .next()
                .unwrap_or_default()
        ),
        format!(
            "URL: {}",
            string_field(&meta, "url", node.url.as_deref().unwrap_or("-"))
        ),
    ];

    if let Ok(versions) = list_dashboard_history_versions_with_request(&mut request_json, uid, 5) {
        if !versions.is_empty() {
            lines.push("Recent versions:".to_string());
            for version in versions {
                lines.push(format!(
                    "v{} | {} | {} | {}",
                    version.version,
                    if version.created.is_empty() {
                        "-"
                    } else {
                        &version.created
                    },
                    if version.created_by.is_empty() {
                        "-"
                    } else {
                        &version.created_by
                    },
                    if version.message.is_empty() {
                        "-"
                    } else {
                        &version.message
                    }
                ));
            }
        }
    }

    Ok(lines)
}
