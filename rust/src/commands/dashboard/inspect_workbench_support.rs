#![cfg(feature = "tui")]
#![cfg_attr(not(test), allow(dead_code))]
use crate::interactive_browser::BrowserItem;

use super::inspect_governance::ExportInspectionGovernanceDocument;
use super::inspect_report::ExportInspectionQueryReport;
use super::inspect_summary::ExportInspectionSummary;

#[path = "inspect_workbench_content.rs"]
mod inspect_workbench_content;

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
    InspectWorkbenchDocument {
        title: "Inspect Workbench".to_string(),
        source_label: source_label.to_string(),
        summary_lines: build_inspect_workbench_summary_lines(source_label, summary, governance),
        groups: build_inspect_workbench_groups(governance, report),
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

fn build_inspect_workbench_summary_lines(
    source_label: &str,
    summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
) -> Vec<String> {
    vec![
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
        "Modes=Overview, Findings, Queries, Dependencies   Use g to switch mode and v to change the current view."
            .to_string(),
    ]
}

fn build_inspect_workbench_groups(
    governance: &ExportInspectionGovernanceDocument,
    report: &ExportInspectionQueryReport,
) -> Vec<InspectWorkbenchGroup> {
    vec![
        InspectWorkbenchGroup {
            kind: "overview".to_string(),
            label: "Overview".to_string(),
            subtitle: "High-level dashboard and datasource review".to_string(),
            views: vec![
                InspectWorkbenchView {
                    label: "Dashboard Summaries".to_string(),
                    items: inspect_workbench_content::build_dashboard_items(governance),
                },
                InspectWorkbenchView {
                    label: "Datasource Usage".to_string(),
                    items: inspect_workbench_content::build_datasource_coverage_items(governance),
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
                    items: inspect_workbench_content::build_finding_items(governance),
                },
                InspectWorkbenchView {
                    label: "Dashboard Summaries".to_string(),
                    items: inspect_workbench_content::build_dashboard_finding_summary_items(
                        governance,
                    ),
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
                    items: inspect_workbench_content::build_query_items(report, false),
                },
                InspectWorkbenchView {
                    label: "Datasource Context".to_string(),
                    items: inspect_workbench_content::build_query_items(report, true),
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
                    items: inspect_workbench_content::build_datasource_coverage_items(governance),
                },
                InspectWorkbenchView {
                    label: "Finding Coverage".to_string(),
                    items: inspect_workbench_content::build_datasource_governance_items(governance),
                },
            ],
        },
    ]
}
