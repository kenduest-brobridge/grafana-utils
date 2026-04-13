//! Query presentation contract regression test facade.
//! Keeps the report-column/output contract coverage and filter/grouping contract coverage split.
#![allow(unused_imports)]

use super::super::test_support;
use super::{load_inspection_analyzer_cases, make_core_family_report_row};
use crate::dashboard::inspect::{
    dispatch_query_analysis, resolve_query_analyzer_family, QueryAnalysis, QueryExtractionContext,
};
use serde_json::Value;

#[cfg(test)]
#[path = "export_focus_report_query_presentation_contract_columns_rust_tests.rs"]
mod export_focus_report_query_presentation_contract_columns_rust_tests;

#[cfg(test)]
#[path = "export_focus_report_query_presentation_contract_filters_rust_tests.rs"]
mod export_focus_report_query_presentation_contract_filters_rust_tests;
