use reqwest::Method;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::common::{message, string_field, Result};

use super::{
    collect_folder_inventory_with_request, DeleteArgs, DEFAULT_DASHBOARD_TITLE,
    DEFAULT_FOLDER_TITLE,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct DashboardDeleteTarget {
    pub uid: String,
    pub title: String,
    pub folder_path: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct FolderDeleteTarget {
    pub uid: String,
    pub title: String,
    pub path: String,
    pub parent_uid: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct DeletePlan {
    pub selector_uid: Option<String>,
    pub selector_path: Option<String>,
    pub delete_folders: bool,
    pub dashboards: Vec<DashboardDeleteTarget>,
    pub folders: Vec<FolderDeleteTarget>,
}

pub(crate) fn normalize_folder_path(path: &str) -> String {
    let normalized = path.trim().replace('\\', "/");
    let parts: Vec<&str> = normalized
        .split('/')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        normalized.trim().to_string()
    } else {
        parts.join(" / ")
    }
}

pub(crate) fn validate_delete_args(args: &DeleteArgs) -> Result<()> {
    let uid = args.uid.as_deref().unwrap_or("").trim();
    let path = normalize_folder_path(args.path.as_deref().unwrap_or(""));
    if !uid.is_empty() && !path.is_empty() {
        return Err(message(
            "Choose either --uid or --path for dashboard delete.",
        ));
    }
    if args.delete_folders && path.is_empty() {
        return Err(message(
            "--delete-folders requires --path for dashboard delete.",
        ));
    }
    if args.table && !args.dry_run {
        return Err(message(
            "--table is only supported with --dry-run for dashboard delete.",
        ));
    }
    if args.json && !args.dry_run {
        return Err(message(
            "--json is only supported with --dry-run for dashboard delete.",
        ));
    }
    if args.no_header && !args.table {
        return Err(message(
            "--no-header is only supported with --dry-run --table for dashboard delete.",
        ));
    }
    if args.interactive {
        if args.table || args.json || args.output_format.is_some() {
            return Err(message(
                "--interactive cannot be combined with machine-readable dashboard delete output.",
            ));
        }
        return Ok(());
    }
    if uid.is_empty() && path.is_empty() {
        return Err(message(
            "Dashboard delete requires --uid or --path unless --interactive is used.",
        ));
    }
    if !args.dry_run && !args.yes {
        return Err(message(
            "Dashboard delete requires --yes unless --dry-run is set.",
        ));
    }
    Ok(())
}

pub(crate) fn build_delete_plan_with_request<F>(
    mut request_json: F,
    args: &DeleteArgs,
) -> Result<DeletePlan>
where
    F: FnMut(Method, &str, &[(String, String)], Option<&Value>) -> Result<Option<Value>>,
{
    let dashboard_summaries =
        super::list_dashboard_summaries_with_request(&mut request_json, args.page_size)?;
    let summaries = super::list::attach_dashboard_folder_paths_with_request(
        &mut request_json,
        &dashboard_summaries,
    )?;
    let uid = args.uid.as_deref().unwrap_or("").trim();
    let root_path = normalize_folder_path(args.path.as_deref().unwrap_or(""));

    let mut dashboards: Vec<DashboardDeleteTarget> = if !uid.is_empty() {
        let summary = summaries
            .iter()
            .find(|item| string_field(item, "uid", "") == uid)
            .ok_or_else(|| message(format!("Dashboard not found by uid: {uid}")))?;
        vec![build_dashboard_target(summary)]
    } else if !root_path.is_empty() {
        let mut matched: Vec<DashboardDeleteTarget> = summaries
            .iter()
            .filter(|item| {
                folder_path_matches(
                    &string_field(item, "folderPath", DEFAULT_FOLDER_TITLE),
                    &root_path,
                )
            })
            .map(build_dashboard_target)
            .collect();
        if matched.is_empty() {
            return Err(message(format!(
                "Dashboard folder path did not match any dashboards: {root_path}"
            )));
        }
        matched.sort_by(|left, right| {
            left.folder_path
                .cmp(&right.folder_path)
                .then(left.title.cmp(&right.title))
                .then(left.uid.cmp(&right.uid))
        });
        matched
    } else {
        return Err(message("Dashboard delete plan requires a selector."));
    };

    let folders = if !root_path.is_empty() && args.delete_folders {
        let folder_inventory = collect_folder_inventory_with_request(
            &mut request_json,
            &summaries
                .iter()
                .filter(|item| {
                    dashboards
                        .iter()
                        .any(|target| target.uid == string_field(item, "uid", ""))
                })
                .cloned()
                .collect::<Vec<Map<String, Value>>>(),
        )?;
        let mut matched: Vec<FolderDeleteTarget> = folder_inventory
            .into_iter()
            .filter(|item| folder_path_matches(&item.path, &root_path))
            .map(|item| FolderDeleteTarget {
                uid: item.uid,
                title: item.title,
                path: item.path,
                parent_uid: item.parent_uid,
            })
            .collect();
        matched.sort_by(|left, right| {
            right
                .path
                .matches(" / ")
                .count()
                .cmp(&left.path.matches(" / ").count())
                .then(left.path.cmp(&right.path))
                .then(left.uid.cmp(&right.uid))
        });
        matched
    } else {
        Vec::new()
    };

    dashboards.sort_by(|left, right| {
        left.folder_path
            .cmp(&right.folder_path)
            .then(left.title.cmp(&right.title))
            .then(left.uid.cmp(&right.uid))
    });
    Ok(DeletePlan {
        selector_uid: if uid.is_empty() {
            None
        } else {
            Some(uid.to_string())
        },
        selector_path: if root_path.is_empty() {
            None
        } else {
            Some(root_path)
        },
        delete_folders: args.delete_folders,
        dashboards,
        folders,
    })
}

fn build_dashboard_target(summary: &Map<String, Value>) -> DashboardDeleteTarget {
    DashboardDeleteTarget {
        uid: string_field(summary, "uid", "").to_string(),
        title: string_field(summary, "title", DEFAULT_DASHBOARD_TITLE).to_string(),
        folder_path: string_field(summary, "folderPath", DEFAULT_FOLDER_TITLE).to_string(),
    }
}

fn folder_path_matches(candidate: &str, root: &str) -> bool {
    candidate == root || candidate.starts_with(&format!("{root} / "))
}
