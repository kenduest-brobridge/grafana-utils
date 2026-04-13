//! Renderers for supported datasource catalog outputs.
//!
//! Responsibilities:
//! - Format catalog rows for table/csv/json/yaml report commands.
//! - Bridge catalog lookup/defaults data into CLI-facing render functions.

use serde_json::{json, Value};

use super::datasource_catalog_data::DatasourceCatalogEntry;
use super::datasource_catalog_defaults::{
    build_add_defaults_document, build_full_add_defaults_document,
};
use super::datasource_catalog_lookup::supported_datasource_catalog;
use crate::common::Result;
use crate::tabular_output::{render_csv, render_table, render_yaml};

fn supported_preset_profiles(entry: &DatasourceCatalogEntry) -> Vec<&'static str> {
    if build_full_add_defaults_document(entry) == build_add_defaults_document(entry) {
        vec!["starter"]
    } else {
        vec!["starter", "full"]
    }
}

fn supported_catalog_rows() -> Vec<Vec<String>> {
    supported_datasource_catalog()
        .iter()
        .map(|entry| {
            let mut defaults = Vec::new();
            if let Some(access) = entry.add_defaults_access {
                defaults.push(format!("access={access}"));
            }
            if let Some(http_method) = entry.add_defaults_http_method {
                defaults.push(format!("jsonData.httpMethod={http_method}"));
            }
            if let Some(time_field) = entry.add_defaults_time_field {
                defaults.push(format!("jsonData.timeField={time_field}"));
            }
            for (key, value) in entry.add_defaults_json_data {
                defaults.push(format!("jsonData.{key}={}", value.to_display_value()));
            }
            vec![
                entry.category.to_string(),
                entry.display_name.to_string(),
                entry.type_id.to_string(),
                entry.profile.to_string(),
                entry.query_language.to_string(),
                if entry.requires_url {
                    "required".to_string()
                } else {
                    "optional".to_string()
                },
                if entry.aliases.is_empty() {
                    "-".to_string()
                } else {
                    entry.aliases.join(", ")
                },
                if entry.suggested_flags.is_empty() {
                    "-".to_string()
                } else {
                    entry.suggested_flags.join(", ")
                },
                if defaults.is_empty() {
                    "-".to_string()
                } else {
                    defaults.join(", ")
                },
                supported_preset_profiles(entry).join(", "),
            ]
        })
        .collect()
}

pub fn render_supported_datasource_catalog_text() -> Vec<String> {
    let mut lines = vec!["Grafana Data Sources Summary".to_string(), String::new()];
    let mut current_category = "";
    for entry in supported_datasource_catalog() {
        if entry.category != current_category {
            if !current_category.is_empty() {
                lines.push(String::new());
            }
            current_category = entry.category;
            lines.push(format!("{}:", entry.category));
        }
        let mut line = format!("  - {} ({})", entry.display_name, entry.type_id);
        line.push_str(&format!(
            " profile={} query={}",
            entry.profile, entry.query_language
        ));
        if entry.requires_url {
            line.push_str(" url=required");
        } else {
            line.push_str(" url=optional");
        }
        let mut default_bits = Vec::new();
        if let Some(access) = entry.add_defaults_access {
            default_bits.push(format!("access={access}"));
        }
        if let Some(http_method) = entry.add_defaults_http_method {
            default_bits.push(format!("jsonData.httpMethod={http_method}"));
        }
        if let Some(time_field) = entry.add_defaults_time_field {
            default_bits.push(format!("jsonData.timeField={time_field}"));
        }
        for (key, value) in entry.add_defaults_json_data {
            default_bits.push(format!("jsonData.{key}={}", value.to_display_value()));
        }
        if !default_bits.is_empty() {
            line.push_str(&format!(" defaults: {}", default_bits.join(", ")));
        }
        if !entry.aliases.is_empty() {
            line.push_str(&format!(" aliases: {}", entry.aliases.join(", ")));
        }
        if !entry.suggested_flags.is_empty() {
            line.push_str(&format!(" flags: {}", entry.suggested_flags.join(", ")));
        }
        lines.push(line);
    }
    lines
}

pub fn render_supported_datasource_catalog_table() -> Vec<String> {
    render_table(
        &[
            "category",
            "display_name",
            "type",
            "profile",
            "query_language",
            "requires_url",
            "aliases",
            "flags",
            "defaults",
            "preset_profiles",
        ],
        &supported_catalog_rows(),
    )
}

pub fn render_supported_datasource_catalog_csv() -> Vec<String> {
    render_csv(
        &[
            "category",
            "display_name",
            "type",
            "profile",
            "query_language",
            "requires_url",
            "aliases",
            "flags",
            "defaults",
            "preset_profiles",
        ],
        &supported_catalog_rows(),
    )
}

pub fn render_supported_datasource_catalog_json() -> Value {
    let categories =
        supported_datasource_catalog()
            .iter()
            .fold(Vec::<Value>::new(), |mut rows, entry| {
                if let Some(last) = rows.last_mut() {
                    let same_category = last
                        .get("category")
                        .and_then(Value::as_str)
                        .map(|value| value == entry.category)
                        .unwrap_or(false);
                    if same_category {
                        last.get_mut("types")
                            .and_then(Value::as_array_mut)
                            .expect("types array")
                            .push(json!({
                                "type": entry.type_id,
                                "displayName": entry.display_name,
                                "aliases": entry.aliases,
                                "profile": entry.profile,
                                "queryLanguage": entry.query_language,
                                "requiresDatasourceUrl": entry.requires_url,
                                "suggestedFlags": entry.suggested_flags,
                                "presetProfiles": supported_preset_profiles(entry),
                                "addDefaults": build_add_defaults_document(entry),
                                "fullAddDefaults": build_full_add_defaults_document(entry),
                            }));
                        return rows;
                    }
                }
                rows.push(json!({
                        "category": entry.category,
                        "types": [{
                        "type": entry.type_id,
                        "displayName": entry.display_name,
                        "aliases": entry.aliases,
                        "profile": entry.profile,
                        "queryLanguage": entry.query_language,
                        "requiresDatasourceUrl": entry.requires_url,
                        "suggestedFlags": entry.suggested_flags,
                        "presetProfiles": supported_preset_profiles(entry),
                        "addDefaults": build_add_defaults_document(entry),
                        "fullAddDefaults": build_full_add_defaults_document(entry),
                    }],
                }));
                rows
            });
    json!({
        "kind": "grafana-utils-datasource-supported-types",
        "categories": categories,
    })
}

pub fn render_supported_datasource_catalog_yaml() -> Result<String> {
    render_yaml(&render_supported_datasource_catalog_json())
}
