//! Shared render/format helpers for access CLI output.
//! Centralizes normalization, tabular formatting, and access-specific row shaping.

#[path = "render_access.rs"]
mod access_rows;
#[path = "render_normalization.rs"]
mod normalization;
#[path = "render_tabular.rs"]
mod tabular;

pub(crate) use crate::tabular_output::render_yaml;
pub(crate) use access_rows::{
    access_delete_summary_line, access_diff_review_line, access_diff_summary_line,
    access_export_summary_line, access_import_summary_line, build_access_delete_review_document,
    build_access_diff_review_document, normalize_service_account_row, normalize_team_row,
    normalize_user_row,
};
pub(crate) use normalization::{
    bool_label, map_get_text, normalize_org_role, scalar_text, service_account_role_to_api,
    user_account_scope_text, user_scope_text, value_bool,
};
pub(crate) use tabular::{
    format_table, paginate_rows, render_csv, render_objects_json, service_account_list_column_ids,
    service_account_summary_line, service_account_table_headers, service_account_table_rows,
    team_list_column_ids, team_summary_line, team_table_headers, team_table_rows,
    user_list_column_ids, user_matches, user_summary_line, user_table_headers, user_table_rows,
};
