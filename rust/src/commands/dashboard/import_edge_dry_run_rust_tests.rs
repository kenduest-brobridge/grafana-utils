//! Import edge-case dashboard regression tests.
#![allow(unused_imports)]

use super::super::super::test_support;
use super::super::super::test_support::{
    diff_dashboards_with_request, import_dashboards_with_request, DiffArgs, ImportArgs,
    DATASOURCE_INVENTORY_FILENAME, EXPORT_METADATA_FILENAME, FOLDER_INVENTORY_FILENAME,
    TOOL_SCHEMA_VERSION,
};
use super::{
    make_basic_common_args, make_common_args, make_import_args,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata,
};
use crate::common::api_response;
use crate::dashboard::DashboardImportInputFormat;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[cfg(test)]
#[path = "import_edge_dry_run_preflight_rust_tests.rs"]
mod import_edge_dry_run_preflight_rust_tests;

#[cfg(test)]
#[path = "import_edge_dry_run_update_existing_rust_tests.rs"]
mod import_edge_dry_run_update_existing_rust_tests;
