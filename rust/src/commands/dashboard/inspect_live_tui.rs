#![cfg(feature = "tui")]
#![allow(dead_code)]
use crate::common::Result;
use crate::dashboard::inspect_report::ExportInspectionQueryReport;

use super::inspect_governance::ExportInspectionGovernanceDocument;
use super::inspect_workbench::run_inspect_workbench;
use super::inspect_workbench_support::build_inspect_workbench_document;
use super::ExportInspectionSummary;

#[allow(unused_imports)]
pub(crate) use super::inspect_workbench_support::{
    build_inspect_live_tui_groups, filter_inspect_live_tui_items, InspectLiveGroup,
};

pub(crate) fn run_inspect_live_interactive(
    summary: &ExportInspectionSummary,
    governance: &ExportInspectionGovernanceDocument,
    report: &ExportInspectionQueryReport,
) -> Result<()> {
    let document = build_inspect_workbench_document("live snapshot", summary, governance, report);
    run_inspect_workbench(document)
}
