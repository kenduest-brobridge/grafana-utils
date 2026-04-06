//! Datasource diff model and normalization helpers.
//! Holds compare records/status used by list/import/export drift detection and report rendering.
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

use super::datasource_import_export::DatasourceImportRecord;

pub(crate) type DatasourceDiffRecord = DatasourceImportRecord;

/// Struct definition for DatasourceFieldDifference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DatasourceFieldDifference {
    pub(crate) field: &'static str,
    #[serde(rename = "before")]
    pub(crate) expected: String,
    #[serde(rename = "after")]
    pub(crate) actual: String,
}

/// Enum definition for DatasourceDiffStatus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DatasourceDiffStatus {
    Matches,
    Different,
    MissingInLive,
    MissingInExport,
    AmbiguousLiveMatch,
}

impl DatasourceDiffStatus {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            DatasourceDiffStatus::Matches => "same",
            DatasourceDiffStatus::Different => "different",
            DatasourceDiffStatus::MissingInLive => "missing-remote",
            DatasourceDiffStatus::MissingInExport => "extra-remote",
            DatasourceDiffStatus::AmbiguousLiveMatch => "ambiguous",
        }
    }
}

/// Struct definition for DatasourceDiffEntry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasourceDiffEntry {
    pub(crate) key: String,
    pub(crate) status: DatasourceDiffStatus,
    pub(crate) export_record: Option<DatasourceDiffRecord>,
    pub(crate) live_record: Option<DatasourceDiffRecord>,
    pub(crate) differences: Vec<DatasourceFieldDifference>,
}

/// Struct definition for DatasourceDiffSummary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasourceDiffSummary {
    pub(crate) compared_count: usize,
    pub(crate) matches_count: usize,
    pub(crate) different_count: usize,
    pub(crate) missing_in_live_count: usize,
    pub(crate) missing_in_export_count: usize,
    pub(crate) ambiguous_live_match_count: usize,
}

/// Struct definition for DatasourceDiffReport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasourceDiffReport {
    pub(crate) entries: Vec<DatasourceDiffEntry>,
    pub(crate) summary: DatasourceDiffSummary,
}

trait DatasourceDiffRecordExt {
    fn comparison_key(&self) -> String;
}

impl DatasourceDiffRecordExt for DatasourceDiffRecord {
    fn comparison_key(&self) -> String {
        if !self.uid.is_empty() {
            return format!("uid:{}", self.uid);
        }
        format!("name:{}", self.name)
    }
}

/// Purpose: implementation note.
pub(crate) fn build_datasource_diff_report(
    export_records: &[DatasourceDiffRecord],
    live_records: &[DatasourceDiffRecord],
) -> DatasourceDiffReport {
    let mut live_by_uid: BTreeMap<String, usize> = BTreeMap::new();
    let mut live_by_name: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut live_matched = vec![false; live_records.len()];
    let mut entries = Vec::new();

    for (index, record) in live_records.iter().enumerate() {
        if !record.uid.is_empty() {
            live_by_uid.insert(record.uid.clone(), index);
        }
        if !record.name.is_empty() {
            live_by_name
                .entry(record.name.clone())
                .or_default()
                .push(index);
        }
    }

    for export_record in export_records {
        let key = export_record.comparison_key();
        if !export_record.uid.is_empty() {
            if let Some(index) = live_by_uid.get(&export_record.uid) {
                live_matched[*index] = true;
                entries.push(build_entry_from_pair(
                    key,
                    export_record.clone(),
                    live_records[*index].clone(),
                ));
                continue;
            }
        }

        let name_matches = live_by_name
            .get(&export_record.name)
            .cloned()
            .unwrap_or_default();
        if name_matches.is_empty() {
            entries.push(DatasourceDiffEntry {
                key,
                status: DatasourceDiffStatus::MissingInLive,
                export_record: Some(export_record.clone()),
                live_record: None,
                differences: Vec::new(),
            });
            continue;
        }
        if name_matches.len() > 1 {
            entries.push(DatasourceDiffEntry {
                key,
                status: DatasourceDiffStatus::AmbiguousLiveMatch,
                export_record: Some(export_record.clone()),
                live_record: None,
                differences: Vec::new(),
            });
            continue;
        }

        let index = name_matches[0];
        live_matched[index] = true;
        entries.push(build_entry_from_pair(
            key,
            export_record.clone(),
            live_records[index].clone(),
        ));
    }

    for (index, live_record) in live_records.iter().enumerate() {
        if live_matched[index] {
            continue;
        }
        entries.push(DatasourceDiffEntry {
            key: live_record.comparison_key(),
            status: DatasourceDiffStatus::MissingInExport,
            export_record: None,
            live_record: Some(live_record.clone()),
            differences: Vec::new(),
        });
    }

    entries.sort_by(|left, right| left.key.cmp(&right.key));

    DatasourceDiffReport {
        summary: build_summary(&entries),
        entries,
    }
}

fn build_entry_from_pair(
    key: String,
    export_record: DatasourceDiffRecord,
    live_record: DatasourceDiffRecord,
) -> DatasourceDiffEntry {
    let differences = diff_records(&export_record, &live_record);
    let status = if differences.is_empty() {
        DatasourceDiffStatus::Matches
    } else {
        DatasourceDiffStatus::Different
    };
    DatasourceDiffEntry {
        key,
        status,
        export_record: Some(export_record),
        live_record: Some(live_record),
        differences,
    }
}

fn build_summary(entries: &[DatasourceDiffEntry]) -> DatasourceDiffSummary {
    let mut summary = DatasourceDiffSummary {
        compared_count: entries.len(),
        matches_count: 0,
        different_count: 0,
        missing_in_live_count: 0,
        missing_in_export_count: 0,
        ambiguous_live_match_count: 0,
    };

    for entry in entries {
        match entry.status {
            DatasourceDiffStatus::Matches => summary.matches_count += 1,
            DatasourceDiffStatus::Different => summary.different_count += 1,
            DatasourceDiffStatus::MissingInLive => summary.missing_in_live_count += 1,
            DatasourceDiffStatus::MissingInExport => summary.missing_in_export_count += 1,
            DatasourceDiffStatus::AmbiguousLiveMatch => summary.ambiguous_live_match_count += 1,
        }
    }

    summary
}

fn diff_records(
    expected: &DatasourceDiffRecord,
    actual: &DatasourceDiffRecord,
) -> Vec<DatasourceFieldDifference> {
    let mut differences = Vec::new();

    push_difference(&mut differences, "uid", &expected.uid, &actual.uid);
    push_difference(&mut differences, "name", &expected.name, &actual.name);
    push_difference(
        &mut differences,
        "type",
        &expected.datasource_type,
        &actual.datasource_type,
    );
    push_difference(&mut differences, "access", &expected.access, &actual.access);
    push_difference(&mut differences, "url", &expected.url, &actual.url);
    push_difference(
        &mut differences,
        "isDefault",
        if expected.is_default { "true" } else { "false" },
        if actual.is_default { "true" } else { "false" },
    );
    push_difference(&mut differences, "orgId", &expected.org_id, &actual.org_id);

    differences
}

fn push_difference(
    differences: &mut Vec<DatasourceFieldDifference>,
    field: &'static str,
    expected: &str,
    actual: &str,
) {
    if expected == actual {
        return;
    }
    differences.push(DatasourceFieldDifference {
        field,
        expected: expected.to_string(),
        actual: actual.to_string(),
    });
}

// Convert exported record map rows into a typed diff representation.
/// Purpose: implementation note.
pub(crate) fn normalize_export_records(values: &[Value]) -> Vec<DatasourceDiffRecord> {
    values
        .iter()
        .filter_map(Value::as_object)
        .map(DatasourceImportRecord::from_generic_map)
        .collect()
}

// Convert live API rows into a typed diff representation.
/// Purpose: implementation note.
pub(crate) fn normalize_live_records(values: &[Value]) -> Vec<DatasourceDiffRecord> {
    values
        .iter()
        .filter_map(Value::as_object)
        .map(DatasourceImportRecord::from_generic_map)
        .collect()
}
