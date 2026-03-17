//! Renderer implementations for inspection output.
//! Converts normalized query rows into plain text table and CSV representations.
use super::dashboard_inspect_report::{
    normalize_query_report, render_query_report_column, report_column_header,
    ExportInspectionQueryReport,
};

/// Purpose: implementation note.
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

/// Purpose: implementation note.
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

/// Purpose: implementation note.
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
        let dashboard_datasources = if dashboard.datasources.is_empty() {
            "none".to_string()
        } else {
            dashboard.datasources.join(",")
        };
        let dashboard_families = if dashboard.datasource_families.is_empty() {
            "none".to_string()
        } else {
            dashboard.datasource_families.join(",")
        };
        let org_segment = if dashboard.org.is_empty() && dashboard.org_id.is_empty() {
            String::new()
        } else if dashboard.org.is_empty() {
            format!(", orgId={}", dashboard.org_id)
        } else if dashboard.org_id.is_empty() {
            format!(", org={}", dashboard.org)
        } else {
            format!(", org={}, orgId={}", dashboard.org, dashboard.org_id)
        };
        lines.push(format!(
            "[{}] Dashboard: {} (uid={}, folder={}, panels={}, queries={}, datasources={}, families={}{})",
            index + 1,
            dashboard.dashboard_title,
            dashboard.dashboard_uid,
            dashboard.folder_path,
            panel_count,
            query_count,
            dashboard_datasources,
            dashboard_families,
            org_segment
        ));
        if !dashboard.file_path.is_empty() {
            lines.push(format!("  File: {}", dashboard.file_path));
        }
        for panel in dashboard.panels {
            let panel_datasources = if panel.datasources.is_empty() {
                "none".to_string()
            } else {
                panel.datasources.join(",")
            };
            let panel_families = if panel.datasource_families.is_empty() {
                "none".to_string()
            } else {
                panel.datasource_families.join(",")
            };
            let query_fields = if panel.query_fields.is_empty() {
                "none".to_string()
            } else {
                panel.query_fields.join(",")
            };
            lines.push(format!(
                "  Panel: {} (id={}, type={}, queries={}, datasources={}, families={}, fields={})",
                panel.panel_title,
                panel.panel_id,
                panel.panel_type,
                panel.queries.len(),
                panel_datasources,
                panel_families,
                query_fields
            ));
            for query in panel.queries {
                let mut details = vec![format!("refId={}", query.ref_id)];
                if !query.datasource.is_empty() {
                    details.push(format!("datasource={}", query.datasource));
                }
                if !query.datasource_uid.is_empty() {
                    details.push(format!("datasourceUid={}", query.datasource_uid));
                }
                if !query.datasource_type.is_empty() {
                    details.push(format!("datasourceType={}", query.datasource_type));
                }
                if !query.datasource_family.is_empty() {
                    details.push(format!("datasourceFamily={}", query.datasource_family));
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

/// Purpose: implementation note.
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
        let dashboard_datasources = if dashboard.datasources.is_empty() {
            "none".to_string()
        } else {
            dashboard.datasources.join(",")
        };
        let dashboard_families = if dashboard.datasource_families.is_empty() {
            "none".to_string()
        } else {
            dashboard.datasource_families.join(",")
        };
        let org_segment = if dashboard.org.is_empty() && dashboard.org_id.is_empty() {
            String::new()
        } else if dashboard.org.is_empty() {
            format!(", orgId={}", dashboard.org_id)
        } else if dashboard.org_id.is_empty() {
            format!(", org={}", dashboard.org)
        } else {
            format!(", org={}, orgId={}", dashboard.org, dashboard.org_id)
        };
        lines.push(format!(
            "[{}] Dashboard: {} (uid={}, folder={}, panels={}, queries={}, datasources={}, families={}{})",
            index + 1,
            dashboard.dashboard_title,
            dashboard.dashboard_uid,
            dashboard.folder_path,
            panel_count,
            query_count,
            dashboard_datasources,
            dashboard_families,
            org_segment
        ));
        if !dashboard.file_path.is_empty() {
            lines.push(format!("File: {}", dashboard.file_path));
        }
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
