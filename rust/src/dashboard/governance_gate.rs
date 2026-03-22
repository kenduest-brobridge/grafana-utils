//! Dashboard governance gate evaluator.
//! Consumes governance-json and query-report JSON artifacts plus a small policy JSON.
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use crate::common::{message, Result};

use super::{write_json_document, GovernanceGateArgs, GovernanceGateOutputFormat};

#[derive(Clone)]
struct QueryThresholdPolicy {
    allowed_families: BTreeSet<String>,
    allowed_uids: BTreeSet<String>,
    forbid_unknown: bool,
    forbid_mixed_families: bool,
    max_queries_per_dashboard: Option<usize>,
    max_queries_per_panel: Option<usize>,
    fail_on_warnings: bool,
}

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

fn load_object(path: &std::path::Path) -> Result<Value> {
    let raw = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&raw)?;
    if !value.is_object() {
        return Err(message(format!(
            "JSON document at {} must be an object.",
            path.display()
        )));
    }
    Ok(value)
}

fn value_to_usize(value: Option<&Value>) -> Result<Option<usize>> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number
            .as_u64()
            .map(|value| Some(value as usize))
            .ok_or_else(|| message("Expected a non-negative integer in governance policy.")),
        Some(other) => Err(message(format!(
            "Expected a non-negative integer in governance policy, got {other}."
        ))),
    }
}

fn value_to_bool(value: Option<&Value>, default: bool) -> Result<bool> {
    match value {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(flag)) => Ok(*flag),
        Some(other) => Err(message(format!(
            "Expected a boolean in governance policy, got {other}."
        ))),
    }
}

fn value_to_string_set(value: Option<&Value>) -> Result<BTreeSet<String>> {
    match value {
        None | Some(Value::Null) => Ok(BTreeSet::new()),
        Some(Value::Array(values)) => Ok(values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect()),
        Some(other) => Err(message(format!(
            "Expected an array of strings in governance policy, got {other}."
        ))),
    }
}

fn parse_query_threshold_policy(policy: &Value) -> Result<QueryThresholdPolicy> {
    let Some(policy_object) = policy.as_object() else {
        return Err(message("Governance policy JSON must be an object."));
    };
    if let Some(version) = policy_object.get("version") {
        match version {
            Value::Number(number) if number.as_i64() == Some(1) => {}
            _ => {
                return Err(message(
                    "Governance policy version is not supported. Expected version 1.",
                ))
            }
        }
    }
    let queries = policy_object
        .get("queries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let datasources = policy_object
        .get("datasources")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let enforcement = policy_object
        .get("enforcement")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    Ok(QueryThresholdPolicy {
        allowed_families: value_to_string_set(datasources.get("allowedFamilies"))?,
        allowed_uids: value_to_string_set(datasources.get("allowedUids"))?,
        forbid_unknown: value_to_bool(datasources.get("forbidUnknown"), false)?,
        forbid_mixed_families: value_to_bool(datasources.get("forbidMixedFamilies"), false)?,
        max_queries_per_dashboard: value_to_usize(queries.get("maxQueriesPerDashboard"))?,
        max_queries_per_panel: value_to_usize(queries.get("maxQueriesPerPanel"))?,
        fail_on_warnings: value_to_bool(enforcement.get("failOnWarnings"), false)?,
    })
}

fn string_field(record: &Value, key: &str) -> String {
    record
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

fn build_checked_rules(policy: QueryThresholdPolicy) -> Value {
    serde_json::json!({
        "datasourceAllowedFamilies": policy.allowed_families,
        "datasourceAllowedUids": policy.allowed_uids,
        "forbidUnknown": policy.forbid_unknown,
        "forbidMixedFamilies": policy.forbid_mixed_families,
        "maxQueriesPerDashboard": policy.max_queries_per_dashboard,
        "maxQueriesPerPanel": policy.max_queries_per_panel,
        "failOnWarnings": policy.fail_on_warnings,
    })
}

fn build_query_violation(
    code: &str,
    message_text: String,
    query: &Value,
) -> DashboardGovernanceGateFinding {
    DashboardGovernanceGateFinding {
        severity: "error".to_string(),
        code: code.to_string(),
        message: message_text,
        dashboard_uid: string_field(query, "dashboardUid"),
        dashboard_title: string_field(query, "dashboardTitle"),
        panel_id: string_field(query, "panelId"),
        panel_title: string_field(query, "panelTitle"),
        ref_id: string_field(query, "refId"),
        datasource: string_field(query, "datasource"),
        datasource_uid: string_field(query, "datasourceUid"),
        datasource_family: string_field(query, "datasourceFamily"),
        risk_kind: String::new(),
    }
}

fn build_dashboard_violation(
    code: &str,
    message_text: String,
    dashboard: &Value,
) -> DashboardGovernanceGateFinding {
    DashboardGovernanceGateFinding {
        severity: "error".to_string(),
        code: code.to_string(),
        message: message_text,
        dashboard_uid: string_field(dashboard, "dashboardUid"),
        dashboard_title: string_field(dashboard, "dashboardTitle"),
        panel_id: String::new(),
        panel_title: String::new(),
        ref_id: String::new(),
        datasource: String::new(),
        datasource_uid: String::new(),
        datasource_family: String::new(),
        risk_kind: String::new(),
    }
}

pub(crate) fn evaluate_dashboard_governance_gate(
    policy: &Value,
    governance_document: &Value,
    query_document: &Value,
) -> Result<DashboardGovernanceGateResult> {
    let policy = parse_query_threshold_policy(policy)?;
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

    let mut dashboard_counts = BTreeMap::<String, (String, usize)>::new();
    let mut panel_counts = BTreeMap::<(String, String), (String, String, usize)>::new();
    let mut violations = Vec::new();
    for query in queries {
        let dashboard_uid = string_field(query, "dashboardUid");
        let dashboard_title = string_field(query, "dashboardTitle");
        let panel_id = string_field(query, "panelId");
        let panel_title = string_field(query, "panelTitle");
        let datasource = string_field(query, "datasource");
        let datasource_uid = string_field(query, "datasourceUid");
        let datasource_family = string_field(query, "datasourceFamily");
        let dashboard_entry = dashboard_counts
            .entry(dashboard_uid.clone())
            .or_insert((dashboard_title.clone(), 0usize));
        dashboard_entry.1 += 1;
        let panel_entry = panel_counts.entry((dashboard_uid, panel_id)).or_insert((
            dashboard_title,
            panel_title,
            0usize,
        ));
        panel_entry.2 += 1;

        if policy.forbid_unknown
            && (datasource_family.is_empty()
                || datasource_family.eq_ignore_ascii_case("unknown")
                || datasource.is_empty())
        {
            violations.push(build_query_violation(
                "datasource-unknown",
                "Datasource identity could not be resolved for this query row.".to_string(),
                query,
            ));
        }
        if !policy.allowed_families.is_empty()
            && !policy.allowed_families.contains(&datasource_family)
        {
            let family = if datasource_family.is_empty() {
                "unknown".to_string()
            } else {
                datasource_family.clone()
            };
            violations.push(build_query_violation(
                "datasource-family-not-allowed",
                format!("Datasource family {family} is not allowed by policy."),
                query,
            ));
        }
        if !policy.allowed_uids.is_empty()
            && !datasource_uid.is_empty()
            && !policy.allowed_uids.contains(&datasource_uid)
        {
            violations.push(build_query_violation(
                "datasource-uid-not-allowed",
                format!("Datasource uid {datasource_uid} is not allowed by policy."),
                query,
            ));
        }
    }

    if let Some(limit) = policy.max_queries_per_dashboard {
        for (dashboard_uid, (dashboard_title, query_count)) in &dashboard_counts {
            if *query_count > limit {
                violations.push(DashboardGovernanceGateFinding {
                    severity: "error".to_string(),
                    code: "max-queries-per-dashboard".to_string(),
                    message: format!(
                        "Dashboard query count {query_count} exceeds policy maxQueriesPerDashboard={limit}."
                    ),
                    dashboard_uid: dashboard_uid.clone(),
                    dashboard_title: dashboard_title.clone(),
                    panel_id: String::new(),
                    panel_title: String::new(),
                    ref_id: String::new(),
                    datasource: String::new(),
                    datasource_uid: String::new(),
                    datasource_family: String::new(),
                    risk_kind: String::new(),
                });
            }
        }
    }
    if let Some(limit) = policy.max_queries_per_panel {
        for ((dashboard_uid, panel_id), (dashboard_title, panel_title, query_count)) in
            &panel_counts
        {
            if *query_count > limit {
                violations.push(DashboardGovernanceGateFinding {
                    severity: "error".to_string(),
                    code: "max-queries-per-panel".to_string(),
                    message: format!(
                        "Panel query count {query_count} exceeds policy maxQueriesPerPanel={limit}."
                    ),
                    dashboard_uid: dashboard_uid.clone(),
                    dashboard_title: dashboard_title.clone(),
                    panel_id: panel_id.clone(),
                    panel_title: panel_title.clone(),
                    ref_id: String::new(),
                    datasource: String::new(),
                    datasource_uid: String::new(),
                    datasource_family: String::new(),
                    risk_kind: String::new(),
                });
            }
        }
    }

    if policy.forbid_mixed_families {
        let dashboards = governance_document
            .get("dashboardGovernance")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                message("Dashboard governance JSON must contain a dashboardGovernance array.")
            })?;
        for dashboard in dashboards {
            let mixed = dashboard
                .get("mixedDatasource")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if mixed {
                let families = dashboard
                    .get("datasourceFamilies")
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<&str>>()
                            .join(",")
                    })
                    .unwrap_or_default();
                violations.push(build_dashboard_violation(
                    "mixed-datasource-families-not-allowed",
                    format!(
                        "Dashboard uses mixed datasource families{}{}.",
                        if families.is_empty() { "" } else { ": " },
                        families
                    ),
                    dashboard,
                ));
            }
        }
    }

    let warnings = governance_document
        .get("riskRecords")
        .and_then(Value::as_array)
        .ok_or_else(|| message("Dashboard governance JSON must contain a riskRecords array."))?
        .iter()
        .map(|record| DashboardGovernanceGateFinding {
            severity: "warning".to_string(),
            code: string_field(record, "kind"),
            message: record
                .get("recommendation")
                .and_then(Value::as_str)
                .map(str::to_string)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| {
                    let detail = string_field(record, "detail");
                    if detail.is_empty() {
                        "Governance warning surfaced from inspect report.".to_string()
                    } else {
                        detail
                    }
                }),
            dashboard_uid: string_field(record, "dashboardUid"),
            dashboard_title: String::new(),
            panel_id: string_field(record, "panelId"),
            panel_title: String::new(),
            ref_id: String::new(),
            datasource: string_field(record, "datasource"),
            datasource_uid: String::new(),
            datasource_family: String::new(),
            risk_kind: string_field(record, "kind"),
        })
        .collect::<Vec<DashboardGovernanceGateFinding>>();

    let ok = violations.is_empty() && !(policy.fail_on_warnings && !warnings.is_empty());
    Ok(DashboardGovernanceGateResult {
        ok,
        summary: DashboardGovernanceGateSummary {
            dashboard_count,
            query_record_count,
            violation_count: violations.len(),
            warning_count: warnings.len(),
            checked_rules: build_checked_rules(policy),
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
    let policy = load_object(&args.policy)?;
    let governance = load_object(&args.governance)?;
    let queries = load_object(&args.queries)?;
    let result = evaluate_dashboard_governance_gate(&policy, &governance, &queries)?;

    if let Some(output_path) = args.json_output.as_ref() {
        write_json_document(&result, output_path)?;
    }
    match args.output_format {
        GovernanceGateOutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
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
