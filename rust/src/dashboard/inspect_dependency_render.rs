use crate::dashboard_inspection_dependency_contract::OfflineDependencyReportDocument;

use super::inspect_render::{join_or_none, render_simple_table};

fn render_dependency_section(
    lines: &mut Vec<String>,
    title: &str,
    headers: &[&str],
    rows: &[Vec<String>],
) {
    lines.push(String::new());
    lines.push(title.to_string());
    if rows.is_empty() {
        lines.push("(none)".to_string());
        return;
    }
    lines.extend(render_simple_table(headers, rows, true));
}

fn normalize_dependency_cell(value: &str) -> String {
    if value.trim().is_empty() {
        "(none)".to_string()
    } else {
        value.to_string()
    }
}

fn render_orphan_row(
    item: &crate::dashboard_inspection_dependency_contract::DependencyOrphanSummary,
) -> Vec<String> {
    vec![
        normalize_dependency_cell(&item.org),
        normalize_dependency_cell(&item.org_id),
        normalize_dependency_cell(&item.uid),
        normalize_dependency_cell(&item.name),
        normalize_dependency_cell(&item.datasource_type),
        normalize_dependency_cell(&item.family),
        normalize_dependency_cell(&item.access),
        normalize_dependency_cell(&item.is_default),
        normalize_dependency_cell(&item.url),
        normalize_dependency_cell(&item.database),
        normalize_dependency_cell(&item.default_bucket),
        normalize_dependency_cell(&item.organization),
        normalize_dependency_cell(&item.index_pattern),
    ]
}

pub(super) fn render_export_inspection_dependency_table_report(
    import_dir: &str,
    document: &OfflineDependencyReportDocument,
) -> Vec<String> {
    let mut lines = vec![
        format!("Export inspection dependency: {}", import_dir),
        String::new(),
    ];

    lines.push("# Summary".to_string());
    let summary_rows = vec![vec![
        document
            .summary
            .get("queryCount")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .to_string(),
        document
            .summary
            .get("dashboardCount")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .to_string(),
        document
            .summary
            .get("panelCount")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .to_string(),
        document
            .summary
            .get("datasourceCount")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .to_string(),
        document
            .summary
            .get("orphanedDatasourceCount")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .to_string(),
    ]];
    lines.extend(render_simple_table(
        &[
            "QUERY_COUNT",
            "DASHBOARD_COUNT",
            "PANEL_COUNT",
            "DATASOURCE_COUNT",
            "ORPHANED_DATASOURCE_COUNT",
        ],
        &summary_rows,
        true,
    ));

    let usage_rows = document
        .usage
        .iter()
        .map(|item| {
            vec![
                item.datasource_identity.clone(),
                item.datasource_uid.clone(),
                item.datasource_type.clone(),
                item.family.clone(),
                item.query_count.to_string(),
                item.dashboard_count.to_string(),
                item.panel_count.to_string(),
                item.folder_count.to_string(),
                if item.high_blast_radius {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
                if item.cross_folder {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
                item.reference_count.to_string(),
                join_or_none(&item.folder_paths, ","),
                join_or_none(&item.dashboard_titles, ","),
                join_or_none(&item.query_fields, ","),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    render_dependency_section(
        &mut lines,
        "# Datasource usage",
        &[
            "DATASOURCE",
            "UID",
            "TYPE",
            "FAMILY",
            "QUERIES",
            "DASHBOARDS",
            "PANELS",
            "FOLDERS",
            "HIGH_BLAST_RADIUS",
            "CROSS_FOLDER",
            "REFS",
            "FOLDER_PATHS",
            "DASHBOARD_TITLES",
            "QUERY_FIELDS",
        ],
        &usage_rows,
    );

    let dashboard_rows = document
        .dashboard_dependencies
        .iter()
        .map(|item| {
            vec![
                item.dashboard_uid.clone(),
                item.dashboard_title.clone(),
                item.query_count.to_string(),
                item.panel_count.to_string(),
                item.datasource_count.to_string(),
                item.datasource_family_count.to_string(),
                join_or_none(&item.query_fields, ","),
                join_or_none(&item.metrics, ","),
                join_or_none(&item.functions, ","),
                join_or_none(&item.measurements, ","),
                join_or_none(&item.buckets, ","),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    render_dependency_section(
        &mut lines,
        "# Dashboard dependencies",
        &[
            "DASHBOARD_UID",
            "TITLE",
            "QUERIES",
            "PANELS",
            "DATASOURCES",
            "FAMILIES",
            "QUERY_FIELDS",
            "METRICS",
            "FUNCTIONS",
            "MEASUREMENTS",
            "BUCKETS",
        ],
        &dashboard_rows,
    );

    let orphan_rows = document
        .orphaned
        .iter()
        .map(render_orphan_row)
        .collect::<Vec<Vec<String>>>();
    render_dependency_section(
        &mut lines,
        "# Orphaned datasources",
        &[
            "ORG",
            "ORG_ID",
            "UID",
            "NAME",
            "TYPE",
            "FAMILY",
            "ACCESS",
            "IS_DEFAULT",
            "URL",
            "DATABASE",
            "DEFAULT_BUCKET",
            "ORGANIZATION",
            "INDEX_PATTERN",
        ],
        &orphan_rows,
    );

    lines
}

#[cfg(test)]
mod tests {
    use super::render_export_inspection_dependency_table_report;
    use crate::dashboard::test_support::make_core_family_report_row;
    use crate::dashboard::DatasourceInventoryItem;
    use crate::dashboard_inspection_dependency_contract::build_offline_dependency_contract_document_from_report_rows;

    #[test]
    fn render_dependency_table_report_normalizes_empty_orphan_cells() {
        let report_rows = vec![make_core_family_report_row(
            "cpu-main",
            "7",
            "A",
            "prom-main",
            "Prometheus Main",
            "prometheus",
            "prometheus",
            "sum(rate(up[5m]))",
            &["job=\"api\""],
        )];
        let datasource_inventory = vec![
            DatasourceInventoryItem {
                uid: "prom-main".to_string(),
                name: "Prometheus Main".to_string(),
                datasource_type: "prometheus".to_string(),
                access: "proxy".to_string(),
                url: "http://prometheus:9090".to_string(),
                database: String::new(),
                default_bucket: String::new(),
                organization: String::new(),
                index_pattern: String::new(),
                is_default: "true".to_string(),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
            },
            DatasourceInventoryItem {
                uid: "unused-main".to_string(),
                name: "Unused Main".to_string(),
                datasource_type: "postgres".to_string(),
                access: "proxy".to_string(),
                url: String::new(),
                database: String::new(),
                default_bucket: String::new(),
                organization: String::new(),
                index_pattern: String::new(),
                is_default: "false".to_string(),
                org: "Main Org.".to_string(),
                org_id: "1".to_string(),
            },
        ];

        let document = build_offline_dependency_contract_document_from_report_rows(
            &report_rows,
            &datasource_inventory,
        );
        let output = render_export_inspection_dependency_table_report("demo", &document).join("\n");

        assert!(output.contains("# Datasource usage"));
        assert!(output.contains("FOLDERS"));
        assert!(output.contains("HIGH_BLAST_RADIUS"));
        assert!(output.contains("CROSS_FOLDER"));
        assert!(output.contains("FOLDER_PATHS"));
        assert!(output.contains("DASHBOARD_TITLES"));
        assert!(output.contains("General"));
        assert!(output.contains("# Orphaned datasources"));
        assert!(output.contains("Unused Main"));
        assert!(output.contains("(none)"));
    }
}
