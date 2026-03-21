//! Governance report builder for inspect mode.
//! Computes datasource-family coverage and risk summaries from the shared query inspection data.
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

use super::inspect_render::render_simple_table;
use super::inspect_report::{ExportInspectionQueryReport, ExportInspectionQueryRow};
use super::ExportInspectionSummary;

/// Struct definition for GovernanceSummary.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct GovernanceSummary {
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "queryRecordCount")]
    pub(crate) query_record_count: usize,
    #[serde(rename = "datasourceInventoryCount")]
    pub(crate) datasource_inventory_count: usize,
    #[serde(rename = "datasourceFamilyCount")]
    pub(crate) datasource_family_count: usize,
    #[serde(rename = "datasourceCoverageCount")]
    pub(crate) datasource_coverage_count: usize,
    #[serde(rename = "mixedDatasourceDashboardCount")]
    pub(crate) mixed_datasource_dashboard_count: usize,
    #[serde(rename = "orphanedDatasourceCount")]
    pub(crate) orphaned_datasource_count: usize,
    #[serde(rename = "riskRecordCount")]
    pub(crate) risk_record_count: usize,
}

/// Struct definition for DatasourceFamilyCoverageRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DatasourceFamilyCoverageRow {
    pub(crate) family: String,
    #[serde(rename = "datasourceTypes")]
    pub(crate) datasource_types: Vec<String>,
    #[serde(rename = "datasourceCount")]
    pub(crate) datasource_count: usize,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
}

/// Struct definition for DatasourceCoverageRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DatasourceCoverageRow {
    #[serde(rename = "datasourceUid")]
    pub(crate) datasource_uid: String,
    pub(crate) datasource: String,
    pub(crate) family: String,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    #[serde(rename = "dashboardCount")]
    pub(crate) dashboard_count: usize,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryFields")]
    pub(crate) query_fields: Vec<String>,
    pub(crate) orphaned: bool,
}

/// Struct definition for DashboardDependencyRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DashboardDependencyRow {
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    pub(crate) dashboard_title: String,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    #[serde(rename = "file")]
    pub(crate) file_path: String,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    pub(crate) datasources: Vec<String>,
    #[serde(rename = "datasourceFamilies")]
    pub(crate) datasource_families: Vec<String>,
}

/// Struct definition for GovernanceRiskRow.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct GovernanceRiskRow {
    pub(crate) kind: String,
    pub(crate) severity: String,
    pub(crate) category: String,
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "panelId")]
    pub(crate) panel_id: String,
    pub(crate) datasource: String,
    pub(crate) detail: String,
    pub(crate) recommendation: String,
}

/// Struct definition for ExportInspectionGovernanceDocument.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct ExportInspectionGovernanceDocument {
    pub(crate) summary: GovernanceSummary,
    #[serde(rename = "datasourceFamilies")]
    pub(crate) datasource_families: Vec<DatasourceFamilyCoverageRow>,
    #[serde(rename = "dashboardDependencies")]
    pub(crate) dashboard_dependencies: Vec<DashboardDependencyRow>,
    pub(crate) datasources: Vec<DatasourceCoverageRow>,
    #[serde(rename = "riskRecords")]
    pub(crate) risk_records: Vec<GovernanceRiskRow>,
}

#[derive(Clone, Debug)]
struct ResolvedDatasourceIdentity {
    uid: String,
    name: String,
    datasource_type: String,
}

// Collapse datasource type names into normalized family labels used in governance
// summaries and risk grouping.
/// Purpose: implementation note.
pub(crate) fn normalize_family_name(datasource_type: &str) -> String {
    match datasource_type.trim().to_ascii_lowercase().as_str() {
        "" => "unknown".to_string(),
        "influxdb" | "flux" => "flux".to_string(),
        "mysql" | "postgres" | "mssql" => "sql".to_string(),
        "grafana-postgresql-datasource" | "grafana-mysql-datasource" => "sql".to_string(),
        "elasticsearch" | "opensearch" | "grafana-opensearch-datasource" => "search".to_string(),
        "tempo" | "jaeger" | "zipkin" => "tracing".to_string(),
        value => value.to_string(),
    }
}

fn normalize_family_list(families: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for family in families {
        let family = normalize_family_name(family);
        if !normalized.iter().any(|value| value == &family) {
            normalized.push(family);
        }
    }
    normalized
}

type InventoryIdentity = (String, String, String);
type InventoryLookup = BTreeMap<String, InventoryIdentity>;

fn build_inventory_lookup(summary: &ExportInspectionSummary) -> (InventoryLookup, InventoryLookup) {
    let mut by_uid = BTreeMap::new();
    let mut by_name = BTreeMap::new();
    for datasource in &summary.datasource_inventory {
        let identity = (
            datasource.uid.clone(),
            datasource.name.clone(),
            datasource.datasource_type.clone(),
        );
        if !datasource.uid.trim().is_empty() {
            by_uid.insert(datasource.uid.clone(), identity.clone());
        }
        if !datasource.name.trim().is_empty() {
            by_name.insert(datasource.name.clone(), identity);
        }
    }
    (by_uid, by_name)
}

fn resolve_datasource_identity(
    row: &ExportInspectionQueryRow,
    inventory_by_uid: &BTreeMap<String, (String, String, String)>,
    inventory_by_name: &BTreeMap<String, (String, String, String)>,
) -> ResolvedDatasourceIdentity {
    let normalized_family = normalize_family_name(&row.datasource_type);
    let datasource_type = if matches!(normalized_family.as_str(), "search" | "tracing") {
        row.datasource_type.clone()
    } else {
        "unknown".to_string()
    };
    if !row.datasource_uid.trim().is_empty() {
        if let Some((uid, name, datasource_type)) = inventory_by_uid.get(&row.datasource_uid) {
            return ResolvedDatasourceIdentity {
                uid: uid.clone(),
                name: name.clone(),
                datasource_type: datasource_type.clone(),
            };
        }
    }
    if !row.datasource.trim().is_empty() {
        if let Some((uid, name, datasource_type)) = inventory_by_uid
            .get(&row.datasource)
            .or_else(|| inventory_by_name.get(&row.datasource))
        {
            return ResolvedDatasourceIdentity {
                uid: uid.clone(),
                name: name.clone(),
                datasource_type: datasource_type.clone(),
            };
        }
    }
    if !row.datasource_uid.trim().is_empty() {
        return ResolvedDatasourceIdentity {
            uid: row.datasource_uid.clone(),
            name: if row.datasource.trim().is_empty() {
                row.datasource_uid.clone()
            } else {
                row.datasource.clone()
            },
            datasource_type,
        };
    }
    if !row.datasource.trim().is_empty() {
        return ResolvedDatasourceIdentity {
            uid: row.datasource.clone(),
            name: row.datasource.clone(),
            datasource_type,
        };
    }
    ResolvedDatasourceIdentity {
        uid: "unknown".to_string(),
        name: "unknown".to_string(),
        datasource_type,
    }
}

fn build_risk_metadata(kind: &str) -> (&'static str, &'static str, &'static str) {
    match kind {
        "mixed-datasource-dashboard" => (
            "topology",
            "medium",
            "Split panel queries by datasource or document why mixed datasource composition is required.",
        ),
        "orphaned-datasource" => (
            "inventory",
            "low",
            "Remove the unused datasource or reattach it to retained dashboards before the next cleanup cycle.",
        ),
        "unknown-datasource-family" => (
            "coverage",
            "medium",
            "Map this datasource plugin type to a known governance family or extend analyzer support for it.",
        ),
        "empty-query-analysis" => (
            "coverage",
            "low",
            "Review the query text and extend analyzer coverage if this datasource family should emit governance signals.",
        ),
        _ => (
            "other",
            "low",
            "Review this governance finding and assign a follow-up owner if action is needed.",
        ),
    }
}

/// Purpose: implementation note.
pub(crate) fn build_datasource_family_coverage_rows(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> Vec<DatasourceFamilyCoverageRow> {
    let (inventory_by_uid, inventory_by_name) = build_inventory_lookup(summary);
    let mut coverage = BTreeMap::<
        String,
        (
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            usize,
        ),
    >::new();
    for row in &report.queries {
        let identity = resolve_datasource_identity(row, &inventory_by_uid, &inventory_by_name);
        let family = normalize_family_name(&identity.datasource_type);
        let record = coverage.entry(family).or_insert_with(|| {
            (
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                0usize,
            )
        });
        record.0.insert(identity.datasource_type);
        record.1.insert(identity.uid);
        record.2.insert(row.dashboard_uid.clone());
        record
            .3
            .insert(format!("{}:{}", row.dashboard_uid, row.panel_id));
        record.4 += 1;
    }
    coverage
        .into_iter()
        .map(
            |(
                family,
                (datasource_types, datasource_uids, dashboard_uids, panel_keys, query_count),
            )| {
                DatasourceFamilyCoverageRow {
                    family,
                    datasource_types: datasource_types.into_iter().collect(),
                    datasource_count: datasource_uids.len(),
                    dashboard_count: dashboard_uids.len(),
                    panel_count: panel_keys.len(),
                    query_count,
                }
            },
        )
        .collect()
}

/// Purpose: implementation note.
pub(crate) fn build_datasource_coverage_rows(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> Vec<DatasourceCoverageRow> {
    let (inventory_by_uid, inventory_by_name) = build_inventory_lookup(summary);
    let mut coverage = BTreeMap::<
        String,
        (
            String,
            String,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            usize,
            bool,
        ),
    >::new();
    for datasource in &summary.datasource_inventory {
        let key = if datasource.uid.trim().is_empty() {
            datasource.name.clone()
        } else {
            datasource.uid.clone()
        };
        coverage.insert(
            key,
            (
                datasource.name.clone(),
                normalize_family_name(&datasource.datasource_type),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                0usize,
                datasource.reference_count == 0 && datasource.dashboard_count == 0,
            ),
        );
    }
    for row in &report.queries {
        let identity = resolve_datasource_identity(row, &inventory_by_uid, &inventory_by_name);
        let key = identity.uid.clone();
        let record = coverage.entry(key.clone()).or_insert_with(|| {
            (
                identity.name.clone(),
                normalize_family_name(&identity.datasource_type),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                0usize,
                false,
            )
        });
        if !row.query_field.trim().is_empty() {
            record.2.insert(row.query_field.clone());
        }
        record.3.insert(row.dashboard_uid.clone());
        record
            .4
            .insert(format!("{}:{}", row.dashboard_uid, row.panel_id));
        record.5 += 1;
        record.6 = false;
    }
    coverage
        .into_iter()
        .map(
            |(
                datasource_uid,
                (datasource, family, query_fields, dashboards, panels, query_count, orphaned),
            )| {
                DatasourceCoverageRow {
                    datasource_uid,
                    datasource,
                    family,
                    query_count,
                    dashboard_count: dashboards.len(),
                    panel_count: panels.len(),
                    query_fields: query_fields.into_iter().collect(),
                    orphaned,
                }
            },
        )
        .collect()
}

/// Purpose: implementation note.
pub(crate) fn build_dashboard_dependency_rows(
    report: &ExportInspectionQueryReport,
) -> Vec<DashboardDependencyRow> {
    let normalized = super::inspect_report::normalize_query_report(report);
    normalized
        .dashboards
        .into_iter()
        .map(|dashboard| DashboardDependencyRow {
            dashboard_uid: dashboard.dashboard_uid,
            dashboard_title: dashboard.dashboard_title,
            folder_path: dashboard.folder_path,
            file_path: dashboard.file_path,
            panel_count: dashboard.panels.len(),
            query_count: dashboard
                .panels
                .iter()
                .map(|panel| panel.queries.len())
                .sum::<usize>(),
            datasources: dashboard.datasources,
            datasource_families: normalize_family_list(&dashboard.datasource_families),
        })
        .collect()
}

/// Purpose: implementation note.
pub(crate) fn build_governance_risk_rows(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> Vec<GovernanceRiskRow> {
    let (inventory_by_uid, inventory_by_name) = build_inventory_lookup(summary);
    let mut seen = BTreeSet::new();
    let mut risks = Vec::new();

    for dashboard in &summary.mixed_dashboards {
        let risk = GovernanceRiskRow {
            kind: "mixed-datasource-dashboard".to_string(),
            severity: build_risk_metadata("mixed-datasource-dashboard")
                .1
                .to_string(),
            category: build_risk_metadata("mixed-datasource-dashboard")
                .0
                .to_string(),
            dashboard_uid: dashboard.uid.clone(),
            panel_id: String::new(),
            datasource: dashboard.datasources.join(","),
            detail: dashboard.title.clone(),
            recommendation: build_risk_metadata("mixed-datasource-dashboard")
                .2
                .to_string(),
        };
        if seen.insert(risk.clone()) {
            risks.push(risk);
        }
    }
    for datasource in &summary.datasource_inventory {
        if datasource.reference_count != 0 || datasource.dashboard_count != 0 {
            continue;
        }
        let risk = GovernanceRiskRow {
            kind: "orphaned-datasource".to_string(),
            severity: build_risk_metadata("orphaned-datasource").1.to_string(),
            category: build_risk_metadata("orphaned-datasource").0.to_string(),
            dashboard_uid: String::new(),
            panel_id: String::new(),
            datasource: if datasource.uid.trim().is_empty() {
                datasource.name.clone()
            } else {
                datasource.uid.clone()
            },
            detail: datasource.datasource_type.clone(),
            recommendation: build_risk_metadata("orphaned-datasource").2.to_string(),
        };
        if seen.insert(risk.clone()) {
            risks.push(risk);
        }
    }
    for row in &report.queries {
        let identity = resolve_datasource_identity(row, &inventory_by_uid, &inventory_by_name);
        if normalize_family_name(&identity.datasource_type) == "unknown" {
            let risk = GovernanceRiskRow {
                kind: "unknown-datasource-family".to_string(),
                severity: build_risk_metadata("unknown-datasource-family")
                    .1
                    .to_string(),
                category: build_risk_metadata("unknown-datasource-family")
                    .0
                    .to_string(),
                dashboard_uid: row.dashboard_uid.clone(),
                panel_id: row.panel_id.clone(),
                datasource: identity.name.clone(),
                detail: row.query_field.clone(),
                recommendation: build_risk_metadata("unknown-datasource-family")
                    .2
                    .to_string(),
            };
            if seen.insert(risk.clone()) {
                risks.push(risk);
            }
        }
        if row.metrics.is_empty() && row.measurements.is_empty() && row.buckets.is_empty() {
            let risk = GovernanceRiskRow {
                kind: "empty-query-analysis".to_string(),
                severity: build_risk_metadata("empty-query-analysis").1.to_string(),
                category: build_risk_metadata("empty-query-analysis").0.to_string(),
                dashboard_uid: row.dashboard_uid.clone(),
                panel_id: row.panel_id.clone(),
                datasource: identity.name,
                detail: row.query_field.clone(),
                recommendation: build_risk_metadata("empty-query-analysis").2.to_string(),
            };
            if seen.insert(risk.clone()) {
                risks.push(risk);
            }
        }
    }
    risks.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then(left.kind.cmp(&right.kind))
            .then(left.dashboard_uid.cmp(&right.dashboard_uid))
            .then(left.panel_id.cmp(&right.panel_id))
            .then(left.datasource.cmp(&right.datasource))
    });
    risks
}

/// Purpose: implementation note.
pub(crate) fn build_export_inspection_governance_document(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> ExportInspectionGovernanceDocument {
    let datasource_families = build_datasource_family_coverage_rows(summary, report);
    let dashboard_dependencies = build_dashboard_dependency_rows(report);
    let datasources = build_datasource_coverage_rows(summary, report);
    let risk_records = build_governance_risk_rows(summary, report);
    ExportInspectionGovernanceDocument {
        summary: GovernanceSummary {
            dashboard_count: summary.dashboard_count,
            query_record_count: report.summary.report_row_count,
            datasource_inventory_count: summary.datasource_inventory_count,
            datasource_family_count: datasource_families.len(),
            datasource_coverage_count: datasources.len(),
            mixed_datasource_dashboard_count: summary.mixed_dashboard_count,
            orphaned_datasource_count: summary
                .datasource_inventory
                .iter()
                .filter(|item| item.reference_count == 0 && item.dashboard_count == 0)
                .count(),
            risk_record_count: risk_records.len(),
        },
        datasource_families,
        dashboard_dependencies,
        datasources,
        risk_records,
    }
}

/// Purpose: implementation note.
pub(crate) fn render_governance_table_report(
    import_dir: &str,
    document: &ExportInspectionGovernanceDocument,
) -> Vec<String> {
    let mut lines = vec![
        format!("Export inspection governance: {import_dir}"),
        String::new(),
    ];

    lines.push("# Summary".to_string());
    lines.extend(render_simple_table(
        &["DASHBOARDS", "QUERIES", "FAMILIES", "DATASOURCES", "RISKS"],
        &[vec![
            document.summary.dashboard_count.to_string(),
            document.summary.query_record_count.to_string(),
            document.summary.datasource_family_count.to_string(),
            document.summary.datasource_coverage_count.to_string(),
            document.summary.risk_record_count.to_string(),
        ]],
        true,
    ));

    lines.push(String::new());
    lines.push("# Datasource Families".to_string());
    let family_rows = document
        .datasource_families
        .iter()
        .map(|row| {
            vec![
                row.family.clone(),
                row.datasource_types.join(","),
                row.datasource_count.to_string(),
                row.dashboard_count.to_string(),
                row.panel_count.to_string(),
                row.query_count.to_string(),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if family_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "FAMILY",
                "TYPES",
                "DATASOURCES",
                "DASHBOARDS",
                "PANELS",
                "QUERIES",
            ],
            &family_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Dashboard Dependencies".to_string());
    let dashboard_rows = document
        .dashboard_dependencies
        .iter()
        .map(|row| {
            vec![
                row.dashboard_uid.clone(),
                row.dashboard_title.clone(),
                row.folder_path.clone(),
                row.panel_count.to_string(),
                row.query_count.to_string(),
                row.datasources.join(","),
                row.datasource_families.join(","),
                row.file_path.clone(),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if dashboard_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "DASHBOARD_UID",
                "TITLE",
                "FOLDER_PATH",
                "PANELS",
                "QUERIES",
                "DATASOURCES",
                "FAMILIES",
                "FILE",
            ],
            &dashboard_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Datasources".to_string());
    let datasource_rows = document
        .datasources
        .iter()
        .map(|row| {
            vec![
                row.datasource_uid.clone(),
                row.datasource.clone(),
                row.family.clone(),
                row.query_count.to_string(),
                row.dashboard_count.to_string(),
                if row.orphaned {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if datasource_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "UID",
                "DATASOURCE",
                "FAMILY",
                "QUERIES",
                "DASHBOARDS",
                "ORPHANED",
            ],
            &datasource_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Risks".to_string());
    let risk_rows = document
        .risk_records
        .iter()
        .map(|row| {
            vec![
                row.severity.clone(),
                row.category.clone(),
                row.kind.clone(),
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                row.datasource.clone(),
                row.detail.clone(),
                row.recommendation.clone(),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if risk_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "SEVERITY",
                "CATEGORY",
                "KIND",
                "DASHBOARD_UID",
                "PANEL_ID",
                "DATASOURCE",
                "DETAIL",
                "RECOMMENDATION",
            ],
            &risk_rows,
            true,
        ));
    }
    lines
}
