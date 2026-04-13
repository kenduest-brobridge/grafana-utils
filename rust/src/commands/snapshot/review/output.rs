//! Snapshot review output routing and tabular shaping helpers.

use serde_json::Value;

use crate::common::{render_json_value, Result};
use crate::overview::OverviewOutputFormat;
use crate::tabular_output::{print_lines, render_csv, render_table, render_yaml};

#[cfg(feature = "tui")]
use super::browser::run_snapshot_review_interactive;
use super::common::{review_summary, review_warnings};
use super::render::render_snapshot_review_text;

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
    let summary = review_summary(document)?;
    let mut rows = vec![vec![
        "overall".to_string(),
        "snapshot".to_string(),
        "ready".to_string(),
        summary
            .get("dashboardCount")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .to_string(),
        review_warnings(document).len().to_string(),
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
    for warning in review_warnings(document) {
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
