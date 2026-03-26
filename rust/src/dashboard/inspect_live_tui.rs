#![cfg(feature = "tui")]
#![allow(dead_code)]
use crate::common::Result;
use crate::dashboard::inspect_report::ExportInspectionQueryReport;
use crate::interactive_browser::BrowserItem;

use super::inspect_governance::ExportInspectionGovernanceDocument;
use super::inspect_workbench::run_inspect_workbench;
use super::inspect_workbench_support::{
    build_inspect_live_tui_groups as build_shared_groups, build_inspect_workbench_document,
    filter_inspect_live_tui_items as filter_shared_items, InspectLiveGroup,
};
use super::ExportInspectionSummary;

pub(crate) fn build_inspect_live_tui_groups(
    summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
    report: &ExportInspectionQueryReport,
) -> Vec<InspectLiveGroup> {
    build_shared_groups(summary, governance, report)
}

pub(crate) fn filter_inspect_live_tui_items(
    summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
    report: &ExportInspectionQueryReport,
    group_kind: &str,
) -> Vec<BrowserItem> {
    filter_shared_items(summary, governance, report, group_kind)
}

pub(crate) fn run_inspect_live_interactive(
    summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
    report: &ExportInspectionQueryReport,
) -> Result<()> {
    let document = build_inspect_workbench_document("live snapshot", summary, governance, report);
    run_inspect_workbench(document)
}
