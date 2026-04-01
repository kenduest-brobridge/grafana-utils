//! Resolve and validate export-scope directories for dashboard and datasource snapshots.
//!
//! Responsibilities:
//! - Detect per-org scope directories for dashboard and datasource exports.
//! - Map scoped paths into standardized variants (`raw`, `prompt`, `provisioning`).
//! - Provide consistent directory inference used by both review and import flows.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn looks_like_dashboard_scope_dir(path: &Path) -> bool {
    path.join("raw").join("index.json").is_file()
        || path.join("prompt").join("index.json").is_file()
        || path.join("provisioning").join("index.json").is_file()
}

fn discover_dashboard_org_scope_dirs(dashboard_root: &Path) -> Vec<PathBuf> {
    let mut scopes = Vec::new();
    if let Ok(entries) = fs::read_dir(dashboard_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if name.starts_with("org_") && looks_like_dashboard_scope_dir(&path) {
                scopes.push(path);
            }
        }
    }
    scopes.sort();
    scopes
}

pub(crate) fn resolve_dashboard_export_scope_dirs(
    dashboard_root: &Path,
    dashboard_metadata: &Value,
) -> Vec<PathBuf> {
    let discovered = discover_dashboard_org_scope_dirs(dashboard_root);
    if !discovered.is_empty() {
        return discovered;
    }
    if let Some(orgs) = dashboard_metadata.get("orgs").and_then(Value::as_array) {
        let scopes = orgs
            .iter()
            .filter_map(|org| {
                org.get("exportDir").and_then(Value::as_str).map(|path| {
                    let export_path = PathBuf::from(path);
                    if export_path.is_absolute() || export_path.exists() {
                        export_path
                    } else {
                        dashboard_root.join(path)
                    }
                })
            })
            .filter(|path| path.is_dir())
            .collect::<Vec<PathBuf>>();
        if !scopes.is_empty() {
            return scopes;
        }
    }
    vec![dashboard_root.to_path_buf()]
}

pub(crate) fn resolve_datasource_export_scope_dirs(datasource_root: &Path) -> Vec<PathBuf> {
    let mut scopes = Vec::new();
    if datasource_root.join("datasources.json").is_file()
        || datasource_root.join("export-metadata.json").is_file()
    {
        scopes.push(datasource_root.to_path_buf());
    }
    if let Ok(entries) = fs::read_dir(datasource_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !name.starts_with("org_") {
                continue;
            }
            if path.join("datasources.json").is_file()
                || path.join("provisioning").join("datasources.yaml").is_file()
            {
                scopes.push(path);
            }
        }
    }
    scopes
}
