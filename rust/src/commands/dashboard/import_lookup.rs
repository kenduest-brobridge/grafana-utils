//! Import orchestration for Dashboard resources, including input normalization and apply contract handling.

#[path = "import_lookup_cache.rs"]
mod import_lookup_cache;
#[path = "import_lookup_folder.rs"]
mod import_lookup_folder;
#[path = "import_lookup_org.rs"]
mod import_lookup_org;

#[allow(unused_imports)]
pub(crate) use import_lookup_cache::{
    dashboard_exists_with_summary, dashboard_summary_folder_uid,
    dashboard_summary_folder_uid_with_client, determine_dashboard_import_action_with_client,
    determine_dashboard_import_action_with_request, fetch_dashboard_if_exists_cached,
    fetch_dashboard_if_exists_cached_with_client, fetch_folder_if_exists_cached,
    fetch_folder_if_exists_cached_with_client, ImportLookupCache,
};
#[allow(unused_imports)]
pub(crate) use import_lookup_folder::{
    apply_folder_path_guard_to_action, build_folder_path_match_result,
    collect_folder_inventory_statuses_cached, collect_folder_inventory_statuses_with_client,
    determine_import_folder_uid_override_with_client,
    determine_import_folder_uid_override_with_request, ensure_folder_inventory_entry_cached,
    ensure_folder_inventory_entry_with_client, normalize_folder_path,
    resolve_dashboard_import_folder_path_with_client,
    resolve_dashboard_import_folder_path_with_request,
    resolve_existing_dashboard_folder_path_with_client,
    resolve_existing_dashboard_folder_path_with_request, resolve_source_dashboard_folder_path,
};
#[allow(unused_imports)]
pub(crate) use import_lookup_org::{
    list_orgs_cached, list_orgs_cached_with_client, resolve_import_target_org_id_with_client,
    resolve_import_target_org_id_with_request,
};
