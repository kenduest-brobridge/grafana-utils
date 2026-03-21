//! Inspection report model and aggregation surface.
//! Defines summary/row schemas and grouped/report helpers used by both CLI renderers and tests.
use serde::Serialize;

use crate::common::{message, Result};

use super::InspectExportReportFormat;

/// Struct definition for QueryReportSummary.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct QueryReportSummary {
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    #[serde(rename = "queryRecordCount")]
    pub(crate) report_row_count: usize,
}

/// Struct definition for ExportInspectionQueryRow.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub(crate) struct ExportInspectionQueryRow {
    pub(crate) org: String,
    #[serde(rename = "orgId")]
    pub(crate) org_id: String,
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    pub(crate) dashboard_title: String,
    #[serde(rename = "dashboardTags")]
    pub(crate) dashboard_tags: Vec<String>,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    #[serde(rename = "folderFullPath")]
    pub(crate) folder_full_path: String,
    #[serde(rename = "folderLevel")]
    pub(crate) folder_level: String,
    #[serde(rename = "folderUid")]
    pub(crate) folder_uid: String,
    #[serde(rename = "parentFolderUid")]
    pub(crate) parent_folder_uid: String,
    #[serde(rename = "panelId")]
    pub(crate) panel_id: String,
    #[serde(rename = "panelTitle")]
    pub(crate) panel_title: String,
    #[serde(rename = "panelType")]
    pub(crate) panel_type: String,
    #[serde(rename = "panelTargetCount")]
    pub(crate) panel_target_count: usize,
    #[serde(rename = "panelQueryCount")]
    pub(crate) panel_query_count: usize,
    #[serde(rename = "panelDatasourceCount")]
    pub(crate) panel_datasource_count: usize,
    #[serde(rename = "panelVariables")]
    pub(crate) panel_variables: Vec<String>,
    #[serde(rename = "refId")]
    pub(crate) ref_id: String,
    pub(crate) datasource: String,
    #[serde(rename = "datasourceName")]
    pub(crate) datasource_name: String,
    #[serde(rename = "datasourceUid")]
    pub(crate) datasource_uid: String,
    #[serde(rename = "datasourceOrg")]
    pub(crate) datasource_org: String,
    #[serde(rename = "datasourceOrgId")]
    pub(crate) datasource_org_id: String,
    #[serde(rename = "datasourceDatabase")]
    pub(crate) datasource_database: String,
    #[serde(rename = "datasourceBucket")]
    pub(crate) datasource_bucket: String,
    #[serde(rename = "datasourceOrganization")]
    pub(crate) datasource_organization: String,
    #[serde(rename = "datasourceIndexPattern")]
    pub(crate) datasource_index_pattern: String,
    #[serde(rename = "datasourceType")]
    pub(crate) datasource_type: String,
    #[serde(rename = "datasourceFamily")]
    pub(crate) datasource_family: String,
    #[serde(rename = "queryField")]
    pub(crate) query_field: String,
    #[serde(rename = "targetHidden")]
    pub(crate) target_hidden: String,
    #[serde(rename = "targetDisabled")]
    pub(crate) target_disabled: String,
    #[serde(rename = "query")]
    pub(crate) query_text: String,
    #[serde(rename = "queryVariables")]
    pub(crate) query_variables: Vec<String>,
    pub(crate) metrics: Vec<String>,
    pub(crate) functions: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
    #[serde(rename = "file")]
    pub(crate) file_path: String,
}

/// Struct definition for ExportInspectionQueryReport.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionQueryReport {
    pub(crate) import_dir: String,
    pub(crate) summary: QueryReportSummary,
    pub(crate) queries: Vec<ExportInspectionQueryRow>,
}

/// Struct definition for ExportInspectionQueryReportJsonSummary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionQueryReportJsonSummary {
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "queryRecordCount")]
    pub(crate) query_record_count: usize,
}

/// Struct definition for ExportInspectionQueryReportDocument.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionQueryReportDocument {
    pub(crate) summary: ExportInspectionQueryReportJsonSummary,
    pub(crate) queries: Vec<ExportInspectionQueryRow>,
}

/// Struct definition for GroupedQueryPanel.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GroupedQueryPanel {
    pub(crate) panel_id: String,
    pub(crate) panel_title: String,
    pub(crate) panel_type: String,
    pub(crate) panel_target_count: usize,
    pub(crate) panel_query_count: usize,
    pub(crate) datasources: Vec<String>,
    pub(crate) datasource_families: Vec<String>,
    pub(crate) query_fields: Vec<String>,
    pub(crate) queries: Vec<ExportInspectionQueryRow>,
}

/// Struct definition for GroupedQueryDashboard.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GroupedQueryDashboard {
    pub(crate) org: String,
    pub(crate) org_id: String,
    pub(crate) dashboard_uid: String,
    pub(crate) dashboard_title: String,
    pub(crate) folder_path: String,
    pub(crate) folder_uid: String,
    pub(crate) parent_folder_uid: String,
    pub(crate) file_path: String,
    pub(crate) datasources: Vec<String>,
    pub(crate) datasource_families: Vec<String>,
    pub(crate) panels: Vec<GroupedQueryPanel>,
}

/// Struct definition for NormalizedQueryReport.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NormalizedQueryReport {
    pub(crate) import_dir: String,
    pub(crate) summary: QueryReportSummary,
    pub(crate) dashboards: Vec<GroupedQueryDashboard>,
}

/// Constant for default report column ids.
pub(crate) const DEFAULT_REPORT_COLUMN_IDS: &[&str] = &[
    "org",
    "org_id",
    "dashboard_uid",
    "dashboard_title",
    "dashboard_tags",
    "folder_path",
    "folder_full_path",
    "folder_level",
    "folder_uid",
    "parent_folder_uid",
    "panel_id",
    "panel_title",
    "panel_type",
    "panel_query_count",
    "panel_datasource_count",
    "panel_variables",
    "ref_id",
    "datasource",
    "datasource_name",
    "datasource_org",
    "datasource_org_id",
    "datasource_database",
    "datasource_bucket",
    "datasource_organization",
    "datasource_index_pattern",
    "datasource_type",
    "datasource_family",
    "query_field",
    "query_variables",
    "metrics",
    "functions",
    "measurements",
    "buckets",
    "query",
    "file",
];

/// Constant for supported report column ids.
pub(crate) const SUPPORTED_REPORT_COLUMN_IDS: &[&str] = &[
    "org",
    "org_id",
    "dashboard_uid",
    "dashboard_title",
    "dashboard_tags",
    "folder_path",
    "folder_full_path",
    "folder_level",
    "folder_uid",
    "parent_folder_uid",
    "panel_id",
    "panel_title",
    "panel_type",
    "panel_target_count",
    "panel_query_count",
    "panel_datasource_count",
    "panel_variables",
    "ref_id",
    "datasource",
    "datasource_name",
    "datasource_uid",
    "datasource_org",
    "datasource_org_id",
    "datasource_database",
    "datasource_bucket",
    "datasource_organization",
    "datasource_index_pattern",
    "datasource_type",
    "datasource_family",
    "query_field",
    "target_hidden",
    "target_disabled",
    "query_variables",
    "metrics",
    "functions",
    "measurements",
    "buckets",
    "query",
    "file",
];

fn normalize_report_column_id(value: &str) -> &str {
    match value {
        "orgId" => "org_id",
        "dashboardUid" => "dashboard_uid",
        "dashboardTitle" => "dashboard_title",
        "dashboardTags" => "dashboard_tags",
        "folderPath" => "folder_path",
        "folderFullPath" => "folder_full_path",
        "folderLevel" => "folder_level",
        "folderUid" => "folder_uid",
        "parentFolderUid" => "parent_folder_uid",
        "panelId" => "panel_id",
        "panelTitle" => "panel_title",
        "panelType" => "panel_type",
        "panelTargetCount" => "panel_target_count",
        "panelQueryCount" => "panel_query_count",
        "panelDatasourceCount" => "panel_datasource_count",
        "panelVariables" => "panel_variables",
        "refId" => "ref_id",
        "datasourceName" => "datasource_name",
        "datasourceUid" => "datasource_uid",
        "datasourceOrg" => "datasource_org",
        "datasourceOrgId" => "datasource_org_id",
        "datasourceDatabase" => "datasource_database",
        "datasourceBucket" => "datasource_bucket",
        "datasourceOrganization" => "datasource_organization",
        "datasourceIndexPattern" => "datasource_index_pattern",
        "datasourceType" => "datasource_type",
        "datasourceFamily" => "datasource_family",
        "queryField" => "query_field",
        "targetHidden" => "target_hidden",
        "targetDisabled" => "target_disabled",
        "queryVariables" => "query_variables",
        "functions" => "functions",
        _ => value,
    }
}

/// Purpose: implementation note.
pub(crate) fn build_query_report(
    import_dir: String,
    dashboard_count: usize,
    panel_count: usize,
    query_count: usize,
    queries: Vec<ExportInspectionQueryRow>,
) -> ExportInspectionQueryReport {
    ExportInspectionQueryReport {
        import_dir,
        summary: QueryReportSummary {
            dashboard_count,
            panel_count,
            query_count,
            report_row_count: queries.len(),
        },
        queries,
    }
}

/// Purpose: implementation note.
pub(crate) fn build_export_inspection_query_report_document(
    report: &ExportInspectionQueryReport,
) -> ExportInspectionQueryReportDocument {
    ExportInspectionQueryReportDocument {
        summary: ExportInspectionQueryReportJsonSummary {
            dashboard_count: report.summary.dashboard_count,
            query_record_count: report.queries.len(),
        },
        queries: report.queries.clone(),
    }
}

/// refresh filtered query report summary.
pub(crate) fn refresh_filtered_query_report_summary(report: &mut ExportInspectionQueryReport) {
    report.summary.dashboard_count = report
        .queries
        .iter()
        .map(|row| row.dashboard_uid.clone())
        .collect::<std::collections::BTreeSet<String>>()
        .len();
    report.summary.panel_count = report
        .queries
        .iter()
        .map(|row| {
            (
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                row.panel_title.clone(),
            )
        })
        .collect::<std::collections::BTreeSet<(String, String, String)>>()
        .len();
    report.summary.query_count = report.queries.len();
    report.summary.report_row_count = report.queries.len();
}

#[cfg_attr(not(test), allow(dead_code))]
/// Purpose: implementation note.
pub(crate) fn resolve_report_column_ids(selected: &[String]) -> Result<Vec<String>> {
    resolve_report_column_ids_for_format(None, selected)
}

/// Purpose: implementation note.
pub(crate) fn resolve_report_column_ids_for_format(
    report_format: Option<InspectExportReportFormat>,
    selected: &[String],
) -> Result<Vec<String>> {
    if selected.is_empty() {
        let defaults = if matches!(report_format, Some(InspectExportReportFormat::Csv)) {
            SUPPORTED_REPORT_COLUMN_IDS
        } else {
            DEFAULT_REPORT_COLUMN_IDS
        };
        return Ok(defaults.iter().map(|value| value.to_string()).collect());
    }
    let mut result = Vec::new();
    for value in selected {
        let normalized = normalize_report_column_id(value.trim());
        if normalized.is_empty() {
            continue;
        }
        if normalized == "all" {
            return Ok(SUPPORTED_REPORT_COLUMN_IDS
                .iter()
                .map(|value| value.to_string())
                .collect());
        }
        if !SUPPORTED_REPORT_COLUMN_IDS.contains(&normalized) {
            return Err(message(format!(
                "Unsupported --report-columns value {:?}. Supported columns: {}",
                normalized,
                std::iter::once("all")
                    .chain(SUPPORTED_REPORT_COLUMN_IDS.iter().copied())
                    .collect::<Vec<&str>>()
                    .join(",")
            )));
        }
        if !result.iter().any(|item| item == normalized) {
            result.push(normalized.to_string());
        }
    }
    if result.is_empty() {
        return Err(message(format!(
            "--report-columns did not include any supported columns. Supported columns: {}",
            std::iter::once("all")
                .chain(SUPPORTED_REPORT_COLUMN_IDS.iter().copied())
                .collect::<Vec<&str>>()
                .join(",")
        )));
    }
    Ok(result)
}

/// report column header.
pub(crate) fn report_column_header(column_id: &str) -> &'static str {
    match column_id {
        "org" => "ORG",
        "org_id" => "ORG_ID",
        "dashboard_uid" => "DASHBOARD_UID",
        "dashboard_title" => "DASHBOARD_TITLE",
        "dashboard_tags" => "DASHBOARD_TAGS",
        "folder_path" => "FOLDER_PATH",
        "folder_full_path" => "FOLDER_FULL_PATH",
        "folder_level" => "FOLDER_LEVEL",
        "folder_uid" => "FOLDER_UID",
        "parent_folder_uid" => "PARENT_FOLDER_UID",
        "panel_id" => "PANEL_ID",
        "panel_title" => "PANEL_TITLE",
        "panel_type" => "PANEL_TYPE",
        "panel_target_count" => "PANEL_TARGET_COUNT",
        "panel_query_count" => "PANEL_EFFECTIVE_QUERY_COUNT",
        "panel_datasource_count" => "PANEL_TOTAL_DATASOURCE_COUNT",
        "panel_variables" => "PANEL_VARIABLES",
        "ref_id" => "REF_ID",
        "datasource" => "DATASOURCE",
        "datasource_name" => "DATASOURCE_NAME",
        "datasource_uid" => "DATASOURCE_UID",
        "datasource_org" => "DATASOURCE_ORG",
        "datasource_org_id" => "DATASOURCE_ORG_ID",
        "datasource_database" => "DATASOURCE_DATABASE",
        "datasource_bucket" => "DATASOURCE_BUCKET",
        "datasource_organization" => "DATASOURCE_ORGANIZATION",
        "datasource_index_pattern" => "DATASOURCE_INDEX_PATTERN",
        "datasource_type" => "DATASOURCE_TYPE",
        "datasource_family" => "DATASOURCE_FAMILY",
        "query_field" => "QUERY_FIELD",
        "target_hidden" => "TARGET_HIDDEN",
        "target_disabled" => "TARGET_DISABLED",
        "query_variables" => "QUERY_VARIABLES",
        "metrics" => "METRICS",
        "functions" => "FUNCTIONS",
        "measurements" => "MEASUREMENTS",
        "buckets" => "BUCKETS",
        "query" => "QUERY",
        "file" => "FILE",
        _ => unreachable!("unsupported report column header"),
    }
}

/// Purpose: implementation note.
pub(crate) fn render_query_report_column(
    row: &ExportInspectionQueryRow,
    column_id: &str,
) -> String {
    match column_id {
        "org" => row.org.clone(),
        "org_id" => row.org_id.clone(),
        "dashboard_uid" => row.dashboard_uid.clone(),
        "dashboard_title" => row.dashboard_title.clone(),
        "dashboard_tags" => row.dashboard_tags.join(","),
        "folder_path" => row.folder_path.clone(),
        "folder_full_path" => row.folder_full_path.clone(),
        "folder_level" => row.folder_level.clone(),
        "folder_uid" => row.folder_uid.clone(),
        "parent_folder_uid" => row.parent_folder_uid.clone(),
        "panel_id" => row.panel_id.clone(),
        "panel_title" => row.panel_title.clone(),
        "panel_type" => row.panel_type.clone(),
        "panel_target_count" => row.panel_target_count.to_string(),
        "panel_query_count" => row.panel_query_count.to_string(),
        "panel_datasource_count" => row.panel_datasource_count.to_string(),
        "panel_variables" => row.panel_variables.join(","),
        "ref_id" => row.ref_id.clone(),
        "datasource" => row.datasource.clone(),
        "datasource_name" => row.datasource_name.clone(),
        "datasource_uid" => row.datasource_uid.clone(),
        "datasource_org" => row.datasource_org.clone(),
        "datasource_org_id" => row.datasource_org_id.clone(),
        "datasource_database" => row.datasource_database.clone(),
        "datasource_bucket" => row.datasource_bucket.clone(),
        "datasource_organization" => row.datasource_organization.clone(),
        "datasource_index_pattern" => row.datasource_index_pattern.clone(),
        "datasource_type" => row.datasource_type.clone(),
        "datasource_family" => row.datasource_family.clone(),
        "query_field" => row.query_field.clone(),
        "target_hidden" => row.target_hidden.clone(),
        "target_disabled" => row.target_disabled.clone(),
        "query_variables" => row.query_variables.join(","),
        "metrics" => row.metrics.join(","),
        "functions" => row.functions.join(","),
        "measurements" => row.measurements.join(","),
        "buckets" => row.buckets.join(","),
        "query" => row.query_text.clone(),
        "file" => row.file_path.clone(),
        _ => unreachable!("unsupported report column value"),
    }
}

/// report format supports columns.
pub(crate) fn report_format_supports_columns(format: InspectExportReportFormat) -> bool {
    matches!(
        format,
        InspectExportReportFormat::Table
            | InspectExportReportFormat::Csv
            | InspectExportReportFormat::TreeTable
    )
}

// Group query rows by dashboard/panel so report output is deterministic and renderable.
/// Purpose: implementation note.
pub(crate) fn normalize_query_report(
    report: &ExportInspectionQueryReport,
) -> NormalizedQueryReport {
    let mut dashboards = Vec::new();
    for row in &report.queries {
        let dashboard_index = dashboards
            .iter()
            .position(|item: &GroupedQueryDashboard| item.dashboard_uid == row.dashboard_uid)
            .unwrap_or_else(|| {
                dashboards.push(GroupedQueryDashboard {
                    org: row.org.clone(),
                    org_id: row.org_id.clone(),
                    dashboard_uid: row.dashboard_uid.clone(),
                    dashboard_title: row.dashboard_title.clone(),
                    folder_path: row.folder_path.clone(),
                    folder_uid: row.folder_uid.clone(),
                    parent_folder_uid: row.parent_folder_uid.clone(),
                    file_path: row.file_path.clone(),
                    datasources: Vec::new(),
                    datasource_families: Vec::new(),
                    panels: Vec::new(),
                });
                dashboards.len() - 1
            });
        if !row.file_path.is_empty() && dashboards[dashboard_index].file_path.is_empty() {
            dashboards[dashboard_index].file_path = row.file_path.clone();
        }
        if dashboards[dashboard_index].org.is_empty() {
            dashboards[dashboard_index].org = row.org.clone();
        }
        if dashboards[dashboard_index].org_id.is_empty() {
            dashboards[dashboard_index].org_id = row.org_id.clone();
        }
        if dashboards[dashboard_index].folder_uid.is_empty() {
            dashboards[dashboard_index].folder_uid = row.folder_uid.clone();
        }
        if dashboards[dashboard_index].parent_folder_uid.is_empty() {
            dashboards[dashboard_index].parent_folder_uid = row.parent_folder_uid.clone();
        }
        if !row.datasource.is_empty()
            && !dashboards[dashboard_index]
                .datasources
                .iter()
                .any(|value| value == &row.datasource)
        {
            dashboards[dashboard_index]
                .datasources
                .push(row.datasource.clone());
        }
        if !row.datasource_family.is_empty()
            && !dashboards[dashboard_index]
                .datasource_families
                .iter()
                .any(|value| value == &row.datasource_family)
        {
            dashboards[dashboard_index]
                .datasource_families
                .push(row.datasource_family.clone());
        }
        let panels = &mut dashboards[dashboard_index].panels;
        let panel_index = panels
            .iter()
            .position(|item| {
                item.panel_id == row.panel_id
                    && item.panel_title == row.panel_title
                    && item.panel_type == row.panel_type
            })
            .unwrap_or_else(|| {
                panels.push(GroupedQueryPanel {
                    panel_id: row.panel_id.clone(),
                    panel_title: row.panel_title.clone(),
                    panel_type: row.panel_type.clone(),
                    panel_target_count: row.panel_target_count,
                    panel_query_count: row.panel_query_count,
                    datasources: Vec::new(),
                    datasource_families: Vec::new(),
                    query_fields: Vec::new(),
                    queries: Vec::new(),
                });
                panels.len() - 1
            });
        panels[panel_index].panel_target_count = panels[panel_index]
            .panel_target_count
            .max(row.panel_target_count);
        panels[panel_index].panel_query_count = panels[panel_index]
            .panel_query_count
            .max(row.panel_query_count);
        if !row.datasource.is_empty()
            && !panels[panel_index]
                .datasources
                .iter()
                .any(|value| value == &row.datasource)
        {
            panels[panel_index].datasources.push(row.datasource.clone());
        }
        if !row.datasource_family.is_empty()
            && !panels[panel_index]
                .datasource_families
                .iter()
                .any(|value| value == &row.datasource_family)
        {
            panels[panel_index]
                .datasource_families
                .push(row.datasource_family.clone());
        }
        if !row.query_field.is_empty()
            && !panels[panel_index]
                .query_fields
                .iter()
                .any(|value| value == &row.query_field)
        {
            panels[panel_index]
                .query_fields
                .push(row.query_field.clone());
        }
        panels[panel_index].queries.push(row.clone());
    }
    NormalizedQueryReport {
        import_dir: report.import_dir.clone(),
        summary: report.summary.clone(),
        dashboards,
    }
}
