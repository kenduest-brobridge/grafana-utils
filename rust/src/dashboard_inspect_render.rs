//! Renderer implementations for inspection output.
//! Converts normalized query rows into plain text table and CSV representations.
use super::dashboard_inspect_report::{
    normalize_query_report, render_query_report_column, report_column_header,
    ExportInspectionDatasourceSummaryDocument, ExportInspectionQueryReport,
};

pub(crate) fn render_csv(headers: &[&str], rows: &[Vec<String>]) -> Vec<String> {
    fn escape_csv(value: &str) -> String {
        if value.contains(',') || value.contains('"') || value.contains('\n') {
            format!("\"{}\"", value.replace('"', "\"\""))
        } else {
            value.to_string()
        }
    }

    let mut lines = Vec::new();
    lines.push(
        headers
            .iter()
            .map(|value| escape_csv(value))
            .collect::<Vec<String>>()
            .join(","),
    );
    for row in rows {
        lines.push(
            row.iter()
                .map(|value| escape_csv(value))
                .collect::<Vec<String>>()
                .join(","),
        );
    }
    lines
}

pub(crate) fn render_simple_table(
    headers: &[&str],
    rows: &[Vec<String>],
    include_header: bool,
) -> Vec<String> {
    let mut widths = headers
        .iter()
        .map(|value| value.len())
        .collect::<Vec<usize>>();
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            if index >= widths.len() {
                widths.push(value.len());
            } else {
                widths[index] = widths[index].max(value.len());
            }
        }
    }
    let format_row = |values: &[String]| -> String {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
            .collect::<Vec<String>>()
            .join("  ")
    };
    let mut lines = Vec::new();
    if include_header {
        let header_row = headers
            .iter()
            .map(|value| value.to_string())
            .collect::<Vec<String>>();
        let divider_row = widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<String>>();
        lines.push(format_row(&header_row));
        lines.push(format_row(&divider_row));
    }
    for row in rows {
        lines.push(format_row(row));
    }
    lines
}

pub(crate) fn render_grouped_query_report(report: &ExportInspectionQueryReport) -> Vec<String> {
    let normalized = normalize_query_report(report);
    let mut lines = Vec::new();
    lines.push(format!(
        "Export inspection report: {}",
        normalized.import_dir
    ));
    lines.push(String::new());
    lines.push("# Summary".to_string());
    lines.push(format!(
        "dashboards={} panels={} queries={} rows={}",
        normalized.summary.dashboard_count,
        normalized.summary.panel_count,
        normalized.summary.query_count,
        normalized.summary.report_row_count
    ));
    lines.push(String::new());
    lines.push("# Dashboard tree".to_string());
    for (index, dashboard) in normalized.dashboards.into_iter().enumerate() {
        let panel_count = dashboard.panels.len();
        let query_count = dashboard
            .panels
            .iter()
            .map(|panel| panel.queries.len())
            .sum::<usize>();
        lines.push(format!(
            "[{}] Dashboard: {} (uid={}, folder={}, panels={}, queries={})",
            index + 1,
            dashboard.dashboard_title,
            dashboard.dashboard_uid,
            dashboard.folder_path,
            panel_count,
            query_count
        ));
        for panel in dashboard.panels {
            lines.push(format!(
                "  Panel: {} (id={}, type={}, queries={})",
                panel.panel_title,
                panel.panel_id,
                panel.panel_type,
                panel.queries.len()
            ));
            for query in panel.queries {
                let mut details = vec![format!("refId={}", query.ref_id)];
                if !query.datasource.is_empty() {
                    details.push(format!("datasource={}", query.datasource));
                }
                if !query.datasource_uid.is_empty() {
                    details.push(format!("datasourceUid={}", query.datasource_uid));
                }
                if !query.query_field.is_empty() {
                    details.push(format!("field={}", query.query_field));
                }
                if !query.metrics.is_empty() {
                    details.push(format!("metrics={}", query.metrics.join(",")));
                }
                if !query.measurements.is_empty() {
                    details.push(format!("measurements={}", query.measurements.join(",")));
                }
                if !query.buckets.is_empty() {
                    details.push(format!("buckets={}", query.buckets.join(",")));
                }
                lines.push(format!("    Query: {}", details.join(" ")));
                if !query.query_text.is_empty() {
                    lines.push(format!("      {}", query.query_text));
                }
            }
        }
    }
    lines
}

pub(crate) fn render_grouped_query_table_report(
    report: &ExportInspectionQueryReport,
    column_ids: &[String],
    include_header: bool,
) -> Vec<String> {
    let normalized = normalize_query_report(report);
    let headers = column_ids
        .iter()
        .map(|column_id| report_column_header(column_id))
        .collect::<Vec<&str>>();
    let mut lines = Vec::new();
    lines.push(format!(
        "Export inspection tree-table report: {}",
        normalized.import_dir
    ));
    lines.push(String::new());
    lines.push("# Summary".to_string());
    lines.push(format!(
        "dashboards={} panels={} queries={} rows={}",
        normalized.summary.dashboard_count,
        normalized.summary.panel_count,
        normalized.summary.query_count,
        normalized.summary.report_row_count
    ));
    lines.push(String::new());
    lines.push("# Dashboard sections".to_string());
    for (index, dashboard) in normalized.dashboards.into_iter().enumerate() {
        let panel_count = dashboard.panels.len();
        let query_count = dashboard
            .panels
            .iter()
            .map(|panel| panel.queries.len())
            .sum::<usize>();
        lines.push(format!(
            "[{}] Dashboard: {} (uid={}, folder={}, panels={}, queries={})",
            index + 1,
            dashboard.dashboard_title,
            dashboard.dashboard_uid,
            dashboard.folder_path,
            panel_count,
            query_count
        ));
        let rows = dashboard
            .panels
            .iter()
            .flat_map(|panel| panel.queries.iter())
            .map(|query| {
                column_ids
                    .iter()
                    .map(|column_id| render_query_report_column(query, column_id))
                    .collect::<Vec<String>>()
            })
            .collect::<Vec<Vec<String>>>();
        for line in render_simple_table(&headers, &rows, include_header) {
            lines.push(line);
        }
        lines.push(String::new());
    }
    while matches!(lines.last(), Some(last) if last.is_empty()) {
        lines.pop();
    }
    lines
}

pub(crate) fn render_datasource_summary_report(
    import_dir: &str,
    document: &ExportInspectionDatasourceSummaryDocument,
    include_header: bool,
) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "Export inspection datasource summary: {import_dir}"
    ));
    lines.push(String::new());
    lines.push("# Summary".to_string());
    let summary_rows = vec![
        vec![
            "dashboard_count".to_string(),
            document.summary.dashboard_count.to_string(),
        ],
        vec![
            "query_record_count".to_string(),
            document.summary.query_record_count.to_string(),
        ],
        vec![
            "datasource_count".to_string(),
            document.summary.datasource_count.to_string(),
        ],
        vec![
            "active_datasource_count".to_string(),
            document.summary.active_datasource_count.to_string(),
        ],
        vec![
            "orphaned_datasource_count".to_string(),
            document.summary.orphaned_datasource_count.to_string(),
        ],
    ];
    for line in render_simple_table(&["METRIC", "VALUE"], &summary_rows, include_header) {
        lines.push(line);
    }
    if !document.datasources.is_empty() {
        lines.push(String::new());
        lines.push("# Datasource dependencies".to_string());
        let rows = document
            .datasources
            .iter()
            .map(|item| {
                vec![
                    item.datasource.clone(),
                    item.datasource_uid.clone(),
                    item.datasource_type.clone(),
                    item.family.clone(),
                    item.dashboard_count.to_string(),
                    item.panel_count.to_string(),
                    item.query_count.to_string(),
                    item.orphaned.clone(),
                    item.metrics.join(","),
                    item.measurements.join(","),
                    item.buckets.join(","),
                ]
            })
            .collect::<Vec<Vec<String>>>();
        for line in render_simple_table(
            &[
                "DATASOURCE",
                "DATASOURCE_UID",
                "TYPE",
                "FAMILY",
                "DASHBOARD_COUNT",
                "PANEL_COUNT",
                "QUERY_COUNT",
                "ORPHANED",
                "METRICS",
                "MEASUREMENTS",
                "BUCKETS",
            ],
            &rows,
            include_header,
        ) {
            lines.push(line);
        }
    }
    lines
}
