//! Snapshot review browser shaping and interactive launcher helpers.

use serde_json::Value;

use crate::common::Result;

#[cfg(any(feature = "tui", test))]
use crate::interactive_browser::{run_interactive_browser, BrowserItem};

use super::common::{review_summary, review_warnings};
#[cfg(feature = "tui")]
use super::render::build_snapshot_review_summary_lines;

#[cfg_attr(not(any(feature = "tui", test)), allow(dead_code))]
fn snapshot_review_folder_depth(path: &str) -> usize {
    let separator = if path.contains(" / ") { " / " } else { "/" };
    path.split(separator)
        .filter(|segment| !segment.trim().is_empty())
        .count()
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn build_snapshot_review_browser_items(document: &Value) -> Result<Vec<BrowserItem>> {
    let summary = review_summary(document)?;
    let mut items = vec![BrowserItem {
        kind: "snapshot".to_string(),
        title: "Snapshot summary".to_string(),
        meta: format!(
            "{} org(s)  {} dashboard(s)  {} folder(s)  {} datasource(s)",
            summary.get("orgCount").and_then(Value::as_u64).unwrap_or(0),
            summary
                .get("dashboardCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("folderCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("datasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        details: vec![
            format!(
                "Combined orgs: {}",
                summary.get("orgCount").and_then(Value::as_u64).unwrap_or(0)
            ),
            format!(
                "Dashboard orgs: {}",
                summary
                    .get("dashboardOrgCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Datasource orgs: {}",
                summary
                    .get("datasourceOrgCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Dashboards: {}",
                summary
                    .get("dashboardCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Folders: {}",
                summary
                    .get("folderCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Datasources: {}",
                summary
                    .get("datasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Datasource types: {}",
                summary
                    .get("datasourceTypeCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Default datasources: {}",
                summary
                    .get("defaultDatasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Access users: {}",
                summary
                    .get("accessUserCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Access teams: {}",
                summary
                    .get("accessTeamCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Access orgs: {}",
                summary
                    .get("accessOrgCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            format!(
                "Access service accounts: {}",
                summary
                    .get("accessServiceAccountCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
        ],
    }];

    for warning in review_warnings(document) {
        let warning = warning.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review warning entry must be an object.")
        })?;
        let code = warning
            .get("code")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let message = warning
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default();
        items.push(BrowserItem {
            kind: "warning".to_string(),
            title: code.to_string(),
            meta: message.to_string(),
            details: vec![format!("Code: {}", code), format!("Message: {}", message)],
        });
    }

    if let Some(lanes) = document.get("lanes").and_then(Value::as_object) {
        if let Some(dashboard) = lanes.get("dashboard").and_then(Value::as_object) {
            items.push(BrowserItem {
                kind: "lane".to_string(),
                title: "Dashboard lanes".to_string(),
                meta: format!(
                    "raw {}/{}  prompt {}/{}  provisioning {}/{}",
                    dashboard
                        .get("rawScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    dashboard
                        .get("scopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    dashboard
                        .get("promptScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    dashboard
                        .get("scopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    dashboard
                        .get("provisioningScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    dashboard
                        .get("scopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                ),
                details: vec![
                    format!(
                        "Raw scopes: {}/{}",
                        dashboard
                            .get("rawScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        dashboard
                            .get("scopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    ),
                    format!(
                        "Prompt scopes: {}/{}",
                        dashboard
                            .get("promptScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        dashboard
                            .get("scopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    ),
                    format!(
                        "Provisioning scopes: {}/{}",
                        dashboard
                            .get("provisioningScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        dashboard
                            .get("scopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    ),
                ],
            });
        }
        if let Some(datasource) = lanes.get("datasource").and_then(Value::as_object) {
            items.push(BrowserItem {
                kind: "lane".to_string(),
                title: "Datasource lanes".to_string(),
                meta: format!(
                    "inventory {}/{}  provisioning {}/{}",
                    datasource
                        .get("inventoryScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    datasource
                        .get("inventoryExpectedScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    datasource
                        .get("provisioningScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    datasource
                        .get("provisioningExpectedScopeCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                ),
                details: vec![
                    format!(
                        "Inventory scopes: {}/{}",
                        datasource
                            .get("inventoryScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        datasource
                            .get("inventoryExpectedScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    ),
                    format!(
                        "Provisioning scopes: {}/{}",
                        datasource
                            .get("provisioningScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        datasource
                            .get("provisioningExpectedScopeCount")
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    ),
                ],
            });
        }
        if let Some(access) = lanes.get("access").and_then(Value::as_object) {
            if access
                .get("present")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                items.push(BrowserItem {
                    kind: "lane".to_string(),
                    title: "Access lanes".to_string(),
                    meta: format!(
                        "users {}  teams {}  orgs {}  service-accounts {}",
                        access
                            .get("users")
                            .and_then(Value::as_object)
                            .and_then(|lane| lane.get("recordCount"))
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        access
                            .get("teams")
                            .and_then(Value::as_object)
                            .and_then(|lane| lane.get("recordCount"))
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        access
                            .get("orgs")
                            .and_then(Value::as_object)
                            .and_then(|lane| lane.get("recordCount"))
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                        access
                            .get("serviceAccounts")
                            .and_then(Value::as_object)
                            .and_then(|lane| lane.get("recordCount"))
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                    ),
                    details: vec![
                        format!(
                            "Users: {}",
                            access
                                .get("users")
                                .and_then(Value::as_object)
                                .and_then(|lane| lane.get("recordCount"))
                                .and_then(Value::as_u64)
                                .unwrap_or(0)
                        ),
                        format!(
                            "Teams: {}",
                            access
                                .get("teams")
                                .and_then(Value::as_object)
                                .and_then(|lane| lane.get("recordCount"))
                                .and_then(Value::as_u64)
                                .unwrap_or(0)
                        ),
                        format!(
                            "Orgs: {}",
                            access
                                .get("orgs")
                                .and_then(Value::as_object)
                                .and_then(|lane| lane.get("recordCount"))
                                .and_then(Value::as_u64)
                                .unwrap_or(0)
                        ),
                        format!(
                            "Service accounts: {}",
                            access
                                .get("serviceAccounts")
                                .and_then(Value::as_object)
                                .and_then(|lane| lane.get("recordCount"))
                                .and_then(Value::as_u64)
                                .unwrap_or(0)
                        ),
                    ],
                });
            }
        }
    }

    let orgs = document
        .get("orgs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for org in orgs {
        let org = org.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review org entry must be an object.")
        })?;
        let org_name = org
            .get("org")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("unknown");
        let org_id = org
            .get("orgId")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("unknown");
        let dashboard_count = org
            .get("dashboardCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let datasource_count = org
            .get("datasourceCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        items.push(BrowserItem {
            kind: "org".to_string(),
            title: org_name.to_string(),
            meta: format!(
                "orgId={}  dashboards={}  folders={}  datasources={}  defaults={}",
                org_id,
                dashboard_count,
                org.get("folderCount").and_then(Value::as_u64).unwrap_or(0),
                datasource_count,
                org.get("defaultDatasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            details: vec![
                format!("Org: {}", org_name),
                format!("Org ID: {}", org_id),
                format!("Dashboards: {}", dashboard_count),
                format!(
                    "Folders: {}",
                    org.get("folderCount").and_then(Value::as_u64).unwrap_or(0)
                ),
                format!("Datasources: {}", datasource_count),
                format!(
                    "Default datasources: {}",
                    org.get("defaultDatasourceCount")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                ),
                format!(
                    "Datasource types: {}",
                    org.get("datasourceTypes")
                        .and_then(Value::as_object)
                        .map(|types| {
                            types
                                .iter()
                                .map(|(name, count)| {
                                    format!("{}:{}", name, count.as_u64().unwrap_or(0))
                                })
                                .collect::<Vec<String>>()
                                .join(", ")
                        })
                        .unwrap_or_else(|| "none".to_string())
                ),
            ],
        });
    }

    let datasource_types = document
        .get("datasourceTypes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for datasource_type in datasource_types {
        let datasource_type = datasource_type.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review datasource type entry must be an object.")
        })?;
        items.push(BrowserItem {
            kind: "datasource-type".to_string(),
            title: datasource_type
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            meta: format!(
                "count={}",
                datasource_type
                    .get("count")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
            details: vec![
                format!(
                    "Type: {}",
                    datasource_type
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                ),
                format!(
                    "Count: {}",
                    datasource_type
                        .get("count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                ),
            ],
        });
    }

    let datasources = document
        .get("datasources")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for datasource in datasources {
        let datasource = datasource.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review datasource entry must be an object.")
        })?;
        items.push(BrowserItem {
            kind: "datasource".to_string(),
            title: datasource
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            meta: format!(
                "{}  org={}  default={}",
                datasource
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                datasource
                    .get("org")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                if datasource
                    .get("isDefault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "true"
                } else {
                    "false"
                }
            ),
            details: vec![
                format!(
                    "Name: {}",
                    datasource
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "UID: {}",
                    datasource
                        .get("uid")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "Type: {}",
                    datasource
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "Org: {} ({})",
                    datasource
                        .get("org")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    datasource
                        .get("orgId")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "URL: {}",
                    datasource
                        .get("url")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "Access: {}",
                    datasource
                        .get("access")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                ),
                format!(
                    "Default: {}",
                    if datasource
                        .get("isDefault")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        "true"
                    } else {
                        "false"
                    }
                ),
            ],
        });
    }

    for folder in document
        .get("folders")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let folder = folder.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review folder entry must be an object.")
        })?;
        let title = folder
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let path = folder
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let org = folder
            .get("org")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let org_id = folder
            .get("orgId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let uid = folder
            .get("uid")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let depth = snapshot_review_folder_depth(path);
        items.push(BrowserItem {
            kind: "folder".to_string(),
            title: title.to_string(),
            meta: format!("depth={} path={} org={} uid={}", depth, path, org, uid),
            details: vec![
                format!("Title: {}", title),
                format!("Depth: {}", depth),
                format!("Path: {}", path),
                format!("Org: {}", org),
                format!("Org ID: {}", org_id),
                format!("UID: {}", uid),
            ],
        });
    }

    Ok(items)
}

#[cfg(feature = "tui")]
pub(super) fn run_snapshot_review_interactive(document: &Value) -> Result<()> {
    let summary_lines = build_snapshot_review_summary_lines(document)?;
    let items = build_snapshot_review_browser_items(document)?;
    run_interactive_browser("Snapshot review", &summary_lines, &items)
}
