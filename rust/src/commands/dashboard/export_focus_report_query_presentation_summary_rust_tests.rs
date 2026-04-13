//! Query presentation report-build and import-prep regression test facade.
//! Keeps summary-row generation separate from inventory-backed query-report and import-prep cases.

#[cfg(test)]
#[path = "export_focus_report_query_presentation_summary_rows_rust_tests.rs"]
mod export_focus_report_query_presentation_summary_rows_rust_tests;

#[cfg(test)]
#[path = "export_focus_report_query_presentation_summary_inventory_rust_tests.rs"]
mod export_focus_report_query_presentation_summary_inventory_rust_tests;
