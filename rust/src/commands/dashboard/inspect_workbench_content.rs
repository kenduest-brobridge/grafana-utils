#![cfg(feature = "tui")]
#![cfg_attr(not(test), allow(dead_code))]
use crate::interactive_browser::BrowserItem;

use super::super::inspect_governance::ExportInspectionGovernanceDocument;
use super::super::inspect_render::{bool_text, join_or_none};
use super::super::inspect_report::ExportInspectionQueryReport;

pub(crate) fn build_dashboard_items(
    governance: &ExportInspectionGovernanceDocument,
) -> Vec<BrowserItem> {
    governance
        .dashboard_governance
        .iter()
        .map(|row| BrowserItem {
            kind: "dashboard-summary".to_string(),
            title: row.dashboard_title.clone(),
            meta: format!(
                "uid={} findings={} ds-families={}",
                row.dashboard_uid, row.risk_count, row.datasource_family_count
            ),
            details: vec![
                fact("Dashboard UID", &row.dashboard_uid),
                fact("Title", &row.dashboard_title),
                fact("Folder", &row.folder_path),
                fact("Panels", row.panel_count),
                fact("Queries", row.query_count),
                fact("Datasources", join_or_none(&row.datasources, ", ")),
                fact("Families", join_or_none(&row.datasource_families, ", ")),
                fact(
                    "Mixed Datasource",
                    bool_text(row.mixed_datasource, "yes", "no"),
                ),
                fact("Finding Count", row.risk_count),
                fact("Finding Kinds", join_or_none(&row.risk_kinds, ", ")),
            ],
        })
        .collect()
}

pub(crate) fn build_dashboard_finding_summary_items(
    governance: &ExportInspectionGovernanceDocument,
) -> Vec<BrowserItem> {
    governance
        .dashboard_governance
        .iter()
        .filter(|row| row.risk_count != 0)
        .map(|row| BrowserItem {
            kind: "dashboard-finding-summary".to_string(),
            title: row.dashboard_title.clone(),
            meta: format!("uid={} findings={}", row.dashboard_uid, row.risk_count),
            details: vec![
                fact("Dashboard UID", &row.dashboard_uid),
                fact("Title", &row.dashboard_title),
                fact("Folder", &row.folder_path),
                fact("Finding Count", row.risk_count),
                fact("Finding Kinds", join_or_none(&row.risk_kinds, ", ")),
                fact("Datasources", join_or_none(&row.datasources, ", ")),
                fact(
                    "Datasource Families",
                    join_or_none(&row.datasource_families, ", "),
                ),
            ],
        })
        .collect()
}

pub(crate) fn build_query_items(
    report: &ExportInspectionQueryReport,
    datasource_view: bool,
) -> Vec<BrowserItem> {
    let mut items = report
        .queries
        .iter()
        .map(|row| {
            let title = if datasource_view {
                format!(
                    "{} / {} / {}",
                    blank_or(&row.datasource_name, "(unknown datasource)"),
                    row.dashboard_title,
                    row.panel_title
                )
            } else {
                format!("{} / {}", row.dashboard_title, row.panel_title)
            };
            let meta = if datasource_view {
                format!(
                    "{} {} panel={} metrics={}",
                    row.datasource_family,
                    row.ref_id,
                    row.panel_id,
                    row.metrics.len()
                )
            } else {
                format!(
                    "{} {} ds={} panel={}",
                    row.datasource_family,
                    row.ref_id,
                    blank_or(&row.datasource_name, "-"),
                    row.panel_id
                )
            };
            BrowserItem {
                kind: "query".to_string(),
                title,
                meta,
                details: vec![
                    fact("Org", blank_or(&row.org, "-")),
                    fact("Dashboard UID", &row.dashboard_uid),
                    fact("Dashboard", &row.dashboard_title),
                    fact("Folder", &row.folder_path),
                    fact("Panel ID", &row.panel_id),
                    fact("Panel", &row.panel_title),
                    fact("Panel Type", &row.panel_type),
                    fact("Ref ID", &row.ref_id),
                    fact("Datasource", blank_or(&row.datasource_name, "-")),
                    fact("Datasource UID", blank_or(&row.datasource_uid, "-")),
                    fact("Datasource Family", blank_or(&row.datasource_family, "-")),
                    fact("Query Field", blank_or(&row.query_field, "-")),
                    fact("Metrics", join_or_none(&row.metrics, ", ")),
                    fact("Functions", join_or_none(&row.functions, ", ")),
                    fact("Measurements", join_or_none(&row.measurements, ", ")),
                    fact("Buckets", join_or_none(&row.buckets, ", ")),
                    fact("Variables", join_or_none(&row.query_variables, ", ")),
                    String::new(),
                    fact("Query", blank_or(&row.query_text, "-")),
                ],
            }
        })
        .collect::<Vec<_>>();
    if datasource_view {
        items.sort_by(|left, right| {
            left.title
                .cmp(&right.title)
                .then(left.meta.cmp(&right.meta))
        });
    }
    items
}

pub(crate) fn build_finding_items(
    governance: &ExportInspectionGovernanceDocument,
) -> Vec<BrowserItem> {
    let mut items = Vec::new();

    let mut risks = governance.risk_records.clone();
    risks.sort_by(|left, right| {
        right
            .severity
            .cmp(&left.severity)
            .then_with(|| left.dashboard_uid.cmp(&right.dashboard_uid))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.panel_id.cmp(&right.panel_id))
    });
    items.extend(risks.into_iter().map(|risk| BrowserItem {
        kind: "finding".to_string(),
        title: format!("{} / {}", risk.dashboard_uid, risk.kind),
        meta: format!("severity={} panel={}", risk.severity, risk.panel_id),
        details: vec![
            fact("Kind", &risk.kind),
            fact("Severity", &risk.severity),
            fact("Category", &risk.category),
            fact("Dashboard UID", &risk.dashboard_uid),
            fact("Panel ID", &risk.panel_id),
            fact("Datasource", blank_or(&risk.datasource, "-")),
            fact("Detail", &risk.detail),
            fact("Recommendation", &risk.recommendation),
        ],
    }));
    items.extend(governance.query_audits.iter().map(|audit| BrowserItem {
        kind: "query-review".to_string(),
        title: format!(
            "{} / {} / {}",
            audit.dashboard_title, audit.panel_title, audit.ref_id
        ),
        meta: format!("severity={} score={}", audit.severity, audit.score),
        details: vec![
            fact("Dashboard UID", &audit.dashboard_uid),
            fact("Dashboard", &audit.dashboard_title),
            fact("Folder", &audit.folder_path),
            fact("Panel ID", &audit.panel_id),
            fact("Panel", &audit.panel_title),
            fact("Ref ID", &audit.ref_id),
            fact("Datasource", blank_or(&audit.datasource, "-")),
            fact("Datasource UID", blank_or(&audit.datasource_uid, "-")),
            fact("Datasource Family", blank_or(&audit.datasource_family, "-")),
            fact("Aggregation Depth", audit.aggregation_depth),
            fact("Regex Matcher Count", audit.regex_matcher_count),
            fact("Estimated Series Risk", &audit.estimated_series_risk),
            fact("Query Cost Score", audit.query_cost_score),
            fact("Score", audit.score),
            fact("Severity", &audit.severity),
            fact("Reasons", join_or_none(&audit.reasons, ", ")),
            fact(
                "Recommendations",
                join_or_none(&audit.recommendations, ", "),
            ),
        ],
    }));
    items
}

pub(crate) fn build_datasource_coverage_items(
    governance: &ExportInspectionGovernanceDocument,
) -> Vec<BrowserItem> {
    let mut items = governance
        .datasources
        .iter()
        .map(|row| BrowserItem {
            kind: "datasource-usage".to_string(),
            title: blank_or(&row.datasource, "(unknown datasource)").to_string(),
            meta: format!(
                "{} uid={} queries={} dashboards={}",
                blank_or(&row.family, "unknown"),
                blank_or(&row.datasource_uid, "-"),
                row.query_count,
                row.dashboard_count
            ),
            details: vec![
                fact("Datasource", blank_or(&row.datasource, "-")),
                fact("Datasource UID", blank_or(&row.datasource_uid, "-")),
                fact("Family", blank_or(&row.family, "-")),
                fact("Query Count", row.query_count),
                fact("Dashboard Count", row.dashboard_count),
                fact("Panel Count", row.panel_count),
                fact("Dashboard UIDs", join_or_none(&row.dashboard_uids, ", ")),
                fact("Query Fields", join_or_none(&row.query_fields, ", ")),
                fact("Orphaned", bool_text(row.orphaned, "yes", "no")),
            ],
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.title.cmp(&right.title));
    items
}

pub(crate) fn build_datasource_governance_items(
    governance: &ExportInspectionGovernanceDocument,
) -> Vec<BrowserItem> {
    let mut items = governance
        .datasource_governance
        .iter()
        .map(|row| BrowserItem {
            kind: "datasource-finding-coverage".to_string(),
            title: blank_or(&row.datasource, "(unknown datasource)").to_string(),
            meta: format!(
                "{} findings={} mixed={} orphaned={}",
                blank_or(&row.family, "unknown"),
                row.risk_count,
                row.mixed_dashboard_count,
                bool_text(row.orphaned, "yes", "no")
            ),
            details: vec![
                fact("Datasource", blank_or(&row.datasource, "-")),
                fact("Datasource UID", blank_or(&row.datasource_uid, "-")),
                fact("Family", blank_or(&row.family, "-")),
                fact("Query Count", row.query_count),
                fact("Dashboard Count", row.dashboard_count),
                fact("Panel Count", row.panel_count),
                fact("Mixed Dashboard Count", row.mixed_dashboard_count),
                fact("Folder Count", row.folder_count),
                fact(
                    "High Blast Radius",
                    bool_text(row.high_blast_radius, "yes", "no"),
                ),
                fact("Cross Folder", bool_text(row.cross_folder, "yes", "no")),
                fact("Folder Paths", join_or_none(&row.folder_paths, ", ")),
                fact("Finding Count", row.risk_count),
                fact("Finding Kinds", join_or_none(&row.risk_kinds, ", ")),
                fact("Dashboard UIDs", join_or_none(&row.dashboard_uids, ", ")),
                fact(
                    "Dashboard Titles",
                    join_or_none(&row.dashboard_titles, ", "),
                ),
                fact("Orphaned", bool_text(row.orphaned, "yes", "no")),
            ],
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.title.cmp(&right.title));
    items
}

fn fact(label: &str, value: impl std::fmt::Display) -> String {
    format!("{label}: {value}")
}

fn blank_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() {
        fallback
    } else {
        value
    }
}
