//! Catalog lookup/modeling logic for Core data sources and metadata.

use super::datasource_catalog_data::{DatasourceCatalogEntry, DATASOURCE_CATALOG};

pub fn supported_datasource_catalog() -> &'static [DatasourceCatalogEntry] {
    DATASOURCE_CATALOG
}

pub fn find_supported_datasource_entry(
    type_or_alias: &str,
) -> Option<&'static DatasourceCatalogEntry> {
    let candidate = type_or_alias.trim().to_ascii_lowercase();
    if candidate.is_empty() {
        return None;
    }
    supported_datasource_catalog().iter().find(|entry| {
        candidate == entry.type_id || entry.aliases.iter().any(|alias| candidate == *alias)
    })
}

pub fn normalize_supported_datasource_type(type_or_alias: &str) -> String {
    find_supported_datasource_entry(type_or_alias)
        .map(|entry| entry.type_id.to_string())
        .unwrap_or_else(|| type_or_alias.trim().to_string())
}
