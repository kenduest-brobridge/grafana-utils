//! Repair old dashboard export folder layouts from local artifact metadata.

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{message, render_json_value, sanitize_path_component, string_field, Result};
use crate::tabular_output::render_yaml;

use super::inspect_render::render_simple_table;
use super::{
    load_export_metadata, load_folder_inventory, load_json_file, write_json_document,
    DashboardIndexItem, ExportLayoutArgs, ExportLayoutOutputFormat, ExportLayoutVariant,
    FolderInventoryItem, RootExportIndex, VariantIndexEntry, DASHBOARD_PERMISSION_BUNDLE_FILENAME,
    DATASOURCE_INVENTORY_FILENAME, DEFAULT_ORG_ID, EXPORT_METADATA_FILENAME,
    FOLDER_INVENTORY_FILENAME, PROMPT_EXPORT_SUBDIR, RAW_EXPORT_SUBDIR,
};

const EXPORT_LAYOUT_PLAN_KIND: &str = "grafana-util-dashboard-export-layout-plan";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LayoutVariant {
    Raw,
    Prompt,
}

impl LayoutVariant {
    fn as_str(self) -> &'static str {
        match self {
            Self::Raw => RAW_EXPORT_SUBDIR,
            Self::Prompt => PROMPT_EXPORT_SUBDIR,
        }
    }

    fn index_field(self, item: &mut DashboardIndexItem) -> &mut Option<String> {
        match self {
            Self::Raw => &mut item.raw_path,
            Self::Prompt => &mut item.prompt_path,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExportLayoutPlan {
    pub kind: String,
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(rename = "inputDir")]
    pub input_dir: String,
    #[serde(rename = "outputDir", skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
    pub variants: Vec<String>,
    pub operations: Vec<ExportLayoutOperation>,
    #[serde(rename = "extraFiles")]
    pub extra_files: Vec<ExportLayoutExtraFile>,
    pub summary: ExportLayoutSummary,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExportLayoutOperation {
    pub action: String,
    pub variant: String,
    pub uid: String,
    #[serde(rename = "orgId", skip_serializing_if = "String::is_empty")]
    pub org_id: String,
    pub from: String,
    pub to: String,
    #[serde(rename = "folderUid", skip_serializing_if = "Option::is_none")]
    pub folder_uid: Option<String>,
    #[serde(rename = "folderPath", skip_serializing_if = "Option::is_none")]
    pub folder_path: Option<String>,
    #[serde(skip_serializing)]
    variant_scope: PathBuf,
    #[serde(skip_serializing)]
    from_relative: PathBuf,
    #[serde(skip_serializing)]
    to_relative: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExportLayoutExtraFile {
    pub variant: String,
    pub path: String,
    pub handling: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExportLayoutSummary {
    #[serde(rename = "moveCount")]
    pub move_count: usize,
    #[serde(rename = "unchangedCount")]
    pub unchanged_count: usize,
    #[serde(rename = "blockedCount")]
    pub blocked_count: usize,
    #[serde(rename = "extraFileCount")]
    pub extra_file_count: usize,
}

struct VariantDir {
    variant: LayoutVariant,
    path: PathBuf,
    scope: PathBuf,
}

struct FolderLookup {
    by_key: BTreeMap<String, String>,
    by_title: BTreeMap<String, Vec<FolderInventoryItem>>,
}

struct VariantPlan {
    operations: Vec<ExportLayoutOperation>,
    extra_files: Vec<ExportLayoutExtraFile>,
}

pub(crate) fn run_export_layout_repair(args: &ExportLayoutArgs) -> Result<()> {
    let variants = selected_variants(args);
    let output_dir = args.output_dir.as_deref();
    let plan = build_export_layout_plan(args, &variants)?;
    print_export_layout_plan(
        &plan,
        args.output_format,
        args.no_header,
        args.show_operations,
    )?;

    if args.dry_run {
        return Ok(());
    }
    if plan.summary.blocked_count > 0 {
        return Err(message(format!(
            "dashboard export-layout repair blocked {} operation(s).",
            plan.summary.blocked_count
        )));
    }

    let execution_root = if args.in_place {
        args.input_dir.as_path()
    } else {
        let Some(output_dir) = output_dir else {
            return Err(message("Missing --output-dir for export-layout repair."));
        };
        copy_dir_recursive(&args.input_dir, output_dir, args.overwrite)?;
        output_dir
    };

    apply_export_layout_plan(args, execution_root, &plan)?;
    Ok(())
}

fn selected_variants(args: &ExportLayoutArgs) -> BTreeSet<LayoutVariant> {
    if args.variant.is_empty() {
        return BTreeSet::from([LayoutVariant::Raw, LayoutVariant::Prompt]);
    }
    args.variant
        .iter()
        .map(|variant| match variant {
            ExportLayoutVariant::Raw => LayoutVariant::Raw,
            ExportLayoutVariant::Prompt => LayoutVariant::Prompt,
        })
        .collect()
}

fn build_export_layout_plan(
    args: &ExportLayoutArgs,
    variants: &BTreeSet<LayoutVariant>,
) -> Result<ExportLayoutPlan> {
    let variant_dirs = discover_variant_dirs(&args.input_dir, variants)?;
    let mut operations = Vec::new();
    let mut extra_files = Vec::new();
    for variant_dir in variant_dirs {
        let variant_plan = plan_variant_dir(args, &variant_dir)?;
        operations.extend(variant_plan.operations);
        extra_files.extend(variant_plan.extra_files);
    }
    let summary = summarize_operations(&operations, &extra_files);
    Ok(ExportLayoutPlan {
        kind: EXPORT_LAYOUT_PLAN_KIND.to_string(),
        schema_version: 1,
        input_dir: args.input_dir.display().to_string(),
        output_dir: args
            .output_dir
            .as_ref()
            .map(|path| path.display().to_string()),
        variants: variants
            .iter()
            .map(|variant| variant.as_str().to_string())
            .collect(),
        operations,
        extra_files,
        summary,
    })
}

fn discover_variant_dirs(
    input_dir: &Path,
    variants: &BTreeSet<LayoutVariant>,
) -> Result<Vec<VariantDir>> {
    let mut dirs = Vec::new();
    for variant in variants {
        if is_variant_dir(input_dir, *variant)? {
            dirs.push(VariantDir {
                variant: *variant,
                path: input_dir.to_path_buf(),
                scope: PathBuf::new(),
            });
            continue;
        }
        let child = input_dir.join(variant.as_str());
        if is_variant_dir(&child, *variant)? {
            dirs.push(VariantDir {
                variant: *variant,
                path: child,
                scope: PathBuf::from(variant.as_str()),
            });
        }
    }
    discover_nested_variant_dirs(input_dir, input_dir, variants, &mut dirs)?;
    dirs.sort_by(|left, right| left.scope.cmp(&right.scope));
    dirs.dedup_by(|left, right| left.path == right.path && left.variant == right.variant);
    Ok(dirs)
}

fn discover_nested_variant_dirs(
    root: &Path,
    dir: &Path,
    variants: &BTreeSet<LayoutVariant>,
    dirs: &mut Vec<VariantDir>,
) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        for variant in variants {
            if path.file_name().and_then(|value| value.to_str()) == Some(variant.as_str())
                && is_variant_dir(&path, *variant)?
            {
                dirs.push(VariantDir {
                    variant: *variant,
                    scope: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                    path: path.clone(),
                });
            }
        }
        discover_nested_variant_dirs(root, &path, variants, dirs)?;
    }
    Ok(())
}

fn is_variant_dir(dir: &Path, variant: LayoutVariant) -> Result<bool> {
    let Some(metadata) = load_export_metadata(dir, None)? else {
        return Ok(false);
    };
    Ok(metadata.variant == variant.as_str())
}

fn plan_variant_dir(args: &ExportLayoutArgs, variant_dir: &VariantDir) -> Result<VariantPlan> {
    let entries = load_variant_index_entries(&variant_dir.path)?;
    let raw_dir = resolve_raw_metadata_dir(args, variant_dir);
    let folder_lookup = load_folder_lookup(args, &raw_dir)?;
    let raw_entries_by_uid = load_raw_entries_by_uid(&raw_dir).unwrap_or_default();
    let dashboard_items_by_uid = load_dashboard_items_by_uid(variant_dir.path.parent());
    let mut operations = Vec::new();
    let mut indexed_paths = BTreeSet::new();
    for entry in entries {
        let from_relative =
            entry_relative_path(&entry.path, &variant_dir.path, variant_dir.variant)?;
        indexed_paths.insert(from_relative.clone());
        let from_path = variant_dir.path.join(&from_relative);
        let (mut folder_uid, mut reason) = resolve_entry_folder_uid(
            variant_dir.variant,
            &entry,
            &from_path,
            &raw_dir,
            &raw_entries_by_uid,
        );
        if folder_uid.is_none() {
            if let Some(uid) = resolve_folder_uid_by_root_index(&entry, &dashboard_items_by_uid) {
                folder_uid = Some(uid);
                reason = None;
            }
        }
        if folder_uid.is_none() {
            if let Some(folder) = resolve_unique_folder_by_index_title(
                &entry,
                &dashboard_items_by_uid,
                &folder_lookup,
            ) {
                folder_uid = Some(folder.uid.clone());
                reason = None;
            }
        }
        let Some(folder_uid) = folder_uid else {
            operations.push(blocked_operation(
                variant_dir,
                &entry,
                &from_relative,
                reason.unwrap_or_else(|| "missing folderUid".to_string()),
            ));
            continue;
        };
        let key = format!(
            "{}:{}",
            if entry.org_id.trim().is_empty() {
                DEFAULT_ORG_ID
            } else {
                entry.org_id.as_str()
            },
            folder_uid
        );
        let Some(folder_path) = folder_lookup.by_key.get(&key) else {
            operations.push(blocked_operation(
                variant_dir,
                &entry,
                &from_relative,
                format!("folder inventory missing {key}"),
            ));
            continue;
        };
        let file_name = from_relative
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("dashboard.json");
        let to_relative = folder_path_to_relative(folder_path).join(file_name);
        let target_path = variant_dir.path.join(&to_relative);
        if from_relative == to_relative {
            operations.push(ExportLayoutOperation {
                action: "same".to_string(),
                variant: variant_dir.variant.as_str().to_string(),
                uid: entry.uid,
                org_id: entry.org_id,
                from: from_relative.display().to_string(),
                to: to_relative.display().to_string(),
                folder_uid: Some(folder_uid),
                folder_path: Some(folder_path.clone()),
                variant_scope: variant_dir.scope.clone(),
                from_relative,
                to_relative,
                reason: None,
            });
            continue;
        }
        if target_path.exists() && !args.overwrite {
            operations.push(blocked_operation(
                variant_dir,
                &entry,
                &from_relative,
                format!("target exists: {}", target_path.display()),
            ));
            continue;
        }
        operations.push(ExportLayoutOperation {
            action: "move".to_string(),
            variant: variant_dir.variant.as_str().to_string(),
            uid: entry.uid,
            org_id: entry.org_id,
            from: from_relative.display().to_string(),
            to: to_relative.display().to_string(),
            folder_uid: Some(folder_uid),
            folder_path: Some(folder_path.clone()),
            variant_scope: variant_dir.scope.clone(),
            from_relative,
            to_relative,
            reason: None,
        });
    }
    let extra_files = collect_extra_files(variant_dir, &indexed_paths)?;
    Ok(VariantPlan {
        operations,
        extra_files,
    })
}

fn load_variant_index_entries(variant_dir: &Path) -> Result<Vec<VariantIndexEntry>> {
    let index_path = variant_dir.join("index.json");
    let raw = fs::read_to_string(&index_path).map_err(|error| {
        message(format!(
            "Failed to read dashboard export index {}: {error}",
            index_path.display()
        ))
    })?;
    serde_json::from_str(&raw).map_err(|error| {
        message(format!(
            "Invalid dashboard export index {}: {error}",
            index_path.display()
        ))
    })
}

fn load_raw_entries_by_uid(raw_dir: &Path) -> Result<BTreeMap<String, VariantIndexEntry>> {
    Ok(load_variant_index_entries(raw_dir)?
        .into_iter()
        .map(|entry| (entry.uid.clone(), entry))
        .collect())
}

fn resolve_raw_metadata_dir(args: &ExportLayoutArgs, variant_dir: &VariantDir) -> PathBuf {
    if let Some(raw_dir) = &args.raw_dir {
        return raw_dir.clone();
    }
    if variant_dir.variant == LayoutVariant::Raw {
        return variant_dir.path.clone();
    }
    if variant_dir
        .path
        .file_name()
        .and_then(|value| value.to_str())
        == Some(PROMPT_EXPORT_SUBDIR)
    {
        if let Some(parent) = variant_dir.path.parent() {
            return parent.join(RAW_EXPORT_SUBDIR);
        }
    }
    variant_dir.path.join(RAW_EXPORT_SUBDIR)
}

fn load_folder_lookup(args: &ExportLayoutArgs, raw_dir: &Path) -> Result<FolderLookup> {
    let folders = if let Some(folders_file) = &args.folders_file {
        let folders_path = if folders_file.is_absolute() {
            folders_file.clone()
        } else {
            raw_dir.join(folders_file)
        };
        let raw = fs::read_to_string(&folders_path).map_err(|error| {
            message(format!(
                "Failed to read folder inventory {}: {error}",
                folders_path.display()
            ))
        })?;
        serde_json::from_str::<Vec<FolderInventoryItem>>(&raw).map_err(|error| {
            message(format!(
                "Invalid folder inventory {}: {error}",
                folders_path.display()
            ))
        })?
    } else {
        let metadata = load_export_metadata(raw_dir, Some(RAW_EXPORT_SUBDIR))?;
        load_folder_inventory(raw_dir, metadata.as_ref())?
    };
    if folders.is_empty() {
        return Err(message(format!(
            "Folder inventory not found for dashboard export-layout repair. Expected {} under {} or pass --folders-file.",
            FOLDER_INVENTORY_FILENAME,
            raw_dir.display()
        )));
    }
    let mut by_key = BTreeMap::new();
    let mut by_title: BTreeMap<String, Vec<FolderInventoryItem>> = BTreeMap::new();
    for folder in folders {
        by_key.insert(
            format!("{}:{}", folder.org_id, folder.uid),
            folder.path.clone(),
        );
        by_title
            .entry(format!("{}:{}", folder.org_id, folder.title))
            .or_default()
            .push(folder);
    }
    Ok(FolderLookup { by_key, by_title })
}

fn load_dashboard_items_by_uid(root_dir: Option<&Path>) -> BTreeMap<String, DashboardIndexItem> {
    let Some(root_dir) = root_dir else {
        return BTreeMap::new();
    };
    let index_path = root_dir.join("index.json");
    let Ok(value) = load_json_file(&index_path) else {
        return BTreeMap::new();
    };
    let Ok(index) = serde_json::from_value::<RootExportIndex>(value) else {
        return BTreeMap::new();
    };
    index
        .items
        .into_iter()
        .map(|item| (format!("{}:{}", item.org_id, item.uid), item))
        .collect()
}

fn resolve_unique_folder_by_index_title(
    entry: &VariantIndexEntry,
    dashboard_items_by_uid: &BTreeMap<String, DashboardIndexItem>,
    folder_lookup: &FolderLookup,
) -> Option<FolderInventoryItem> {
    let org_id = if entry.org_id.trim().is_empty() {
        DEFAULT_ORG_ID
    } else {
        entry.org_id.as_str()
    };
    let item = dashboard_items_by_uid.get(&format!("{org_id}:{}", entry.uid))?;
    let matches = folder_lookup
        .by_title
        .get(&format!("{org_id}:{}", item.folder_title))?;
    if matches.len() == 1 {
        matches.first().cloned()
    } else {
        None
    }
}

fn resolve_folder_uid_by_root_index(
    entry: &VariantIndexEntry,
    dashboard_items_by_uid: &BTreeMap<String, DashboardIndexItem>,
) -> Option<String> {
    let org_id = if entry.org_id.trim().is_empty() {
        DEFAULT_ORG_ID
    } else {
        entry.org_id.as_str()
    };
    let item = dashboard_items_by_uid.get(&format!("{org_id}:{}", entry.uid))?;
    if item.folder_uid.trim().is_empty() {
        None
    } else {
        Some(item.folder_uid.clone())
    }
}

fn resolve_entry_folder_uid(
    variant: LayoutVariant,
    entry: &VariantIndexEntry,
    from_path: &Path,
    raw_dir: &Path,
    raw_entries_by_uid: &BTreeMap<String, VariantIndexEntry>,
) -> (Option<String>, Option<String>) {
    if !entry.folder_uid.trim().is_empty() {
        return (Some(entry.folder_uid.clone()), None);
    }
    if variant == LayoutVariant::Raw {
        return match read_folder_uid_from_dashboard(from_path) {
            Ok(value) => (value, None),
            Err(error) => (None, Some(error.to_string())),
        };
    }
    let Some(raw_entry) = raw_entries_by_uid.get(&entry.uid) else {
        return (
            None,
            Some(format!(
                "raw index entry missing for prompt dashboard uid={}",
                entry.uid
            )),
        );
    };
    let raw_relative = match entry_relative_path(&raw_entry.path, raw_dir, LayoutVariant::Raw) {
        Ok(path) => path,
        Err(error) => return (None, Some(error.to_string())),
    };
    match read_folder_uid_from_dashboard(&raw_dir.join(raw_relative)) {
        Ok(value) => (value, None),
        Err(error) => (None, Some(error.to_string())),
    }
}

fn read_folder_uid_from_dashboard(path: &Path) -> Result<Option<String>> {
    let value = load_json_file(path)?;
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    let meta = object.get("meta").and_then(Value::as_object);
    let folder_uid = meta
        .map(|item| string_field(item, "folderUid", ""))
        .unwrap_or_default();
    if !folder_uid.is_empty() {
        return Ok(Some(folder_uid));
    }
    let top_level = string_field(object, "folderUid", "");
    if top_level.is_empty() {
        Ok(None)
    } else {
        Ok(Some(top_level))
    }
}

fn entry_relative_path(
    path_text: &str,
    variant_dir: &Path,
    variant: LayoutVariant,
) -> Result<PathBuf> {
    let path = PathBuf::from(path_text);
    if path.is_absolute() {
        if let Ok(relative) = path.strip_prefix(variant_dir) {
            return Ok(relative.to_path_buf());
        }
        return path.file_name().map(PathBuf::from).ok_or_else(|| {
            message(format!(
                "Cannot resolve absolute dashboard index path: {}",
                path.display()
            ))
        });
    }
    let components = path
        .components()
        .map(|component| component.as_os_str().to_owned())
        .collect::<Vec<_>>();
    if let Some(index) = components
        .iter()
        .position(|component| component == variant.as_str())
    {
        return Ok(components.iter().skip(index + 1).collect::<PathBuf>());
    }
    Ok(path)
}

fn collect_extra_files(
    variant_dir: &VariantDir,
    indexed_paths: &BTreeSet<PathBuf>,
) -> Result<Vec<ExportLayoutExtraFile>> {
    let mut files = Vec::new();
    collect_unindexed_files(
        &variant_dir.path,
        &variant_dir.path,
        variant_dir,
        indexed_paths,
        &mut files,
    )?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

fn collect_unindexed_files(
    root: &Path,
    dir: &Path,
    variant_dir: &VariantDir,
    indexed_paths: &BTreeSet<PathBuf>,
    files: &mut Vec<ExportLayoutExtraFile>,
) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_unindexed_files(root, &path, variant_dir, indexed_paths, files)?;
            continue;
        }
        if !path.is_file() || is_export_contract_file(&path) {
            continue;
        }
        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        if path_listed_in_index(root, &relative, indexed_paths) {
            continue;
        }
        files.push(ExportLayoutExtraFile {
            variant: variant_dir.variant.as_str().to_string(),
            path: relative.display().to_string(),
            handling: "preserve".to_string(),
            reason: "file is not listed in the export index; copy-mode preserves it and in-place repair leaves it untouched".to_string(),
        });
    }
    Ok(())
}

fn path_listed_in_index(root: &Path, relative: &Path, indexed_paths: &BTreeSet<PathBuf>) -> bool {
    if indexed_paths.contains(relative) {
        return true;
    }
    let actual = root.join(relative);
    let Ok(actual_canonical) = actual.canonicalize() else {
        return false;
    };
    indexed_paths.iter().any(|indexed| {
        root.join(indexed)
            .canonicalize()
            .map(|indexed_canonical| indexed_canonical == actual_canonical)
            .unwrap_or(false)
    })
}

fn is_export_contract_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|value| value.to_str()),
        Some("index.json")
            | Some(EXPORT_METADATA_FILENAME)
            | Some(FOLDER_INVENTORY_FILENAME)
            | Some(DATASOURCE_INVENTORY_FILENAME)
            | Some(DASHBOARD_PERMISSION_BUNDLE_FILENAME)
    )
}

fn folder_path_to_relative(folder_path: &str) -> PathBuf {
    folder_path
        .split(" / ")
        .filter(|segment| !segment.trim().is_empty())
        .fold(PathBuf::new(), |path, segment| {
            path.join(sanitize_path_component(segment.trim()))
        })
}

fn blocked_operation(
    variant_dir: &VariantDir,
    entry: &VariantIndexEntry,
    from_relative: &Path,
    reason: String,
) -> ExportLayoutOperation {
    ExportLayoutOperation {
        action: "blocked".to_string(),
        variant: variant_dir.variant.as_str().to_string(),
        uid: entry.uid.clone(),
        org_id: entry.org_id.clone(),
        from: from_relative.display().to_string(),
        to: from_relative.display().to_string(),
        folder_uid: None,
        folder_path: None,
        variant_scope: variant_dir.scope.clone(),
        from_relative: from_relative.to_path_buf(),
        to_relative: from_relative.to_path_buf(),
        reason: Some(reason),
    }
}

fn summarize_operations(
    operations: &[ExportLayoutOperation],
    extra_files: &[ExportLayoutExtraFile],
) -> ExportLayoutSummary {
    ExportLayoutSummary {
        move_count: operations
            .iter()
            .filter(|operation| operation.action == "move")
            .count(),
        unchanged_count: operations
            .iter()
            .filter(|operation| operation.action == "same")
            .count(),
        blocked_count: operations
            .iter()
            .filter(|operation| operation.action == "blocked")
            .count(),
        extra_file_count: extra_files.len(),
    }
}

fn apply_export_layout_plan(
    args: &ExportLayoutArgs,
    execution_root: &Path,
    plan: &ExportLayoutPlan,
) -> Result<()> {
    let mut changed_variants = BTreeSet::new();
    for operation in &plan.operations {
        if operation.action != "move" {
            continue;
        }
        let variant_dir = execution_root.join(&operation.variant_scope);
        let source = variant_dir.join(&operation.from_relative);
        let target = variant_dir.join(&operation.to_relative);
        if args.in_place {
            backup_file(args, execution_root, &source)?;
            if target.exists() {
                backup_file(args, execution_root, &target)?;
            }
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        if target.exists() && args.overwrite {
            fs::remove_file(&target)?;
        }
        fs::rename(&source, &target)?;
        changed_variants.insert((operation.variant_scope.clone(), operation.variant.clone()));
    }

    for (variant_scope, variant) in changed_variants {
        let variant_dir = execution_root.join(&variant_scope);
        update_variant_index(&variant_dir, &variant_scope, &variant, plan)?;
        update_root_index(&variant_dir, &variant_scope, &variant, plan)?;
    }
    update_aggregate_root_index(execution_root, plan)?;
    Ok(())
}

fn backup_file(args: &ExportLayoutArgs, input_root: &Path, path: &Path) -> Result<()> {
    let Some(backup_dir) = args.backup_dir.as_ref() else {
        return Ok(());
    };
    if !path.is_file() {
        return Ok(());
    }
    let relative = path.strip_prefix(input_root).unwrap_or(path);
    let target = backup_dir.join(relative);
    if target.exists() && !args.overwrite {
        return Err(message(format!(
            "Refusing to overwrite backup file: {}. Use --overwrite.",
            target.display()
        )));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(path, target)?;
    Ok(())
}

fn update_variant_index(
    variant_dir: &Path,
    variant_scope: &Path,
    variant: &str,
    plan: &ExportLayoutPlan,
) -> Result<()> {
    let index_path = variant_dir.join("index.json");
    if !index_path.is_file() {
        return Ok(());
    }
    let mut entries = load_variant_index_entries(variant_dir)?;
    for entry in &mut entries {
        let Some(operation) = plan.operations.iter().find(|operation| {
            operation.variant == variant
                && operation.uid == entry.uid
                && operation.org_id == entry.org_id
                && operation.action == "move"
                && operation.variant_scope == variant_scope
        }) else {
            continue;
        };
        entry.path = variant_dir
            .join(&operation.to_relative)
            .display()
            .to_string();
    }
    write_json_document(&entries, &index_path)
}

fn update_root_index(
    variant_dir: &Path,
    variant_scope: &Path,
    variant: &str,
    plan: &ExportLayoutPlan,
) -> Result<()> {
    let Some(root_dir) = variant_dir.parent() else {
        return Ok(());
    };
    let index_path = root_dir.join("index.json");
    if !index_path.is_file() {
        return Ok(());
    }
    let value = load_json_file(&index_path)?;
    let mut root_index: RootExportIndex = match serde_json::from_value(value) {
        Ok(index) => index,
        Err(_) => return Ok(()),
    };
    if variant == RAW_EXPORT_SUBDIR {
        root_index.variants.raw = Some(variant_dir.join("index.json").display().to_string());
    } else {
        root_index.variants.prompt = Some(variant_dir.join("index.json").display().to_string());
    }
    for item in &mut root_index.items {
        let Some(operation) = plan.operations.iter().find(|operation| {
            operation.variant == variant
                && operation.uid == item.uid
                && operation.org_id == item.org_id
                && operation.action == "move"
                && operation.variant_scope == variant_scope
        }) else {
            continue;
        };
        let variant = if variant == RAW_EXPORT_SUBDIR {
            LayoutVariant::Raw
        } else {
            LayoutVariant::Prompt
        };
        *variant.index_field(item) = Some(
            variant_dir
                .join(&operation.to_relative)
                .display()
                .to_string(),
        );
    }
    write_json_document(&root_index, &index_path)
}

fn update_aggregate_root_index(execution_root: &Path, plan: &ExportLayoutPlan) -> Result<()> {
    let index_path = execution_root.join("index.json");
    if !index_path.is_file() {
        return Ok(());
    }
    let value = load_json_file(&index_path)?;
    let mut root_index: RootExportIndex = match serde_json::from_value(value) {
        Ok(index) => index,
        Err(_) => return Ok(()),
    };
    let mut changed = false;
    for item in &mut root_index.items {
        for operation in plan.operations.iter().filter(|operation| {
            operation.uid == item.uid
                && operation.org_id == item.org_id
                && operation.action == "move"
        }) {
            let variant_dir = execution_root.join(&operation.variant_scope);
            let repaired_path = variant_dir
                .join(&operation.to_relative)
                .display()
                .to_string();
            if operation.variant == RAW_EXPORT_SUBDIR {
                item.raw_path = Some(repaired_path);
                changed = true;
            } else if operation.variant == PROMPT_EXPORT_SUBDIR {
                item.prompt_path = Some(repaired_path);
                changed = true;
            }
        }
    }
    if changed {
        write_json_document(&root_index, &index_path)?;
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path, overwrite: bool) -> Result<()> {
    if target.exists() && !overwrite {
        return Err(message(format!(
            "Refusing to overwrite existing output directory: {}. Use --overwrite.",
            target.display()
        )));
    }
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path, overwrite)?;
        } else {
            if target_path.exists() && !overwrite {
                return Err(message(format!(
                    "Refusing to overwrite existing file: {}. Use --overwrite.",
                    target_path.display()
                )));
            }
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}

fn print_export_layout_plan(
    plan: &ExportLayoutPlan,
    output_format: ExportLayoutOutputFormat,
    no_header: bool,
    show_operations: bool,
) -> Result<()> {
    match output_format {
        ExportLayoutOutputFormat::Json => print!("{}", render_json_value(plan)?),
        ExportLayoutOutputFormat::Yaml => print!("{}", render_yaml(plan)?),
        ExportLayoutOutputFormat::Csv => {
            print!("{}", render_export_layout_plan_csv(plan, show_operations))
        }
        ExportLayoutOutputFormat::Table => {
            for line in render_export_layout_plan_table(plan, show_operations, !no_header) {
                println!("{line}");
            }
        }
        ExportLayoutOutputFormat::Text => {
            print!("{}", render_export_layout_plan_text(plan, show_operations));
        }
    }
    Ok(())
}

fn render_export_layout_plan_table(
    plan: &ExportLayoutPlan,
    show_operations: bool,
    include_header: bool,
) -> Vec<String> {
    if show_operations {
        let mut rows = plan
            .operations
            .iter()
            .map(|operation| {
                vec![
                    operation.action.to_uppercase(),
                    operation.variant.clone(),
                    operation.uid.clone(),
                    operation.from.clone(),
                    operation.to.clone(),
                    operation.reason.clone().unwrap_or_else(|| "-".to_string()),
                ]
            })
            .collect::<Vec<_>>();
        rows.extend(plan.extra_files.iter().map(|extra_file| {
            vec![
                "EXTRA".to_string(),
                extra_file.variant.clone(),
                "-".to_string(),
                extra_file.path.clone(),
                extra_file.path.clone(),
                extra_file.reason.clone(),
            ]
        }));
        return render_simple_table(
            &["ACTION", "VARIANT", "UID", "FROM", "TO", "REASON"],
            &rows,
            include_header,
        );
    }

    render_simple_table(
        &["FIELD", "VALUE"],
        &export_layout_summary_rows(plan),
        include_header,
    )
}

fn export_layout_summary_rows(plan: &ExportLayoutPlan) -> Vec<Vec<String>> {
    let mut rows = vec![
        vec![
            "dashboards".to_string(),
            (plan.summary.move_count + plan.summary.unchanged_count + plan.summary.blocked_count)
                .to_string(),
        ],
        vec!["variants".to_string(), plan.variants.join(",")],
        vec!["move".to_string(), plan.summary.move_count.to_string()],
        vec!["same".to_string(), plan.summary.unchanged_count.to_string()],
        vec![
            "blocked".to_string(),
            plan.summary.blocked_count.to_string(),
        ],
        vec![
            "extra".to_string(),
            plan.summary.extra_file_count.to_string(),
        ],
    ];
    if let Some(output_dir) = &plan.output_dir {
        rows.push(vec!["output".to_string(), output_dir.clone()]);
    }
    rows
}

fn render_export_layout_plan_csv(plan: &ExportLayoutPlan, show_operations: bool) -> String {
    let mut rows = Vec::new();
    if show_operations {
        rows.push(vec![
            "action".to_string(),
            "variant".to_string(),
            "uid".to_string(),
            "from".to_string(),
            "to".to_string(),
            "reason".to_string(),
        ]);
        rows.extend(plan.operations.iter().map(|operation| {
            vec![
                operation.action.clone(),
                operation.variant.clone(),
                operation.uid.clone(),
                operation.from.clone(),
                operation.to.clone(),
                operation.reason.clone().unwrap_or_default(),
            ]
        }));
        rows.extend(plan.extra_files.iter().map(|extra_file| {
            vec![
                "extra".to_string(),
                extra_file.variant.clone(),
                String::new(),
                extra_file.path.clone(),
                extra_file.path.clone(),
                extra_file.reason.clone(),
            ]
        }));
    } else {
        rows.push(vec![
            "dashboards".to_string(),
            "variants".to_string(),
            "move".to_string(),
            "same".to_string(),
            "blocked".to_string(),
            "extra".to_string(),
            "output".to_string(),
        ]);
        rows.push(vec![
            (plan.summary.move_count + plan.summary.unchanged_count + plan.summary.blocked_count)
                .to_string(),
            plan.variants.join("|"),
            plan.summary.move_count.to_string(),
            plan.summary.unchanged_count.to_string(),
            plan.summary.blocked_count.to_string(),
            plan.summary.extra_file_count.to_string(),
            plan.output_dir.clone().unwrap_or_default(),
        ]);
    }
    rows.into_iter()
        .map(|row| {
            row.into_iter()
                .map(|field| csv_escape(&field))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn render_export_layout_plan_text(plan: &ExportLayoutPlan, show_operations: bool) -> String {
    let mut lines = Vec::new();
    lines.push("Export layout repair plan".to_string());
    lines.push(format!(
        "  Dashboards: {}",
        plan.summary.move_count + plan.summary.unchanged_count + plan.summary.blocked_count
    ));
    lines.push(format!("  Variants: {}", plan.variants.join(", ")));
    lines.push(format!(
        "  Operations: {} move, {} same, {} blocked, {} extra",
        plan.summary.move_count,
        plan.summary.unchanged_count,
        plan.summary.blocked_count,
        plan.summary.extra_file_count
    ));
    if let Some(output_dir) = &plan.output_dir {
        lines.push(format!("  Output: {output_dir}"));
    }
    if show_operations {
        lines.push(String::new());
        lines.push("Operations".to_string());
        lines.extend(plan.operations.iter().map(|operation| {
            let reason = operation
                .reason
                .as_ref()
                .map(|value| format!(" reason={value}"))
                .unwrap_or_default();
            format!(
                "  {} {} uid={} {} -> {}{}",
                operation.action.to_uppercase(),
                operation.variant,
                operation.uid,
                operation.from,
                operation.to,
                reason
            )
        }));
        lines.extend(plan.extra_files.iter().map(|extra_file| {
            format!(
                "  EXTRA {} path={} handling={} reason={}",
                extra_file.variant, extra_file.path, extra_file.handling, extra_file.reason
            )
        }));
    }
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    fn write_json(path: &Path, value: Value) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, serde_json::to_string_pretty(&value).unwrap() + "\n").unwrap();
    }

    fn write_old_export(root: &Path) {
        write_json(
            &root.join("index.json"),
            json!({
                "schemaVersion": 1,
                "kind": "grafana-utils-dashboard-export-index",
                "items": [
                    {
                        "uid": "cpu-main",
                        "title": "CPU",
                        "folderTitle": "Infra",
                        "org": "Main Org.",
                        "orgId": "1",
                        "raw_path": root.join("raw/Infra/CPU__cpu-main.json").display().to_string(),
                        "prompt_path": root.join("prompt/Infra/CPU__cpu-main.json").display().to_string()
                    }
                ],
                "variants": {
                    "raw": root.join("raw/index.json").display().to_string(),
                    "prompt": root.join("prompt/index.json").display().to_string(),
                    "provisioning": null
                },
                "folders": []
            }),
        );
        write_json(
            &root.join("raw/export-metadata.json"),
            json!({
                "schemaVersion": 1,
                "kind": "grafana-utils-dashboard-export-index",
                "variant": "raw",
                "dashboardCount": 1,
                "indexFile": "index.json",
                "foldersFile": "folders.json"
            }),
        );
        write_json(
            &root.join("raw/folders.json"),
            json!([
                {
                    "uid": "infra",
                    "title": "Infra",
                    "path": "Platform / Team / Infra",
                    "org": "Main Org.",
                    "orgId": "1"
                }
            ]),
        );
        write_json(
            &root.join("raw/index.json"),
            json!([
                {
                    "uid": "cpu-main",
                    "title": "CPU",
                    "path": root.join("raw/Infra/CPU__cpu-main.json").display().to_string(),
                    "format": "grafana-web-import-preserve-uid",
                    "org": "Main Org.",
                    "orgId": "1"
                }
            ]),
        );
        write_json(
            &root.join("prompt/export-metadata.json"),
            json!({
                "schemaVersion": 1,
                "kind": "grafana-utils-dashboard-export-index",
                "variant": "prompt",
                "dashboardCount": 1,
                "indexFile": "index.json"
            }),
        );
        write_json(
            &root.join("prompt/index.json"),
            json!([
                {
                    "uid": "cpu-main",
                    "title": "CPU",
                    "path": root.join("prompt/Infra/CPU__cpu-main.json").display().to_string(),
                    "format": "grafana-web-import-with-datasource-inputs",
                    "org": "Main Org.",
                    "orgId": "1"
                }
            ]),
        );
        write_json(
            &root.join("raw/Infra/CPU__cpu-main.json"),
            json!({
                "uid": "cpu-main",
                "title": "CPU",
                "meta": {"folderUid": "infra"}
            }),
        );
        write_json(
            &root.join("prompt/Infra/CPU__cpu-main.json"),
            json!({"uid": "cpu-main", "title": "CPU"}),
        );
    }

    #[test]
    fn export_layout_plan_repairs_raw_and_prompt_from_folder_inventory() {
        let temp = tempdir().unwrap();
        let input = temp.path().join("dashboards");
        write_old_export(&input);
        let args = ExportLayoutArgs {
            input_dir: input,
            output_dir: Some(temp.path().join("fixed")),
            in_place: false,
            backup_dir: None,
            variant: Vec::new(),
            raw_dir: None,
            folders_file: None,
            dry_run: true,
            overwrite: false,
            show_operations: false,
            output_format: ExportLayoutOutputFormat::Json,
            no_header: false,
            color: crate::common::CliColorChoice::Never,
        };

        let plan = build_export_layout_plan(&args, &selected_variants(&args)).unwrap();

        assert_eq!(plan.summary.move_count, 2);
        assert_eq!(plan.summary.blocked_count, 0);
        assert!(plan
            .operations
            .iter()
            .any(|operation| operation.to == "Platform/Team/Infra/CPU__cpu-main.json"));
    }

    #[test]
    fn export_layout_plan_uses_unique_root_index_folder_title_when_raw_meta_is_missing() {
        let temp = tempdir().unwrap();
        let input = temp.path().join("dashboards");
        write_old_export(&input);
        write_json(
            &input.join("raw/Infra/CPU__cpu-main.json"),
            json!({
                "uid": "cpu-main",
                "title": "CPU"
            }),
        );
        let args = ExportLayoutArgs {
            input_dir: input,
            output_dir: Some(temp.path().join("fixed")),
            in_place: false,
            backup_dir: None,
            variant: Vec::new(),
            raw_dir: None,
            folders_file: None,
            dry_run: true,
            overwrite: false,
            show_operations: false,
            output_format: ExportLayoutOutputFormat::Json,
            no_header: false,
            color: crate::common::CliColorChoice::Never,
        };

        let plan = build_export_layout_plan(&args, &selected_variants(&args)).unwrap();

        assert_eq!(plan.summary.move_count, 2);
        assert_eq!(plan.summary.blocked_count, 0);
        assert!(plan
            .operations
            .iter()
            .all(|operation| operation.folder_uid.as_deref() == Some("infra")));
    }

    #[test]
    fn export_layout_plan_reports_files_not_listed_in_variant_index() {
        let temp = tempdir().unwrap();
        let input = temp.path().join("dashboards");
        write_old_export(&input);
        write_json(
            &input.join("raw/Infra/Manual__manual.json"),
            json!({"uid": "manual", "title": "Manual"}),
        );
        fs::write(input.join("prompt/Infra/operator-note.txt"), "local note\n").unwrap();
        let args = ExportLayoutArgs {
            input_dir: input,
            output_dir: Some(temp.path().join("fixed")),
            in_place: false,
            backup_dir: None,
            variant: Vec::new(),
            raw_dir: None,
            folders_file: None,
            dry_run: true,
            overwrite: false,
            show_operations: false,
            output_format: ExportLayoutOutputFormat::Json,
            no_header: false,
            color: crate::common::CliColorChoice::Never,
        };

        let plan = build_export_layout_plan(&args, &selected_variants(&args)).unwrap();

        assert_eq!(plan.summary.extra_file_count, 2);
        assert!(plan
            .extra_files
            .iter()
            .any(|file| file.path == "Infra/Manual__manual.json"));
        assert!(plan
            .extra_files
            .iter()
            .any(|file| file.path == "Infra/operator-note.txt"));
    }

    #[test]
    fn export_layout_entry_path_keeps_folder_segments_named_like_variant() {
        let variant_dir = Path::new("/tmp/export/raw");
        let relative = entry_relative_path(
            "dashboards/org_1_Main_Org/raw/Team/raw/CPU__cpu-main.json",
            variant_dir,
            LayoutVariant::Raw,
        )
        .unwrap();

        assert_eq!(relative, PathBuf::from("Team/raw/CPU__cpu-main.json"));
    }

    #[test]
    fn export_layout_text_output_defaults_to_summary_only() {
        let temp = tempdir().unwrap();
        let input = temp.path().join("dashboards");
        write_old_export(&input);
        let args = ExportLayoutArgs {
            input_dir: input,
            output_dir: Some(temp.path().join("fixed")),
            in_place: false,
            backup_dir: None,
            variant: Vec::new(),
            raw_dir: None,
            folders_file: None,
            dry_run: true,
            overwrite: false,
            show_operations: false,
            output_format: ExportLayoutOutputFormat::Text,
            no_header: false,
            color: crate::common::CliColorChoice::Never,
        };
        let plan = build_export_layout_plan(&args, &selected_variants(&args)).unwrap();

        let output = render_export_layout_plan_text(&plan, false);

        assert!(output.contains("Dashboards: 2"));
        assert!(output.contains("Operations: 2 move, 0 same, 0 blocked, 0 extra"));
        assert!(!output.contains("MOVE raw uid=cpu-main"));
    }

    #[test]
    fn export_layout_text_output_show_operations_includes_operations() {
        let temp = tempdir().unwrap();
        let input = temp.path().join("dashboards");
        write_old_export(&input);
        let args = ExportLayoutArgs {
            input_dir: input,
            output_dir: Some(temp.path().join("fixed")),
            in_place: false,
            backup_dir: None,
            variant: Vec::new(),
            raw_dir: None,
            folders_file: None,
            dry_run: true,
            overwrite: false,
            show_operations: true,
            output_format: ExportLayoutOutputFormat::Text,
            no_header: false,
            color: crate::common::CliColorChoice::Never,
        };
        let plan = build_export_layout_plan(&args, &selected_variants(&args)).unwrap();

        let output = render_export_layout_plan_text(&plan, true);

        assert!(output.contains("Dashboards: 2"));
        assert!(output.contains("Operations\n"));
        assert!(output.contains("MOVE raw uid=cpu-main"));
        assert!(
            output.contains("Infra/CPU__cpu-main.json -> Platform/Team/Infra/CPU__cpu-main.json")
        );
    }

    #[test]
    fn export_layout_table_output_defaults_to_summary_rows() {
        let temp = tempdir().unwrap();
        let input = temp.path().join("dashboards");
        write_old_export(&input);
        let args = ExportLayoutArgs {
            input_dir: input,
            output_dir: Some(temp.path().join("fixed")),
            in_place: false,
            backup_dir: None,
            variant: Vec::new(),
            raw_dir: None,
            folders_file: None,
            dry_run: true,
            overwrite: false,
            show_operations: false,
            output_format: ExportLayoutOutputFormat::Table,
            no_header: false,
            color: crate::common::CliColorChoice::Never,
        };
        let plan = build_export_layout_plan(&args, &selected_variants(&args)).unwrap();

        let output = render_export_layout_plan_table(&plan, false, true).join("\n");

        assert!(output.contains("FIELD"));
        assert!(output.contains("dashboards"));
        assert!(output.contains("move"));
        assert!(!output.contains("ACTION"));
        assert!(!output.contains("cpu-main"));
    }

    #[test]
    fn export_layout_csv_output_supports_summary_and_operation_rows() {
        let temp = tempdir().unwrap();
        let input = temp.path().join("dashboards");
        write_old_export(&input);
        let args = ExportLayoutArgs {
            input_dir: input,
            output_dir: Some(temp.path().join("fixed")),
            in_place: false,
            backup_dir: None,
            variant: Vec::new(),
            raw_dir: None,
            folders_file: None,
            dry_run: true,
            overwrite: false,
            show_operations: false,
            output_format: ExportLayoutOutputFormat::Csv,
            no_header: false,
            color: crate::common::CliColorChoice::Never,
        };
        let plan = build_export_layout_plan(&args, &selected_variants(&args)).unwrap();

        let summary = render_export_layout_plan_csv(&plan, false);
        let operations = render_export_layout_plan_csv(&plan, true);

        assert!(summary.starts_with("dashboards,variants,move,same,blocked,extra,output\n"));
        assert!(summary.contains("2,raw|prompt,2,0,0,0,"));
        assert!(operations.starts_with("action,variant,uid,from,to,reason\n"));
        assert!(operations.contains("move,raw,cpu-main"));
    }

    #[test]
    fn export_layout_repair_copy_mode_updates_files_and_indexes() {
        let temp = tempdir().unwrap();
        let input = temp.path().join("dashboards");
        let output = temp.path().join("fixed");
        write_old_export(&input);
        let args = ExportLayoutArgs {
            input_dir: input,
            output_dir: Some(output.clone()),
            in_place: false,
            backup_dir: None,
            variant: Vec::new(),
            raw_dir: None,
            folders_file: None,
            dry_run: false,
            overwrite: true,
            show_operations: false,
            output_format: ExportLayoutOutputFormat::Text,
            no_header: false,
            color: crate::common::CliColorChoice::Never,
        };

        run_export_layout_repair(&args).unwrap();

        assert!(output
            .join("raw/Platform/Team/Infra/CPU__cpu-main.json")
            .is_file());
        assert!(output
            .join("prompt/Platform/Team/Infra/CPU__cpu-main.json")
            .is_file());
        let raw_index: Vec<VariantIndexEntry> =
            serde_json::from_str(&fs::read_to_string(output.join("raw/index.json")).unwrap())
                .unwrap();
        assert!(raw_index[0].path.contains("Platform/Team/Infra"));
        let root_index: RootExportIndex =
            serde_json::from_str(&fs::read_to_string(output.join("index.json")).unwrap()).unwrap();
        assert!(root_index.items[0]
            .raw_path
            .as_ref()
            .unwrap()
            .contains("Platform/Team/Infra"));
    }

    #[test]
    fn export_layout_repair_in_place_writes_backup_before_move() {
        let temp = tempdir().unwrap();
        let input = temp.path().join("dashboards");
        let backup = temp.path().join("backup");
        write_old_export(&input);
        let args = ExportLayoutArgs {
            input_dir: input.clone(),
            output_dir: None,
            in_place: true,
            backup_dir: Some(backup.clone()),
            variant: vec![ExportLayoutVariant::Raw],
            raw_dir: None,
            folders_file: None,
            dry_run: false,
            overwrite: true,
            show_operations: false,
            output_format: ExportLayoutOutputFormat::Text,
            no_header: false,
            color: crate::common::CliColorChoice::Never,
        };

        run_export_layout_repair(&args).unwrap();

        assert!(input
            .join("raw/Platform/Team/Infra/CPU__cpu-main.json")
            .is_file());
        assert!(backup.join("raw/Infra/CPU__cpu-main.json").is_file());
        assert!(input.join("prompt/Infra/CPU__cpu-main.json").is_file());
    }
}
