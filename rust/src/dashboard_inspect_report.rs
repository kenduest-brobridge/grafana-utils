//! Inspection report model and aggregation surface.
//! Defines summary/row schemas and grouped/report helpers used by both CLI renderers and tests.
use serde::Serialize;

use crate::common::{message, Result};

use super::{ExportInspectionSummary, InspectExportReportFormat};

#[derive(Clone, Debug, PartialEq, Serialize)]
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
    #[serde(rename = "query")]
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionQueryReportJsonSummary {
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "datasourceCount")]
    pub(crate) datasource_count: usize,
    #[serde(rename = "datasourceInventoryCount")]
    pub(crate) datasource_inventory_count: usize,
    #[serde(rename = "queryRecordCount")]
    pub(crate) query_record_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionQueryReportDocument {
    pub(crate) summary: ExportInspectionQueryReportJsonSummary,
    pub(crate) queries: Vec<ExportInspectionQueryRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionDatasourceSummaryRow {
    pub(crate) datasource: String,
    #[serde(rename = "datasourceUid")]
    pub(crate) datasource_uid: String,
    #[serde(rename = "type")]
    pub(crate) datasource_type: String,
    pub(crate) family: String,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    pub(crate) orphaned: String,
    pub(crate) metrics: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionDatasourceSummaryDocumentSummary {
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "queryRecordCount")]
    pub(crate) query_record_count: usize,
    #[serde(rename = "datasourceCount")]
    pub(crate) datasource_count: usize,
    #[serde(rename = "activeDatasourceCount")]
    pub(crate) active_datasource_count: usize,
    #[serde(rename = "orphanedDatasourceCount")]
    pub(crate) orphaned_datasource_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionDatasourceSummaryDocument {
    pub(crate) summary: ExportInspectionDatasourceSummaryDocumentSummary,
    pub(crate) datasources: Vec<ExportInspectionDatasourceSummaryRow>,
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

pub(crate) fn build_export_inspection_query_report_document(
    report: &ExportInspectionQueryReport,
) -> ExportInspectionQueryReportDocument {
    ExportInspectionQueryReportDocument {
        summary: ExportInspectionQueryReportJsonSummary {
            dashboard_count: report.summary.dashboard_count,
            datasource_count: report
                .queries
                .iter()
                .map(|row| {
                    if row.datasource_uid.is_empty() {
                        row.datasource.clone()
                    } else {
                        row.datasource_uid.clone()
                    }
                })
                .filter(|value| !value.is_empty())
                .collect::<std::collections::BTreeSet<String>>()
                .len(),
            datasource_inventory_count: 0,
            query_record_count: report.queries.len(),
        },
        queries: report.queries.clone(),
    }
}

fn ordered_unique_strings(values: &[String]) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
            continue;
        }
        normalized.push(trimmed.to_string());
    }
    normalized
}

fn normalize_datasource_family(datasource_type: &str) -> String {
    match datasource_type.trim().to_ascii_lowercase().as_str() {
        "" => "unknown".to_string(),
        "grafana-postgresql-datasource" => "postgres".to_string(),
        "grafana-mysql-datasource" => "mysql".to_string(),
        "postgres" => "postgres".to_string(),
        "prometheus" => "prometheus".to_string(),
        "loki" => "loki".to_string(),
        "influxdb" => "influxdb".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn build_export_inspection_datasource_summary_document(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
    datasource_filter: Option<&str>,
) -> ExportInspectionDatasourceSummaryDocument {
    #[derive(Default)]
    struct RowState {
        datasource: String,
        datasource_uid: String,
        dashboards: std::collections::BTreeSet<String>,
        panels: std::collections::BTreeSet<(String, String)>,
        query_count: usize,
        metrics: Vec<String>,
        measurements: Vec<String>,
        buckets: Vec<String>,
    }

    let datasource_filter = datasource_filter.map(str::trim).filter(|value| !value.is_empty());
    let mut by_key = std::collections::BTreeMap::<String, RowState>::new();

    for row in &report.queries {
        let key = if !row.datasource_uid.is_empty() {
            row.datasource_uid.clone()
        } else {
            row.datasource.clone()
        };
        if key.is_empty() {
            continue;
        }
        if let Some(filter) = datasource_filter {
            if row.datasource != filter && row.datasource_uid != filter {
                continue;
            }
        }
        let state = by_key.entry(key).or_default();
        if state.datasource.is_empty() {
            state.datasource = row.datasource.clone();
        }
        if state.datasource_uid.is_empty() {
            state.datasource_uid = row.datasource_uid.clone();
        }
        state.dashboards.insert(row.dashboard_uid.clone());
        state
            .panels
            .insert((row.dashboard_uid.clone(), row.panel_id.clone()));
        state.query_count += 1;
        state.metrics.extend(row.metrics.clone());
        state.measurements.extend(row.measurements.clone());
        state.buckets.extend(row.buckets.clone());
    }

    let mut rows = Vec::new();
    for inventory in &summary.datasource_inventory {
        let key = if inventory.uid.is_empty() {
            inventory.name.clone()
        } else {
            inventory.uid.clone()
        };
        let matches_filter = datasource_filter.map_or(true, |filter| {
            inventory.uid == filter || inventory.name == filter
        });
        let state = by_key.remove(&key);
        if !matches_filter && state.is_none() {
            continue;
        }
        let (dashboard_count, panel_count, query_count, metrics, measurements, buckets) =
            if let Some(state) = state {
                (
                    state.dashboards.len(),
                    state.panels.len(),
                    state.query_count,
                    ordered_unique_strings(&state.metrics),
                    ordered_unique_strings(&state.measurements),
                    ordered_unique_strings(&state.buckets),
                )
            } else {
                (0, 0, 0, Vec::new(), Vec::new(), Vec::new())
            };
        rows.push(ExportInspectionDatasourceSummaryRow {
            datasource: inventory.name.clone(),
            datasource_uid: inventory.uid.clone(),
            datasource_type: inventory.datasource_type.clone(),
            family: normalize_datasource_family(&inventory.datasource_type),
            dashboard_count,
            panel_count,
            query_count,
            orphaned: (dashboard_count == 0 && query_count == 0).to_string(),
            metrics,
            measurements,
            buckets,
        });
    }

    for (key, state) in by_key {
        if let Some(filter) = datasource_filter {
            if key != filter && state.datasource != filter && state.datasource_uid != filter {
                continue;
            }
        }
        rows.push(ExportInspectionDatasourceSummaryRow {
            datasource: state.datasource,
            datasource_uid: state.datasource_uid,
            datasource_type: String::new(),
            family: "unknown".to_string(),
            dashboard_count: state.dashboards.len(),
            panel_count: state.panels.len(),
            query_count: state.query_count,
            orphaned: "false".to_string(),
            metrics: ordered_unique_strings(&state.metrics),
            measurements: ordered_unique_strings(&state.measurements),
            buckets: ordered_unique_strings(&state.buckets),
        });
    }

    rows.sort_by(|left, right| {
        left.datasource
            .cmp(&right.datasource)
            .then(left.datasource_uid.cmp(&right.datasource_uid))
    });
    let active_count = rows
        .iter()
        .filter(|item| item.orphaned != "true")
        .count();
    let orphaned_count = rows.iter().filter(|item| item.orphaned == "true").count();

    ExportInspectionDatasourceSummaryDocument {
        summary: ExportInspectionDatasourceSummaryDocumentSummary {
            dashboard_count: report.summary.dashboard_count,
            query_record_count: report.queries.len(),
            datasource_count: rows.len(),
            active_datasource_count: active_count,
            orphaned_datasource_count: orphaned_count,
        },
        datasources: rows,
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

// Group query rows by dashboard/panel so report output is deterministic and renderable.
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
