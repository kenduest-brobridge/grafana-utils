//! Routing module for dashboard browse workflow regression tests.
//!
//! The real test bodies live in sibling modules grouped by workflow so this file
//! stays as the stable include point used by the dashboard render test suite.

#[allow(unused_imports)]
use super::*;
#[allow(unused_imports)]
use serde_json::json;

#[path = "dashboard_browse_delete_rust_tests.rs"]
mod dashboard_browse_delete_rust_tests;
#[path = "dashboard_browse_tree_state_rust_tests.rs"]
mod dashboard_browse_tree_state_rust_tests;
#[path = "dashboard_browse_edit_rust_tests.rs"]
mod dashboard_browse_edit_rust_tests;
#[path = "dashboard_browse_view_history_rust_tests.rs"]
mod dashboard_browse_view_history_rust_tests;
#[path = "dashboard_browse_raw_edit_rust_tests.rs"]
mod dashboard_browse_raw_edit_rust_tests;
#[path = "dashboard_browse_workflow_interactive_import_rust_tests.rs"]
mod dashboard_browse_workflow_interactive_import_rust_tests;
