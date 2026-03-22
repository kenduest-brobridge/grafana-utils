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
    #[serde(rename = "dashboardDatasourceEdgeCount")]
    pub(crate) dashboard_datasource_edge_count: usize,
    #[serde(rename = "datasourceRiskCoverageCount")]
    pub(crate) datasource_risk_coverage_count: usize,
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
    #[serde(rename = "orphanedDatasourceCount")]
    pub(crate) orphaned_datasource_count: usize,
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
    #[serde(rename = "dashboardUids")]
    pub(crate) dashboard_uids: Vec<String>,
    #[serde(rename = "queryFields")]
    pub(crate) query_fields: Vec<String>,
    pub(crate) orphaned: bool,
}

/// Struct definition for DatasourceGovernanceRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DatasourceGovernanceRow {
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
    #[serde(rename = "mixedDashboardCount")]
    pub(crate) mixed_dashboard_count: usize,
    #[serde(rename = "riskCount")]
    pub(crate) risk_count: usize,
    #[serde(rename = "riskKinds")]
    pub(crate) risk_kinds: Vec<String>,
    #[serde(rename = "dashboardUids")]
    pub(crate) dashboard_uids: Vec<String>,
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
    #[serde(rename = "datasourceCount")]
    pub(crate) datasource_count: usize,
    #[serde(rename = "datasourceFamilyCount")]
    pub(crate) datasource_family_count: usize,
    pub(crate) datasources: Vec<String>,
    #[serde(rename = "datasourceFamilies")]
    pub(crate) datasource_families: Vec<String>,
    #[serde(rename = "queryFields")]
    pub(crate) query_fields: Vec<String>,
    pub(crate) metrics: Vec<String>,
    pub(crate) functions: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
}

/// Struct definition for DashboardDatasourceEdgeRow.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct DashboardDatasourceEdgeRow {
    #[serde(rename = "dashboardUid")]
    pub(crate) dashboard_uid: String,
    #[serde(rename = "dashboardTitle")]
    pub(crate) dashboard_title: String,
    #[serde(rename = "folderPath")]
    pub(crate) folder_path: String,
    #[serde(rename = "datasourceUid")]
    pub(crate) datasource_uid: String,
    pub(crate) datasource: String,
    #[serde(rename = "datasourceType")]
    pub(crate) datasource_type: String,
    pub(crate) family: String,
    #[serde(rename = "panelCount")]
    pub(crate) panel_count: usize,
    #[serde(rename = "queryCount")]
    pub(crate) query_count: usize,
    #[serde(rename = "queryFields")]
    pub(crate) query_fields: Vec<String>,
    pub(crate) metrics: Vec<String>,
    pub(crate) functions: Vec<String>,
    pub(crate) measurements: Vec<String>,
    pub(crate) buckets: Vec<String>,
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
    #[serde(rename = "dashboardDatasourceEdges")]
    pub(crate) dashboard_datasource_edges: Vec<DashboardDatasourceEdgeRow>,
    #[serde(rename = "datasourceGovernance")]
    pub(crate) datasource_governance: Vec<DatasourceGovernanceRow>,
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
    let lowered = datasource_type.trim().to_ascii_lowercase();
    let normalized = lowered
        .strip_prefix("grafana-")
        .and_then(|value| value.strip_suffix("-datasource"))
        .unwrap_or_else(|| lowered.strip_suffix("-datasource").unwrap_or(&lowered));
    match normalized {
        "" => "unknown".to_string(),
        "influxdb" | "flux" => "flux".to_string(),
        "prometheus" => "prometheus".to_string(),
        "loki" => "loki".to_string(),
        "mysql" | "postgres" | "mssql" => "sql".to_string(),
        "postgresql" => "sql".to_string(),
        "elasticsearch" | "opensearch" => "search".to_string(),
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

fn collect_unique_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

type InventoryIdentity = (String, String, String);
type InventoryLookup = BTreeMap<String, InventoryIdentity>;
type FamilyCoverage = (
    BTreeSet<String>,
    BTreeSet<String>,
    BTreeSet<String>,
    BTreeSet<String>,
    usize,
    usize,
);

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

pub(crate) fn governance_risk_spec(kind: &str) -> (&'static str, &'static str, &'static str) {
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
        "broad-loki-selector" => (
            "cost",
            "medium",
            "Narrow the Loki stream selector before running expensive line filters or aggregations.",
        ),
        _ => (
            "other",
            "low",
            "Review this governance finding and assign a follow-up owner if action is needed.",
        ),
    }
}

fn build_governance_risk_row(
    kind: &str,
    dashboard_uid: String,
    panel_id: String,
    datasource: String,
    detail: String,
) -> GovernanceRiskRow {
    let (category, severity, recommendation) = governance_risk_spec(kind);
    GovernanceRiskRow {
        kind: kind.to_string(),
        severity: severity.to_string(),
        category: category.to_string(),
        dashboard_uid,
        panel_id,
        datasource,
        detail,
        recommendation: recommendation.to_string(),
    }
}

fn extract_loki_stream_selectors(query_text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut in_quotes = false;
    let mut escaped = false;
    let mut capture_start: Option<usize> = None;
    for (index, character) in query_text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' if in_quotes => {
                escaped = true;
            }
            '"' => {
                in_quotes = !in_quotes;
            }
            '{' if !in_quotes => {
                capture_start = Some(index);
            }
            '}' if !in_quotes => {
                if let Some(start) = capture_start.take() {
                    let selector = &query_text[start..index + character.len_utf8()];
                    if !values.iter().any(|value| value == selector) {
                        values.push(selector.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    values
}

fn split_loki_selector_matchers(selector: &str) -> Vec<String> {
    let mut matchers = Vec::new();
    let mut start = 0usize;
    let mut in_quotes = false;
    let mut escaped = false;
    for (index, character) in selector.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' if in_quotes => {
                escaped = true;
            }
            '"' => {
                in_quotes = !in_quotes;
            }
            ',' if !in_quotes => {
                matchers.push(selector[start..index].trim().to_string());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    matchers.push(selector[start..].trim().to_string());
    matchers
}

fn loki_regex_is_wildcard(value: &str) -> bool {
    let trimmed = value.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(trimmed);
    matches!(unquoted.trim(), ".*" | "^.*$" | ".+" | "^.+$")
}

fn loki_selector_is_broad(selector: &str) -> bool {
    let trimmed = selector.trim();
    let Some(inner) = trimmed
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
    else {
        return false;
    };
    if inner.trim().is_empty() {
        return true;
    }
    let mut saw_matcher = false;
    for matcher in split_loki_selector_matchers(inner) {
        let matcher = matcher.trim();
        if matcher.is_empty() {
            continue;
        }
        saw_matcher = true;
        if let Some((_, value)) = matcher.split_once("=~") {
            if !loki_regex_is_wildcard(value) {
                return false;
            }
            continue;
        }
        if matcher.contains("!~") || matcher.contains("!=") || matcher.contains('=') {
            return false;
        }
        return false;
    }
    saw_matcher
}

fn find_broad_loki_selector(query_text: &str) -> Option<String> {
    extract_loki_stream_selectors(query_text)
        .into_iter()
        .find(|selector| loki_selector_is_broad(selector))
}

/// Purpose: implementation note.
pub(crate) fn build_datasource_family_coverage_rows(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> Vec<DatasourceFamilyCoverageRow> {
    let (inventory_by_uid, inventory_by_name) = build_inventory_lookup(summary);
    let mut coverage = BTreeMap::<String, FamilyCoverage>::new();
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
    for datasource in &summary.datasource_inventory {
        if datasource.reference_count != 0 || datasource.dashboard_count != 0 {
            continue;
        }
        let family = normalize_family_name(&datasource.datasource_type);
        let record = coverage.entry(family).or_insert_with(|| {
            (
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                0usize,
                0usize,
            )
        });
        if !datasource.datasource_type.trim().is_empty() {
            record.0.insert(datasource.datasource_type.clone());
        }
        record.5 += 1;
    }
    coverage
        .into_iter()
        .map(
            |(
                family,
                (
                    datasource_types,
                    datasource_uids,
                    dashboard_uids,
                    panel_keys,
                    query_count,
                    orphaned_count,
                ),
            )| {
                DatasourceFamilyCoverageRow {
                    family,
                    datasource_types: datasource_types.into_iter().collect(),
                    datasource_count: datasource_uids.len(),
                    orphaned_datasource_count: orphaned_count,
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
                    dashboard_uids: dashboards.into_iter().collect(),
                    query_fields: query_fields.into_iter().collect(),
                    orphaned,
                }
            },
        )
        .collect()
}

/// Purpose: implementation note.
pub(crate) fn build_datasource_governance_rows(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> Vec<DatasourceGovernanceRow> {
    let (inventory_by_uid, inventory_by_name) = build_inventory_lookup(summary);
    let mixed_dashboard_uids = summary
        .mixed_dashboards
        .iter()
        .map(|dashboard| dashboard.uid.clone())
        .collect::<BTreeSet<String>>();
    let mut coverage = BTreeMap::<
        String,
        (
            String,
            String,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<(String, String, String)>,
            BTreeSet<String>,
            bool,
            usize,
        ),
    >::new();

    for datasource in &summary.datasource_inventory {
        let key = if datasource.uid.trim().is_empty() {
            datasource.name.clone()
        } else {
            datasource.uid.clone()
        };
        let orphaned = datasource.reference_count == 0 && datasource.dashboard_count == 0;
        let record = coverage.entry(key).or_insert_with(|| {
            (
                datasource.name.clone(),
                normalize_family_name(&datasource.datasource_type),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                orphaned,
                0usize,
            )
        });
        record.7 = orphaned;
        if orphaned {
            record.5.insert((
                "orphaned-datasource".to_string(),
                String::new(),
                String::new(),
            ));
            record.6.insert("orphaned-datasource".to_string());
        }
    }

    for row in &report.queries {
        let identity = resolve_datasource_identity(row, &inventory_by_uid, &inventory_by_name);
        let family = normalize_family_name(&identity.datasource_type);
        let record = coverage.entry(identity.uid.clone()).or_insert_with(|| {
            (
                identity.name.clone(),
                family.clone(),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                BTreeSet::new(),
                false,
                0usize,
            )
        });
        record.2.insert(row.dashboard_uid.clone());
        record
            .3
            .insert(format!("{}:{}", row.dashboard_uid, row.panel_id));
        record.4.insert(row.query_field.clone());
        record.8 += 1;
        record.7 = false;

        if mixed_dashboard_uids.contains(&row.dashboard_uid) {
            record.5.insert((
                "mixed-datasource-dashboard".to_string(),
                row.dashboard_uid.clone(),
                String::new(),
            ));
            record.6.insert("mixed-datasource-dashboard".to_string());
        }
        if family == "unknown" {
            record.5.insert((
                "unknown-datasource-family".to_string(),
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
            ));
            record.6.insert("unknown-datasource-family".to_string());
        }
        if family == "loki" && find_broad_loki_selector(&row.query_text).is_some() {
            record.5.insert((
                "broad-loki-selector".to_string(),
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
            ));
            record.6.insert("broad-loki-selector".to_string());
        }
        if row.metrics.is_empty()
            && row.functions.is_empty()
            && row.measurements.is_empty()
            && row.buckets.is_empty()
        {
            record.5.insert((
                "empty-query-analysis".to_string(),
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
            ));
            record.6.insert("empty-query-analysis".to_string());
        }
    }

    let mut rows = coverage
        .into_iter()
        .map(
            |(
                datasource_uid,
                (
                    datasource,
                    family,
                    dashboard_uids,
                    panel_keys,
                    _query_fields,
                    risk_occurrences,
                    risk_kinds,
                    orphaned,
                    query_count,
                ),
            )| DatasourceGovernanceRow {
                datasource_uid,
                datasource,
                family,
                query_count,
                dashboard_count: dashboard_uids.len(),
                panel_count: panel_keys.len(),
                mixed_dashboard_count: risk_occurrences
                    .iter()
                    .filter(|(kind, _, _)| kind == "mixed-datasource-dashboard")
                    .count(),
                risk_count: risk_occurrences.len(),
                risk_kinds: risk_kinds.into_iter().collect(),
                dashboard_uids: dashboard_uids.into_iter().collect(),
                orphaned,
            },
        )
        .collect::<Vec<DatasourceGovernanceRow>>();

    rows.sort_by(|left, right| {
        right
            .risk_count
            .cmp(&left.risk_count)
            .then(right.mixed_dashboard_count.cmp(&left.mixed_dashboard_count))
            .then(right.query_count.cmp(&left.query_count))
            .then(left.datasource_uid.cmp(&right.datasource_uid))
    });
    rows
}

/// Purpose: implementation note.
pub(crate) fn build_dashboard_dependency_rows(
    report: &ExportInspectionQueryReport,
) -> Vec<DashboardDependencyRow> {
    let normalized = super::inspect_report::normalize_query_report(report);
    normalized
        .dashboards
        .into_iter()
        .map(|dashboard| {
            let dashboard_uid = dashboard.dashboard_uid;
            let dashboard_title = dashboard.dashboard_title;
            let folder_path = dashboard.folder_path;
            let file_path = dashboard.file_path;
            let panel_count = dashboard.panels.len();
            let query_count = dashboard
                .panels
                .iter()
                .map(|panel| panel.queries.len())
                .sum::<usize>();
            let datasources = dashboard.datasources;
            let datasource_families = normalize_family_list(&dashboard.datasource_families);
            let query_fields = collect_unique_strings(
                dashboard
                    .panels
                    .iter()
                    .flat_map(|panel| panel.query_fields.iter().cloned()),
            );
            let metrics = collect_unique_strings(dashboard.panels.iter().flat_map(|panel| {
                panel
                    .queries
                    .iter()
                    .flat_map(|row| row.metrics.iter().cloned())
            }));
            let functions = collect_unique_strings(dashboard.panels.iter().flat_map(|panel| {
                panel
                    .queries
                    .iter()
                    .flat_map(|row| row.functions.iter().cloned())
            }));
            let measurements = collect_unique_strings(dashboard.panels.iter().flat_map(|panel| {
                panel
                    .queries
                    .iter()
                    .flat_map(|row| row.measurements.iter().cloned())
            }));
            let buckets = collect_unique_strings(dashboard.panels.iter().flat_map(|panel| {
                panel
                    .queries
                    .iter()
                    .flat_map(|row| row.buckets.iter().cloned())
            }));

            DashboardDependencyRow {
                dashboard_uid,
                dashboard_title,
                folder_path,
                file_path,
                panel_count,
                query_count,
                datasource_count: datasources.len(),
                datasource_family_count: datasource_families.len(),
                datasources,
                datasource_families,
                query_fields,
                metrics,
                functions,
                measurements,
                buckets,
            }
        })
        .collect()
}

/// Purpose: implementation note.
pub(crate) fn build_dashboard_datasource_edge_rows(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> Vec<DashboardDatasourceEdgeRow> {
    let (inventory_by_uid, inventory_by_name) = build_inventory_lookup(summary);
    let mut edges = BTreeMap::<
        (String, String),
        (
            String,
            String,
            String,
            String,
            String,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            BTreeSet<String>,
            usize,
        ),
    >::new();
    for row in &report.queries {
        let identity = resolve_datasource_identity(row, &inventory_by_uid, &inventory_by_name);
        let edge = edges
            .entry((row.dashboard_uid.clone(), identity.uid.clone()))
            .or_insert_with(|| {
                (
                    row.dashboard_title.clone(),
                    row.folder_path.clone(),
                    identity.name.clone(),
                    identity.datasource_type.clone(),
                    normalize_family_name(&identity.datasource_type),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    BTreeSet::new(),
                    0usize,
                )
            });
        edge.5
            .insert(format!("{}:{}", row.dashboard_uid, row.panel_id));
        if !row.query_field.trim().is_empty() {
            edge.6.insert(row.query_field.clone());
        }
        edge.7.extend(row.metrics.iter().cloned());
        edge.8.extend(row.functions.iter().cloned());
        edge.9.extend(row.measurements.iter().cloned());
        edge.10.extend(row.buckets.iter().cloned());
        edge.11 += 1;
    }
    edges
        .into_iter()
        .map(
            |(
                (dashboard_uid, datasource_uid),
                (
                    dashboard_title,
                    folder_path,
                    datasource,
                    datasource_type,
                    family,
                    panel_keys,
                    query_fields,
                    metrics,
                    functions,
                    measurements,
                    buckets,
                    query_count,
                ),
            )| DashboardDatasourceEdgeRow {
                dashboard_uid,
                dashboard_title,
                folder_path,
                datasource_uid,
                datasource,
                datasource_type,
                family,
                panel_count: panel_keys.len(),
                query_count,
                query_fields: query_fields.into_iter().collect(),
                metrics: metrics.into_iter().collect(),
                functions: functions.into_iter().collect(),
                measurements: measurements.into_iter().collect(),
                buckets: buckets.into_iter().collect(),
            },
        )
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
        let risk = build_governance_risk_row(
            "mixed-datasource-dashboard",
            dashboard.uid.clone(),
            String::new(),
            dashboard.datasources.join(","),
            dashboard.title.clone(),
        );
        if seen.insert(risk.clone()) {
            risks.push(risk);
        }
    }
    for datasource in &summary.datasource_inventory {
        if datasource.reference_count != 0 || datasource.dashboard_count != 0 {
            continue;
        }
        let risk = build_governance_risk_row(
            "orphaned-datasource",
            String::new(),
            String::new(),
            if datasource.uid.trim().is_empty() {
                datasource.name.clone()
            } else {
                datasource.uid.clone()
            },
            datasource.datasource_type.clone(),
        );
        if seen.insert(risk.clone()) {
            risks.push(risk);
        }
    }
    for row in &report.queries {
        let identity = resolve_datasource_identity(row, &inventory_by_uid, &inventory_by_name);
        if normalize_family_name(&identity.datasource_type) == "unknown" {
            let risk = build_governance_risk_row(
                "unknown-datasource-family",
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                identity.name.clone(),
                row.query_field.clone(),
            );
            if seen.insert(risk.clone()) {
                risks.push(risk);
            }
        }
        if normalize_family_name(&identity.datasource_type) == "loki" {
            if let Some(selector) = find_broad_loki_selector(&row.query_text) {
                let risk = build_governance_risk_row(
                    "broad-loki-selector",
                    row.dashboard_uid.clone(),
                    row.panel_id.clone(),
                    identity.name.clone(),
                    selector,
                );
                if seen.insert(risk.clone()) {
                    risks.push(risk);
                }
            }
        }
        if row.metrics.is_empty()
            && row.functions.is_empty()
            && row.measurements.is_empty()
            && row.buckets.is_empty()
        {
            let risk = build_governance_risk_row(
                "empty-query-analysis",
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
                identity.name,
                row.query_field.clone(),
            );
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
    let dashboard_datasource_edges = build_dashboard_datasource_edge_rows(summary, report);
    let datasource_governance = build_datasource_governance_rows(summary, report);
    let datasources = build_datasource_coverage_rows(summary, report);
    let risk_records = build_governance_risk_rows(summary, report);
    ExportInspectionGovernanceDocument {
        summary: GovernanceSummary {
            dashboard_count: summary.dashboard_count,
            query_record_count: report.summary.report_row_count,
            datasource_inventory_count: summary.datasource_inventory_count,
            datasource_family_count: datasource_families.len(),
            datasource_coverage_count: datasources.len(),
            dashboard_datasource_edge_count: dashboard_datasource_edges.len(),
            datasource_risk_coverage_count: datasource_governance
                .iter()
                .filter(|row| row.risk_count != 0)
                .count(),
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
        dashboard_datasource_edges,
        datasource_governance,
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
        &[
            "DASHBOARDS",
            "QUERIES",
            "FAMILIES",
            "DATASOURCES",
            "DASHBOARD_DATASOURCE_EDGES",
            "DATASOURCES_WITH_RISKS",
            "MIXED_DASHBOARDS",
            "ORPHANED_DATASOURCES",
            "RISKS",
        ],
        &[vec![
            document.summary.dashboard_count.to_string(),
            document.summary.query_record_count.to_string(),
            document.summary.datasource_family_count.to_string(),
            document.summary.datasource_coverage_count.to_string(),
            document.summary.dashboard_datasource_edge_count.to_string(),
            document.summary.datasource_risk_coverage_count.to_string(),
            document
                .summary
                .mixed_datasource_dashboard_count
                .to_string(),
            document.summary.orphaned_datasource_count.to_string(),
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
                row.orphaned_datasource_count.to_string(),
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
                "ORPHANED_DATASOURCES",
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
                row.datasource_count.to_string(),
                row.datasource_family_count.to_string(),
                row.datasources.join(","),
                row.datasource_families.join(","),
                row.query_fields.join(","),
                row.metrics.join(","),
                row.functions.join(","),
                row.measurements.join(","),
                row.buckets.join(","),
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
                "DATASOURCE_COUNT",
                "DATASOURCE_FAMILY_COUNT",
                "DATASOURCES",
                "FAMILIES",
                "QUERY_FIELDS",
                "METRICS",
                "FUNCTIONS",
                "MEASUREMENTS",
                "BUCKETS",
                "FILE",
            ],
            &dashboard_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Dashboard Datasource Edges".to_string());
    let edge_rows = document
        .dashboard_datasource_edges
        .iter()
        .map(|row| {
            vec![
                row.dashboard_uid.clone(),
                row.dashboard_title.clone(),
                row.folder_path.clone(),
                row.datasource_uid.clone(),
                row.datasource.clone(),
                row.datasource_type.clone(),
                row.family.clone(),
                row.panel_count.to_string(),
                row.query_count.to_string(),
                row.query_fields.join(","),
                row.metrics.join(","),
                row.functions.join(","),
                row.measurements.join(","),
                row.buckets.join(","),
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if edge_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "DASHBOARD_UID",
                "TITLE",
                "FOLDER_PATH",
                "DATASOURCE_UID",
                "DATASOURCE",
                "DATASOURCE_TYPE",
                "FAMILY",
                "PANELS",
                "QUERIES",
                "QUERY_FIELDS",
                "METRICS",
                "FUNCTIONS",
                "MEASUREMENTS",
                "BUCKETS",
            ],
            &edge_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Datasource Governance".to_string());
    let datasource_governance_rows = document
        .datasource_governance
        .iter()
        .map(|row| {
            let dashboard_uids = if row.dashboard_uids.is_empty() {
                "(none)".to_string()
            } else {
                row.dashboard_uids.join(",")
            };
            let risk_kinds = if row.risk_kinds.is_empty() {
                "(none)".to_string()
            } else {
                row.risk_kinds.join(",")
            };
            vec![
                row.datasource_uid.clone(),
                row.datasource.clone(),
                row.family.clone(),
                row.query_count.to_string(),
                row.dashboard_count.to_string(),
                row.panel_count.to_string(),
                row.mixed_dashboard_count.to_string(),
                row.risk_count.to_string(),
                risk_kinds,
                dashboard_uids,
                if row.orphaned {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ]
        })
        .collect::<Vec<Vec<String>>>();
    if datasource_governance_rows.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(render_simple_table(
            &[
                "UID",
                "DATASOURCE",
                "FAMILY",
                "QUERIES",
                "DASHBOARDS",
                "PANELS",
                "MIXED_DASHBOARDS",
                "RISKS",
                "RISK_KINDS",
                "DASHBOARD_UIDS",
                "ORPHANED",
            ],
            &datasource_governance_rows,
            true,
        ));
    }

    lines.push(String::new());
    lines.push("# Datasources".to_string());
    let datasource_rows = document
        .datasources
        .iter()
        .map(|row| {
            let dashboard_uids = if row.dashboard_uids.is_empty() {
                "(none)".to_string()
            } else {
                row.dashboard_uids.join(",")
            };
            let query_fields = if row.query_fields.is_empty() {
                "(none)".to_string()
            } else {
                row.query_fields.join(",")
            };
            vec![
                row.datasource_uid.clone(),
                row.datasource.clone(),
                row.family.clone(),
                row.query_count.to_string(),
                row.dashboard_count.to_string(),
                row.panel_count.to_string(),
                dashboard_uids,
                query_fields,
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
                "PANELS",
                "DASHBOARD_UIDS",
                "QUERY_FIELDS",
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
