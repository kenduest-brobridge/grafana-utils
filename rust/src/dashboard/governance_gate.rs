//! Dashboard governance gate evaluator.
//! Consumes governance-json and query-report JSON artifacts plus a small policy JSON.
use serde::Serialize;
use serde_json::Value;
#[cfg(any(feature = "tui", test))]
use std::cmp::Reverse;

use crate::common::{load_json_object_file, message, render_json_value, Result};

use super::governance_gate_rules as rules;
#[cfg(all(feature = "tui", not(test)))]
use super::governance_gate_tui::run_governance_gate_interactive;
use super::{
    load_governance_policy, write_json_document, GovernanceGateArgs, GovernanceGateOutputFormat,
};
#[cfg(test)]
use crate::interactive_browser::run_interactive_browser;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DashboardGovernanceGateSummary {
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "queryRecordCount")]
    pub(crate) query_record_count: usize,
    #[serde(rename = "violationCount")]
    pub(crate) violation_count: usize,
    #[serde(rename = "warningCount")]
    pub(crate) warning_count: usize,
    #[serde(rename = "checkedRules")]
    pub(crate) checked_rules: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DashboardGovernanceGateFinding {
    pub(crate) severity: String,
    pub(crate) code: String,
    pub(crate) message: String,
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    pub(crate) dashboard_title: String,
    #[serde(rename = "panelId")]
    pub(crate) panel_id: String,
    #[serde(rename = "panelTitle")]
    pub(crate) panel_title: String,
    #[serde(rename = "refId")]
    pub(crate) ref_id: String,
    pub(crate) datasource: String,
    #[serde(rename = "datasourceUid")]
    pub(crate) datasource_uid: String,
    #[serde(rename = "datasourceFamily")]
    pub(crate) datasource_family: String,
    #[serde(rename = "riskKind", skip_serializing_if = "String::is_empty")]
    pub(crate) risk_kind: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DashboardGovernanceGateResult {
    pub(crate) ok: bool,
    pub(crate) summary: DashboardGovernanceGateSummary,
    pub(crate) violations: Vec<DashboardGovernanceGateFinding>,
    pub(crate) warnings: Vec<DashboardGovernanceGateFinding>,
}

fn load_object(path: &std::path::Path, label: &str) -> Result<Value> {
    load_json_object_file(path, label)
}

#[cfg(any(feature = "tui", test))]
fn field_or_dash(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "-"
    } else {
        trimmed
    }
}

#[cfg(any(feature = "tui", test))]
fn shorten_text(value: &str, max_chars: usize) -> String {
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let head = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{head}...")
    } else {
        head
    }
}

#[cfg(any(feature = "tui", test))]
fn paired_label(value: &str, label_name: &str, label_value: &str) -> String {
    let value = value.trim();
    let label_value = label_value.trim();
    match (value.is_empty(), label_value.is_empty()) {
        (true, true) => "-".to_string(),
        (false, true) => value.to_string(),
        (true, false) => format!("{label_name}={label_value}"),
        (false, false) => format!("{value} ({label_name}={label_value})"),
    }
}

#[cfg(any(feature = "tui", test))]
fn datasource_label(record: &DashboardGovernanceGateFinding) -> String {
    let mut parts = Vec::new();
    if !record.datasource.is_empty() {
        parts.push(format!("name={}", shorten_text(&record.datasource, 32)));
    }
    if !record.datasource_uid.is_empty() {
        parts.push(format!("uid={}", record.datasource_uid));
    }
    if !record.datasource_family.is_empty() {
        parts.push(format!("family={}", record.datasource_family));
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(" ")
    }
}

#[cfg(any(feature = "tui", test))]
fn finding_context_score(record: &DashboardGovernanceGateFinding) -> usize {
    [
        !record.dashboard_uid.trim().is_empty(),
        !record.dashboard_title.trim().is_empty(),
        !record.panel_id.trim().is_empty(),
        !record.panel_title.trim().is_empty(),
        !record.ref_id.trim().is_empty(),
        !record.datasource.trim().is_empty(),
        !record.datasource_uid.trim().is_empty(),
        !record.datasource_family.trim().is_empty(),
        !record.risk_kind.trim().is_empty(),
    ]
    .into_iter()
    .filter(|value| *value)
    .count()
}

#[cfg(any(feature = "tui", test))]
fn finding_scope_title(record: &DashboardGovernanceGateFinding) -> String {
    let mut parts = Vec::new();
    if !record.dashboard_title.trim().is_empty() || !record.dashboard_uid.trim().is_empty() {
        parts.push(paired_label(
            &record.dashboard_title,
            "uid",
            &record.dashboard_uid,
        ));
    }
    if !record.panel_title.trim().is_empty() || !record.panel_id.trim().is_empty() {
        parts.push(paired_label(&record.panel_title, "id", &record.panel_id));
    }
    if parts.is_empty() {
        if !record.ref_id.trim().is_empty() {
            parts.push(format!("ref={}", record.ref_id.trim()));
        } else if !record.datasource.trim().is_empty() {
            parts.push(shorten_text(&record.datasource, 32));
        } else if !record.datasource_family.trim().is_empty() {
            parts.push(record.datasource_family.trim().to_string());
        } else {
            parts.push("unscoped".to_string());
        }
    }
    parts.join(" / ")
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn finding_sort_key(
    record: &DashboardGovernanceGateFinding,
) -> (
    u8,
    Reverse<usize>,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
) {
    let severity_rank = match record.severity.as_str() {
        "error" => 0,
        "warning" => 1,
        _ => 2,
    };
    (
        severity_rank,
        Reverse(finding_context_score(record)),
        record.dashboard_title.to_ascii_lowercase(),
        record.dashboard_uid.to_ascii_lowercase(),
        record.panel_title.to_ascii_lowercase(),
        record.panel_id.to_ascii_lowercase(),
        record.ref_id.to_ascii_lowercase(),
        record.code.to_ascii_lowercase(),
        record.message.to_ascii_lowercase(),
    )
}

#[cfg(any(feature = "tui", test))]
fn finding_row_title(record: &DashboardGovernanceGateFinding) -> String {
    finding_scope_title(record)
}

#[cfg(any(feature = "tui", test))]
fn finding_row_meta(record: &DashboardGovernanceGateFinding) -> String {
    let mut parts = Vec::new();
    if !record.severity.trim().is_empty() {
        parts.push(format!("sev={}", record.severity.trim()));
    }
    if !record.code.trim().is_empty() {
        parts.push(format!("code={}", record.code.trim()));
    }
    if !record.ref_id.trim().is_empty() {
        parts.push(format!("ref={}", record.ref_id.trim()));
    }
    if !record.dashboard_uid.trim().is_empty() {
        parts.push(format!("dashboardUid={}", record.dashboard_uid.trim()));
    } else if !record.dashboard_title.trim().is_empty() {
        parts.push(format!(
            "dashboard={}",
            shorten_text(&record.dashboard_title, 32)
        ));
    }
    if !record.panel_id.trim().is_empty() {
        parts.push(format!("panelId={}", record.panel_id.trim()));
    } else if !record.panel_title.trim().is_empty() {
        parts.push(format!("panel={}", shorten_text(&record.panel_title, 32)));
    }
    if !record.datasource.trim().is_empty() {
        parts.push(format!("ds={}", shorten_text(&record.datasource, 24)));
    }
    if !record.datasource_uid.trim().is_empty() {
        parts.push(format!("dsUid={}", record.datasource_uid.trim()));
    }
    if !record.datasource_family.trim().is_empty() {
        parts.push(format!("family={}", record.datasource_family.trim()));
    }
    if !record.risk_kind.trim().is_empty() {
        parts.push(format!("risk={}", record.risk_kind.trim()));
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(" ")
    }
}

#[cfg(any(feature = "tui", test))]
fn finding_row_details(record: &DashboardGovernanceGateFinding) -> Vec<String> {
    vec![
        format!("Scope: {}", finding_scope_title(record)),
        format!("Reason: {}", field_or_dash(&record.message)),
        format!("Severity: {}", field_or_dash(&record.severity)),
        format!("Code: {}", field_or_dash(&record.code)),
        format!("Risk kind: {}", field_or_dash(&record.risk_kind)),
        format!(
            "Dashboard: {}",
            paired_label(&record.dashboard_title, "uid", &record.dashboard_uid)
        ),
        format!(
            "Panel: {}",
            paired_label(&record.panel_title, "id", &record.panel_id)
        ),
        format!("Ref ID: {}", field_or_dash(&record.ref_id)),
        format!("Datasource: {}", datasource_label(record)),
    ]
}

#[cfg(any(feature = "tui", test))]
pub(crate) fn build_browser_item(
    kind: &str,
    record: &DashboardGovernanceGateFinding,
) -> crate::interactive_browser::BrowserItem {
    crate::interactive_browser::BrowserItem {
        kind: kind.to_string(),
        title: finding_row_title(record),
        meta: finding_row_meta(record),
        details: finding_row_details(record),
    }
}

pub(crate) fn evaluate_dashboard_governance_gate(
    policy: &Value,
    governance_document: &Value,
    query_document: &Value,
) -> Result<DashboardGovernanceGateResult> {
    let policy = rules::parse_query_threshold_policy(policy)?;
    let queries = query_document
        .get("queries")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Dashboard query report JSON must contain a queries array."))?;
    let dashboard_count = query_document
        .get("summary")
        .and_then(|summary| summary.get("dashboardCount"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let query_record_count = query_document
        .get("summary")
        .and_then(|summary| summary.get("queryRecordCount"))
        .or_else(|| {
            query_document
                .get("summary")
                .and_then(|summary| summary.get("reportRowCount"))
        })
        .and_then(Value::as_u64)
        .unwrap_or(queries.len() as u64) as usize;

    let violations = rules::evaluate_dashboard_governance_gate_violations(
        &policy,
        governance_document,
        queries,
    )?;
    let warnings = rules::build_governance_warning_findings(governance_document)?;

    let ok = violations.is_empty() && (!policy.fail_on_warnings || warnings.is_empty());
    Ok(DashboardGovernanceGateResult {
        ok,
        summary: DashboardGovernanceGateSummary {
            dashboard_count,
            query_record_count,
            violation_count: violations.len(),
            warning_count: warnings.len(),
            checked_rules: rules::build_checked_rules(&policy),
        },
        violations,
        warnings,
    })
}

pub(crate) fn render_dashboard_governance_gate_result(
    result: &DashboardGovernanceGateResult,
) -> String {
    let mut lines = vec![
        format!(
            "Dashboard governance gate: {}",
            if result.ok { "PASS" } else { "FAIL" }
        ),
        format!(
            "Dashboards: {}  Queries: {}  Violations: {}  Warnings: {}",
            result.summary.dashboard_count,
            result.summary.query_record_count,
            result.summary.violation_count,
            result.summary.warning_count
        ),
    ];
    if !result.violations.is_empty() {
        lines.push(String::new());
        lines.push("Violations:".to_string());
        for record in &result.violations {
            lines.push(format!(
                "  ERROR [{}] dashboard={} panel={} datasource={}: {}",
                record.code,
                if record.dashboard_uid.is_empty() {
                    "-"
                } else {
                    &record.dashboard_uid
                },
                if record.panel_id.is_empty() {
                    "-"
                } else {
                    &record.panel_id
                },
                if record.datasource_uid.is_empty() {
                    "-"
                } else {
                    &record.datasource_uid
                },
                record.message
            ));
        }
    }
    if !result.warnings.is_empty() {
        lines.push(String::new());
        lines.push("Warnings:".to_string());
        for record in &result.warnings {
            lines.push(format!(
                "  WARN [{}] dashboard={} panel={} datasource={}: {}",
                if record.risk_kind.is_empty() {
                    &record.code
                } else {
                    &record.risk_kind
                },
                if record.dashboard_uid.is_empty() {
                    "-"
                } else {
                    &record.dashboard_uid
                },
                if record.panel_id.is_empty() {
                    "-"
                } else {
                    &record.panel_id
                },
                if record.datasource.is_empty() {
                    "-"
                } else {
                    &record.datasource
                },
                record.message
            ));
        }
    }
    lines.join("\n")
}

pub(crate) fn run_dashboard_governance_gate(args: &GovernanceGateArgs) -> Result<()> {
    let policy = load_governance_policy(args)?;
    let governance = load_object(&args.governance, "Dashboard governance JSON")?;
    let queries = load_object(&args.queries, "Dashboard query report JSON")?;
    let result = evaluate_dashboard_governance_gate(&policy, &governance, &queries)?;

    if let Some(output_path) = args.json_output.as_ref() {
        write_json_document(&result, output_path)?;
    }
    if args.interactive {
        #[cfg(all(feature = "tui", not(test)))]
        {
            run_governance_gate_interactive(&result)?;
            return if result.ok {
                Ok(())
            } else {
                Err(message(
                    "Dashboard governance gate reported policy violations.",
                ))
            };
        }
        #[cfg(test)]
        {
            let summary_lines = render_dashboard_governance_gate_result(&result)
                .lines()
                .map(|line| line.to_string())
                .collect::<Vec<_>>();
            run_interactive_browser(
                "Dashboard Governance Gate",
                &summary_lines,
                &super::governance_gate_tui::build_governance_gate_tui_items(&result, "all"),
            )?;
            return if result.ok {
                Ok(())
            } else {
                Err(message(
                    "Dashboard governance gate reported policy violations.",
                ))
            };
        }
        #[cfg(not(feature = "tui"))]
        {
            return super::tui_not_built("governance-gate --interactive");
        }
    }
    match args.output_format {
        GovernanceGateOutputFormat::Json => {
            println!("{}", render_json_value(&result)?);
        }
        GovernanceGateOutputFormat::Text => {
            println!("{}", render_dashboard_governance_gate_result(&result));
        }
    }
    if result.ok {
        Ok(())
    } else {
        Err(message(
            "Dashboard governance gate reported policy violations.",
        ))
    }
}
