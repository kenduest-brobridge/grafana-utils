use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::common::{message, Result};
use crate::dashboard::DEFAULT_ORG_ID;
use crate::http::JsonHttpClient;

use super::datasource_import_export_support_io::{
    load_import_records, resolve_provisioning_import_source_path,
};
use super::{fetch_current_org, EXPORT_METADATA_FILENAME};
use super::{DatasourceExportOrgScope, DatasourceImportArgs, DatasourceImportInputFormat};

fn collect_source_org_ids(records: &[super::DatasourceImportRecord]) -> BTreeSet<String> {
    records
        .iter()
        .filter(|record| !record.org_id.is_empty())
        .map(|record| record.org_id.clone())
        .collect()
}

fn collect_source_org_names(records: &[super::DatasourceImportRecord]) -> BTreeSet<String> {
    records
        .iter()
        .filter(|record| !record.org_name.is_empty())
        .map(|record| record.org_name.clone())
        .collect()
}

fn parse_export_org_scope(
    scope_dir: &Path,
    input_format: DatasourceImportInputFormat,
) -> Result<DatasourceExportOrgScope> {
    let (_, records) = load_import_records(scope_dir, input_format)?;
    let export_org_ids = collect_source_org_ids(&records);
    let (source_org_id, source_org_name_from_dir) = if export_org_ids.is_empty() {
        // Older or minimized exports may omit org metadata inside the payloads.
        // In that case the directory name is the last fallback for routed import.
        let scope_name = scope_dir
            .file_name()
            .and_then(|item| item.to_str())
            .unwrap_or_default();
        if let Some(rest) = scope_name.strip_prefix("org_") {
            let mut parts = rest.splitn(2, '_');
            let source_org_id_text = parts.next().unwrap_or_default();
            let source_org_name = parts
                .next()
                .unwrap_or_default()
                .replace('_', " ")
                .trim()
                .to_string();
            let source_org_id = source_org_id_text.parse::<i64>().map_err(|_| {
                message(format!(
                    "Cannot route datasource import by export org for {}: export orgId '{}' from the org directory name is not a valid integer.",
                    scope_dir.display(),
                    source_org_id_text
                ))
            })?;
            (source_org_id, source_org_name)
        } else {
            return Err(message(format!(
                "Cannot route datasource import by export org for {}: export orgId metadata was not found in datasources.json or index.json.",
                scope_dir.display()
            )));
        }
    } else {
        if export_org_ids.len() > 1 {
            return Err(message(format!(
                "Cannot route datasource import by export org for {}: found multiple export orgIds ({}).",
                scope_dir.display(),
                export_org_ids.into_iter().collect::<Vec<String>>().join(", ")
            )));
        }
        let source_org_id_text = export_org_ids.into_iter().next().unwrap_or_default();
        let source_org_id = source_org_id_text.parse::<i64>().map_err(|_| {
            message(format!(
                "Cannot route datasource import by export org for {}: export orgId '{}' is not a valid integer.",
                scope_dir.display(),
                source_org_id_text
            ))
        })?;
        (source_org_id, String::new())
    };
    let org_names = collect_source_org_names(&records);
    if org_names.len() > 1 {
        return Err(message(format!(
            "Cannot route datasource import by export org for {}: found multiple export org names ({}).",
            scope_dir.display(),
            org_names.into_iter().collect::<Vec<String>>().join(", ")
        )));
    }
    let source_org_name = org_names
        .into_iter()
        .next()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| {
            if !source_org_name_from_dir.is_empty() {
                source_org_name_from_dir
            } else {
                scope_dir
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("org")
                    .to_string()
            }
        });
    Ok(DatasourceExportOrgScope {
        source_org_id,
        source_org_name,
        input_dir: scope_dir.to_path_buf(),
    })
}

pub(crate) fn discover_export_org_import_scopes(
    args: &DatasourceImportArgs,
) -> Result<Vec<DatasourceExportOrgScope>> {
    if !args.use_export_org {
        return Ok(Vec::new());
    }
    let import_root = super::resolve_datasource_export_root_dir(&args.input_dir)?;
    let metadata_path = import_root.join(EXPORT_METADATA_FILENAME);
    if !metadata_path.is_file() {
        return Err(message(format!(
            "Datasource import with --use-export-org requires export-metadata.json at the combined datasource export root: {}",
            metadata_path.display()
        )));
    }
    let root_manifest = super::load_datasource_export_root_manifest(&metadata_path)?;
    if !root_manifest.scope_kind.is_aggregate() {
        return Err(message(format!(
            "Datasource import with --use-export-org expects a combined datasource export root with scopeKind all-orgs-root or workspace-root: {}",
            metadata_path.display()
        )));
    }
    let selected_org_ids: BTreeSet<i64> = args.only_org_id.iter().copied().collect();
    let mut scopes = Vec::new();
    let mut matched_source_org_ids = BTreeSet::new();
    for entry in fs::read_dir(&import_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|item| item.to_str()) else {
            continue;
        };
        if !name.starts_with("org_") {
            continue;
        }
        let is_scope_dir = match args.input_format {
            DatasourceImportInputFormat::Inventory => path.join(EXPORT_METADATA_FILENAME).is_file(),
            DatasourceImportInputFormat::Provisioning => {
                resolve_provisioning_import_source_path(&path).is_ok()
            }
        };
        if !is_scope_dir {
            continue;
        }
        // Each child scope must be self-contained; do not infer org routing from
        // partial directories or siblings without an importable datasource bundle.
        let scope = parse_export_org_scope(&path, args.input_format)?;
        if !selected_org_ids.is_empty() && !selected_org_ids.contains(&scope.source_org_id) {
            continue;
        }
        matched_source_org_ids.insert(scope.source_org_id);
        scopes.push(scope);
    }
    scopes.sort_by(|left, right| left.source_org_id.cmp(&right.source_org_id));
    if !selected_org_ids.is_empty() {
        let missing: Vec<String> = selected_org_ids
            .difference(&matched_source_org_ids)
            .map(|item| item.to_string())
            .collect();
        if !missing.is_empty() {
            return Err(message(format!(
                "Selected exported org IDs were not found in {}: {}",
                args.input_dir.display(),
                missing.join(", ")
            )));
        }
    }
    if scopes.is_empty() {
        match args.input_format {
            DatasourceImportInputFormat::Inventory => {
                if args.input_dir.join(EXPORT_METADATA_FILENAME).is_file() {
                    return Err(message(
                        "Datasource import with --use-export-org expects the combined export root, not one org export directory.",
                    ));
                }
            }
            DatasourceImportInputFormat::Provisioning => {
                if resolve_provisioning_import_source_path(&args.input_dir).is_ok() {
                    return Err(message(
                        "Datasource import with --use-export-org expects the combined export root, not one org provisioning directory or YAML file.",
                    ));
                }
            }
        }
        if !selected_org_ids.is_empty() {
            return Err(message(format!(
                "Datasource import with --use-export-org did not find the selected exported org IDs ({}) under {}.",
                selected_org_ids
                    .into_iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                import_root.display()
            )));
        }
        return Err(message(format!(
            "Datasource import with --use-export-org did not find any org-scoped datasource exports under {}.",
            import_root.display()
        )));
    }
    let found_org_ids: BTreeSet<i64> = scopes.iter().map(|scope| scope.source_org_id).collect();
    let missing_org_ids: Vec<String> = selected_org_ids
        .difference(&found_org_ids)
        .map(|id| id.to_string())
        .collect();
    if !missing_org_ids.is_empty() {
        return Err(message(format!(
            "Datasource import with --use-export-org did not find the selected exported org IDs ({}).",
            missing_org_ids.join(", ")
        )));
    }
    Ok(scopes)
}

pub(crate) fn validate_matching_export_org(
    client: &JsonHttpClient,
    args: &DatasourceImportArgs,
    records: &[super::DatasourceImportRecord],
) -> Result<()> {
    if !args.require_matching_export_org {
        return Ok(());
    }
    // This guardrail is intentionally strict: one import bundle must map to one
    // target org, otherwise a mismatched client/org selection can mutate the
    // wrong Grafana org with valid-looking datasource records.
    let source_org_ids = collect_source_org_ids(records);
    if source_org_ids.is_empty() {
        return Err(message(
            "Cannot verify datasource export org: no stable orgId metadata found in the selected datasource import input.",
        ));
    }
    if source_org_ids.len() > 1 {
        return Err(message(format!(
            "Cannot verify datasource export org: found multiple export orgIds ({}).",
            source_org_ids
                .into_iter()
                .collect::<Vec<String>>()
                .join(", ")
        )));
    }
    let source_org_id = source_org_ids.into_iter().next().unwrap_or_default();
    let target_org = fetch_current_org(client)?;
    let target_org_id = target_org
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| DEFAULT_ORG_ID.to_string());
    if source_org_id != target_org_id {
        return Err(message(format!(
            "Datasource import export org mismatch: raw export orgId {source_org_id} does not match target org {target_org_id}. Use matching credentials/org selection or omit --require-matching-export-org."
        )));
    }
    Ok(())
}
