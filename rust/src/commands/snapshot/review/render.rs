//! Snapshot review text rendering helpers.

use serde_json::Value;

use crate::common::Result;

use super::common::{review_summary, review_warnings};

pub fn render_snapshot_review_text(document: &Value) -> Result<Vec<String>> {
    let summary = review_summary(document)?;
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
    let warnings = review_warnings(document);
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
                    .unwrap_or_else(|| "none".to_string())
            ));
        }
    }
    Ok(lines)
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn build_snapshot_review_summary_lines(document: &Value) -> Result<Vec<String>> {
    let summary = review_summary(document)?;
    let warnings = review_warnings(document);
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
