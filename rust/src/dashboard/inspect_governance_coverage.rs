//! Datasource/family coverage builders for inspect governance.
use std::collections::{BTreeMap, BTreeSet};

use crate::dashboard::ExportInspectionSummary;

use super::super::inspect_family::normalize_family_name;
use super::{
    build_dashboard_dependency_rows, find_broad_loki_selector, resolve_datasource_identity,
    DatasourceCoverageRow, DatasourceFamilyCoverageRow, DatasourceGovernanceRow,
    ExportInspectionQueryReport, GOVERNANCE_RISK_KIND_BROAD_LOKI_SELECTOR,
    GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE,
    GOVERNANCE_RISK_KIND_DATASOURCE_HIGH_BLAST_RADIUS, GOVERNANCE_RISK_KIND_EMPTY_QUERY_ANALYSIS,
    GOVERNANCE_RISK_KIND_MIXED_DASHBOARD, GOVERNANCE_RISK_KIND_ORPHANED_DATASOURCE,
    GOVERNANCE_RISK_KIND_UNKNOWN_DATASOURCE_FAMILY,
};

type InventoryIdentity = (String, String, String);
type InventoryLookup = BTreeMap<String, InventoryIdentity>;
const HIGH_BLAST_RADIUS_DASHBOARD_THRESHOLD: usize = 3;
const HIGH_BLAST_RADIUS_FOLDER_THRESHOLD: usize = 2;

fn has_high_blast_radius(dashboard_count: usize, folder_count: usize) -> bool {
    dashboard_count >= HIGH_BLAST_RADIUS_DASHBOARD_THRESHOLD
        || (dashboard_count >= 2 && folder_count >= HIGH_BLAST_RADIUS_FOLDER_THRESHOLD)
}

type FamilyCoverage = (
    BTreeSet<String>,
    BTreeSet<String>,
    BTreeSet<String>,
    BTreeSet<String>,
    usize,
    usize,
);

pub(crate) fn build_inventory_lookup(
    summary: &ExportInspectionSummary,
) -> (InventoryLookup, InventoryLookup) {
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

pub(crate) fn build_datasource_governance_rows(
    summary: &ExportInspectionSummary,
    report: &ExportInspectionQueryReport,
) -> Vec<DatasourceGovernanceRow> {
    let (inventory_by_uid, inventory_by_name) = build_inventory_lookup(summary);
    let pressured_dashboard_uids = build_dashboard_dependency_rows(report)
        .into_iter()
        .filter(|row| row.panel_count > 30)
        .map(|row| row.dashboard_uid)
        .collect::<BTreeSet<String>>();
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
                BTreeSet::new(),
                BTreeSet::new(),
                orphaned,
                0usize,
            )
        });
        record.9 = orphaned;
        if orphaned {
            record.7.insert((
                GOVERNANCE_RISK_KIND_ORPHANED_DATASOURCE.to_string(),
                String::new(),
                String::new(),
            ));
            record
                .8
                .insert(GOVERNANCE_RISK_KIND_ORPHANED_DATASOURCE.to_string());
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
                BTreeSet::new(),
                BTreeSet::new(),
                false,
                0usize,
            )
        });
        record.2.insert(row.dashboard_uid.clone());
        record.3.insert(row.dashboard_title.clone());
        if !row.folder_path.trim().is_empty() {
            record.4.insert(row.folder_path.clone());
        }
        record
            .5
            .insert(format!("{}:{}", row.dashboard_uid, row.panel_id));
        record.6.insert(row.query_field.clone());
        record.10 += 1;
        record.9 = false;

        if mixed_dashboard_uids.contains(&row.dashboard_uid) {
            record.7.insert((
                GOVERNANCE_RISK_KIND_MIXED_DASHBOARD.to_string(),
                row.dashboard_uid.clone(),
                String::new(),
            ));
            record
                .8
                .insert(GOVERNANCE_RISK_KIND_MIXED_DASHBOARD.to_string());
        }
        if family == "unknown" {
            record.7.insert((
                GOVERNANCE_RISK_KIND_UNKNOWN_DATASOURCE_FAMILY.to_string(),
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
            ));
            record
                .8
                .insert(GOVERNANCE_RISK_KIND_UNKNOWN_DATASOURCE_FAMILY.to_string());
        }
        if family == "loki" && find_broad_loki_selector(&row.query_text).is_some() {
            record.7.insert((
                GOVERNANCE_RISK_KIND_BROAD_LOKI_SELECTOR.to_string(),
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
            ));
            record
                .8
                .insert(GOVERNANCE_RISK_KIND_BROAD_LOKI_SELECTOR.to_string());
        }
        if pressured_dashboard_uids.contains(&row.dashboard_uid) {
            record.7.insert((
                GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE.to_string(),
                row.dashboard_uid.clone(),
                String::new(),
            ));
            record
                .8
                .insert(GOVERNANCE_RISK_KIND_DASHBOARD_PANEL_PRESSURE.to_string());
        }
        if row.metrics.is_empty()
            && row.functions.is_empty()
            && row.measurements.is_empty()
            && row.buckets.is_empty()
        {
            record.7.insert((
                GOVERNANCE_RISK_KIND_EMPTY_QUERY_ANALYSIS.to_string(),
                row.dashboard_uid.clone(),
                row.panel_id.clone(),
            ));
            record
                .8
                .insert(GOVERNANCE_RISK_KIND_EMPTY_QUERY_ANALYSIS.to_string());
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
                    dashboard_titles,
                    folder_paths,
                    panel_keys,
                    _query_fields,
                    risk_occurrences,
                    risk_kinds,
                    orphaned,
                    query_count,
                ),
            )| {
                let high_blast_radius =
                    has_high_blast_radius(dashboard_uids.len(), folder_paths.len());
                let mut risk_occurrences = risk_occurrences;
                let mut risk_kinds = risk_kinds;
                if high_blast_radius {
                    risk_occurrences.insert((
                        GOVERNANCE_RISK_KIND_DATASOURCE_HIGH_BLAST_RADIUS.to_string(),
                        String::new(),
                        String::new(),
                    ));
                    risk_kinds
                        .insert(GOVERNANCE_RISK_KIND_DATASOURCE_HIGH_BLAST_RADIUS.to_string());
                }
                DatasourceGovernanceRow {
                    datasource_uid,
                    datasource,
                    family,
                    query_count,
                    dashboard_count: dashboard_uids.len(),
                    panel_count: panel_keys.len(),
                    mixed_dashboard_count: risk_occurrences
                        .iter()
                        .filter(|(kind, _, _)| kind == GOVERNANCE_RISK_KIND_MIXED_DASHBOARD)
                        .count(),
                    risk_count: risk_occurrences.len(),
                    risk_kinds: risk_kinds.into_iter().collect(),
                    folder_count: folder_paths.len(),
                    high_blast_radius,
                    cross_folder: folder_paths.len() > 1,
                    folder_paths: folder_paths.into_iter().collect(),
                    dashboard_uids: dashboard_uids.into_iter().collect(),
                    dashboard_titles: dashboard_titles.into_iter().collect(),
                    orphaned,
                }
            },
        )
        .collect::<Vec<_>>();
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

pub(crate) fn dashboard_dependency_unique_strings(
    values: impl IntoIterator<Item = String>,
) -> Vec<String> {
    collect_unique_strings(values)
}

pub(crate) fn dashboard_dependency_normalize_family_list(families: &[String]) -> Vec<String> {
    normalize_family_list(families)
}
