use serde::Serialize;

use crate::common::{message, Result};

use super::InspectExportReportFormat;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct QueryReportSummary {
    pub(crate) dashboard_count: usize,
    pub(crate) panel_count: usize,
    pub(crate) query_count: usize,
    pub(crate) report_row_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionQueryRow {
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    pub(crate) dashboard_title: String,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    #[serde(rename = "panelId")]
    pub(crate) panel_id: String,
    #[serde(rename = "panelTitle")]
    pub(crate) panel_title: String,
    #[serde(rename = "panelType")]
    pub(crate) panel_type: String,
    #[serde(rename = "refId")]
    pub(crate) ref_id: String,
    pub(crate) datasource: String,
    #[serde(rename = "datasourceUid")]
    pub(crate) datasource_uid: String,
    #[serde(rename = "queryField")]
    pub(crate) query_field: String,
    #[serde(rename = "queryText")]
    pub(crate) query_text: String,
    pub(crate) metrics: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionQueryReport {
    pub(crate) import_dir: String,
    pub(crate) summary: QueryReportSummary,
    pub(crate) queries: Vec<ExportInspectionQueryRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GroupedQueryPanel {
    pub(crate) panel_id: String,
    pub(crate) panel_title: String,
    pub(crate) panel_type: String,
    pub(crate) queries: Vec<ExportInspectionQueryRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GroupedQueryDashboard {
    pub(crate) dashboard_uid: String,
    pub(crate) dashboard_title: String,
    pub(crate) folder_path: String,
    pub(crate) panels: Vec<GroupedQueryPanel>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NormalizedQueryReport {
    pub(crate) import_dir: String,
    pub(crate) summary: QueryReportSummary,
    pub(crate) dashboards: Vec<GroupedQueryDashboard>,
}

pub(crate) const DEFAULT_REPORT_COLUMN_IDS: &[&str] = &[
    "dashboard_uid",
    "dashboard_title",
    "folder_path",
    "panel_id",
    "panel_title",
    "panel_type",
    "ref_id",
    "datasource",
    "query_field",
    "metrics",
    "measurements",
    "buckets",
    "query",
];

pub(crate) const SUPPORTED_REPORT_COLUMN_IDS: &[&str] = &[
    "dashboard_uid",
    "dashboard_title",
    "folder_path",
    "panel_id",
    "panel_title",
    "panel_type",
    "ref_id",
    "datasource",
    "datasource_uid",
    "query_field",
    "metrics",
    "measurements",
    "buckets",
    "query",
];

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

pub(crate) fn resolve_report_column_ids(selected: &[String]) -> Result<Vec<String>> {
    if selected.is_empty() {
        return Ok(DEFAULT_REPORT_COLUMN_IDS
            .iter()
            .map(|value| value.to_string())
            .collect());
    }
    let mut result = Vec::new();
    for value in selected {
        let normalized = value.trim();
        if normalized.is_empty() {
            continue;
        }
        if !SUPPORTED_REPORT_COLUMN_IDS.contains(&normalized) {
            return Err(message(format!(
                "Unsupported --report-columns value {:?}. Supported columns: {}",
                normalized,
                SUPPORTED_REPORT_COLUMN_IDS.join(",")
            )));
        }
        if !result.iter().any(|item| item == normalized) {
            result.push(normalized.to_string());
        }
    }
    if result.is_empty() {
        return Err(message(format!(
            "--report-columns did not include any supported columns. Supported columns: {}",
            SUPPORTED_REPORT_COLUMN_IDS.join(",")
        )));
    }
    Ok(result)
}

pub(crate) fn report_column_header(column_id: &str) -> &'static str {
    match column_id {
        "dashboard_uid" => "DASHBOARD_UID",
        "dashboard_title" => "DASHBOARD_TITLE",
        "folder_path" => "FOLDER_PATH",
        "panel_id" => "PANEL_ID",
        "panel_title" => "PANEL_TITLE",
        "panel_type" => "PANEL_TYPE",
        "ref_id" => "REF_ID",
        "datasource" => "DATASOURCE",
        "datasource_uid" => "DATASOURCE_UID",
        "query_field" => "QUERY_FIELD",
        "metrics" => "METRICS",
        "measurements" => "MEASUREMENTS",
        "buckets" => "BUCKETS",
        "query" => "QUERY",
        _ => unreachable!("unsupported report column header"),
    }
}

pub(crate) fn render_query_report_column(
    row: &ExportInspectionQueryRow,
    column_id: &str,
) -> String {
    match column_id {
        "dashboard_uid" => row.dashboard_uid.clone(),
        "dashboard_title" => row.dashboard_title.clone(),
        "folder_path" => row.folder_path.clone(),
        "panel_id" => row.panel_id.clone(),
        "panel_title" => row.panel_title.clone(),
        "panel_type" => row.panel_type.clone(),
        "ref_id" => row.ref_id.clone(),
        "datasource" => row.datasource.clone(),
        "datasource_uid" => row.datasource_uid.clone(),
        "query_field" => row.query_field.clone(),
        "metrics" => row.metrics.join(","),
        "measurements" => row.measurements.join(","),
        "buckets" => row.buckets.join(","),
        "query" => row.query_text.clone(),
        _ => unreachable!("unsupported report column value"),
    }
}

pub(crate) fn report_format_supports_columns(format: InspectExportReportFormat) -> bool {
    matches!(
        format,
        InspectExportReportFormat::Table
            | InspectExportReportFormat::Csv
            | InspectExportReportFormat::TreeTable
    )
}

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
                    dashboard_uid: row.dashboard_uid.clone(),
                    dashboard_title: row.dashboard_title.clone(),
                    folder_path: row.folder_path.clone(),
                    panels: Vec::new(),
                });
                dashboards.len() - 1
            });
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
                    queries: Vec::new(),
                });
                panels.len() - 1
            });
        panels[panel_index].queries.push(row.clone());
    }
    NormalizedQueryReport {
        import_dir: report.import_dir.clone(),
        summary: report.summary.clone(),
        dashboards,
    }
}
