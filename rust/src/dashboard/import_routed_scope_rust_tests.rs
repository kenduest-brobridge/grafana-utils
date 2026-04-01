//! Import-focused dashboard regression tests for auth and routed org-scope behavior.
#![allow(unused_imports)]

use super::test_support;
use super::{
    build_import_auth_context, import_dashboards_with_org_clients, import_dashboards_with_request,
    make_basic_common_args, make_common_args, make_import_args,
    with_dashboard_import_live_preflight, write_basic_raw_export,
    write_combined_export_root_metadata, ImportArgs, EXPORT_METADATA_FILENAME, TOOL_SCHEMA_VERSION,
};
use crate::dashboard::DashboardImportInputFormat;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[cfg(test)]
#[path = "import_routed_scope_auth_rust_tests.rs"]
mod import_routed_scope_auth_rust_tests;

#[cfg(test)]
#[path = "import_routed_scope_matrix_rust_tests.rs"]
mod import_routed_scope_matrix_rust_tests;
