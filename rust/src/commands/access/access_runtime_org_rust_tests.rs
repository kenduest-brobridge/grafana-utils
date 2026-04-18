//! Access org runtime test facade and shared helpers.

use super::*;

fn write_local_access_bundle(dir: &std::path::Path, file_name: &str, payload: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join(file_name), payload).unwrap();
    fs::write(
        dir.join("export-metadata.json"),
        r#"{"kind":"grafana-utils-access-export-metadata","version":1}"#,
    )
    .unwrap();
}

#[path = "access_runtime_user_rust_tests.rs"]
mod access_runtime_user_rust_tests;

#[path = "access_runtime_org_routing_rust_tests.rs"]
mod access_runtime_org_routing_rust_tests;

#[path = "access_runtime_org_diff_rust_tests.rs"]
mod access_runtime_org_diff_rust_tests;

#[path = "access_runtime_org_import_rust_tests.rs"]
mod access_runtime_org_import_rust_tests;

#[path = "access_runtime_org_local_list_rust_tests.rs"]
mod access_runtime_org_local_list_rust_tests;
