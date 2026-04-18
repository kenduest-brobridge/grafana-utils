//! Dashboard export/import, topology, and import-format regression tests.
#![allow(unused_imports)]

use super::*;
use crate::dashboard::{resolve_dashboard_import_source, DashboardImportInputFormat};
use std::path::Path;

#[path = "dashboard_export_contract_rust_tests.rs"]
mod dashboard_export_contract_rust_tests;
#[path = "dashboard_import_render_rust_tests.rs"]
mod dashboard_import_render_rust_tests;
#[path = "dashboard_routed_import_rust_tests.rs"]
mod dashboard_routed_import_rust_tests;
#[path = "dashboard_topology_import_format_rust_tests.rs"]
mod dashboard_topology_import_format_rust_tests;
