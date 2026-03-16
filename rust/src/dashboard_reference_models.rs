//! Typed reference shapes for dashboard inspection and dependency workflows.
//!
//! These structs are intentionally conservative and independent from command
//! execution. Workers can wire them into Rust Rust dashboard analyzers later.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

fn normalize_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasourceReference {
    pub uid: String,
    pub name: String,
    #[serde(default)]
    pub datasource_type: String,
    #[serde(default)]
    pub plugin_id: String,
    #[serde(default)]
    pub org: String,
    #[serde(default)]
    pub org_id: String,
    #[serde(default)]
    pub access: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub source_file: String,
}

impl DatasourceReference {
    pub fn from_value(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        let uid = normalize_text(object.get("uid").or_else(|| object.get("identity")));
        let name = normalize_text(object.get("name"));
        let source_file = normalize_text(object.get("sourcePath"));
        let identity = if !uid.is_empty() { uid } else { name.clone() };
        if identity.is_empty() {
            return None;
        }
        Some(Self {
            uid: identity.clone(),
            name: if !name.is_empty() { name } else { identity },
            datasource_type: normalize_text(object.get("type")),
            plugin_id: normalize_text(object.get("pluginId").or_else(|| object.get("type"))),
            org: normalize_text(object.get("org")),
            org_id: normalize_text(object.get("orgId")),
            access: normalize_text(object.get("access")),
            url: normalize_text(object.get("url")),
            source_file,
        })
    }

    pub fn identity(&self) -> &str {
        if !self.uid.is_empty() {
            &self.uid
        } else {
            &self.name
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardReference {
    pub uid: String,
    pub title: String,
    #[serde(default)]
    pub folder_path: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub org: String,
    #[serde(default)]
    pub org_id: String,
}

impl DashboardReference {
    pub fn from_value(value: &Value) -> Option<Self> {
        let object = value.as_object()?;
        let uid = normalize_text(object.get("uid"));
        let title = normalize_text(object.get("title"));
        let identity = if !uid.is_empty() {
            uid.clone()
        } else {
            title.clone()
        };
        if identity.is_empty() {
            return None;
        }
        Some(Self {
            uid: identity.clone(),
            title: if !title.is_empty() {
                title
            } else {
                identity.clone()
            },
            folder_path: normalize_text(
                object
                    .get("folderPath")
                    .or_else(|| object.get("folder"))
                    .or_else(|| object.get("path")),
            ),
            file: normalize_text(object.get("file")),
            org: normalize_text(object.get("org")),
            org_id: normalize_text(object.get("orgId")),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelReference {
    pub dashboard_uid: String,
    pub panel_id: String,
    #[serde(default)]
    pub ref_id: String,
    #[serde(default)]
    pub panel_type: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardQueryReference {
    pub dashboard_uid: String,
    pub dashboard_title: String,
    pub panel_id: String,
    #[serde(default)]
    pub panel_title: String,
    #[serde(default)]
    pub panel_type: String,
    #[serde(default)]
    pub ref_id: String,
    #[serde(default)]
    pub datasource_uid: String,
    #[serde(default)]
    pub datasource_name: String,
    #[serde(default)]
    pub datasource_type: String,
    #[serde(default)]
    pub datasource_family: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub query_field: String,
    #[serde(default)]
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFeatureSet {
    #[serde(default)]
    pub metrics: Vec<String>,
    #[serde(default)]
    pub measurements: Vec<String>,
    #[serde(default)]
    pub buckets: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
}

impl QueryFeatureSet {
    pub fn blank() -> Self {
        Self {
            metrics: Vec::new(),
            measurements: Vec::new(),
            buckets: Vec::new(),
            labels: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAnalysisRow {
    pub query: DashboardQueryReference,
    pub feature: QueryFeatureSet,
}

#[derive(Debug, Clone)]
pub struct DashboardDependencySummary {
    pub datasource_identity: String,
    pub family: String,
    pub query_count: usize,
    pub dashboard_count: usize,
    pub reference_count: usize,
    pub query_fields: Vec<String>,
}

pub fn normalize_family_name(raw_type: &str) -> String {
    let normalized = raw_type.trim().to_lowercase();
    let aliases = vec![
        ("grafana-prometheus-datasource", "prometheus"),
        ("grafana-loki-datasource", "loki"),
        ("grafana-influxdb-flux-datasource", "flux"),
        ("grafana-influxdb-datasource", "influxdb"),
        ("grafana-mysql-datasource", "mysql"),
        ("grafana-postgresql-datasource", "postgresql"),
    ];
    for (source, target) in aliases {
        if normalized == source {
            return target.to_string();
        }
    }
    if normalized.is_empty() {
        "unknown".to_string()
    } else {
        normalized
    }
}

pub fn build_query_reference_payload(row: &Value) -> Option<DashboardQueryReference> {
    let object = row.as_object()?;
    let dashboard_uid = normalize_text(object.get("dashboardUid"));
    if dashboard_uid.is_empty() {
        return None;
    }
    let dashboard_title = normalize_text(object.get("dashboardTitle"));
    let panel_id = normalize_text(object.get("panelId"));
    let panel_title = normalize_text(object.get("panelTitle"));
    let panel_type = normalize_text(object.get("panelType"));
    let datasource_uid = normalize_text(object.get("datasourceUid"));
    let datasource_name = normalize_text(object.get("datasource"));
    let datasource_type = normalize_text(object.get("datasourceType"));
    let datasource_family = normalize_family_name(&datasource_type);
    let query = normalize_text(object.get("query"));
    Some(DashboardQueryReference {
        dashboard_uid: dashboard_uid.clone(),
        dashboard_title: if dashboard_title.is_empty() {
            dashboard_uid.clone()
        } else {
            dashboard_title
        },
        panel_id: if panel_id.is_empty() {
            "unknown".to_string()
        } else {
            panel_id
        },
        panel_title: panel_title,
        panel_type: panel_type,
        ref_id: normalize_text(object.get("refId")),
        datasource_uid: datasource_uid.clone(),
        datasource_name: if !datasource_name.is_empty() {
            datasource_name
        } else {
            datasource_uid
        },
        datasource_type: datasource_type,
        datasource_family,
        file: normalize_text(object.get("file")),
        query_field: normalize_text(object.get("queryField")),
        query,
    })
}

pub fn dedupe_strings(values: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    let mut seen = BTreeMap::new();
    for item in values {
        if seen.contains_key(item) {
            continue;
        }
        seen.insert(item.clone(), ());
        result.push(item.clone());
    }
    result
}

pub fn build_dependency_lookup(
    datasource_inventory: &[Value],
) -> BTreeMap<String, DatasourceReference> {
    let mut lookup = BTreeMap::new();
    for value in datasource_inventory {
        let Some(reference) = DatasourceReference::from_value(value) else {
            continue;
        };
        let identity = reference.identity().to_string();
        lookup.insert(identity, reference);
    }
    lookup
}
