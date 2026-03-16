//! Datasource diff model and normalization helpers.
//! Holds compare records/status used by list/import/export drift detection and report rendering.
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasourceDiffRecord {
    pub(crate) uid: String,
    pub(crate) name: String,
    pub(crate) datasource_type: String,
    pub(crate) access: String,
    pub(crate) url: String,
    pub(crate) is_default: bool,
    pub(crate) org_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasourceFieldDifference {
    pub(crate) field: &'static str,
    pub(crate) expected: String,
    pub(crate) actual: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DatasourceDiffStatus {
    Matches,
    Different,
    MissingInLive,
    MissingInExport,
    AmbiguousLiveMatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasourceDiffEntry {
    pub(crate) key: String,
    pub(crate) status: DatasourceDiffStatus,
    pub(crate) export_record: Option<DatasourceDiffRecord>,
    pub(crate) live_record: Option<DatasourceDiffRecord>,
    pub(crate) differences: Vec<DatasourceFieldDifference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasourceDiffSummary {
    pub(crate) compared_count: usize,
    pub(crate) matches_count: usize,
    pub(crate) different_count: usize,
    pub(crate) missing_in_live_count: usize,
    pub(crate) missing_in_export_count: usize,
    pub(crate) ambiguous_live_match_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DatasourceDiffReport {
    pub(crate) entries: Vec<DatasourceDiffEntry>,
    pub(crate) summary: DatasourceDiffSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RecordOrigin {
    Export,
    Live,
}

impl DatasourceDiffRecord {
    pub(crate) fn from_export_map(record: &Map<String, Value>) -> Self {
        Self::from_map(record, RecordOrigin::Export)
    }

    pub(crate) fn from_live_map(record: &Map<String, Value>) -> Self {
        Self::from_map(record, RecordOrigin::Live)
    }

    fn from_map(record: &Map<String, Value>, origin: RecordOrigin) -> Self {
        Self {
            uid: string_field(record, "uid"),
            name: string_field(record, "name"),
            datasource_type: string_field(record, "type"),
            access: string_field(record, "access"),
            url: string_field(record, "url"),
            is_default: bool_field(record, "isDefault"),
            org_id: normalize_org_id(record, &origin),
        }
    }

    fn comparison_key(&self) -> String {
        if !self.uid.is_empty() {
            return format!("uid:{}", self.uid);
        }
        format!("name:{}", self.name)
    }
}

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
                .or_insert_with(Vec::new)
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

fn string_field(record: &Map<String, Value>, key: &str) -> String {
    match record.get(key) {
        Some(Value::String(value)) => value.trim().to_string(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => {
            if *value {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn bool_field(record: &Map<String, Value>, key: &str) -> bool {
    match record.get(key) {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "true" | "1" | "yes"
        ),
        Some(Value::Number(value)) => value.as_i64().unwrap_or(0) != 0,
        _ => false,
    }
}

// Pull orgId consistently from export/live records so diff rows compare with a
// unified representation.
fn normalize_org_id(record: &Map<String, Value>, origin: &RecordOrigin) -> String {
    let key = match *origin {
        RecordOrigin::Export => "orgId",
        RecordOrigin::Live => "orgId",
    };
    string_field(record, key)
}

// Convert exported record map rows into a typed diff representation.
pub(crate) fn normalize_export_records(values: &[Value]) -> Vec<DatasourceDiffRecord> {
    values
        .iter()
        .filter_map(Value::as_object)
        .map(DatasourceDiffRecord::from_export_map)
        .collect()
}

// Convert live API rows into a typed diff representation.
pub(crate) fn normalize_live_records(values: &[Value]) -> Vec<DatasourceDiffRecord> {
    values
        .iter()
        .filter_map(Value::as_object)
        .map(DatasourceDiffRecord::from_live_map)
        .collect()
}
