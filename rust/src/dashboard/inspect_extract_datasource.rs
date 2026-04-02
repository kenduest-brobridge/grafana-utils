//! Inspection path for Dashboard resources: analysis, extraction, and report shaping.

use serde_json::Value;

use crate::dashboard::inspect_query::{
    resolve_query_analyzer_family_from_datasource_type,
    resolve_query_analyzer_family_from_query_signature, QueryExtractionContext,
    DATASOURCE_FAMILY_UNKNOWN,
};
use crate::dashboard::models::DatasourceInventoryItem;
use crate::dashboard::prompt::{
    datasource_type_alias, is_builtin_datasource_ref, is_placeholder_string,
};

#[derive(Clone, Copy, Debug)]
enum DatasourceReference<'a> {
    String(&'a str),
    Object(DatasourceReferenceObject<'a>),
}

#[derive(Clone, Copy, Debug)]
struct DatasourceReferenceObject<'a> {
    uid: Option<&'a str>,
    name: Option<&'a str>,
    plugin_id: Option<&'a str>,
    datasource_type: Option<&'a str>,
}

impl<'a> DatasourceReferenceObject<'a> {
    fn from_value(reference: &'a Value) -> Option<Self> {
        let object = reference.as_object()?;
        let uid = object
            .get("uid")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && !is_placeholder_string(value));
        let name = object
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && !is_placeholder_string(value));
        let plugin_id = object
            .get("pluginId")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && !is_placeholder_string(value));
        let datasource_type = object
            .get("type")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty() && !is_placeholder_string(value));
        if uid.is_none() && name.is_none() && plugin_id.is_none() && datasource_type.is_none() {
            None
        } else {
            Some(Self {
                uid,
                name,
                plugin_id,
                datasource_type,
            })
        }
    }

    fn summary_label(self) -> Option<&'a str> {
        self.name
            .or(self.uid)
            .or(self.plugin_id)
            .or(self.datasource_type)
    }

    fn uid_label(self) -> Option<&'a str> {
        self.uid
    }

    fn inventory_item(
        self,
        datasource_inventory: &'a [DatasourceInventoryItem],
    ) -> Option<&'a DatasourceInventoryItem> {
        datasource_inventory.iter().find(|datasource| {
            self.uid
                .map(|value| datasource.uid == value)
                .unwrap_or(false)
                || self
                    .name
                    .map(|value| datasource.name == value)
                    .unwrap_or(false)
        })
    }

    fn name_label(self, datasource_inventory: &'a [DatasourceInventoryItem]) -> Option<String> {
        if let Some(datasource) = self.inventory_item(datasource_inventory) {
            if !datasource.name.is_empty() {
                return Some(datasource.name.clone());
            }
        }
        self.uid
            .map(str::to_string)
            .or_else(|| self.name.map(str::to_string))
    }

    fn type_label(self, datasource_inventory: &'a [DatasourceInventoryItem]) -> Option<String> {
        if let Some(datasource) = self.inventory_item(datasource_inventory) {
            if !datasource.datasource_type.is_empty() {
                return Some(datasource.datasource_type.clone());
            }
        }
        self.datasource_type
            .or(self.plugin_id)
            .map(|value| datasource_type_alias(value).to_string())
    }
}

impl<'a> DatasourceReference<'a> {
    fn parse(reference: &'a Value) -> Option<Self> {
        if reference.is_null() {
            return None;
        }
        match reference {
            Value::String(text) => {
                if is_builtin_datasource_ref(reference) {
                    return None;
                }
                let normalized = text.trim();
                if normalized.is_empty() {
                    None
                } else {
                    Some(Self::String(normalized))
                }
            }
            Value::Object(_) => {
                let object = DatasourceReferenceObject::from_value(reference)?;
                if object.datasource_type.is_none() && is_builtin_datasource_ref(reference) {
                    None
                } else {
                    Some(Self::Object(object))
                }
            }
            _ => None,
        }
    }

    fn summary_label(self) -> Option<String> {
        match self {
            Self::String(text) => {
                if is_placeholder_string(text) {
                    None
                } else {
                    Some(text.to_string())
                }
            }
            Self::Object(reference) => reference.summary_label().map(str::to_string),
        }
    }

    fn uid_label(self) -> Option<String> {
        match self {
            Self::String(text) => {
                if is_placeholder_string(text) {
                    None
                } else {
                    Some(text.to_string())
                }
            }
            Self::Object(reference) => reference.uid_label().map(str::to_string),
        }
    }

    fn name_label(self, datasource_inventory: &'a [DatasourceInventoryItem]) -> Option<String> {
        match self {
            Self::String(text) => {
                let normalized = text.trim();
                if normalized.is_empty() || is_placeholder_string(normalized) {
                    return None;
                }
                datasource_inventory
                    .iter()
                    .find(|datasource| {
                        datasource.uid == normalized || datasource.name == normalized
                    })
                    .map(|datasource| datasource.name.clone())
                    .or_else(|| Some(text.to_string()))
            }
            Self::Object(reference) => reference.name_label(datasource_inventory),
        }
    }

    fn type_label(self, datasource_inventory: &'a [DatasourceInventoryItem]) -> Option<String> {
        match self {
            Self::String(text) => {
                let normalized = text.trim();
                if normalized.is_empty() || is_placeholder_string(normalized) {
                    None
                } else {
                    datasource_inventory
                        .iter()
                        .find(|datasource| {
                            datasource.uid == normalized || datasource.name == normalized
                        })
                        .map(|datasource| datasource.datasource_type.clone())
                        .or_else(|| Some(datasource_type_alias(normalized).to_string()))
                }
            }
            Self::Object(reference) => reference.type_label(datasource_inventory),
        }
    }

    fn inventory_item(
        self,
        datasource_inventory: &'a [DatasourceInventoryItem],
    ) -> Option<&'a DatasourceInventoryItem> {
        match self {
            Self::String(text) => {
                let normalized = text.trim();
                if normalized.is_empty() || is_placeholder_string(normalized) {
                    None
                } else {
                    datasource_inventory.iter().find(|datasource| {
                        datasource.uid == normalized || datasource.name == normalized
                    })
                }
            }
            Self::Object(reference) => reference.inventory_item(datasource_inventory),
        }
    }
}

pub(crate) fn summarize_datasource_ref(reference: &Value) -> Option<String> {
    DatasourceReference::parse(reference)?.summary_label()
}

pub(crate) fn summarize_datasource_uid(reference: &Value) -> Option<String> {
    DatasourceReference::parse(reference)?.uid_label()
}

pub(crate) fn summarize_datasource_name(
    reference: &Value,
    datasource_inventory: &[DatasourceInventoryItem],
) -> Option<String> {
    DatasourceReference::parse(reference)?.name_label(datasource_inventory)
}

pub(crate) fn summarize_datasource_type(
    reference: &Value,
    datasource_inventory: &[DatasourceInventoryItem],
) -> Option<String> {
    DatasourceReference::parse(reference)?.type_label(datasource_inventory)
}

pub(crate) fn resolve_datasource_inventory_item<'a>(
    reference: &'a Value,
    datasource_inventory: &'a [DatasourceInventoryItem],
) -> Option<&'a DatasourceInventoryItem> {
    DatasourceReference::parse(reference)?.inventory_item(datasource_inventory)
}

fn datasource_type_from_reference(reference: &Value) -> Option<String> {
    DatasourceReference::parse(reference)?.type_label(&[])
}

pub(crate) fn summarize_panel_datasource_key(reference: &Value) -> Option<String> {
    if reference.is_null() {
        return None;
    }
    match reference {
        Value::String(text) => {
            let normalized = text.trim();
            if normalized.is_empty() {
                None
            } else {
                Some(normalized.to_string())
            }
        }
        Value::Object(object) => {
            for key in ["uid", "name", "type"] {
                if let Some(value) = object.get(key).and_then(Value::as_str) {
                    let normalized = value.trim();
                    if !normalized.is_empty() && !is_placeholder_string(normalized) {
                        return Some(normalized.to_string());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

pub(crate) fn resolve_query_analyzer_family(context: &QueryExtractionContext<'_>) -> &'static str {
    if let Some(family) = resolve_query_analyzer_family_from_datasource_type(datasource_type_alias(
        context.resolved_datasource_type,
    )) {
        return family;
    }
    for reference in [
        context.target.get("datasource"),
        context.panel.get("datasource"),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(datasource_type) = datasource_type_from_reference(reference) {
            if let Some(family) =
                resolve_query_analyzer_family_from_datasource_type(datasource_type.as_str())
            {
                return family;
            }
        }
    }
    if let Some(family) =
        resolve_query_analyzer_family_from_query_signature(context.query_field, context.query_text)
    {
        return family;
    }
    DATASOURCE_FAMILY_UNKNOWN
}
