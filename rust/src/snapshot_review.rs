//! Snapshot review rendering and interactive-output helpers.

use serde_json::Value;

use crate::common::{render_json_value, Result};
#[cfg(any(feature = "tui", test))]
use crate::interactive_browser::{run_interactive_browser, BrowserItem};
use crate::overview::OverviewOutputFormat;
use crate::tabular_output::{print_lines, render_csv, render_table, render_yaml};

pub fn render_snapshot_review_text(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some(super::SNAPSHOT_REVIEW_KIND) {
        return Err(crate::common::message(
            "Snapshot review document kind is not supported.",
        ));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| crate::common::message("Snapshot review document is missing summary."))?;
    let mut lines = vec![
        "Snapshot review".to_string(),
        format!(
            "Org coverage: {} combined org(s), {} dashboard org(s), {} datasource org(s)",
            summary.get("orgCount").and_then(Value::as_u64).unwrap_or(0),
            summary
                .get("dashboardOrgCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("datasourceOrgCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        format!(
            "Totals: {} dashboard(s), {} folder(s), {} datasource(s)",
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
        format!(
            "Datasource profile: {} type(s), {} default datasource(s)",
            summary
                .get("datasourceTypeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("defaultDatasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        format!(
            "Access totals: {} user(s), {} team(s), {} org(s), {} service-account(s)",
            summary
                .get("accessUserCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("accessTeamCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("accessOrgCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("accessServiceAccountCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
    ];
    if let Some(lanes) = document.get("lanes").and_then(Value::as_object) {
        let dashboard = lanes
            .get("dashboard")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let datasource = lanes
            .get("datasource")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        lines.push(format!(
            "Dashboard lanes: raw {}/{}, prompt {}/{}, provisioning {}/{}",
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
        ));
        lines.push(format!(
            "Datasource lanes: inventory {}/{}, provisioning {}/{}",
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
        ));
        if let Some(access) = lanes.get("access").and_then(Value::as_object) {
            if !access
                .get("present")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                // Old snapshots may not carry access lanes.
            } else {
                lines.push(format!(
                    "Access lanes: users {}, teams {}, orgs {}, service-accounts {}",
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
                ));
            }
        }
    }
    let datasource_types = document
        .get("datasourceTypes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !datasource_types.is_empty() {
        let summary_text = datasource_types
            .iter()
            .filter_map(|item| {
                item.as_object().map(|item| {
                    format!(
                        "{}:{}",
                        item.get("type")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown"),
                        item.get("count").and_then(Value::as_u64).unwrap_or(0)
                    )
                })
            })
            .collect::<Vec<String>>()
            .join(", ");
        lines.push(format!("Datasource types: {summary_text}"));
    }
    let warnings = document
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if warnings.is_empty() {
        lines.push("Warnings: none".to_string());
    } else {
        lines.push(format!("Warnings: {}", warnings.len()));
        for warning in warnings {
            let warning = warning.as_object().ok_or_else(|| {
                crate::common::message("Snapshot review warning entry must be an object.")
            })?;
            lines.push(format!(
                "- {}: {}",
                warning
                    .get("code")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                warning.get("message").and_then(Value::as_str).unwrap_or("")
            ));
        }
    }
    let orgs = document
        .get("orgs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !orgs.is_empty() {
        lines.push(String::new());
        lines.push("# Orgs".to_string());
        for org in orgs {
            let org = org.as_object().ok_or_else(|| {
                crate::common::message("Snapshot review org entry must be an object.")
            })?;
            lines.push(format!(
                "- org={} orgId={} dashboards={} folders={} datasources={} defaults={} types={}",
                org.get("org").and_then(Value::as_str).unwrap_or("unknown"),
                org.get("orgId")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                org.get("dashboardCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                org.get("folderCount").and_then(Value::as_u64).unwrap_or(0),
                org.get("datasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                org.get("defaultDatasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0),
                org.get("datasourceTypes")
                    .and_then(Value::as_object)
                    .map(|types| {
                        types
                            .iter()
                            .map(|(name, count)| {
                                format!("{}:{}", name, count.as_u64().unwrap_or(0))
                            })
                            .collect::<Vec<String>>()
                            .join(",")
                    })
                    .unwrap_or_default(),
            ));
        }
    }
    Ok(lines)
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn build_snapshot_review_summary_lines(document: &Value) -> Result<Vec<String>> {
    if document.get("kind").and_then(Value::as_str) != Some(super::SNAPSHOT_REVIEW_KIND) {
        return Err(crate::common::message(
            "Snapshot review document kind is not supported.",
        ));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| crate::common::message("Snapshot review document is missing summary."))?;
    let warnings = document
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(vec![
        format!(
            "Org coverage: {} combined org(s), {} dashboard org(s), {} datasource org(s)",
            summary.get("orgCount").and_then(Value::as_u64).unwrap_or(0),
            summary
                .get("dashboardOrgCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("datasourceOrgCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        format!(
            "Totals: {} dashboard(s), {} folder(s), {} datasource(s)",
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
        format!(
            "Datasource profile: {} type(s), {} default datasource(s)",
            summary
                .get("datasourceTypeCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("defaultDatasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        format!(
            "Access totals: {} user(s), {} team(s), {} org(s), {} service-account(s)",
            summary
                .get("accessUserCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("accessTeamCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("accessOrgCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("accessServiceAccountCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
        if warnings.is_empty() {
            "Warnings: none".to_string()
        } else {
            format!("Warnings: {}", warnings.len())
        },
    ])
}

#[cfg_attr(not(any(feature = "tui", test)), allow(dead_code))]
fn snapshot_review_folder_depth(path: &str) -> usize {
    let separator = if path.contains(" / ") { " / " } else { "/" };
    path.split(separator)
        .filter(|segment| !segment.trim().is_empty())
        .count()
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn build_snapshot_review_browser_items(document: &Value) -> Result<Vec<BrowserItem>> {
    if document.get("kind").and_then(Value::as_str) != Some(super::SNAPSHOT_REVIEW_KIND) {
        return Err(crate::common::message(
            "Snapshot review document kind is not supported.",
        ));
    }

    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| crate::common::message("Snapshot review document is missing summary."))?;
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

    let warnings = document
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for warning in &warnings {
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
fn run_snapshot_review_interactive(document: &Value) -> Result<()> {
    let summary_lines = build_snapshot_review_summary_lines(document)?;
    let items = build_snapshot_review_browser_items(document)?;
    run_interactive_browser("Snapshot review", &summary_lines, &items)
}

pub(crate) fn emit_snapshot_review_output(
    document: &Value,
    output: OverviewOutputFormat,
) -> Result<()> {
    match output {
        OverviewOutputFormat::Table => {
            print_lines(&render_table(
                &[
                    "ROW_KIND", "NAME", "STATUS", "PRIMARY", "BLOCKERS", "WARNINGS", "DETAIL",
                ],
                &build_snapshot_review_tabular_rows(document)?,
            ));
        }
        OverviewOutputFormat::Csv => {
            print_lines(&render_csv(
                &[
                    "row_kind", "name", "status", "primary", "blockers", "warnings", "detail",
                ],
                &build_snapshot_review_tabular_rows(document)?,
            ));
        }
        OverviewOutputFormat::Json => print!("{}", render_json_value(document)?),
        OverviewOutputFormat::Text => {
            for line in render_snapshot_review_text(document)? {
                println!("{line}");
            }
        }
        OverviewOutputFormat::Yaml => println!("{}", render_yaml(document)?),
        #[cfg(feature = "tui")]
        OverviewOutputFormat::Interactive => {
            run_snapshot_review_interactive(document)?;
        }
    }
    Ok(())
}

fn build_snapshot_review_tabular_rows(document: &Value) -> Result<Vec<Vec<String>>> {
    if document.get("kind").and_then(Value::as_str) != Some(super::SNAPSHOT_REVIEW_KIND) {
        return Err(crate::common::message(
            "Snapshot review document kind is not supported.",
        ));
    }
    let summary = document
        .get("summary")
        .and_then(Value::as_object)
        .ok_or_else(|| crate::common::message("Snapshot review document is missing summary."))?;
    let mut rows = vec![vec![
        "overall".to_string(),
        "snapshot".to_string(),
        "ready".to_string(),
        summary
            .get("dashboardCount")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .to_string(),
        document
            .get("warnings")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0)
            .to_string(),
        summary
            .get("defaultDatasourceCount")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .to_string(),
        format!(
            "orgs={} datasources={} access-users={} access-teams={} access-orgs={} access-service-accounts={}",
            summary.get("orgCount").and_then(Value::as_u64).unwrap_or(0),
            summary
                .get("datasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("accessUserCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("accessTeamCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("accessOrgCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            summary
                .get("accessServiceAccountCount")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        ),
    ]];
    for org in document
        .get("orgs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let org = org.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review org entry must be an object.")
        })?;
        rows.push(vec![
            "org".to_string(),
            org.get("org")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            org.get("orgId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            org.get("dashboardCount")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .to_string(),
            org.get("folderCount")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .to_string(),
            org.get("datasourceCount")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .to_string(),
            format!(
                "defaults={}",
                org.get("defaultDatasourceCount")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            ),
        ]);
    }
    for warning in document
        .get("warnings")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let warning = warning.as_object().ok_or_else(|| {
            crate::common::message("Snapshot review warning entry must be an object.")
        })?;
        rows.push(vec![
            "warning".to_string(),
            warning
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            "warning".to_string(),
            String::new(),
            String::new(),
            String::new(),
            warning
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ]);
    }
    if let Some(access) = document
        .get("lanes")
        .and_then(Value::as_object)
        .and_then(|lanes| lanes.get("access"))
        .and_then(Value::as_object)
    {
        if access
            .get("present")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            let user_count = access
                .get("users")
                .and_then(Value::as_object)
                .and_then(|lane| lane.get("recordCount"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let team_count = access
                .get("teams")
                .and_then(Value::as_object)
                .and_then(|lane| lane.get("recordCount"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let org_count = access
                .get("orgs")
                .and_then(Value::as_object)
                .and_then(|lane| lane.get("recordCount"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let service_account_count = access
                .get("serviceAccounts")
                .and_then(Value::as_object)
                .and_then(|lane| lane.get("recordCount"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            rows.push(vec![
                "lane".to_string(),
                "access".to_string(),
                "ready".to_string(),
                user_count.to_string(),
                String::new(),
                String::new(),
                format!(
                    "users={} teams={} orgs={} serviceAccounts={}",
                    user_count, team_count, org_count, service_account_count
                ),
            ]);
        }
    }
    Ok(rows)
}
