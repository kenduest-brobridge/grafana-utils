use clap::Parser;
use reqwest::Method;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{
    load_json_object_file, message, resolve_auth_headers, sanitize_path_component, string_field,
    value_as_object, write_json_file, Result,
};
use crate::http::{JsonHttpClient, JsonHttpClientConfig};

pub const DEFAULT_URL: &str = "http://127.0.0.1:3000";
pub const DEFAULT_TIMEOUT: u64 = 30;
pub const DEFAULT_OUTPUT_DIR: &str = "alerts";
pub const RAW_EXPORT_SUBDIR: &str = "raw";
pub const RULES_SUBDIR: &str = "rules";
pub const CONTACT_POINTS_SUBDIR: &str = "contact-points";
pub const MUTE_TIMINGS_SUBDIR: &str = "mute-timings";
pub const POLICIES_SUBDIR: &str = "policies";
pub const TEMPLATES_SUBDIR: &str = "templates";
pub const RULE_KIND: &str = "grafana-alert-rule";
pub const CONTACT_POINT_KIND: &str = "grafana-contact-point";
pub const MUTE_TIMING_KIND: &str = "grafana-mute-timing";
pub const POLICIES_KIND: &str = "grafana-notification-policies";
pub const TEMPLATE_KIND: &str = "grafana-notification-template";
pub const TOOL_API_VERSION: i64 = 1;
pub const TOOL_SCHEMA_VERSION: i64 = 1;
pub const ROOT_INDEX_KIND: &str = "grafana-utils-alert-export-index";

#[derive(Debug, Clone, Parser)]
#[command(
    about = "Export or import Grafana alerting resources.",
    after_help = "Examples:\n\n  Export alerting resources with an API token:\n    export GRAFANA_API_TOKEN='your-token'\n    grafana-alert-utils --url https://grafana.example.com --output-dir ./alerts --overwrite\n\n  Import back into Grafana and update existing resources:\n    grafana-alert-utils --url https://grafana.example.com --import-dir ./alerts/raw --replace-existing\n\n  Import linked alert rules with dashboard and panel remapping:\n    grafana-alert-utils --url https://grafana.example.com --import-dir ./alerts/raw --replace-existing --dashboard-uid-map ./dashboard-map.json --panel-id-map ./panel-map.json"
)]
pub struct AlertCliArgs {
    #[arg(long, default_value = DEFAULT_URL, help = "Grafana base URL.")]
    pub url: String,
    #[arg(
        long = "token",
        visible_alias = "api-token",
        help = "Grafana API token. Preferred flag: --token. Falls back to GRAFANA_API_TOKEN."
    )]
    pub api_token: Option<String>,
    #[arg(
        long = "basic-user",
        visible_alias = "username",
        help = "Grafana Basic auth username. Preferred flag: --basic-user. Falls back to GRAFANA_USERNAME."
    )]
    pub username: Option<String>,
    #[arg(
        long = "basic-password",
        visible_alias = "password",
        help = "Grafana Basic auth password. Preferred flag: --basic-password. Falls back to GRAFANA_PASSWORD."
    )]
    pub password: Option<String>,
    #[arg(
        long,
        default_value_t = false,
        help = "Prompt for the Grafana Basic auth password without echo instead of passing --basic-password on the command line."
    )]
    pub prompt_password: bool,
    #[arg(
        long,
        default_value = DEFAULT_OUTPUT_DIR,
        help = "Directory to write exported alerting resources into. Export writes files under raw/."
    )]
    pub output_dir: PathBuf,
    #[arg(
        long,
        conflicts_with = "diff_dir",
        help = "Import alerting resource JSON from this directory instead of exporting. Point this to the raw/ export directory explicitly."
    )]
    pub import_dir: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "import_dir",
        help = "Compare alerting resource JSON from this directory against Grafana. Point this to the raw/ export directory explicitly."
    )]
    pub diff_dir: Option<PathBuf>,
    #[arg(long, default_value_t = DEFAULT_TIMEOUT, help = "HTTP timeout in seconds.")]
    pub timeout: u64,
    #[arg(
        long,
        default_value_t = false,
        help = "Write rule, contact-point, mute-timing, and template files directly into their resource directories instead of nested subdirectories."
    )]
    pub flat: bool,
    #[arg(long, default_value_t = false, help = "Overwrite existing exported files.")]
    pub overwrite: bool,
    #[arg(long, default_value_t = false, help = "Update existing resources with the same identity instead of failing on import.")]
    pub replace_existing: bool,
    #[arg(long, default_value_t = false, help = "Show whether each import file would create or update resources without changing Grafana.")]
    pub dry_run: bool,
    #[arg(long, help = "JSON file that maps source dashboard UIDs to target dashboard UIDs for linked alert-rule repair during import.")]
    pub dashboard_uid_map: Option<PathBuf>,
    #[arg(long, help = "JSON file that maps source dashboard UID and source panel ID to a target panel ID for linked alert-rule repair during import.")]
    pub panel_id_map: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = false,
        help = "Enable TLS certificate verification. Verification is disabled by default."
    )]
    pub verify_ssl: bool,
}

#[derive(Debug, Clone)]
pub struct AlertAuthContext {
    pub url: String,
    pub timeout: u64,
    pub verify_ssl: bool,
    pub headers: Vec<(String, String)>,
}

pub struct GrafanaAlertClient {
    http: JsonHttpClient,
}

pub fn parse_cli_from<I, T>(iter: I) -> AlertCliArgs
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    AlertCliArgs::parse_from(iter)
}

pub fn build_auth_context(args: &AlertCliArgs) -> Result<AlertAuthContext> {
    Ok(AlertAuthContext {
        url: args.url.clone(),
        timeout: args.timeout,
        verify_ssl: args.verify_ssl,
        headers: resolve_auth_headers(
            args.api_token.as_deref(),
            args.username.as_deref(),
            args.password.as_deref(),
            args.prompt_password,
        )?,
    })
}

impl GrafanaAlertClient {
    pub fn new(context: &AlertAuthContext) -> Result<Self> {
        Ok(Self {
            http: JsonHttpClient::new(JsonHttpClientConfig {
                base_url: context.url.clone(),
                headers: context.headers.clone(),
                timeout_secs: context.timeout,
                verify_ssl: context.verify_ssl,
            })?,
        })
    }

    fn request_json(
        &self,
        method: Method,
        path: &str,
        params: &[(String, String)],
        payload: Option<&Value>,
    ) -> Result<Option<Value>> {
        self.http.request_json(method, path, params, payload)
    }

    pub fn list_alert_rules(&self) -> Result<Vec<Map<String, Value>>> {
        expect_object_list(
            self.request_json(Method::GET, "/api/v1/provisioning/alert-rules", &[], None)?,
            "Unexpected alert-rule list response from Grafana.",
        )
    }

    pub fn search_dashboards(&self, query: &str) -> Result<Vec<Map<String, Value>>> {
        expect_object_list(
            self.request_json(
                Method::GET,
                "/api/search",
                &[
                    ("type".to_string(), "dash-db".to_string()),
                    ("query".to_string(), query.to_string()),
                    ("limit".to_string(), "500".to_string()),
                ],
                None,
            )?,
            "Unexpected dashboard search response from Grafana.",
        )
    }

    pub fn get_dashboard(&self, uid: &str) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::GET,
                &format!("/api/dashboards/uid/{uid}"),
                &[],
                None,
            )?,
            &format!("Unexpected dashboard payload for UID {uid}."),
        )
    }

    pub fn get_alert_rule(&self, uid: &str) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::GET,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                &[],
                None,
            )?,
            &format!("Unexpected alert-rule payload for UID {uid}."),
        )
    }

    pub fn create_alert_rule(&self, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::POST,
                "/api/v1/provisioning/alert-rules",
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected alert-rule create response from Grafana.",
        )
    }

    pub fn update_alert_rule(&self, uid: &str, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/alert-rules/{uid}"),
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected alert-rule update response from Grafana.",
        )
    }

    pub fn list_contact_points(&self) -> Result<Vec<Map<String, Value>>> {
        expect_object_list(
            self.request_json(Method::GET, "/api/v1/provisioning/contact-points", &[], None)?,
            "Unexpected contact-point list response from Grafana.",
        )
    }

    pub fn create_contact_point(&self, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::POST,
                "/api/v1/provisioning/contact-points",
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected contact-point create response from Grafana.",
        )
    }

    pub fn update_contact_point(
        &self,
        uid: &str,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/contact-points/{uid}"),
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected contact-point update response from Grafana.",
        )
    }

    pub fn list_mute_timings(&self) -> Result<Vec<Map<String, Value>>> {
        expect_object_list(
            self.request_json(Method::GET, "/api/v1/provisioning/mute-timings", &[], None)?,
            "Unexpected mute-timing list response from Grafana.",
        )
    }

    pub fn create_mute_timing(&self, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::POST,
                "/api/v1/provisioning/mute-timings",
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected mute-timing create response from Grafana.",
        )
    }

    pub fn update_mute_timing(&self, name: &str, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/mute-timings/{name}"),
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected mute-timing update response from Grafana.",
        )
    }

    pub fn get_notification_policies(&self) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(Method::GET, "/api/v1/provisioning/policies", &[], None)?,
            "Unexpected notification policy response from Grafana.",
        )
    }

    pub fn update_notification_policies(
        &self,
        payload: &Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::PUT,
                "/api/v1/provisioning/policies",
                &[],
                Some(&Value::Object(payload.clone())),
            )?,
            "Unexpected notification policy update response from Grafana.",
        )
    }

    pub fn list_templates(&self) -> Result<Vec<Map<String, Value>>> {
        parse_template_list_response(self.request_json(
            Method::GET,
            "/api/v1/provisioning/templates",
            &[],
            None,
        )?)
    }

    pub fn get_template(&self, name: &str) -> Result<Map<String, Value>> {
        expect_object(
            self.request_json(
                Method::GET,
                &format!("/api/v1/provisioning/templates/{name}"),
                &[],
                None,
            )?,
            &format!("Unexpected template payload for name {name}."),
        )
    }

    pub fn update_template(&self, name: &str, payload: &Map<String, Value>) -> Result<Map<String, Value>> {
        let mut body = payload.clone();
        body.remove("name");
        expect_object(
            self.request_json(
                Method::PUT,
                &format!("/api/v1/provisioning/templates/{name}"),
                &[],
                Some(&Value::Object(body)),
            )?,
            "Unexpected template update response from Grafana.",
        )
    }
}

fn expect_object(value: Option<Value>, error_message: &str) -> Result<Map<String, Value>> {
    match value {
        Some(Value::Object(object)) => Ok(object),
        _ => Err(message(error_message)),
    }
}

fn expect_object_list(value: Option<Value>, error_message: &str) -> Result<Vec<Map<String, Value>>> {
    let array = match value {
        Some(Value::Array(items)) => items,
        _ => return Err(message(error_message)),
    };
    Ok(array
        .into_iter()
        .filter_map(|item| match item {
            Value::Object(object) => Some(object),
            _ => None,
        })
        .collect())
}

fn parse_template_list_response(value: Option<Value>) -> Result<Vec<Map<String, Value>>> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(value) => expect_object_list(Some(value), "Unexpected template list response from Grafana."),
    }
}

pub fn resource_subdir_by_kind() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([
        (RULE_KIND, RULES_SUBDIR),
        (CONTACT_POINT_KIND, CONTACT_POINTS_SUBDIR),
        (MUTE_TIMING_KIND, MUTE_TIMINGS_SUBDIR),
        (POLICIES_KIND, POLICIES_SUBDIR),
        (TEMPLATE_KIND, TEMPLATES_SUBDIR),
    ])
}

pub fn build_rule_output_path(output_dir: &Path, rule: &Map<String, Value>, flat: bool) -> PathBuf {
    let folder_uid = sanitize_path_component(&string_field(rule, "folderUID", "general"));
    let rule_group = sanitize_path_component(&string_field(rule, "ruleGroup", "default"));
    let title = sanitize_path_component(&string_field(rule, "title", "rule"));
    let uid = sanitize_path_component(&string_field(rule, "uid", "unknown"));
    let file_name = format!("{title}__{uid}.json");

    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(folder_uid).join(rule_group).join(file_name)
    }
}

pub fn build_contact_point_output_path(
    output_dir: &Path,
    contact_point: &Map<String, Value>,
    flat: bool,
) -> PathBuf {
    let name = sanitize_path_component(&string_field(contact_point, "name", "contact-point"));
    let uid = sanitize_path_component(&string_field(contact_point, "uid", "unknown"));
    let file_name = format!("{name}__{uid}.json");
    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(&name).join(file_name)
    }
}

pub fn build_mute_timing_output_path(
    output_dir: &Path,
    mute_timing: &Map<String, Value>,
    flat: bool,
) -> PathBuf {
    let name = sanitize_path_component(&string_field(mute_timing, "name", "mute-timing"));
    let file_name = format!("{name}.json");
    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(&name).join(file_name)
    }
}

pub fn build_policies_output_path(output_dir: &Path) -> PathBuf {
    output_dir.join("notification-policies.json")
}

pub fn build_template_output_path(
    output_dir: &Path,
    template: &Map<String, Value>,
    flat: bool,
) -> PathBuf {
    let name = sanitize_path_component(&string_field(template, "name", "template"));
    let file_name = format!("{name}.json");
    if flat {
        output_dir.join(file_name)
    } else {
        output_dir.join(&name).join(file_name)
    }
}

pub fn build_resource_dirs(raw_dir: &Path) -> BTreeMap<&'static str, PathBuf> {
    resource_subdir_by_kind()
        .into_iter()
        .map(|(kind, subdir)| (kind, raw_dir.join(subdir)))
        .collect()
}

pub fn discover_alert_resource_files(import_dir: &Path) -> Result<Vec<PathBuf>> {
    if !import_dir.exists() {
        return Err(message(format!(
            "Import directory does not exist: {}",
            import_dir.display()
        )));
    }
    if !import_dir.is_dir() {
        return Err(message(format!(
            "Import path is not a directory: {}",
            import_dir.display()
        )));
    }
    if import_dir.join(RAW_EXPORT_SUBDIR).is_dir() {
        return Err(message(format!(
            "Import path {} looks like the export root. Point --import-dir at {}.",
            import_dir.display(),
            import_dir.join(RAW_EXPORT_SUBDIR).display()
        )));
    }

    let mut files = Vec::new();
    collect_json_files(import_dir, &mut files)?;
    files.retain(|path| path.file_name().and_then(|value| value.to_str()) != Some("index.json"));
    files.sort();
    if files.is_empty() {
        return Err(message(format!(
            "No alerting resource JSON files found in {}",
            import_dir.display()
        )));
    }
    Ok(files)
}

fn collect_json_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, files)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            files.push(path);
        }
    }
    Ok(())
}

pub fn derive_dashboard_slug(value: &Value) -> String {
    let mut text = value.as_str().unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return String::new();
    }
    if let Some(index) = text.find("/d/") {
        let tail = &text[index + 3..];
        let mut segments = tail.split('/');
        let _uid = segments.next();
        if let Some(slug) = segments.next() {
            return slug
                .split(['?', '#'])
                .next()
                .unwrap_or_default()
                .to_string();
        }
    }
    if text.starts_with('/') {
        text = text.trim_end_matches('/').rsplit('/').next().unwrap_or_default().to_string();
    }
    text
}

pub fn load_string_map(path: Option<&Path>, label: &str) -> Result<BTreeMap<String, String>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let payload = load_json_object_file(path, label)?;
    let object = value_as_object(&payload, &format!("{label} must be a JSON object."))?;
    Ok(object
        .iter()
        .map(|(key, value)| (key.clone(), value_to_string(value)))
        .collect())
}

pub fn load_panel_id_map(path: Option<&Path>) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let payload = load_json_object_file(path, "Panel ID map")?;
    let object = value_as_object(&payload, "Panel ID map must be a JSON object.")?;
    let mut normalized = BTreeMap::new();
    for (dashboard_uid, mapping_value) in object {
        let mapping_object = value_as_object(
            mapping_value,
            "Panel ID map values must be JSON objects keyed by source panel ID.",
        )?;
        normalized.insert(
            dashboard_uid.clone(),
            mapping_object
                .iter()
                .map(|(panel_id, target_panel_id)| {
                    (panel_id.clone(), value_to_string(target_panel_id))
                })
                .collect(),
        );
    }
    Ok(normalized)
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        _ => value.to_string(),
    }
}

pub fn strip_server_managed_fields(kind: &str, payload: &Map<String, Value>) -> Map<String, Value> {
    let managed_fields = match kind {
        RULE_KIND => ["id", "updated", "provenance"].as_slice(),
        CONTACT_POINT_KIND => ["provenance"].as_slice(),
        MUTE_TIMING_KIND => ["version", "provenance"].as_slice(),
        POLICIES_KIND => ["provenance"].as_slice(),
        TEMPLATE_KIND => ["provenance"].as_slice(),
        _ => [].as_slice(),
    };

    payload
        .iter()
        .filter(|(key, _)| !managed_fields.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn build_rule_metadata(rule: &Map<String, Value>) -> Value {
    json!({
        "uid": string_field(rule, "uid", ""),
        "title": string_field(rule, "title", ""),
        "folderUID": string_field(rule, "folderUID", ""),
        "ruleGroup": string_field(rule, "ruleGroup", ""),
    })
}

fn build_contact_point_metadata(contact_point: &Map<String, Value>) -> Value {
    json!({
        "uid": string_field(contact_point, "uid", ""),
        "name": string_field(contact_point, "name", ""),
        "type": string_field(contact_point, "type", ""),
    })
}

fn build_mute_timing_metadata(mute_timing: &Map<String, Value>) -> Value {
    json!({ "name": string_field(mute_timing, "name", "") })
}

fn build_policies_metadata(policies: &Map<String, Value>) -> Value {
    json!({ "receiver": string_field(policies, "receiver", "") })
}

fn build_template_metadata(template: &Map<String, Value>) -> Value {
    json!({ "name": string_field(template, "name", "") })
}

fn build_tool_document(kind: &str, spec: Map<String, Value>, metadata: Value) -> Value {
    json!({
        "schemaVersion": TOOL_SCHEMA_VERSION,
        "apiVersion": TOOL_API_VERSION,
        "kind": kind,
        "metadata": metadata,
        "spec": Value::Object(spec),
    })
}

pub fn build_rule_export_document(rule: &Map<String, Value>) -> Value {
    let mut normalized = strip_server_managed_fields(RULE_KIND, rule);
    let linked_dashboard = normalized.remove("__linkedDashboardMetadata__");
    let mut document = build_tool_document(RULE_KIND, normalized.clone(), build_rule_metadata(&normalized));
    if let Some(Value::Object(linked_dashboard)) = linked_dashboard {
        if let Some(metadata) = document.get_mut("metadata").and_then(Value::as_object_mut) {
            metadata.insert("linkedDashboard".to_string(), Value::Object(linked_dashboard));
        }
    }
    document
}

pub fn build_contact_point_export_document(contact_point: &Map<String, Value>) -> Value {
    let normalized = strip_server_managed_fields(CONTACT_POINT_KIND, contact_point);
    build_tool_document(
        CONTACT_POINT_KIND,
        normalized.clone(),
        build_contact_point_metadata(&normalized),
    )
}

pub fn build_mute_timing_export_document(mute_timing: &Map<String, Value>) -> Value {
    let normalized = strip_server_managed_fields(MUTE_TIMING_KIND, mute_timing);
    build_tool_document(
        MUTE_TIMING_KIND,
        normalized.clone(),
        build_mute_timing_metadata(&normalized),
    )
}

pub fn build_policies_export_document(policies: &Map<String, Value>) -> Value {
    let normalized = strip_server_managed_fields(POLICIES_KIND, policies);
    build_tool_document(POLICIES_KIND, normalized.clone(), build_policies_metadata(&normalized))
}

pub fn build_template_export_document(template: &Map<String, Value>) -> Value {
    let normalized = strip_server_managed_fields(TEMPLATE_KIND, template);
    build_tool_document(TEMPLATE_KIND, normalized.clone(), build_template_metadata(&normalized))
}

pub fn reject_provisioning_export(document: &Map<String, Value>) -> Result<()> {
    if document.contains_key("groups")
        || document.contains_key("contactPoints")
        || document.contains_key("policies")
        || document.contains_key("templates")
    {
        return Err(message(
            "Grafana provisioning export format is not supported for API import. Use files exported by grafana-alert-utils.",
        ));
    }
    Ok(())
}

pub fn detect_document_kind(document: &Map<String, Value>) -> Result<&'static str> {
    if let Some(kind) = document.get("kind").and_then(Value::as_str) {
        if resource_subdir_by_kind().contains_key(kind) {
            return Ok(match kind {
                RULE_KIND => RULE_KIND,
                CONTACT_POINT_KIND => CONTACT_POINT_KIND,
                MUTE_TIMING_KIND => MUTE_TIMING_KIND,
                POLICIES_KIND => POLICIES_KIND,
                TEMPLATE_KIND => TEMPLATE_KIND,
                _ => unreachable!(),
            });
        }
    }

    if document.contains_key("condition") && document.contains_key("data") {
        return Ok(RULE_KIND);
    }
    if document.contains_key("time_intervals") && document.contains_key("name") {
        return Ok(MUTE_TIMING_KIND);
    }
    if document.contains_key("type") && document.contains_key("settings") && document.contains_key("name") {
        return Ok(CONTACT_POINT_KIND);
    }
    if document.contains_key("name") && document.contains_key("template") {
        return Ok(TEMPLATE_KIND);
    }
    if document.contains_key("receiver") || document.contains_key("routes") || document.contains_key("group_by")
    {
        return Ok(POLICIES_KIND);
    }

    Err(message(
        "Cannot determine alerting resource kind from import document.",
    ))
}

fn extract_tool_spec(document: &Map<String, Value>, expected_kind: &str) -> Result<Map<String, Value>> {
    let spec = if document.get("kind").and_then(Value::as_str) == Some(expected_kind) {
        if let Some(api_version) = document.get("apiVersion").and_then(Value::as_i64) {
            if api_version != TOOL_API_VERSION {
                return Err(message(format!(
                    "Unsupported {expected_kind} export version: {:?}",
                    document.get("apiVersion")
                )));
            }
        }
        if let Some(schema_version) = document.get("schemaVersion").and_then(Value::as_i64) {
            if schema_version != TOOL_SCHEMA_VERSION {
                return Err(message(format!(
                    "Unsupported {expected_kind} schema version: {:?}",
                    document.get("schemaVersion")
                )));
            }
        }
        if document.get("apiVersion").is_none() && document.get("schemaVersion").is_none() {
            return Err(message(format!(
                "Unsupported {expected_kind} export version: {:?}",
                document.get("apiVersion")
            )));
        }
        document.get("spec").cloned().ok_or_else(|| {
            message(format!(
                "{expected_kind} import document is missing a valid spec object."
            ))
        })?
    } else {
        Value::Object(document.clone())
    };

    match spec {
        Value::Object(object) => Ok(object),
        _ => Err(message(format!(
            "{expected_kind} import document is missing a valid spec object."
        ))),
    }
}

pub fn build_rule_import_payload(document: &Map<String, Value>) -> Result<Map<String, Value>> {
    reject_provisioning_export(document)?;
    let payload = strip_server_managed_fields(RULE_KIND, &extract_tool_spec(document, RULE_KIND)?);
    for field in ["title", "folderUID", "ruleGroup", "condition", "data"] {
        if !payload.contains_key(field) {
            return Err(message(format!(
                "Alert-rule import document is missing required fields: {field}"
            )));
        }
    }
    if !payload.get("data").map(Value::is_array).unwrap_or(false) {
        return Err(message("Alert-rule field 'data' must be a list."));
    }
    Ok(payload)
}

pub fn build_contact_point_import_payload(document: &Map<String, Value>) -> Result<Map<String, Value>> {
    reject_provisioning_export(document)?;
    let payload =
        strip_server_managed_fields(CONTACT_POINT_KIND, &extract_tool_spec(document, CONTACT_POINT_KIND)?);
    for field in ["name", "type", "settings"] {
        if !payload.contains_key(field) {
            return Err(message(format!(
                "Contact-point import document is missing required fields: {field}"
            )));
        }
    }
    if !payload.get("settings").map(Value::is_object).unwrap_or(false) {
        return Err(message("Contact-point field 'settings' must be an object."));
    }
    Ok(payload)
}

pub fn build_mute_timing_import_payload(document: &Map<String, Value>) -> Result<Map<String, Value>> {
    reject_provisioning_export(document)?;
    let payload =
        strip_server_managed_fields(MUTE_TIMING_KIND, &extract_tool_spec(document, MUTE_TIMING_KIND)?);
    for field in ["name", "time_intervals"] {
        if !payload.contains_key(field) {
            return Err(message(format!(
                "Mute-timing import document is missing required fields: {field}"
            )));
        }
    }
    if !payload
        .get("time_intervals")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        return Err(message("Mute-timing field 'time_intervals' must be a list."));
    }
    Ok(payload)
}

pub fn build_policies_import_payload(document: &Map<String, Value>) -> Result<Map<String, Value>> {
    reject_provisioning_export(document)?;
    extract_tool_spec(document, POLICIES_KIND)
}

pub fn build_template_import_payload(document: &Map<String, Value>) -> Result<Map<String, Value>> {
    reject_provisioning_export(document)?;
    let payload = strip_server_managed_fields(TEMPLATE_KIND, &extract_tool_spec(document, TEMPLATE_KIND)?);
    for field in ["name", "template"] {
        if !payload.contains_key(field) {
            return Err(message(format!(
                "Template import document is missing required fields: {field}"
            )));
        }
    }
    Ok(payload)
}

pub fn build_import_operation(document: &Value) -> Result<(String, Map<String, Value>)> {
    let object = value_as_object(document, "Alerting import document must be a JSON object.")?;
    let kind = detect_document_kind(object)?;
    let payload = match kind {
        RULE_KIND => build_rule_import_payload(object)?,
        CONTACT_POINT_KIND => build_contact_point_import_payload(object)?,
        MUTE_TIMING_KIND => build_mute_timing_import_payload(object)?,
        POLICIES_KIND => build_policies_import_payload(object)?,
        TEMPLATE_KIND => build_template_import_payload(object)?,
        _ => unreachable!(),
    };
    Ok((kind.to_string(), payload))
}

pub fn build_empty_root_index() -> Map<String, Value> {
    [
        (
            "schemaVersion".to_string(),
            Value::Number(TOOL_SCHEMA_VERSION.into()),
        ),
        (
            "apiVersion".to_string(),
            Value::Number(TOOL_API_VERSION.into()),
        ),
        ("kind".to_string(), Value::String(ROOT_INDEX_KIND.to_string())),
        (RULES_SUBDIR.to_string(), Value::Array(Vec::new())),
        (CONTACT_POINTS_SUBDIR.to_string(), Value::Array(Vec::new())),
        (MUTE_TIMINGS_SUBDIR.to_string(), Value::Array(Vec::new())),
        (POLICIES_SUBDIR.to_string(), Value::Array(Vec::new())),
        (TEMPLATES_SUBDIR.to_string(), Value::Array(Vec::new())),
    ]
    .into_iter()
    .collect()
}

fn build_compare_document(kind: &str, payload: &Map<String, Value>) -> Value {
    Value::Object(Map::from_iter([
        ("kind".to_string(), Value::String(kind.to_string())),
        ("spec".to_string(), Value::Object(payload.clone())),
    ]))
}

fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_value).collect()),
        Value::Object(object) => {
            let sorted = object
                .iter()
                .map(|(key, item)| (key.clone(), canonicalize_value(item)))
                .collect::<BTreeMap<_, _>>();
            Value::Object(Map::from_iter(sorted.into_iter()))
        }
        _ => value.clone(),
    }
}

fn serialize_compare_document(document: &Value) -> Result<String> {
    Ok(serde_json::to_string(&canonicalize_value(document))?)
}

fn build_compare_diff_text(
    remote_compare: &Value,
    local_compare: &Value,
    identity: &str,
    resource_file: &Path,
) -> Result<String> {
    let remote_pretty = serde_json::to_string_pretty(&canonicalize_value(remote_compare))?;
    let local_pretty = serde_json::to_string_pretty(&canonicalize_value(local_compare))?;
    let mut text = String::new();
    let _ = writeln!(&mut text, "--- remote:{identity}");
    let _ = writeln!(&mut text, "+++ {}", resource_file.display());
    for line in remote_pretty.lines() {
        let _ = writeln!(&mut text, "-{line}");
    }
    for line in local_pretty.lines() {
        let _ = writeln!(&mut text, "+{line}");
    }
    Ok(text)
}

fn build_resource_identity(kind: &str, payload: &Map<String, Value>) -> String {
    match kind {
        RULE_KIND => string_field(payload, "uid", "unknown"),
        CONTACT_POINT_KIND => {
            let uid = string_field(payload, "uid", "");
            if uid.is_empty() {
                string_field(payload, "name", "unknown")
            } else {
                uid
            }
        }
        MUTE_TIMING_KIND | TEMPLATE_KIND => string_field(payload, "name", "unknown"),
        POLICIES_KIND => string_field(payload, "receiver", "root"),
        _ => "unknown".to_string(),
    }
}

fn append_root_index_item(root_index: &mut Map<String, Value>, subdir: &str, item: Value) {
    let items = root_index
        .entry(subdir.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(entries) = items {
        entries.push(item);
    }
}

fn write_resource_indexes(
    resource_dirs: &BTreeMap<&'static str, PathBuf>,
    root_index: &Map<String, Value>,
) -> Result<()> {
    for (kind, subdir) in resource_subdir_by_kind() {
        let Some(Value::Array(items)) = root_index.get(subdir) else {
            continue;
        };
        write_json_file(
            &resource_dirs[kind].join("index.json"),
            &Value::Array(items.clone()),
            true,
        )?;
    }
    Ok(())
}

fn format_export_summary(root_index: &Map<String, Value>, index_path: &Path) -> String {
    format!(
        "Exported {} alert rules, {} contact points, {} mute timings, {} notification policy documents, {} templates. Root index: {}",
        root_index.get(RULES_SUBDIR).and_then(Value::as_array).map(Vec::len).unwrap_or(0),
        root_index
            .get(CONTACT_POINTS_SUBDIR)
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        root_index
            .get(MUTE_TIMINGS_SUBDIR)
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        root_index
            .get(POLICIES_SUBDIR)
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        root_index
            .get(TEMPLATES_SUBDIR)
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
        index_path.display(),
    )
}

fn get_rule_linkage(rule: &Map<String, Value>) -> Option<BTreeMap<String, String>> {
    let annotations = rule.get("annotations")?.as_object()?;
    let dashboard_uid = annotations
        .get("__dashboardUid__")
        .map(value_to_string)
        .unwrap_or_default()
        .trim()
        .to_string();
    if dashboard_uid.is_empty() {
        return None;
    }
    let mut linkage = BTreeMap::from([("dashboardUid".to_string(), dashboard_uid)]);
    if let Some(panel_id) = annotations.get("__panelId__") {
        linkage.insert("panelId".to_string(), value_to_string(panel_id));
    }
    Some(linkage)
}

fn find_panel_by_id(panels: Option<&Vec<Value>>, panel_id: &str) -> Option<Map<String, Value>> {
    let panels = panels?;
    for panel in panels {
        let object = panel.as_object()?;
        if object.get("id").map(value_to_string).as_deref() == Some(panel_id) {
            return Some(object.clone());
        }
        let nested = object.get("panels").and_then(Value::as_array);
        if let Some(found) = find_panel_by_id(nested, panel_id) {
            return Some(found);
        }
    }
    None
}

fn build_linked_dashboard_metadata(
    client: &GrafanaAlertClient,
    rule: &Map<String, Value>,
) -> Result<Option<Map<String, Value>>> {
    let Some(linkage) = get_rule_linkage(rule) else {
        return Ok(None);
    };

    let dashboard_uid = linkage.get("dashboardUid").cloned().unwrap_or_default();
    let mut metadata = Map::new();
    for (key, value) in linkage {
        metadata.insert(key, Value::String(value));
    }

    let dashboard_payload = match client.get_dashboard(&dashboard_uid) {
        Ok(payload) => payload,
        Err(error) if error.status_code() == Some(404) => return Ok(Some(metadata)),
        Err(error) => return Err(error),
    };

    if let Some(dashboard) = dashboard_payload.get("dashboard").and_then(Value::as_object) {
        metadata.insert(
            "dashboardTitle".to_string(),
            Value::String(string_field(dashboard, "title", "")),
        );
        if let Some(panel_id) = metadata.get("panelId").and_then(Value::as_str) {
            if let Some(panel) = find_panel_by_id(dashboard.get("panels").and_then(Value::as_array), panel_id) {
                metadata.insert(
                    "panelTitle".to_string(),
                    Value::String(string_field(&panel, "title", "")),
                );
                metadata.insert(
                    "panelType".to_string(),
                    Value::String(string_field(&panel, "type", "")),
                );
            }
        }
    }

    if let Some(meta) = dashboard_payload.get("meta").and_then(Value::as_object) {
        metadata.insert(
            "folderTitle".to_string(),
            Value::String(string_field(meta, "folderTitle", "")),
        );
        metadata.insert(
            "folderUid".to_string(),
            Value::String(string_field(meta, "folderUid", "")),
        );
        let slug_source = meta
            .get("url")
            .or_else(|| meta.get("slug"))
            .cloned()
            .unwrap_or(Value::String(String::new()));
        metadata.insert(
            "dashboardSlug".to_string(),
            Value::String(derive_dashboard_slug(&slug_source)),
        );
    }

    Ok(Some(metadata))
}

fn filter_dashboard_search_matches(
    candidates: Vec<Map<String, Value>>,
    linked_dashboard: &Map<String, Value>,
) -> Vec<Map<String, Value>> {
    let dashboard_title = string_field(linked_dashboard, "dashboardTitle", "");
    let mut filtered: Vec<Map<String, Value>> = candidates
        .into_iter()
        .filter(|item| string_field(item, "title", "") == dashboard_title)
        .collect();

    let folder_title = string_field(linked_dashboard, "folderTitle", "");
    if !folder_title.is_empty() {
        let folder_matches: Vec<Map<String, Value>> = filtered
            .iter()
            .filter(|item| string_field(item, "folderTitle", "") == folder_title)
            .cloned()
            .collect();
        if !folder_matches.is_empty() {
            filtered = folder_matches;
        }
    }

    let slug = derive_dashboard_slug(linked_dashboard.get("dashboardSlug").unwrap_or(&Value::Null));
    if !slug.is_empty() {
        let slug_matches: Vec<Map<String, Value>> = filtered
            .iter()
            .filter(|item| {
                derive_dashboard_slug(item.get("url").or_else(|| item.get("slug")).unwrap_or(&Value::Null))
                    == slug
            })
            .cloned()
            .collect();
        if !slug_matches.is_empty() {
            filtered = slug_matches;
        }
    }

    filtered
}

fn resolve_dashboard_uid_fallback(
    client: &GrafanaAlertClient,
    linked_dashboard: &Map<String, Value>,
) -> Result<String> {
    let dashboard_title = string_field(linked_dashboard, "dashboardTitle", "");
    if dashboard_title.is_empty() {
        return Err(message(
            "Alert rule references a dashboard UID that does not exist on the target Grafana, and the export file does not include dashboard title metadata for fallback matching.",
        ));
    }

    let filtered = filter_dashboard_search_matches(client.search_dashboards(&dashboard_title)?, linked_dashboard);
    if filtered.len() == 1 {
        let resolved_uid = string_field(&filtered[0], "uid", "");
        if !resolved_uid.is_empty() {
            return Ok(resolved_uid);
        }
    }

    let folder_title = string_field(linked_dashboard, "folderTitle", "");
    let slug = derive_dashboard_slug(linked_dashboard.get("dashboardSlug").unwrap_or(&Value::Null));
    if filtered.is_empty() {
        return Err(message(format!(
            "Cannot resolve linked dashboard for alert rule. No dashboard matched title={dashboard_title:?}, folderTitle={folder_title:?}, slug={slug:?}.",
        )));
    }

    Err(message(format!(
        "Cannot resolve linked dashboard for alert rule. Multiple dashboards matched title={dashboard_title:?}, folderTitle={folder_title:?}, slug={slug:?}.",
    )))
}

fn rewrite_rule_dashboard_linkage(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    document: &Value,
    dashboard_uid_map: &BTreeMap<String, String>,
    panel_id_map: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<Map<String, Value>> {
    let Some(linkage) = get_rule_linkage(payload) else {
        return Ok(payload.clone());
    };

    let source_dashboard_uid = linkage.get("dashboardUid").cloned().unwrap_or_default();
    let source_panel_id = linkage.get("panelId").cloned().unwrap_or_default();
    let dashboard_uid = dashboard_uid_map
        .get(&source_dashboard_uid)
        .cloned()
        .unwrap_or(source_dashboard_uid.clone());
    let mapped_panel_id = panel_id_map
        .get(&source_dashboard_uid)
        .and_then(|mapping| mapping.get(&source_panel_id))
        .cloned();

    let mut normalized = payload.clone();
    let annotations = normalized
        .entry("annotations".to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| message("Alert-rule annotations must be an object."))?;
    annotations.insert("__dashboardUid__".to_string(), Value::String(dashboard_uid.clone()));
    if let Some(panel_id) = mapped_panel_id {
        annotations.insert("__panelId__".to_string(), Value::String(panel_id));
    }

    match client.get_dashboard(&dashboard_uid) {
        Ok(_) => return Ok(normalized),
        Err(error) if error.status_code() != Some(404) => return Err(error),
        Err(_) => {}
    }

    let linked_dashboard = document
        .get("metadata")
        .and_then(Value::as_object)
        .and_then(|metadata| metadata.get("linkedDashboard"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            message(format!(
                "Alert rule references dashboard UID {dashboard_uid:?}, but that dashboard does not exist on the target Grafana and the export file has no linked dashboard metadata for fallback matching.",
            ))
        })?;
    let replacement_uid = resolve_dashboard_uid_fallback(client, linked_dashboard)?;
    annotations.insert("__dashboardUid__".to_string(), Value::String(replacement_uid));
    Ok(normalized)
}

fn export_alerting_resources(args: &AlertCliArgs) -> Result<()> {
    let client = GrafanaAlertClient::new(&build_auth_context(args)?)?;
    let output_dir = args.output_dir.clone();
    let raw_dir = output_dir.join(RAW_EXPORT_SUBDIR);
    fs::create_dir_all(&raw_dir)?;

    let resource_dirs = build_resource_dirs(&raw_dir);
    for path in resource_dirs.values() {
        fs::create_dir_all(path)?;
    }

    let rules = client.list_alert_rules()?;
    let contact_points = client.list_contact_points()?;
    let mute_timings = client.list_mute_timings()?;
    let policies = client.get_notification_policies()?;
    let templates = client.list_templates()?;

    let mut root_index = build_empty_root_index();

    for rule in rules {
        let mut normalized_rule = rule.clone();
        if let Some(linked_dashboard) = build_linked_dashboard_metadata(&client, &rule)? {
            normalized_rule.insert(
                "__linkedDashboardMetadata__".to_string(),
                Value::Object(linked_dashboard),
            );
        }
        let document = build_rule_export_document(&normalized_rule);
        let spec = document["spec"].as_object().ok_or_else(|| message("Rule export spec must be an object."))?;
        let output_path = build_rule_output_path(&resource_dirs[RULE_KIND], spec, args.flat);
        write_json_file(&output_path, &document, args.overwrite)?;
        append_root_index_item(
            &mut root_index,
            RULES_SUBDIR,
            json!({
                "kind": RULE_KIND,
                "uid": string_field(spec, "uid", ""),
                "title": string_field(spec, "title", ""),
                "folderUID": string_field(spec, "folderUID", ""),
                "ruleGroup": string_field(spec, "ruleGroup", ""),
                "path": output_path.to_string_lossy(),
            }),
        );
        println!("Exported alert rule {} -> {}", string_field(spec, "uid", "unknown"), output_path.display());
    }

    for contact_point in contact_points {
        let document = build_contact_point_export_document(&contact_point);
        let spec = document["spec"].as_object().ok_or_else(|| message("Contact-point export spec must be an object."))?;
        let output_path = build_contact_point_output_path(&resource_dirs[CONTACT_POINT_KIND], spec, args.flat);
        write_json_file(&output_path, &document, args.overwrite)?;
        append_root_index_item(
            &mut root_index,
            CONTACT_POINTS_SUBDIR,
            json!({
                "kind": CONTACT_POINT_KIND,
                "uid": string_field(spec, "uid", ""),
                "name": string_field(spec, "name", ""),
                "type": string_field(spec, "type", ""),
                "path": output_path.to_string_lossy(),
            }),
        );
        println!("Exported contact point {} -> {}", string_field(spec, "uid", "unknown"), output_path.display());
    }

    for mute_timing in mute_timings {
        let document = build_mute_timing_export_document(&mute_timing);
        let spec = document["spec"].as_object().ok_or_else(|| message("Mute-timing export spec must be an object."))?;
        let output_path = build_mute_timing_output_path(&resource_dirs[MUTE_TIMING_KIND], spec, args.flat);
        write_json_file(&output_path, &document, args.overwrite)?;
        append_root_index_item(
            &mut root_index,
            MUTE_TIMINGS_SUBDIR,
            json!({
                "kind": MUTE_TIMING_KIND,
                "name": string_field(spec, "name", ""),
                "path": output_path.to_string_lossy(),
            }),
        );
        println!("Exported mute timing {} -> {}", string_field(spec, "name", "unknown"), output_path.display());
    }

    let policies_document = build_policies_export_document(&policies);
    let policies_path = build_policies_output_path(&resource_dirs[POLICIES_KIND]);
    write_json_file(&policies_path, &policies_document, args.overwrite)?;
    append_root_index_item(
        &mut root_index,
        POLICIES_SUBDIR,
        json!({
            "kind": POLICIES_KIND,
            "receiver": policies_document["spec"]["receiver"],
            "path": policies_path.to_string_lossy(),
        }),
    );
    println!(
        "Exported notification policies {} -> {}",
        policies_document["spec"]["receiver"].as_str().unwrap_or("unknown"),
        policies_path.display()
    );

    for template in templates {
        let document = build_template_export_document(&template);
        let spec = document["spec"].as_object().ok_or_else(|| message("Template export spec must be an object."))?;
        let output_path = build_template_output_path(&resource_dirs[TEMPLATE_KIND], spec, args.flat);
        write_json_file(&output_path, &document, args.overwrite)?;
        append_root_index_item(
            &mut root_index,
            TEMPLATES_SUBDIR,
            json!({
                "kind": TEMPLATE_KIND,
                "name": string_field(spec, "name", ""),
                "path": output_path.to_string_lossy(),
            }),
        );
        println!("Exported template {} -> {}", string_field(spec, "name", "unknown"), output_path.display());
    }

    write_resource_indexes(&resource_dirs, &root_index)?;
    let index_path = output_dir.join("index.json");
    write_json_file(&index_path, &Value::Object(root_index.clone()), true)?;
    println!("{}", format_export_summary(&root_index, &index_path));
    Ok(())
}

fn count_policy_documents(kind: &str, policies_seen: usize) -> Result<usize> {
    if kind != POLICIES_KIND {
        return Ok(policies_seen);
    }
    let next = policies_seen + 1;
    if next > 1 {
        return Err(message(
            "Multiple notification policy documents found in import set. Import only one policy tree at a time.",
        ));
    }
    Ok(next)
}

fn prepare_import_payload_for_target(
    client: &GrafanaAlertClient,
    kind: &str,
    payload: &Map<String, Value>,
    document: &Value,
    dashboard_uid_map: &BTreeMap<String, String>,
    panel_id_map: &BTreeMap<String, BTreeMap<String, String>>,
) -> Result<Map<String, Value>> {
    if kind == RULE_KIND {
        return rewrite_rule_dashboard_linkage(
            client,
            payload,
            document,
            dashboard_uid_map,
            panel_id_map,
        );
    }
    Ok(payload.clone())
}

fn determine_rule_import_action(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str> {
    let uid = string_field(payload, "uid", "");
    if uid.is_empty() {
        return Ok("would-create");
    }
    match client.get_alert_rule(&uid) {
        Ok(_) if replace_existing => Ok("would-update"),
        Ok(_) => Ok("would-fail-existing"),
        Err(error) if error.status_code() == Some(404) => Ok("would-create"),
        Err(error) => Err(error),
    }
}

fn determine_contact_point_import_action(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str> {
    let uid = string_field(payload, "uid", "");
    let exists = client
        .list_contact_points()?
        .into_iter()
        .any(|item| string_field(&item, "uid", "") == uid);
    if exists {
        if replace_existing {
            Ok("would-update")
        } else {
            Ok("would-fail-existing")
        }
    } else {
        Ok("would-create")
    }
}

fn determine_mute_timing_import_action(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str> {
    let name = string_field(payload, "name", "");
    let exists = client
        .list_mute_timings()?
        .into_iter()
        .any(|item| string_field(&item, "name", "") == name);
    if exists {
        if replace_existing {
            Ok("would-update")
        } else {
            Ok("would-fail-existing")
        }
    } else {
        Ok("would-create")
    }
}

fn determine_template_import_action(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str> {
    let name = string_field(payload, "name", "");
    let exists = client
        .list_templates()?
        .into_iter()
        .any(|item| string_field(&item, "name", "") == name);
    if exists {
        if replace_existing {
            Ok("would-update")
        } else {
            Ok("would-fail-existing")
        }
    } else {
        Ok("would-create")
    }
}

fn determine_import_action(
    client: &GrafanaAlertClient,
    kind: &str,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<&'static str> {
    match kind {
        RULE_KIND => determine_rule_import_action(client, payload, replace_existing),
        CONTACT_POINT_KIND => determine_contact_point_import_action(client, payload, replace_existing),
        MUTE_TIMING_KIND => determine_mute_timing_import_action(client, payload, replace_existing),
        TEMPLATE_KIND => determine_template_import_action(client, payload, replace_existing),
        POLICIES_KIND => Ok("would-update"),
        _ => unreachable!(),
    }
}

fn fetch_live_compare_document(
    client: &GrafanaAlertClient,
    kind: &str,
    payload: &Map<String, Value>,
) -> Result<Option<Value>> {
    match kind {
        RULE_KIND => {
            let uid = string_field(payload, "uid", "");
            if uid.is_empty() {
                return Ok(None);
            }
            match client.get_alert_rule(&uid) {
                Ok(remote) => Ok(Some(build_compare_document(kind, &strip_server_managed_fields(kind, &remote)))),
                Err(error) if error.status_code() == Some(404) => Ok(None),
                Err(error) => Err(error),
            }
        }
        CONTACT_POINT_KIND => {
            let uid = string_field(payload, "uid", "");
            let remote = client
                .list_contact_points()?
                .into_iter()
                .find(|item| string_field(item, "uid", "") == uid);
            Ok(remote.map(|item| build_compare_document(kind, &strip_server_managed_fields(kind, &item))))
        }
        MUTE_TIMING_KIND => {
            let name = string_field(payload, "name", "");
            let remote = client
                .list_mute_timings()?
                .into_iter()
                .find(|item| string_field(item, "name", "") == name);
            Ok(remote.map(|item| build_compare_document(kind, &strip_server_managed_fields(kind, &item))))
        }
        TEMPLATE_KIND => {
            let name = string_field(payload, "name", "");
            match client.get_template(&name) {
                Ok(remote) => Ok(Some(build_compare_document(kind, &strip_server_managed_fields(kind, &remote)))),
                Err(error) if error.status_code() == Some(404) => Ok(None),
                Err(error) => Err(error),
            }
        }
        POLICIES_KIND => {
            let remote = client.get_notification_policies()?;
            Ok(Some(build_compare_document(kind, &strip_server_managed_fields(kind, &remote))))
        }
        _ => unreachable!(),
    }
}

fn import_rule_document(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<(String, String)> {
    let uid = string_field(payload, "uid", "");
    if replace_existing && !uid.is_empty() {
        match client.get_alert_rule(&uid) {
            Ok(_) => {
                let result = client.update_alert_rule(&uid, payload)?;
                return Ok(("updated".to_string(), string_field(&result, "uid", &uid)));
            }
            Err(error) if error.status_code() == Some(404) => {}
            Err(error) => return Err(error),
        }
    }
    let result = client.create_alert_rule(payload)?;
    Ok(("created".to_string(), string_field(&result, "uid", &uid)))
}

fn import_contact_point_document(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<(String, String)> {
    let uid = string_field(payload, "uid", "");
    if replace_existing && !uid.is_empty() {
        let existing: Vec<String> = client
            .list_contact_points()?
            .into_iter()
            .map(|item| string_field(&item, "uid", ""))
            .collect();
        if existing.iter().any(|item| item == &uid) {
            let result = client.update_contact_point(&uid, payload)?;
            return Ok(("updated".to_string(), string_field(&result, "uid", &uid)));
        }
    }
    let result = client.create_contact_point(payload)?;
    Ok(("created".to_string(), string_field(&result, "uid", &uid)))
}

fn import_mute_timing_document(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<(String, String)> {
    let name = string_field(payload, "name", "");
    if replace_existing && !name.is_empty() {
        let existing: Vec<String> = client
            .list_mute_timings()?
            .into_iter()
            .map(|item| string_field(&item, "name", ""))
            .collect();
        if existing.iter().any(|item| item == &name) {
            let result = client.update_mute_timing(&name, payload)?;
            return Ok(("updated".to_string(), string_field(&result, "name", &name)));
        }
    }
    let result = client.create_mute_timing(payload)?;
    Ok(("created".to_string(), string_field(&result, "name", &name)))
}

fn import_template_document(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
    replace_existing: bool,
) -> Result<(String, String)> {
    let name = string_field(payload, "name", "");
    let existing_templates = client.list_templates()?;
    let exists = existing_templates
        .iter()
        .any(|item| string_field(item, "name", "") == name);
    if exists && !replace_existing {
        return Err(message(format!(
            "Template {name:?} already exists. Use --replace-existing."
        )));
    }

    let mut template_payload = payload.clone();
    if exists {
        let current = client.get_template(&name)?;
        template_payload.insert(
            "version".to_string(),
            Value::String(string_field(&current, "version", "")),
        );
    } else {
        template_payload.insert("version".to_string(), Value::String(String::new()));
    }

    let result = client.update_template(&name, &template_payload)?;
    Ok(((if exists { "updated" } else { "created" }).to_string(), string_field(&result, "name", &name)))
}

fn import_policies_document(
    client: &GrafanaAlertClient,
    payload: &Map<String, Value>,
) -> Result<(String, String)> {
    client.update_notification_policies(payload)?;
    Ok(("updated".to_string(), string_field(payload, "receiver", "root")))
}

fn import_resource_document(
    client: &GrafanaAlertClient,
    kind: &str,
    payload: &Map<String, Value>,
    args: &AlertCliArgs,
) -> Result<(String, String)> {
    match kind {
        RULE_KIND => import_rule_document(client, payload, args.replace_existing),
        CONTACT_POINT_KIND => import_contact_point_document(client, payload, args.replace_existing),
        MUTE_TIMING_KIND => import_mute_timing_document(client, payload, args.replace_existing),
        TEMPLATE_KIND => import_template_document(client, payload, args.replace_existing),
        POLICIES_KIND => import_policies_document(client, payload),
        _ => unreachable!(),
    }
}

fn import_alerting_resources(args: &AlertCliArgs) -> Result<()> {
    let client = GrafanaAlertClient::new(&build_auth_context(args)?)?;
    let import_dir = args
        .import_dir
        .as_ref()
        .ok_or_else(|| message("Import directory is required for alerting import."))?;
    let resource_files = discover_alert_resource_files(import_dir)?;
    let dashboard_uid_map = load_string_map(args.dashboard_uid_map.as_deref(), "Dashboard UID map")?;
    let panel_id_map = load_panel_id_map(args.panel_id_map.as_deref())?;
    let mut policies_seen = 0usize;

    for resource_file in &resource_files {
        let document = load_json_object_file(resource_file, "Alerting resource")?;
        let (kind, payload) = build_import_operation(&document)?;
        let payload = prepare_import_payload_for_target(
            &client,
            &kind,
            &payload,
            &document,
            &dashboard_uid_map,
            &panel_id_map,
        )?;
        policies_seen = count_policy_documents(&kind, policies_seen)?;
        let identity = build_resource_identity(&kind, &payload);
        if args.dry_run {
            let action = determine_import_action(&client, &kind, &payload, args.replace_existing)?;
            println!(
                "Dry-run {} -> kind={} id={} action={}",
                resource_file.display(),
                kind,
                identity,
                action
            );
            continue;
        }

        let (action, identity) = import_resource_document(&client, &kind, &payload, args)?;
        println!(
            "Imported {} -> kind={} id={} action={}",
            resource_file.display(),
            kind,
            identity,
            action
        );
    }

    if args.dry_run {
        println!(
            "Dry-run checked {} alerting resource files from {}",
            resource_files.len(),
            import_dir.display()
        );
    } else {
        println!(
            "Imported {} alerting resource files from {}",
            resource_files.len(),
            import_dir.display()
        );
    }
    Ok(())
}

fn diff_alerting_resources(args: &AlertCliArgs) -> Result<()> {
    let client = GrafanaAlertClient::new(&build_auth_context(args)?)?;
    let diff_dir = args
        .diff_dir
        .as_ref()
        .ok_or_else(|| message("Diff directory is required for alerting diff."))?;
    let resource_files = discover_alert_resource_files(diff_dir)?;
    let dashboard_uid_map = load_string_map(args.dashboard_uid_map.as_deref(), "Dashboard UID map")?;
    let panel_id_map = load_panel_id_map(args.panel_id_map.as_deref())?;
    let mut policies_seen = 0usize;
    let mut differences = 0usize;

    for resource_file in &resource_files {
        let document = load_json_object_file(resource_file, "Alerting resource")?;
        let (kind, payload) = build_import_operation(&document)?;
        let payload = prepare_import_payload_for_target(
            &client,
            &kind,
            &payload,
            &document,
            &dashboard_uid_map,
            &panel_id_map,
        )?;
        policies_seen = count_policy_documents(&kind, policies_seen)?;
        let identity = build_resource_identity(&kind, &payload);
        let local_compare = build_compare_document(&kind, &payload);
        let remote_compare = fetch_live_compare_document(&client, &kind, &payload)?;

        if let Some(remote_compare) = remote_compare {
            if serialize_compare_document(&local_compare)? == serialize_compare_document(&remote_compare)? {
                println!(
                    "Diff same {} -> kind={} id={}",
                    resource_file.display(),
                    kind,
                    identity
                );
                continue;
            }

            println!(
                "Diff different {} -> kind={} id={}",
                resource_file.display(),
                kind,
                identity
            );
            print!(
                "{}",
                build_compare_diff_text(&remote_compare, &local_compare, &identity, resource_file)?
            );
            differences += 1;
            continue;
        }

        println!(
            "Diff missing-remote {} -> kind={} id={}",
            resource_file.display(),
            kind,
            identity
        );
        print!(
            "{}",
            build_compare_diff_text(&json!({}), &local_compare, &identity, resource_file)?
        );
        differences += 1;
    }

    if differences > 0 {
        return Err(message(format!(
            "Found {differences} alerting differences across {} files.",
            resource_files.len()
        )));
    }

    println!("No alerting differences across {} files.", resource_files.len());
    Ok(())
}

pub fn run_alert_cli(args: AlertCliArgs) -> Result<()> {
    if args.import_dir.is_some() {
        return import_alerting_resources(&args);
    }
    if args.diff_dir.is_some() {
        return diff_alerting_resources(&args);
    }
    export_alerting_resources(&args)
}

#[cfg(test)]
#[path = "alert_rust_tests.rs"]
mod alert_rust_tests;
