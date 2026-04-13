//! Compatibility re-exports that should not clutter the dashboard facade.

#[allow(unused_imports)]
pub(crate) use super::governance_policy::{
    load_builtin_governance_policy, load_governance_policy, load_governance_policy_file,
    load_governance_policy_source,
};

#[allow(unused_imports)]
pub(crate) use super::live::{
    build_datasource_inventory_record, build_folder_path, collect_folder_inventory_with_request,
    fetch_dashboard_if_exists_with_request, fetch_dashboard_permissions_with_request,
    fetch_dashboard_with_request, fetch_folder_if_exists_with_request,
    fetch_folder_permissions_with_request, format_folder_inventory_status_line,
    import_dashboard_request_with_request, list_dashboard_summaries_with_request,
    list_datasources_with_request,
};

#[allow(unused_imports)]
pub(crate) use super::live_project_status::{
    build_live_dashboard_domain_status, build_live_dashboard_domain_status_from_inputs,
    collect_live_dashboard_project_status_inputs_with_request, LiveDashboardProjectStatusInputs,
};
