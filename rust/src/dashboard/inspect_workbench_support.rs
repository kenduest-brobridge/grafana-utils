#![cfg(feature = "tui")]
#![cfg_attr(not(test), allow(dead_code))]
use crate::interactive_browser::BrowserItem;

use super::inspect_governance::ExportInspectionGovernanceDocument;
use super::inspect_report::ExportInspectionQueryReport;
use super::inspect_summary::ExportInspectionSummary;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InspectWorkbenchDocument {
    pub(crate) title: String,
    pub(crate) source_label: String,
    pub(crate) summary_lines: Vec<String>,
    pub(crate) groups: Vec<InspectWorkbenchGroup>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InspectWorkbenchGroup {
    pub(crate) kind: String,
    pub(crate) label: String,
    pub(crate) subtitle: String,
    pub(crate) views: Vec<InspectWorkbenchView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InspectWorkbenchView {
    pub(crate) label: String,
    pub(crate) items: Vec<BrowserItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InspectLiveGroup {
    pub(crate) label: String,
    pub(crate) kind: String,
    pub(crate) count: usize,
    pub(crate) subtitle: String,
}

pub(crate) fn build_inspect_workbench_document(
    source_label: &str,
    summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
    report: &ExportInspectionQueryReport,
) -> InspectWorkbenchDocument {
    let groups = vec![
        InspectWorkbenchGroup {
            kind: "overview".to_string(),
            label: "Overview".to_string(),
            subtitle: "High-level dashboard and datasource review".to_string(),
            views: vec![
                InspectWorkbenchView {
                    label: "Dashboard Summaries".to_string(),
                    items: build_dashboard_items(governance),
                },
                InspectWorkbenchView {
                    label: "Datasource Usage".to_string(),
                    items: build_datasource_coverage_items(governance),
                },
            ],
        },
        InspectWorkbenchGroup {
            kind: "findings".to_string(),
            label: "Findings".to_string(),
            subtitle: "Governance findings and query reviews needing attention".to_string(),
            views: vec![
                InspectWorkbenchView {
                    label: "Finding Details".to_string(),
                    items: build_finding_items(governance),
                },
                InspectWorkbenchView {
                    label: "Dashboard Summaries".to_string(),
                    items: build_dashboard_finding_summary_items(governance),
                },
            ],
        },
        InspectWorkbenchGroup {
            kind: "queries".to_string(),
            label: "Queries".to_string(),
            subtitle: "Extracted query rows and datasource context".to_string(),
            views: vec![
                InspectWorkbenchView {
                    label: "Dashboard Context".to_string(),
                    items: build_query_items(report, false),
                },
                InspectWorkbenchView {
                    label: "Datasource Context".to_string(),
                    items: build_query_items(report, true),
                },
            ],
        },
        InspectWorkbenchGroup {
            kind: "dependencies".to_string(),
            label: "Dependencies".to_string(),
            subtitle: "Datasource usage concentration and governance coverage".to_string(),
            views: vec![
                InspectWorkbenchView {
                    label: "Usage Coverage".to_string(),
                    items: build_datasource_coverage_items(governance),
                },
                InspectWorkbenchView {
                    label: "Finding Coverage".to_string(),
                    items: build_datasource_governance_items(governance),
                },
            ],
        },
    ];

    InspectWorkbenchDocument {
        title: "Inspect Workbench".to_string(),
        source_label: source_label.to_string(),
        summary_lines: vec![
            format!(
                "Source={}   dashboards={} panels={} queries={}",
                source_label, summary.dashboard_count, summary.panel_count, summary.query_count
            ),
            format!(
                "datasource-families={} datasource-inventory={} findings={} query-reviews={}",
                governance.summary.datasource_family_count,
                governance.summary.datasource_inventory_count,
                governance.summary.risk_record_count,
                governance.summary.query_audit_count
            ),
            "Modes: Overview, Findings, Queries, and Dependencies. Use v to switch the current mode view."
                .to_string(),
        ],
        groups,
    }
}

pub(crate) fn build_inspect_live_tui_groups(
    summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
    report: &ExportInspectionQueryReport,
) -> Vec<InspectLiveGroup> {
    let document = build_inspect_workbench_document("live snapshot", summary, governance, report);
    document
        .groups
        .into_iter()
        .map(|group| InspectLiveGroup {
            count: group
                .views
                .first()
                .map(|view| view.items.len())
                .unwrap_or(0),
            label: group.label,
            kind: group.kind,
            subtitle: group.subtitle,
        })
        .collect()
}

pub(crate) fn filter_inspect_live_tui_items(
    summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
    report: &ExportInspectionQueryReport,
    group_kind: &str,
) -> Vec<BrowserItem> {
    let document = build_inspect_workbench_document("live snapshot", summary, governance, report);
    document
        .groups
        .into_iter()
        .find(|group| group.kind == group_kind)
        .and_then(|group| group.views.into_iter().next())
        .map(|view| view.items)
        .unwrap_or_default()
}

fn build_dashboard_items(governance: &ExportInspectionGovernanceDocument) -> Vec<BrowserItem> {
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
                fact("Datasources", join_or_none(&row.datasources)),
                fact("Families", join_or_none(&row.datasource_families)),
                fact("Mixed Datasource", yes_no(row.mixed_datasource)),
                fact("Finding Count", row.risk_count),
                fact("Finding Kinds", join_or_none(&row.risk_kinds)),
            ],
        })
        .collect()
}

fn build_dashboard_finding_summary_items(
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
                fact("Finding Kinds", join_or_none(&row.risk_kinds)),
                fact("Datasources", join_or_none(&row.datasources)),
                fact(
                    "Datasource Families",
                    join_or_none(&row.datasource_families),
                ),
            ],
        })
        .collect()
}

fn build_query_items(
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
                    fact("Metrics", join_or_none(&row.metrics)),
                    fact("Functions", join_or_none(&row.functions)),
                    fact("Measurements", join_or_none(&row.measurements)),
                    fact("Buckets", join_or_none(&row.buckets)),
                    fact("Variables", join_or_none(&row.query_variables)),
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

fn build_finding_items(governance: &ExportInspectionGovernanceDocument) -> Vec<BrowserItem> {
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
            fact("Reasons", join_or_none(&audit.reasons)),
            fact("Recommendations", join_or_none(&audit.recommendations)),
        ],
    }));
    items
}

fn build_datasource_coverage_items(
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
                fact("Dashboard UIDs", join_or_none(&row.dashboard_uids)),
                fact("Query Fields", join_or_none(&row.query_fields)),
                fact("Orphaned", yes_no(row.orphaned)),
            ],
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.title.cmp(&right.title));
    items
}

fn build_datasource_governance_items(
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
                yes_no(row.orphaned)
            ),
            details: vec![
                fact("Datasource", blank_or(&row.datasource, "-")),
                fact("Datasource UID", blank_or(&row.datasource_uid, "-")),
                fact("Family", blank_or(&row.family, "-")),
                fact("Query Count", row.query_count),
                fact("Dashboard Count", row.dashboard_count),
                fact("Panel Count", row.panel_count),
                fact("Mixed Dashboard Count", row.mixed_dashboard_count),
                fact("Finding Count", row.risk_count),
                fact("Finding Kinds", join_or_none(&row.risk_kinds)),
                fact("Dashboard UIDs", join_or_none(&row.dashboard_uids)),
                fact("Orphaned", yes_no(row.orphaned)),
            ],
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.title.cmp(&right.title));
    items
}

fn fact(label: &str, value: impl std::fmt::Display) -> String {
    format!("{label}: {value}")
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "(none)".to_string()
    } else {
        values.join(", ")
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn blank_or<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() {
        fallback
    } else {
        value
    }
}
